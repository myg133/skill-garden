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
    /// 按组织 ID 过滤（仅该组织的 Skill）
    pub org_id: Option<Uuid>,
    /// 按 marketplace_status 过滤（如 "listed", "pending_review"）
    pub marketplace_status: Option<String>,
    /// 仅个人 Skill（scope=personal）
    pub scope_personal: Option<bool>,
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
    /// 留空时自动推断：若当前用户有关联组织则为 "organization"，否则为 "user"
    #[serde(default)]
    pub owner_type: Option<String>,
    /// 当 owner_type = "organization" 时，留空则使用当前用户关联的组织
    pub organization_id: Option<uuid::Uuid>,
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

/// Agent 列表响应项
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentListItem {
    pub agent_id: String,
    pub agent_name: Option<String>,
    pub agent_description: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
    pub last_used_at: Option<String>,
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
    pub identity_name: Option<String>,
    pub identity_type: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
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

// Password reset models

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordBody {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct ForgotPasswordResponse {
    pub message: String,
    pub reset_token: String, // In production, this would be sent via email
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordBody {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {
    pub message: String,
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
    pub is_admin: bool,
    pub organizations: Vec<UserOrgInfo>,
    /// 系统级角色列表，如 ["super_admin", "marketplace_admin"]
    #[serde(default)]
    pub system_roles: Vec<String>,
    /// 租户级角色列表
    #[serde(default)]
    pub tenant_roles: Vec<TenantRoleInfo>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 用户所属组织简要信息（登录响应用）
#[derive(Debug, Serialize, Clone)]
pub struct UserOrgInfo {
    pub id: uuid::Uuid,
    pub name: String,
    pub slug: Option<String>,
    pub role: String,
}

/// 租户角色信息（登录/权限响应用）
#[derive(Debug, Serialize, Clone)]
pub struct TenantRoleInfo {
    pub tenant_id: uuid::Uuid,
    pub tenant_name: String,
    pub role_name: String,
}

/// 组织角色信息（权限刷新响应用）
#[derive(Debug, Serialize, Clone)]
pub struct OrgRoleInfo {
    pub org_id: uuid::Uuid,
    pub org_name: String,
    pub role_name: String,
}

/// 组角色信息（权限刷新响应用）
#[derive(Debug, Serialize, Clone)]
pub struct GroupRoleInfo {
    pub group_id: uuid::Uuid,
    pub group_name: String,
    pub role_name: String,
}

/// GET /users/me/permissions 响应
#[derive(Debug, Serialize)]
pub struct MyPermissionsResponse {
    pub system_roles: Vec<String>,
    pub tenant_roles: Vec<TenantRoleInfo>,
    pub org_roles: Vec<OrgRoleInfo>,
    pub group_roles: Vec<GroupRoleInfo>,
    /// 当前用户所有可用的 permission_code 列表
    pub permissions: Vec<String>,
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

/// Session models (sessions auto-created by MCP, admin-only read/end)

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
    #[serde(default)]
    pub docker_image: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExecutePlatformToolBody {
    pub tool_name: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub timeout_seconds: Option<u64>,
}

/// Sandbox management models

#[derive(Debug, Deserialize)]
pub struct ReleaseSandboxBody {
    pub org_id: String,
    pub tool_id: String,
}

#[derive(Debug, Serialize)]
pub struct SandboxStatusResponse {
    pub total: usize,
    pub max: usize,
    pub containers: Vec<SandboxInfoItem>,
}

#[derive(Debug, Serialize)]
pub struct SandboxInfoItem {
    pub key: String,
    pub container_id: String,
    pub image: String,
    pub status: String,
    pub idle_seconds: i64,
    pub created_at: String,
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
    pub organization_id: Option<Uuid>,
    pub name: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub rate_limit: Option<i32>,
    #[serde(default)]
    pub expires_in_days: Option<i32>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl CreateApiKeyBody {
    /// Compute expires_at from expires_in_days if set (takes precedence over raw expires_at)
    pub fn effective_expires_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        if let Some(days) = self.expires_in_days {
            let duration = chrono::Duration::days(days as i64);
            Some(chrono::Utc::now() + duration)
        } else {
            self.expires_at
        }
    }
}

/// User-facing API key creation (identity_id derived from auth context)
#[derive(Debug, Deserialize)]
pub struct CreateMyApiKeyBody {
    pub organization_id: Option<Uuid>,
    pub name: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub rate_limit: Option<i32>,
    #[serde(default)]
    pub expires_in_days: Option<i32>,
    #[serde(default)]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl CreateMyApiKeyBody {
    /// Compute expires_at from expires_in_days if set (takes precedence over raw expires_at)
    pub fn effective_expires_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        if let Some(days) = self.expires_in_days {
            let duration = chrono::Duration::days(days as i64);
            Some(chrono::Utc::now() + duration)
        } else {
            self.expires_at
        }
    }
}

impl From<CreateApiKeyBody> for crate::models::api_key::CreateApiKeyRequest {
    fn from(body: CreateApiKeyBody) -> Self {
        let expires_at = body.effective_expires_at();
        crate::models::api_key::CreateApiKeyRequest {
            identity_id: body.identity_id,
            organization_id: body.organization_id,
            name: body.name,
            scopes: body.scopes.unwrap_or_default(),
            rate_limit: body.rate_limit.unwrap_or(1000),
            expires_at,
        }
    }
}

impl From<CreateMyApiKeyBody> for crate::models::api_key::UserCreateApiKeyRequest {
    fn from(body: CreateMyApiKeyBody) -> Self {
        let expires_at = body.effective_expires_at();
        crate::models::api_key::UserCreateApiKeyRequest {
            organization_id: body.organization_id,
            name: body.name,
            scopes: body.scopes.unwrap_or_default(),
            rate_limit: body.rate_limit.unwrap_or(1000),
            expires_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListApiKeysQuery {
    pub identity_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
}

/// 更新 API Key 状态的请求体（禁用 / 启用）
#[derive(Debug, Deserialize)]
pub struct UpdateApiKeyStatusBody {
    /// 目标状态，仅允许 "disabled" | "active"
    pub status: String,
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

// Role management request bodies

#[derive(Debug, Deserialize)]
pub struct CreateRoleBody {
    pub name: String,
    pub role_type: String,
    pub scope_level: String,
    pub parent_role_id: Option<Uuid>,
    pub permissions: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleBody {
    pub name: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub description: Option<String>,
}

// Identity role management

#[derive(Debug, Deserialize)]
pub struct GrantRoleBody {
    pub role_id: Uuid,
    pub scope_id: Option<Uuid>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeRoleQuery {
    pub scope_id: Option<Uuid>,
}

// System role assignment

#[derive(Debug, Deserialize)]
pub struct AssignSystemRoleBody {
    pub identity_id: Uuid,
    pub role_name: String,
}

#[derive(Debug, Deserialize)]
pub struct RevokeSystemRoleBody {
    pub identity_id: Uuid,
    pub role_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ListSystemRoleAssignmentsQuery {
    pub role_name: Option<String>,
    pub identity_id: Option<Uuid>,
}

// Role permission management

#[derive(Debug, Deserialize)]
pub struct CreateRolePermissionBody {
    pub role_level: String,
    pub role_name: String,
    pub permission_code: String,
    pub scope_restriction: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRolePermissionQuery {
    pub role_level: String,
    pub role_name: String,
    pub permission_code: String,
}

// Permission check

#[derive(Debug, Deserialize)]
pub struct PermissionCheckBody {
    pub permission_code: String,
    pub owner_type: Option<String>,
    pub owner_id: Option<Uuid>,
    pub author_identity_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub group_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct PermissionCheckResponse {
    pub has_permission: bool,
}

#[derive(Debug, Deserialize)]
pub struct MarketplaceQuery {
    pub keyword: Option<String>,
    pub tag: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
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

// --- Admin User Management Models ---

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    #[serde(default)]
    pub identity_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserAdminResponse {
    pub id: uuid::Uuid,
    pub identity_type: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub is_system_admin: bool,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct DisableUserBody {
    pub disabled: bool,
}

// --- Evaluation Query Models ---

#[derive(Debug, Deserialize)]
pub struct ListEvaluationsQuery {
    pub skill_id: Option<String>,
    pub agent_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct EvaluationItemResponse {
    pub id: String,
    pub skill_id: String,
    pub agent_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error_type: Option<String>,
    pub tags: Vec<String>,
    pub timestamp: String,
}

// --- Webhook Management Models ---

#[derive(Debug, Deserialize)]
pub struct AddWebhookBody {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct WebhookItemResponse {
    pub index: usize,
    pub url: String,
}

// --- Skill Upload & Version Management Models ---

/// ZIP 上传的响应
#[derive(Debug, Serialize)]
pub struct SkillUploadResponse {
    pub skill_id: String,
    pub skill_name: String,
    pub version: String,
    pub git_commit: String,
    pub git_tag: String,
    pub git_repo_name: String,
    pub is_new_skill: bool,
    pub files: Vec<String>,
    pub message: String,
}

// --- Skill Upload Preview & Confirm Models ---

/// 预览阶段单文件信息
#[derive(Debug, Serialize)]
pub struct PreviewFileResponse {
    pub path: String,
    pub size: u64,
}

/// 预览元数据
#[derive(Debug, Serialize)]
pub struct PreviewMetadataResponse {
    pub name: String,
    pub description: String,
    pub version: String,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub compatibility: String,
}

/// 上传预览响应
#[derive(Debug, Serialize)]
pub struct SkillUploadPreviewResponse {
    pub preview_id: String,
    pub metadata: PreviewMetadataResponse,
    pub files: Vec<PreviewFileResponse>,
    pub total_files: usize,
    pub total_size: u64,
}

/// 文件内容响应
#[derive(Debug, Serialize)]
pub struct PreviewFileContentResponse {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub is_binary: bool,
    pub content_type: String,
}

/// 确认上传请求
#[derive(Debug, Deserialize)]
pub struct ConfirmUploadBody {
    #[serde(default)]
    pub owner_type: Option<String>,
    pub owner_id: Option<uuid::Uuid>,
    pub author_identity_id: Option<uuid::Uuid>,
    /// 当 owner_type = "organization" 时，前端传入具体的组织 ID
    pub organization_id: Option<uuid::Uuid>,
}

/// 版本回退请求（admin only）
#[derive(Debug, Deserialize)]
pub struct RollbackSkillBody {
    /// 要回退到的目标版本号（不含 v 前缀），如 "1.0.2"
    pub version: String,
}

/// 版本列表项
#[derive(Debug, Serialize)]
pub struct SkillVersionResponse {
    pub id: String,
    pub skill_name: String,
    pub version: String,
    pub git_commit_hash: Option<String>,
    pub git_tag: Option<String>,
    pub changelog: Option<String>,
    pub file_count: i32,
    pub total_size_bytes: i64,
    pub uploaded_by: Option<uuid::Uuid>,
    pub git_remote_url: Option<String>,
    pub created_at: String,
}

/// 版本列表查询
#[derive(Debug, Deserialize)]
pub struct ListVersionsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 版本 diff 查询
#[derive(Debug, Deserialize)]
pub struct VersionDiffQuery {
    pub from: String,
    pub to: String,
}

// --- GitLab Remote Sync Models ---

/// Skill 远程 GitLab 信息响应
#[derive(Debug, Serialize)]
pub struct SkillRemoteInfoResponse {
    pub skill_name: String,
    pub git_remote_url: Option<String>,
    pub gitlab_group: String,
    pub gitlab_url: String,
    pub push_enabled: bool,
    pub local_repo_exists: bool,
}

/// GitLab 同步/克隆请求
#[derive(Debug, Deserialize)]
pub struct SkillSyncBody {
    /// 可选的 skill name 列表，不传则同步全部
    pub skill_names: Option<Vec<String>>,
}

/// GitLab Webhook push event 载荷
#[derive(Debug, Deserialize)]
pub struct GitlabWebhookBody {
    pub object_kind: Option<String>,
    pub project: Option<GitlabWebhookProject>,
}

#[derive(Debug, Deserialize)]
pub struct GitlabWebhookProject {
    pub name: Option<String>,
    pub path_with_namespace: Option<String>,
}

// --- Tenant Role Assignment Models ---

#[derive(Debug, Deserialize)]
pub struct AssignTenantRoleBody {
    pub identity_id: Uuid,
    pub tenant_id: Uuid,
    pub role_name: String,
}

#[derive(Debug, Deserialize)]
pub struct RevokeTenantRoleBody {
    pub identity_id: Uuid,
    pub tenant_id: Uuid,
    pub role_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ListTenantRoleAssignmentsQuery {
    pub tenant_id: Option<Uuid>,
    pub identity_id: Option<Uuid>,
}

// --- Marketplace Reviewer Assignment Models ---

#[derive(Debug, Deserialize)]
pub struct AssignMarketplaceReviewerBody {
    pub identity_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RevokeMarketplaceReviewerBody {
    pub identity_id: Uuid,
}

// --- Marketplace Delist Request Models ---

/// 作者申请下架市场 Skill 的请求体
#[derive(Debug, Deserialize)]
pub struct RequestDelistBody {
    pub reason: Option<String>,
}
