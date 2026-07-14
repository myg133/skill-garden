//! AionHive Library
//!
//! 提供 Skills 共享平台的核心功能

use std::path::PathBuf;

pub mod api;
pub mod db;
pub mod mcp;
pub mod models;
pub mod schemas;
pub mod services;
pub mod utils;
pub use db::error::DbError;

// Explicit re-exports to avoid ambiguous glob re-exports
// (models and services both have organization, session, org_tool modules)
pub use models::api_key::{ApiKey, ApiKeyStatus, AuditLog};
pub use models::error::{AppError, ErrorCode};
pub use models::evaluation::{
    ConfidenceLevel, ErrorType, EvalTag, Evaluation, EvaluationFile, EvaluationResult, SkillStats,
};
pub use models::group::{Group, GroupType, Membership};
pub use models::identity::{Identity, IdentityStatus, IdentityType};
pub use models::org_tool::{OrgTool, ToolImplementation, ToolStatus};
pub use models::organization::{NewOrganization, Organization};
pub use models::response::{ApiError, ApiResponse, HealthStatus};
pub use models::role::{IdentityRole, Role, RoleType, ScopeLevel};
pub use models::session::{RouteTarget, Session, SessionStatus, ToolRouter};
pub use models::skill::{
    InstallResult, NewSkill, Skill, SkillDetail, SkillMetadata, SkillUpdate, SkillsIndex,
};
pub use models::skill_policy::{SkillPolicy, Visibility};
pub use models::tenant::{Tenant, TenantStatus};

// Services re-exports
pub use services::admin::*;
pub use services::admin::{
    ApiKeyService, AuditService, GroupService, IdentityService, RoleService, TenantService,
};
pub use services::evaluator::EvaluatorService;
pub use services::git_proxy::{GitDiff, GitFile, GitProxyConfig, GitProxyService, GitRef, Webhook};
pub use services::org_tool::OrgToolService;
pub use services::organization::OrganizationService;
pub use services::permission::PermissionService;
pub use services::registry::RegistryService;
pub use services::sandbox::{
    PlatformTool, SandboxConfig, SandboxInfo, SandboxService, SandboxStatus, ToolExecutionRequest,
    ToolExecutionResult,
};
pub use services::search::{SearchResult, SearchService};
pub use services::session::SessionService;
pub use services::skill_dependency::{
    DependencyTree, ResolvedSkill, SkillDependency, SkillDependencyService,
};
pub use services::storage::{FileLock, StorageService};
pub use services::tool_router::ToolRouterService;

pub use schemas::*;
pub use utils::*;

#[derive(Debug, Clone)]
pub struct AppState {
    pub registry: services::RegistryService,
    pub search: services::SearchService,
    pub storage: services::StorageService,
    pub evaluator: services::EvaluatorService,
    // v0.4 multi-tenant services
    pub organization: services::OrganizationService,
    pub session: services::SessionService,
    pub org_tool: services::OrgToolService,
    pub tool_router: services::ToolRouterService,
    pub sandbox: services::SandboxService,
    pub git_proxy: services::GitProxyService,
    pub skill_dependency: services::SkillDependencyService,
    // Admin services
    pub tenant: services::admin::TenantService,
    pub identity: services::admin::IdentityService,
    pub role: services::admin::RoleService,
    pub group: services::admin::GroupService,
    pub api_key: services::admin::ApiKeyService,
    pub audit: services::admin::AuditService,
    pub system_role_assignment: services::admin::SystemRoleAssignmentService,
    pub role_permission: services::admin::RolePermissionService,
    pub permission: services::PermissionService,
    pub download_token_repo: db::repositories::DownloadTokenRepository,
    pub data_dir: PathBuf,
    pub skills_dir: PathBuf,
}

impl AppState {
    pub async fn new(data_dir: PathBuf, skills_dir: PathBuf) -> anyhow::Result<Self> {
        let storage = services::StorageService::new(data_dir.clone());

        let pool = sqlx::PgPool::connect(
            &std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost:5432/aionhive".to_string()),
        )
        .await?;

        // Run database migrations
        db::migrations::run_migrations(&pool, &data_dir).await?;

        // Create repositories
        let skill_repo = db::repositories::skill::SkillRepository::new(pool.clone());
        let org_repo = db::repositories::organization::OrganizationRepository::new(pool.clone());
        let session_repo = db::repositories::session::SessionRepository::new(pool.clone());
        let org_tool_repo = db::repositories::org_tool::OrgToolRepository::new(pool.clone());
        let _skill_policy_repo =
            db::repositories::skill_policy::SkillPolicyRepository::new(pool.clone());
        let eval_repo = db::repositories::evaluation::EvaluationRepository::new(pool.clone());
        let session_context_repo = db::repositories::SessionContextRepository::new(pool.clone());

        // Create services
        let download_token_repo = db::repositories::DownloadTokenRepository::new(pool.clone());
        let registry = services::RegistryService::new(
            skills_dir.clone(),
            data_dir.join("registry"),
            skill_repo.clone(),
            download_token_repo.clone(),
        );
        let search = services::SearchService::new(&data_dir.join("search_index"))?;
        let evaluator = services::EvaluatorService::new(data_dir.clone(), eval_repo);

        // v0.4 multi-tenant services
        let organization = services::OrganizationService::new(org_repo);
        let session = services::SessionService::new(session_repo, session_context_repo.clone());
        let org_tool = services::OrgToolService::new(org_tool_repo);
        let tool_router = services::ToolRouterService::new();
        let sandbox = services::SandboxService::new();
        let git_proxy = services::GitProxyService::default();
        let skill_dependency =
            services::SkillDependencyService::new(session_context_repo, skill_repo.clone());

        // Admin services
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
        let api_key = services::admin::ApiKeyService::new(api_key_repo);
        let audit = services::admin::AuditService::new(audit_log_repo);

        // RBAC services
        let system_role_assignment_repo =
            db::repositories::SystemRoleAssignmentRepository::new(pool.clone());
        let role_permission_repo = db::repositories::RolePermissionRepository::new(pool.clone());
        let org_membership_repo = db::repositories::OrgMembershipRepository::new(pool.clone());
        let group_perm_override_repo =
            db::repositories::GroupPermissionOverrideRepository::new(pool.clone());

        let system_role_assignment =
            services::admin::SystemRoleAssignmentService::new(system_role_assignment_repo.clone());
        let role_permission =
            services::admin::RolePermissionService::new(role_permission_repo.clone());
        let permission = services::PermissionService::new(
            system_role_assignment_repo,
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
            role_permission,
            permission,
            download_token_repo,
            data_dir,
            skills_dir,
        })
    }
}

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
