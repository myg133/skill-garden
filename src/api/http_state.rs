//! HTTP Server State

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::db::repositories::{
    group_permission_override::GroupPermissionOverrideRepository, AgentRepository, AuditRepository,
    DownloadTokenRepository, SkillRepository, VersionRepository,
};
use crate::mcp::McpServer;
use crate::services::admin::{
    ApiKeyService, AuditService, GroupService, IdentityService, OrgJoinRequestService,
    RolePermissionService, RoleService, SystemRoleAssignmentService, TenantRoleAssignmentService,
    TenantService,
};
use crate::services::PermissionService;
use crate::services::{
    EvaluatorService, GitProxyService, OrgToolService, OrganizationService, RegistryService,
    SandboxService, SearchService, SessionService, SkillGitService,
};
use crate::utils::RateLimiter;
use crate::TenantConfig;

#[derive(Clone)]
pub struct HttpState {
    pub mcp_server: Arc<RwLock<McpServer>>,
}

/// SSE session with idle tracking
#[derive(Clone)]
pub struct SseSession {
    pub tx: tokio::sync::broadcast::Sender<String>,
    /// Last time a POST /sse/:id message was received
    pub last_activity: Instant,
}

/// Default idle timeout: sessions with no POST messages for this duration will be cleaned up
pub const SSE_IDLE_TIMEOUT_SECS: u64 = 300; // 5 minutes

#[derive(Clone)]
pub struct SseState {
    pub sessions: Arc<RwLock<std::collections::HashMap<String, SseSession>>>,
}

impl SseState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Remove sessions that have been idle for longer than `timeout`.
    /// Returns how many sessions were removed.
    pub async fn cleanup_idle(&self, timeout: Duration) -> usize {
        let mut sessions = self.sessions.write().await;
        let now = Instant::now();
        let before = sessions.len();
        sessions.retain(|_sid, s| now.duration_since(s.last_activity) < timeout);
        let removed = before - sessions.len();
        if removed > 0 {
            tracing::info!(
                "SSE cleanup: removed {} idle sessions, {} remaining",
                removed,
                sessions.len()
            );
        }
        removed
    }
}

#[derive(Clone)]
pub struct AppRouterState {
    pub http: HttpState,
    pub sse: SseState,
    pub registry: RegistryService,
    pub search: SearchService,
    pub evaluator: EvaluatorService,
    pub agent_repo: AgentRepository,
    pub audit_repo: AuditRepository,
    // v0.4 multi-tenant services
    pub organization: OrganizationService,
    pub session: SessionService,
    pub org_tool: OrgToolService,
    pub sandbox: SandboxService,
    pub git_proxy: GitProxyService,
    // Skill version management
    pub skill_git: SkillGitService,
    pub version_repo: VersionRepository,
    pub skill_repo: SkillRepository,
    pub download_token_repo: DownloadTokenRepository,
    // Admin services
    pub tenant: TenantService,
    pub identity: IdentityService,
    pub role: RoleService,
    pub group: GroupService,
    pub api_key: ApiKeyService,
    pub audit: AuditService,
    pub system_role_assignment: SystemRoleAssignmentService,
    pub tenant_role_assignment: TenantRoleAssignmentService,
    pub role_permission: RolePermissionService,
    pub org_join_request: OrgJoinRequestService,
    pub permission: PermissionService,
    pub login_rate_limiter: RateLimiter,
    pub group_perm_override_repo: GroupPermissionOverrideRepository,
    pub tenant_config: TenantConfig,
}
