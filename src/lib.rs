//! AionHive Library
//!
//! 提供 Skills 共享平台的核心功能

use std::path::PathBuf;

pub mod models;
pub mod schemas;
pub mod services;
pub mod utils;
pub mod mcp;
pub mod api;
pub mod db;
pub use db::error::DbError;

// Explicit re-exports to avoid ambiguous glob re-exports
// (models and services both have organization, session, org_tool modules)
pub use models::skill::{Skill, SkillMetadata, SkillDetail, InstallResult, SkillUpdate, NewSkill, SkillsIndex};
pub use models::evaluation::{Evaluation, EvaluationFile, SkillStats, EvaluationResult, ErrorType, EvalTag, ConfidenceLevel};
pub use models::error::{ErrorCode, AppError};
pub use models::response::{ApiResponse, ApiError, HealthStatus};
pub use models::organization::{Organization, NewOrganization};
pub use models::session::{Session, SessionStatus, ToolRouter, RouteTarget};
pub use models::org_tool::{OrgTool, ToolStatus, ToolImplementation};
pub use models::skill_policy::{SkillPolicy, Visibility};

// Services re-exports
pub use services::storage::{StorageService, FileLock};
pub use services::search::{SearchService, SearchResult};
pub use services::registry::RegistryService;
pub use services::evaluator::EvaluatorService;
pub use services::organization::OrganizationService;
pub use services::session::SessionService;
pub use services::org_tool::OrgToolService;
pub use services::tool_router::ToolRouterService;
pub use services::sandbox::{SandboxService, ToolExecutionRequest, ToolExecutionResult};
pub use services::git_proxy::{GitProxyService, GitRef, GitFile, GitDiff};

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
    pub data_dir: PathBuf,
}

impl AppState {
    pub async fn new(data_dir: PathBuf, skills_dir: PathBuf) -> anyhow::Result<Self> {
        let storage = services::StorageService::new(data_dir.clone());

        let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost:5432/aionhive".to_string())).await?;

        // Run database migrations
        db::migrations::run_migrations(&pool, &data_dir).await?;

        // Create repositories
        let skill_repo = db::repositories::skill::SkillRepository::new(pool.clone());
        let agent_repo = db::repositories::agent::AgentRepository::new(pool.clone());
        let org_repo = db::repositories::organization::OrganizationRepository::new(pool.clone());
        let session_repo = db::repositories::session::SessionRepository::new(pool.clone());
        let org_tool_repo = db::repositories::org_tool::OrgToolRepository::new(pool.clone());
        let _skill_policy_repo = db::repositories::skill_policy::SkillPolicyRepository::new(pool.clone());
        let _audit_repo = db::repositories::audit::AuditRepository::new(pool.clone());
        let eval_repo = db::repositories::evaluation::EvaluationRepository::new(pool.clone());

        // Create services
        let registry = services::RegistryService::new(skills_dir, data_dir.join("registry"), skill_repo.clone());
        let search = services::SearchService::new(&data_dir.join("search_index"))?;
        let evaluator = services::EvaluatorService::new(data_dir.clone(), eval_repo);

        // v0.4 multi-tenant services
        let organization = services::OrganizationService::new(org_repo);
        let session = services::SessionService::new(session_repo, agent_repo);
        let org_tool = services::OrgToolService::new(org_tool_repo);
        let tool_router = services::ToolRouterService::new();
        let sandbox = services::SandboxService::new();
        let git_proxy = services::GitProxyService::default();

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
            data_dir,
        })
    }
}

impl From<DbError> for AppError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound(msg) => AppError::SkillNotFound(msg),
            DbError::AlreadyExists(msg) => AppError::SkillAlreadyExists(msg),
            DbError::QueryError(msg) => AppError::InternalError(msg),
            DbError::ConnectionError(msg) => AppError::InternalError(format!("DB connection: {}", msg)),
            DbError::ValidationError(msg) => AppError::ValidationError(msg),
        }
    }
}
