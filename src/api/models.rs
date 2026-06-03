//! API Request/Response Models

use crate::models::SkillStats;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ListResponse<T> {
    pub data: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

impl<T> ListResponse<T> {
    pub fn new(data: Vec<T>, total: usize, page: usize, page_size: usize) -> Self {
        Self {
            data,
            total,
            page,
            page_size,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub skills_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct ListSkillsQuery {
    pub tag: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSkillBody {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub content: String,
    pub version: Option<String>,
    #[serde(default)]
    pub git_url: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSkillBody {
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub content: Option<String>,
    #[serde(default)]
    pub git_url: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEvaluationBody {
    pub skill_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error_type: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SkillCreatedResponse {
    pub message: String,
    pub skill_id: String,
}

#[derive(Debug, Serialize)]
pub struct EvaluationCreatedResponse {
    pub message: String,
    pub evaluation_id: String,
    pub new_stats: SkillStats,
}

#[derive(Debug, Deserialize)]
pub struct RegisterAgentBody {
    pub agent_id: String,
    pub agent_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterAgentResponse {
    pub agent_id: String,
    pub secret: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GetTokenBody {
    pub agent_id: String,
    pub agent_secret: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub agent_id: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub id: String,
    pub agent_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: serde_json::Value,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct AuditLogListResponse {
    pub data: Vec<AuditLogResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct RejectSkillBody {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitSkillReviewBody {
    pub comment: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SkillReviewResponse {
    pub message: String,
    pub skill_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AddSkillToGroupBody {
    pub group_id: Uuid,
    #[serde(default)]
    pub skill_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SkillGroupResponse {
    pub skill_id: String,
    pub group_id: Uuid,
    pub group_name: String,
    pub added_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UserLoginBody {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UserRegisterBody {
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct UserLoginResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfoResponse,
}

#[derive(Debug, Serialize)]
pub struct UserInfoResponse {
    pub id: uuid::Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub identity_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserBody {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserOrgResponse {
    pub id: uuid::Uuid,
    pub name: String,
    pub slug: Option<String>,
    pub role: String,
}

// v0.4 multi-tenant API models

use uuid::Uuid;

/// Organization models

#[derive(Debug, Deserialize)]
pub struct CreateOrgBody {
    pub name: String,
    pub slug: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub tenant_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrgBody {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListOrgsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub tenant_id: Option<Uuid>,
}

/// Session models

#[derive(Debug, Deserialize)]
pub struct CreateSessionBody {
    pub agent_id: String,
    pub org_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SessionDeclareBody {
    pub capabilities: Vec<String>,
}

/// Org Tool models

#[derive(Debug, Deserialize)]
pub struct RegisterOrgToolBody {
    pub org_id: Uuid,
    pub tool_id: String,
    pub name: String,
    pub description: String,
    pub schema: Option<serde_json::Value>,
    pub implementation: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListOrgToolsQuery {
    pub approved_only: Option<bool>,
}

/// Organization member models

#[derive(Debug, Serialize)]
pub struct OrgMemberResponse {
    pub agent_id: String,
    pub name: Option<String>,
    pub capabilities: Vec<String>,
    pub joined_at: String,
}

#[derive(Debug, Serialize)]
pub struct OrgMemberListResponse {
    pub members: Vec<OrgMemberResponse>,
}

#[derive(Debug, Deserialize)]
pub struct AddOrgMemberBody {
    pub agent_id: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrgStatsResponse {
    pub org_id: Uuid,
    pub members_count: i64,
    pub skills_count: i64,
    pub sessions_count: i64,
    pub tools_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct InviteOrgMemberBody {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrgMemberBody {
    pub role: String,
}

/// Group member management models

#[derive(Debug, Deserialize)]
pub struct AddGroupMemberBody {
    pub agent_id: String,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupMemberBody {
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct GroupMemberInfo {
    pub agent_id: String,
    pub name: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

/// Admin user login models

#[derive(Debug, Deserialize)]
pub struct AdminLoginBody {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AdminLoginResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: AdminUserInfo,
}

#[derive(Debug, Serialize)]
pub struct AdminUserInfo {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminMeResponse {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct AdminStatsResponse {
    pub total_skills: i64,
    pub total_agents: i64,
    pub total_organizations: i64,
    pub total_evaluations: i64,
    pub average_success_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct AdminStatusResponse {
    pub version: String,
    pub transport_mode: String,
    pub http_port: u16,
    pub data_dir: String,
    pub skills_dir: String,
    pub db_connected: bool,
    pub db_url: String,
    pub jwt_expiry_hours: u64,
}

// Sandbox API models

use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct SandboxHealthResponse {
    pub docker_connected: bool,
    pub active_containers: u32,
    pub containers: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteToolBody {
    pub tool_id: String,
    pub org_id: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub timeout_seconds: Option<u64>,
}

// Git Proxy API models

#[derive(Debug, Deserialize)]
pub struct ValidateGitUrlBody {
    pub git_url: String,
}

#[derive(Debug, Serialize)]
pub struct GitProxyHealthResponse {
    pub git_proxy_connected: bool,
    pub api_base: String,
}

/// Group permission override models

#[derive(Debug, Deserialize)]
pub struct UpdateGroupPermissionBody {
    pub role_name: String,
    pub permission_code: String,
    pub granted: bool,
}

#[derive(Debug, Serialize)]
pub struct GroupPermissionInfo {
    pub permission_code: String,
    pub granted: bool,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct GroupPermissionListResponse {
    pub lead: Vec<GroupPermissionInfo>,
    pub member: Vec<GroupPermissionInfo>,
}

// Tenant API models

#[derive(Debug, Deserialize)]
pub struct CreateTenantBody {
    pub name: String,
    pub slug: String,
    pub billing_plan: Option<String>,
    pub sso_config: Option<serde_json::Value>,
}

impl From<CreateTenantBody> for crate::models::tenant::NewTenant {
    fn from(body: CreateTenantBody) -> Self {
        crate::models::tenant::NewTenant {
            name: body.name,
            slug: body.slug,
            billing_plan: body.billing_plan,
            sso_config: body.sso_config,
            settings: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateTenantBody {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub status: Option<String>,
    pub billing_plan: Option<String>,
    pub sso_config: Option<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
}

impl From<UpdateTenantBody> for crate::models::tenant::TenantUpdate {
    fn from(body: UpdateTenantBody) -> Self {
        crate::models::tenant::TenantUpdate {
            name: body.name,
            slug: body.slug,
            status: body.status.map(|s| s.as_str().into()),
            billing_plan: body.billing_plan,
            sso_config: body.sso_config,
            settings: body.settings,
        }
    }
}

// Identity API models

#[derive(Debug, Deserialize)]
pub struct CreateIdentityBody {
    pub identity_type: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub external_id: Option<String>,
    pub name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub password: Option<String>,
    pub password_hash: Option<String>,
    #[serde(default)]
    pub is_system_admin: bool,
}

impl From<CreateIdentityBody> for crate::models::identity::NewIdentity {
    fn from(body: CreateIdentityBody) -> Self {
        let password_hash = if let Some(ref pwd) = body.password {
            Some(bcrypt::hash(pwd, bcrypt::DEFAULT_COST).unwrap_or_default())
        } else {
            body.password_hash
        };

        crate::models::identity::NewIdentity {
            identity_type: body.identity_type.as_str().into(),
            username: body.username,
            display_name: body.display_name,
            external_id: body.external_id,
            name: body.name,
            email: body.email,
            avatar_url: body.avatar_url,
            password_hash,
            is_system_admin: body.is_system_admin,
            metadata: Some(serde_json::json!({})),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateIdentityBody {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub password: Option<String>,
    pub status: Option<String>,
    pub is_system_admin: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

impl From<UpdateIdentityBody> for crate::models::identity::IdentityUpdate {
    fn from(body: UpdateIdentityBody) -> Self {
        let password_hash = body
            .password
            .map(|pwd| bcrypt::hash(&pwd, bcrypt::DEFAULT_COST).unwrap_or_default());

        crate::models::identity::IdentityUpdate {
            name: body.name,
            display_name: body.display_name,
            email: body.email,
            avatar_url: body.avatar_url,
            password_hash,
            status: body.status.map(|s| s.as_str().into()),
            is_system_admin: body.is_system_admin,
            metadata: body.metadata,
        }
    }
}

// Group API models

#[derive(Debug, Deserialize, Clone)]
pub struct PermissionOverrideInput {
    pub role_name: String,
    pub permission_code: String,
    pub granted: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupBody {
    pub organization_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub group_type: Option<String>,
    pub permission_overrides: Option<Vec<PermissionOverrideInput>>,
}

impl From<CreateGroupBody> for crate::models::group::NewGroup {
    fn from(body: CreateGroupBody) -> Self {
        crate::models::group::NewGroup {
            organization_id: body.organization_id,
            name: body.name,
            slug: body.slug,
            description: body.description,
            group_type: body
                .group_type
                .map(|g| g.as_str().into())
                .unwrap_or_default(),
            settings: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupBody {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub group_type: Option<String>,
}

impl From<UpdateGroupBody> for crate::models::group::GroupUpdate {
    fn from(body: UpdateGroupBody) -> Self {
        crate::models::group::GroupUpdate {
            name: body.name,
            slug: body.slug,
            description: body.description,
            group_type: body.group_type.map(|g| g.as_str().into()),
            settings: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListGroupsQuery {
    pub organization_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// API Key models

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyBody {
    pub identity_id: Uuid,
    pub organization_id: Uuid,
    pub name: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub rate_limit: Option<i32>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// User-facing API key creation (identity_id derived from auth context)
#[derive(Debug, Deserialize)]
pub struct CreateMyApiKeyBody {
    pub organization_id: Uuid,
    pub name: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub rate_limit: Option<i32>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<CreateApiKeyBody> for crate::models::api_key::CreateApiKeyRequest {
    fn from(body: CreateApiKeyBody) -> Self {
        crate::models::api_key::CreateApiKeyRequest {
            identity_id: body.identity_id,
            organization_id: body.organization_id,
            name: body.name,
            scopes: body.scopes.unwrap_or_default(),
            rate_limit: body.rate_limit.unwrap_or(1000),
            expires_at: body.expires_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListApiKeysQuery {
    pub identity_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
}

// Audit entry models

#[derive(Debug, Deserialize)]
pub struct ListAuditEntriesQuery {
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub identity_id: Option<Uuid>,
    pub action: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// Common pagination query

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// Marketplace query models

#[derive(Debug, Deserialize)]
pub struct MarketplaceQuery {
    pub keyword: Option<String>,
    pub tag: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct InstallSkillResponse {
    pub message: String,
    pub skill_id: String,
    pub install_count: i32,
}

/// Create skill under organization

#[derive(Debug, Deserialize)]
pub struct CreateOrgSkillBody {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub content: String,
    pub version: Option<String>,
    #[serde(default)]
    pub git_url: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    #[serde(default)]
    pub owner_type: Option<String>,
}
