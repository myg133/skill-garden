//! HTTP Server State

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::db::repositories::{
    group_permission_override::GroupPermissionOverrideRepository, AgentRepository, AuditRepository,
};
use crate::mcp::McpServer;
use crate::services::admin::{
    ApiKeyService, AuditService, GroupService, IdentityService, RoleService, TenantService,
};
use crate::services::permission::PermissionService;
use crate::services::{
    EvaluatorService, GitProxyService, OrgToolService, OrganizationService, RegistryService,
    SandboxService, SearchService, SessionService,
};

#[derive(Clone)]
pub struct HttpState {
    pub mcp_server: Arc<RwLock<McpServer>>,
}

#[derive(Clone)]
pub struct SseState {
    pub sessions:
        Arc<RwLock<std::collections::HashMap<String, tokio::sync::broadcast::Sender<String>>>>,
}

impl SseState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
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
    // Admin services
    pub tenant: TenantService,
    pub identity: IdentityService,
    pub role: RoleService,
    pub group: GroupService,
    pub api_key: ApiKeyService,
    pub audit: AuditService,
    pub group_perm_override_repo: GroupPermissionOverrideRepository,
    pub permission: PermissionService,
}
