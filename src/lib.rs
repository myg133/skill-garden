//! AionHive Library
//!
//! 提供 Skills 共享平台的核心功能

// ---- 始终可用的模块 ----
pub mod models;

// ---- cli feature 专用 ----
#[cfg(feature = "cli")]
pub mod cli;

// ---- server feature 专用 ----
#[cfg(feature = "server")]
pub mod api;
#[cfg(feature = "server")]
pub mod db;
#[cfg(feature = "server")]
pub mod mcp;
#[cfg(feature = "server")]
pub mod schemas;
#[cfg(feature = "server")]
pub mod services;
#[cfg(feature = "server")]
pub mod utils;

#[cfg(feature = "server")]
use std::path::PathBuf;

#[cfg(feature = "server")]
pub use db::error::DbError;

// ---- server feature: 重新导出 ----
#[cfg(feature = "server")]
pub use models::api_key::{ApiKey, ApiKeyStatus, AuditLog};
#[cfg(feature = "server")]
pub use models::error::{AppError, ErrorCode};
#[cfg(feature = "server")]
pub use models::evaluation::{
    ConfidenceLevel, ErrorType, EvalTag, Evaluation, EvaluationFile, EvaluationResult, SkillStats,
};
#[cfg(feature = "server")]
pub use models::group::{Group, GroupType, Membership};
#[cfg(feature = "server")]
pub use models::identity::{Identity, IdentityStatus, IdentityType};
#[cfg(feature = "server")]
pub use models::org_tool::{OrgTool, ToolImplementation, ToolStatus};
#[cfg(feature = "server")]
pub use models::organization::{NewOrganization, Organization};
#[cfg(feature = "server")]
pub use models::response::{ApiError, ApiResponse, HealthStatus};
#[cfg(feature = "server")]
pub use models::role::{IdentityRole, Role, RoleType, ScopeLevel};
#[cfg(feature = "server")]
pub use models::session::{RouteTarget, Session, SessionStatus, ToolRouter};
#[cfg(feature = "server")]
pub use models::skill::{
    CliSetupResult, InstallResult, NewSkill, Skill, SkillDetail, SkillMetadata, SkillUpdate,
    SkillsIndex,
};
#[cfg(feature = "server")]
pub use models::skill_policy::{SkillPolicy, Visibility};
#[cfg(feature = "server")]
pub use models::tenant::{Tenant, TenantStatus};

#[cfg(feature = "server")]
pub use services::admin::*;
#[cfg(feature = "server")]
pub use services::admin::{
    ApiKeyService, AuditService, GroupService, IdentityService, RoleService, TenantService,
};
#[cfg(feature = "server")]
pub use services::evaluator::EvaluatorService;
#[cfg(feature = "server")]
pub use services::git_proxy::{GitDiff, GitFile, GitProxyConfig, GitProxyService, GitRef, Webhook};
#[cfg(feature = "server")]
pub use services::org_tool::OrgToolService;
#[cfg(feature = "server")]
pub use services::organization::OrganizationService;
#[cfg(feature = "server")]
pub use services::permission::PermissionService;
#[cfg(feature = "server")]
pub use services::registry::RegistryService;
#[cfg(feature = "server")]
pub use services::sandbox::{
    PlatformTool, SandboxConfig, SandboxInfo, SandboxService, SandboxStatus, ToolExecutionRequest,
    ToolExecutionResult,
};
#[cfg(feature = "server")]
pub use services::search::{SearchResult, SearchService};
#[cfg(feature = "server")]
pub use services::session::SessionService;
#[cfg(feature = "server")]
pub use services::skill_dependency::{
    DependencyTree, ResolvedSkill, SkillDependency, SkillDependencyService,
};
#[cfg(feature = "server")]
pub use services::storage::{FileLock, StorageService};
#[cfg(feature = "server")]
pub use services::tool_router::ToolRouterService;

#[cfg(feature = "server")]
pub use schemas::*;
#[cfg(feature = "server")]
pub use utils::*;

// ---- server feature: AppState ----
#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct AppState {
    pub registry: services::RegistryService,
    pub search: services::SearchService,
    pub storage: services::StorageService,
    pub evaluator: services::EvaluatorService,
    pub organization: services::OrganizationService,
    pub session: services::SessionService,
    pub org_tool: services::OrgToolService,
    pub tool_router: services::ToolRouterService,
    pub sandbox: services::SandboxService,
    pub git_proxy: services::GitProxyService,
    pub skill_dependency: services::SkillDependencyService,
    pub tenant: services::admin::TenantService,
    pub identity: services::admin::IdentityService,
    pub role: services::admin::RoleService,
    pub group: services::admin::GroupService,
    pub api_key: services::admin::ApiKeyService,
    pub audit: services::admin::AuditService,
    pub system_role_assignment: services::admin::SystemRoleAssignmentService,
    pub tenant_role_assignment: services::admin::TenantRoleAssignmentService,
    pub role_permission: services::admin::RolePermissionService,
    pub permission: services::PermissionService,
    pub download_token_repo: db::repositories::DownloadTokenRepository,
    pub data_dir: PathBuf,
}

#[cfg(feature = "server")]
impl AppState {
    pub async fn new(data_dir: PathBuf) -> anyhow::Result<Self> {
        let storage = services::StorageService::new(data_dir.clone());

        let pool = sqlx::PgPool::connect(
            &std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost:5432/aionhive".to_string()),
        )
        .await?;

        db::migrations::run_migrations(&pool, &data_dir).await?;

        let skill_repo = db::repositories::skill::SkillRepository::new(pool.clone());
        let org_repo = db::repositories::organization::OrganizationRepository::new(pool.clone());
        let session_repo = db::repositories::session::SessionRepository::new(pool.clone());
        let org_tool_repo = db::repositories::org_tool::OrgToolRepository::new(pool.clone());
        let _skill_policy_repo =
            db::repositories::skill_policy::SkillPolicyRepository::new(pool.clone());
        let eval_repo = db::repositories::evaluation::EvaluationRepository::new(pool.clone());
        let session_context_repo = db::repositories::SessionContextRepository::new(pool.clone());

        let download_token_repo = db::repositories::DownloadTokenRepository::new(pool.clone());
        let registry = services::RegistryService::new(
            data_dir.join("registry"),
            skill_repo.clone(),
            download_token_repo.clone(),
        );
        let search = services::SearchService::new(&data_dir.join("search_index"))?;
        let evaluator = services::EvaluatorService::new(data_dir.clone(), eval_repo);

        let organization = services::OrganizationService::new(org_repo);
        let session = services::SessionService::new(session_repo, session_context_repo.clone());
        let org_tool = services::OrgToolService::new(org_tool_repo);
        let tool_router = services::ToolRouterService::new();
        let sandbox = services::SandboxService::new();
        let git_proxy = services::GitProxyService::default();
        let skill_dependency =
            services::SkillDependencyService::new(session_context_repo, skill_repo.clone());

        let tenant_repo = db::repositories::TenantRepository::new(pool.clone());
        let identity_repo = db::repositories::IdentityRepository::new(pool.clone());
        let role_repo = db::repositories::RoleRepository::new(pool.clone());
        let group_repo = db::repositories::GroupRepository::new(pool.clone());
        let group_repo_for_perm = group_repo.clone();
        let api_key_repo = db::repositories::ApiKeyRepository::new(pool.clone());
        let audit_log_repo = db::repositories::AuditLogRepository::new(pool.clone());

        let tenant = services::admin::TenantService::new(tenant_repo);
        let identity = services::admin::IdentityService::new(identity_repo.clone());
        let role = services::admin::RoleService::new(role_repo);
        let group = services::admin::GroupService::new(group_repo);
        let api_key = services::admin::ApiKeyService::new(api_key_repo, identity.clone());
        let audit = services::admin::AuditService::new(audit_log_repo);

        let system_role_assignment_repo =
            db::repositories::SystemRoleAssignmentRepository::new(pool.clone());
        let tenant_role_assignment_repo =
            db::repositories::TenantRoleAssignmentRepository::new(pool.clone());
        let role_permission_repo = db::repositories::RolePermissionRepository::new(pool.clone());
        let org_membership_repo = db::repositories::OrgMembershipRepository::new(pool.clone());
        let group_perm_override_repo =
            db::repositories::GroupPermissionOverrideRepository::new(pool.clone());

        let system_role_assignment =
            services::admin::SystemRoleAssignmentService::new(system_role_assignment_repo.clone());
        let tenant_role_assignment =
            services::admin::TenantRoleAssignmentService::new(tenant_role_assignment_repo.clone());
        let role_permission =
            services::admin::RolePermissionService::new(role_permission_repo.clone());
        let permission = services::PermissionService::new(
            system_role_assignment_repo,
            tenant_role_assignment_repo,
            org_membership_repo,
            role_permission_repo,
            group_perm_override_repo.clone(),
            group_repo_for_perm,
            identity_repo.clone(),
        );

        Ok(Self {
            registry,
            search,
            storage,
            evaluator,
            organization,
            session,
            org_tool,
            tool_router,
            sandbox,
            git_proxy,
            skill_dependency,
            tenant,
            identity,
            role,
            group,
            api_key,
            audit,
            system_role_assignment,
            tenant_role_assignment,
            role_permission,
            permission,
            download_token_repo,
            data_dir,
        })
    }
}

#[cfg(feature = "server")]
impl From<DbError> for AppError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound(msg) => AppError::SkillNotFound(msg),
            DbError::AlreadyExists(msg) => AppError::SkillAlreadyExists(msg),
            DbError::QueryError(msg) => AppError::InternalError(msg),
            DbError::ConnectionError(msg) => {
                AppError::InternalError(format!("DB connection: {}", msg))
            }
            DbError::ValidationError(msg) => AppError::ValidationError(msg),
        }
    }
}
