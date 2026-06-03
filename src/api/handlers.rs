//! API Route Handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::api::auth::{require_identity_access, require_tenant_access, tenant_filter_for_user};
use crate::api::error::ApiError;
use crate::api::http_state::AppRouterState;
use crate::api::jwt::{AdminUser, AgentContext};
use crate::models::{NewSkill, SkillUpdate};
use crate::models::evaluation::{ErrorType as EvalErrorType, EvalTag};
use crate::models::error::AppError;

pub type ApiState = Arc<AppRouterState>;

pub async fn health_handler(State(state): State<ApiState>) -> Result<impl IntoResponse, ApiError> {
    let skills_count = state.registry.count().await.map_err(|e| ApiError::InternalError(e.to_string()))?;
    let response = crate::api::models::HealthResponse {
        status: "OK".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        skills_count: skills_count as usize,
    };
    Ok((StatusCode::OK, Json(response)))
}

// Tenant handlers

pub async fn list_tenants_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let is_super = state
        .permission
        .is_super_admin_user(user.identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if !is_super {
        return Err(ApiError::Forbidden("super_admin only".to_string()));
    }
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);
    let tenants = state.tenant.list(limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": tenants }))))
}

pub async fn create_tenant_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::CreateTenantBody>,
) -> Result<impl IntoResponse, ApiError> {
    let is_super = state
        .permission
        .is_super_admin_user(user.identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if !is_super {
        return Err(ApiError::Forbidden("super_admin only".to_string()));
    }
    let tenant = state.tenant.create(body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(tenant).unwrap())))
}

pub async fn get_tenant_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_tenant_access(&state, &user, id).await?;
    let tenant = state.tenant.get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Tenant not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(tenant).unwrap())))
}

pub async fn update_tenant_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateTenantBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_tenant_access(&state, &user, id).await?;
    let tenant = state.tenant.update(id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(tenant).unwrap())))
}

pub async fn delete_tenant_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_tenant_access(&state, &user, id).await?;
    state.tenant.delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

// Identity handlers

pub async fn list_identities_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);
    let (is_super, allowed) = tenant_filter_for_user(&state, &user).await?;
    let identities = if is_super {
        state.identity.list(limit, offset, None).await
    } else {
        state.identity.list_by_tenants(&allowed, limit, offset).await
    }
    .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": identities }))))
}

pub async fn create_identity_handler(
    _user: AdminUser,
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::CreateIdentityBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity = state.identity.create(body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(identity).unwrap())))
}

pub async fn get_identity_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let identity = state.identity.get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Identity not found".to_string()))?;
    require_identity_access(&state, &user, identity.id).await?;
    Ok((StatusCode::OK, Json(serde_json::to_value(identity).unwrap())))
}

pub async fn update_identity_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateIdentityBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity = state.identity.get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Identity not found".to_string()))?;
    require_identity_access(&state, &user, identity.id).await?;
    let updated = state.identity.update(id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(updated).unwrap())))
}

pub async fn delete_identity_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let identity = state.identity.get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Identity not found".to_string()))?;
    require_identity_access(&state, &user, identity.id).await?;
    state.identity.delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

// Group handlers

/// Resolve the tenant_id that owns a group, via the join path
/// `groups.organization_id -> organizations.id -> organizations.tenant_id`.
/// The Group model has no direct `tenant_id`, so the access check has to
/// walk one hop through the parent organization (mirrors the identity
/// pattern from Task 7, which walks through `org_memberships`).
async fn group_tenant_id(state: &ApiState, group_id: Uuid) -> Result<Uuid, ApiError> {
    let group = state
        .group
        .get(group_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Group not found".to_string()))?;
    let org = state
        .organization
        .get_org(group.organization_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    org.tenant_id
        .ok_or_else(|| ApiError::InternalError("Organization has no tenant".to_string()))
}

pub async fn list_groups_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::ListGroupsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    let (is_super, allowed) = tenant_filter_for_user(&state, &user).await?;
    let groups = if is_super {
        state.group.list().await
    } else {
        state.group.list_by_org_tenants(&allowed, limit, offset).await
    }
    .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": groups }))))
}

pub async fn create_group_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    let org_id = body.organization_id;
    let org = state
        .organization
        .get_org(org_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let tenant_id = org
        .tenant_id
        .ok_or_else(|| ApiError::InternalError("Organization has no tenant".to_string()))?;
    require_tenant_access(&state, &user, tenant_id).await?;

    let permission_overrides = body.permission_overrides.clone();
    let new_group: crate::models::group::NewGroup = body.into();
    let group = state.group.create(new_group)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if let Some(overrides) = permission_overrides {
        let creator_id = uuid::Uuid::parse_str(&subject).ok();
        for ov in overrides {
            state.group_perm_override_repo
                .upsert_override(crate::models::group_permission_override::NewGroupPermissionOverride {
                    group_id: group.id,
                    role_name: ov.role_name,
                    permission_code: ov.permission_code,
                    granted: ov.granted,
                    created_by: creator_id,
                })
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        }
    }

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_created".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group.id.to_string()),
            details: serde_json::json!({
                "group_name": group.name,
                "organization_id": group.organization_id,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::to_value(group).unwrap())))
}

pub async fn get_group_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let tenant_id = group_tenant_id(&state, id).await?;
    require_tenant_access(&state, &user, tenant_id).await?;
    let group = state.group.get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Group not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn update_group_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    let tenant_id = group_tenant_id(&state, id).await?;
    require_tenant_access(&state, &user, tenant_id).await?;
    let group = state.group.update(id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn delete_group_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let tenant_id = group_tenant_id(&state, id).await?;
    require_tenant_access(&state, &user, tenant_id).await?;
    state.group.delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

// Role handlers

pub async fn list_roles_handler(
    State(state): State<ApiState>,
) -> Result<impl IntoResponse, ApiError> {
    let roles = state.role.list()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": roles }))))
}

pub async fn get_role_handler(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let role = state.role.get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Role not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(role).unwrap())))
}

// API Key handlers

/// Resolve the tenant_id that owns an api key, via the join path
/// `api_keys.organization_id -> organizations.id ->
/// organizations.tenant_id`. The ApiKey model has no direct
/// `tenant_id`, so the access check has to walk one hop through the
/// parent organization (mirrors the group pattern from Task 8 and the
/// identity pattern from Task 7).
async fn api_key_tenant_id(state: &ApiState, api_key_id: Uuid) -> Result<Uuid, ApiError> {
    let api_key = state
        .api_key
        .get(api_key_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("API key not found".to_string()))?;
    let org = state
        .organization
        .get_org(api_key.organization_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    org.tenant_id
        .ok_or_else(|| ApiError::BadRequest("API key's organization has no tenant".to_string()))
}

pub async fn list_api_keys_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::ListApiKeysQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (is_super, allowed) = tenant_filter_for_user(&state, &user).await?;
    let keys = if is_super {
        if let Some(identity_id) = query.identity_id {
            state.api_key.list_by_identity(identity_id).await
        } else if let Some(org_id) = query.organization_id {
            state.api_key.list_by_organization(org_id).await
        } else {
            state.api_key.list().await
        }
    } else {
        // Non-super: tenant filter is the strongest restriction. The
        // optional identity_id / organization_id filters are dropped
        // intentionally — the caller's accessible tenants are already
        // narrower than those filters in any reasonable case, and
        // adding extra WHERE clauses to list_by_tenants would make the
        // SQL branchy for little operational value.
        let _ = query;
        state.api_key.list_by_tenants(&allowed, 200, 0).await
    }
    .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": keys }))))
}

pub async fn create_api_key_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::CreateApiKeyBody>,
) -> Result<impl IntoResponse, ApiError> {
    let request: crate::models::api_key::CreateApiKeyRequest = body.into();
    let org = state
        .organization
        .get_org(request.organization_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let tenant_id = org
        .tenant_id
        .ok_or_else(|| ApiError::BadRequest("Organization has no tenant".to_string()))?;
    require_tenant_access(&state, &user, tenant_id).await?;
    let key = state.api_key.create(request)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(key).unwrap())))
}

pub async fn delete_api_key_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let tenant_id = api_key_tenant_id(&state, id).await?;
    require_tenant_access(&state, &user, tenant_id).await?;
    state.api_key.delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

// User-facing self-service API Key handlers (6.5)

pub async fn list_my_api_keys_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let keys = state.api_key.list_by_identity(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": keys }))))
}

pub async fn create_my_api_key_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateMyApiKeyBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let request = crate::models::api_key::CreateApiKeyRequest {
        identity_id,
        organization_id: body.organization_id,
        name: body.name,
        scopes: body.scopes.unwrap_or_default(),
        rate_limit: body.rate_limit.unwrap_or(1000),
        expires_at: body.expires_at,
    };
    let key = state.api_key.create(request)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(key).unwrap())))
}

pub async fn revoke_my_api_key_handler(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let key = state.api_key.get(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("API Key not found".to_string()))?;

    if key.identity_id != identity_id {
        return Err(ApiError::Forbidden("Cannot revoke another user's API key".to_string()));
    }

    state.api_key.revoke(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"revoked": id}))))
}

// Audit entries handler

/// List audit log entries from the `audit_log_entries` (a.k.a.
/// `audit_logs` per the SQL in `db/repositories/api_key.rs`) table.
///
/// The table has a direct `tenant_id` column, so the tenant-scope
/// guard (Task 10) filters results to the caller's accessible
/// tenants. super_admin gets all tenants; everyone else gets
/// `tenant_id = ANY(allowed)`. The optional `organization_id` /
/// `identity_id` / `action` filters are honored (they narrow further
/// within the caller's tenants). The `tenant_id` query parameter is
/// intentionally dropped for non-super callers — the caller's
/// accessible tenants are the truth.
pub async fn list_audit_entries_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::ListAuditEntriesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (is_super, allowed) = tenant_filter_for_user(&state, &user).await?;
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    let entries = if is_super {
        let audit_query = crate::models::api_key::AuditLogQuery {
            tenant_id: query.tenant_id,
            organization_id: query.organization_id,
            identity_id: query.identity_id,
            action: query.action,
            resource_type: None,
            limit: Some(limit),
            offset: Some(offset),
        };
        state.audit.query(audit_query).await
    } else {
        // Non-super: tenant filter is the strongest restriction. The
        // optional organization_id / identity_id / action filters
        // narrow further within the caller's tenants and are passed
        // through. The tenant_id query parameter is dropped — the
        // caller's accessible tenants are the truth.
        let _ = query.tenant_id;
        state.audit.list_by_tenants(
            &allowed,
            query.organization_id,
            query.identity_id,
            query.action.as_deref(),
            limit,
            offset,
        )
        .await
    }
    .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": entries }))))
}

/// Sandbox Admin API Handlers

pub async fn list_sandboxes_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let sandboxes = state.sandbox.list_containers().await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": sandboxes }))))
}

pub async fn get_sandbox_health_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let docker_healthy = state.sandbox.health_check().await.unwrap_or(false);
    let containers = state.sandbox.list_containers().await.unwrap_or_default();

    let response = crate::api::models::SandboxHealthResponse {
        docker_connected: docker_healthy,
        active_containers: containers.len() as u32,
        containers: containers.into_iter().map(serde_json::to_value).filter_map(|r| r.ok()).collect(),
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn execute_tool_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::ExecuteToolBody>,
) -> Result<impl IntoResponse, ApiError> {
    let request = crate::services::ToolExecutionRequest {
        tool_id: body.tool_id,
        org_id: body.org_id,
        parameters: body.parameters,
        timeout_seconds: body.timeout_seconds.unwrap_or(30),
    };

    let result = state.sandbox.execute_org_tool(request).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "subject": subject,
        "result": result
    }))))
}

pub async fn remove_sandbox_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    state.sandbox.remove_sandbox(&key).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "removed": key }))))
}

/// Git Proxy Admin API Handlers

pub async fn list_git_branches_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(repo_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let branches = state.git_proxy.list_branches(&repo_id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": branches }))))
}

pub async fn get_git_commits_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((repo_id, limit)): Path<(String, u32)>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let commits = state.git_proxy.get_commits(&repo_id, limit).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": commits }))))
}

pub async fn get_git_file_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((repo_id, path, commit)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let file = state.git_proxy.get_file_at_commit(&repo_id, &path, &commit).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "path": file.path,
        "content": file.content,
        "size": file.size
    }))))
}

pub async fn get_git_diff_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((repo_id, from, to)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let diff = state.git_proxy.get_diff(&repo_id, &from, &to).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "from_commit": diff.from_commit,
        "to_commit": diff.to_commit,
        "files_changed": diff.files_changed,
        "additions": diff.additions,
        "deletions": diff.deletions
    }))))
}

pub async fn validate_git_url_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::ValidateGitUrlBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let valid = state.git_proxy.validate_git_url(&body.git_url).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "valid": valid }))))
}

pub async fn get_git_proxy_health_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let healthy = state.git_proxy.health_check().await.unwrap_or(false);

    let response = crate::api::models::GitProxyHealthResponse {
        git_proxy_connected: healthy,
        api_base: std::env::var("GIT_PROXY_API_BASE")
            .unwrap_or_else(|_| "http://localhost:8081".to_string()),
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn list_skills_handler(
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::ListSkillsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);

    let skills = state.registry.list_skills().await.map_err(|e| ApiError::InternalError(e.to_string()))?;

    let mut filtered: Vec<_> = skills;

    if let Some(ref tag) = query.tag {
        filtered.retain(|s| s.tags.iter().any(|t| t == tag));
    }

    if let Some(ref keyword) = query.keyword {
        let keyword_lower = keyword.to_lowercase();
        filtered.retain(|s| {
            s.name.to_lowercase().contains(&keyword_lower)
                || s.description.to_lowercase().contains(&keyword_lower)
        });
    }

    let total = filtered.len();
    let start = (page - 1) * page_size;
    let end = (start + page_size).min(total);

    let page_items: Vec<_> = if start < total {
        filtered[start..end].to_vec()
    } else {
        vec![]
    };

    let response = crate::api::models::ListResponse::new(page_items, total, page, page_size);
    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let skill = state.registry.get_skill(&skill_id).await
        .map_err(|_| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    let stats = state.evaluator.get_stats(&skill_id).ok();

    let detail = crate::models::SkillDetail {
        metadata: (&skill).into(),
        content: skill.content,
        stats,
    };
    Ok((StatusCode::OK, Json(detail)))
}

pub async fn create_skill_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    let visibility = body.visibility.as_ref().map(|v| match v.as_str() {
        "private" => crate::models::skill_policy::Visibility::Private,
        "org_visible" => crate::models::skill_policy::Visibility::OrgVisible,
        "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
        "shared" => crate::models::skill_policy::Visibility::Shared,
        _ => crate::models::skill_policy::Visibility::OrgVisible,
    });

    let new_skill = NewSkill {
        name: body.name,
        description: body.description,
        tags: body.tags,
        content: body.content,
        version: body.version.unwrap_or_else(|| "1.0.0".to_string()),
        git_url: body.git_url.clone(),
        visibility,
        tools: body.tools.clone(),
        owner_type: "user".to_string(),
        owner_id: None,
    };

    let skill = state.registry.create_skill(new_skill, &subject, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create skill: {}", e)))?;

    let response = crate::api::models::SkillCreatedResponse {
        message: "Skill created successfully".to_string(),
        skill_id: skill.id,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn update_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    let visibility = body.visibility.as_ref().map(|v| match v.as_str() {
        "private" => crate::models::skill_policy::Visibility::Private,
        "org_visible" => crate::models::skill_policy::Visibility::OrgVisible,
        "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
        "shared" => crate::models::skill_policy::Visibility::Shared,
        _ => crate::models::skill_policy::Visibility::OrgVisible,
    });

    let update = SkillUpdate {
        description: body.description,
        tags: body.tags,
        content: body.content,
        git_url: body.git_url.clone(),
        visibility,
        tools: body.tools.clone(),
    };

    state.registry.update_skill(&skill_id, update, &subject, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update skill: {}", e)))?;

    let response = crate::api::models::MessageResponse {
        message: "Skill updated successfully".to_string(),
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn delete_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    state.registry.delete_skill(&skill_id, &state.search).await
        .map_err(|e| ApiError::BadRequest(format!("Failed to delete skill: {}", e)))?;

    let response = crate::api::models::MessageResponse {
        message: "Skill deleted successfully".to_string(),
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_skill_stats_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state.evaluator.get_stats(&skill_id)
        .map_err(|_| ApiError::NotFound(format!("Skill {} not found or has no stats", skill_id)))?;

    Ok((StatusCode::OK, Json(stats)))
}

pub async fn create_evaluation_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateEvaluationBody>,
) -> Result<(StatusCode, Json<crate::api::models::EvaluationCreatedResponse>), ApiError> {
    let error_type = body.error_type.as_ref().and_then(|e| {
        match e.as_str() {
            "timeout" => Some(EvalErrorType::Timeout),
            "crash" => Some(EvalErrorType::Crash),
            "logic_error" => Some(EvalErrorType::LogicError),
            _ => Some(EvalErrorType::Other),
        }
    });

    let tags = body.tags.iter().filter_map(|t| {
        match t.as_str() {
            "reliable" => Some(EvalTag::Reliable),
            "fast" => Some(EvalTag::Fast),
            "stable" => Some(EvalTag::Stable),
            "experimental" => Some(EvalTag::Experimental),
            _ => None,
        }
    }).collect();

    let result = state.evaluator.add_evaluation(
        body.skill_id,
        subject,
        body.success,
        body.duration_ms,
        error_type,
        tags,
    )
    .await
    .map_err(|e: AppError| ApiError::BadRequest(e.to_string()))?;

    let response = crate::api::models::EvaluationCreatedResponse {
        message: "Evaluation recorded successfully".to_string(),
        evaluation_id: result.evaluation_id,
        new_stats: result.new_stats,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn register_agent_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::RegisterAgentBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::agent::NewAgent;

    let secret = uuid::Uuid::new_v4().to_string();

    let new_agent = NewAgent {
        agent_id: body.agent_id.clone(),
        agent_secret: secret.clone(),
        agent_name: body.agent_name.clone(),
        org_id: None,
        capabilities: None,
    };

    state.agent_repo
        .create(new_agent)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to register agent: {}", e)))?;

    let response = crate::api::models::RegisterAgentResponse {
        agent_id: body.agent_id,
        secret,
        message: "Agent registered successfully. Store the secret securely - it will not be shown again.".to_string(),
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_token_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::GetTokenBody>,
) -> Result<impl IntoResponse, ApiError> {
    let valid = state.agent_repo
        .verify_secret(&body.agent_id, &body.agent_secret)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Authentication error: {}", e)))?;

    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let token = crate::api::generate_token(&body.agent_id, &[], &[])
        .map_err(|e| ApiError::Unauthorized(format!("{:?}", e)))?;

    let response = crate::api::models::TokenResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: 86400,
    };
    Ok((StatusCode::OK, Json(response)))
}

/// Admin login handler for human administrators
pub async fn admin_login_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::AdminLoginBody>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::debug!("Admin login attempt for username: {}", body.username);

    let valid = state.identity.verify_password(&body.username, &body.password)
        .await
        .map_err(|e| {
            tracing::error!("verify_password error: {}", e);
            ApiError::Unauthorized(format!("Authentication error: {}", e))
        })?;

    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let user = state.identity.get_by_username(&body.username)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Failed to get user: {}", e)))?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    if !user.is_system_admin {
        return Err(ApiError::Unauthorized("Not a system administrator".to_string()));
    }

    let token = crate::api::generate_token_full(
        &user.id.to_string(),
        Some(user.id),
        true,
        &["admin"],
        &[],
    )
    .map_err(|e| ApiError::Unauthorized(format!("{:?}", e)))?;

    tracing::info!("Admin login success for username: {}", body.username);

    let response = crate::api::models::AdminLoginResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: 86400,
        user: crate::api::models::AdminUserInfo {
            id: user.id.to_string(),
            username: user.username.unwrap_or_else(|| user.name.clone()),
            display_name: user.display_name,
        },
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn user_login_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::UserLoginBody>,
) -> Result<impl IntoResponse, ApiError> {
    let valid = state.identity.verify_password(&body.username, &body.password)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Authentication error: {}", e)))?;

    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let user = state.identity.get_by_username(&body.username)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Failed to get user: {}", e)))?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    let token = crate::api::generate_token(&user.id.to_string(), &["user"], &[])
        .map_err(|e| ApiError::Unauthorized(format!("{:?}", e)))?;

    Ok((StatusCode::OK, Json(crate::api::models::UserLoginResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: 86400,
        user: crate::api::models::UserInfoResponse {
            id: user.id,
            username: user.username.unwrap_or_else(|| user.name.clone()),
            display_name: user.display_name,
            email: user.email,
            avatar_url: user.avatar_url,
            identity_type: user.identity_type.to_string(),
            created_at: user.created_at,
        },
    })))
}

pub async fn user_register_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::UserRegisterBody>,
) -> Result<impl IntoResponse, ApiError> {
    let existing = state.identity.get_by_username(&body.username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if existing.is_some() {
        return Err(ApiError::BadRequest(format!("Username '{}' already exists", body.username)));
    }

    let password_hash = bcrypt::hash(&body.password, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::BadRequest(format!("Failed to hash password: {}", e)))?;

    let new_identity = crate::models::identity::NewIdentity {
        identity_type: crate::models::identity::IdentityType::User,
        name: body.username.clone(),
        external_id: None,
        username: Some(body.username.clone()),
        display_name: body.display_name.clone().or(Some(body.username.clone())),
        email: body.email,
        avatar_url: None,
        password_hash: Some(password_hash),
        is_system_admin: false,
        metadata: None,
    };

    let user = state.identity.create(new_identity)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create user: {}", e)))?;

    let token = crate::api::generate_token(&user.id.to_string(), &["user"], &[])
        .map_err(|e| ApiError::InternalError(format!("{:?}", e)))?;

    Ok((StatusCode::CREATED, Json(crate::api::models::UserLoginResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: 86400,
        user: crate::api::models::UserInfoResponse {
            id: user.id,
            username: user.username.unwrap_or_else(|| user.name.clone()),
            display_name: user.display_name,
            email: user.email,
            avatar_url: user.avatar_url,
            identity_type: user.identity_type.to_string(),
            created_at: user.created_at,
        },
    })))
}

pub async fn get_user_me_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let user = state.identity.get(id)
        .await
        .map_err(|e| ApiError::NotFound(format!("User not found: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    Ok((StatusCode::OK, Json(crate::api::models::UserInfoResponse {
        id: user.id,
        username: user.username.unwrap_or_else(|| user.name.clone()),
        display_name: user.display_name,
        email: user.email,
        avatar_url: user.avatar_url,
        identity_type: if user.is_system_admin {
            "admin".to_string()
        } else {
            user.identity_type.to_string()
        },
        created_at: user.created_at,
    })))
}

pub async fn update_user_me_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateUserBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let password_hash = match body.password {
        Some(pw) => Some(bcrypt::hash(&pw, bcrypt::DEFAULT_COST)
            .map_err(|e| ApiError::BadRequest(format!("Failed to hash password: {}", e)))?),
        None => None,
    };

    let update = crate::models::identity::IdentityUpdate {
        display_name: body.display_name,
        email: body.email,
        avatar_url: body.avatar_url,
        password_hash,
        name: None,
        status: None,
        is_system_admin: None,
        metadata: None,
    };

    let user = state.identity.update(identity_id, update)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update user: {}", e)))?;

    Ok((StatusCode::OK, Json(crate::api::models::UserInfoResponse {
        id: user.id,
        username: user.username.unwrap_or_else(|| user.name.clone()),
        display_name: user.display_name,
        email: user.email,
        avatar_url: user.avatar_url,
        identity_type: user.identity_type.to_string(),
        created_at: user.created_at,
    })))
}

pub async fn get_user_orgs_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let pool = state.agent_repo.pool().clone();
    let org_membership_repo = OrgMembershipRepository::new(pool.clone());
    let org_repo = OrganizationRepository::new(pool);

    let orgs = org_membership_repo.list_user_organizations(identity_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list orgs: {}", e)))?;

    let mut result = Vec::new();
    for (org_id, role) in &orgs {
        let org = org_repo.find_by_id(*org_id)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to fetch org: {}", e)))?;

        result.push(crate::api::models::UserOrgResponse {
            id: *org_id,
            name: org.as_ref().map(|o| o.name.clone()).unwrap_or_else(|| "Unknown".to_string()),
            slug: org.and_then(|o| o.slug),
            role: role.clone(),
        });
    }

    Ok((StatusCode::OK, Json(result)))
}

pub async fn get_user_by_username_handler(
    State(state): State<ApiState>,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state.identity.get_by_username(&username)
        .await
        .map_err(|e| ApiError::NotFound(format!("User not found: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    Ok((StatusCode::OK, Json(crate::api::models::UserInfoResponse {
        id: user.id,
        username: user.username.unwrap_or_else(|| user.name.clone()),
        display_name: user.display_name,
        email: None,
        avatar_url: user.avatar_url,
        identity_type: user.identity_type.to_string(),
        created_at: user.created_at,
    })))
}

/// List audit logs from the legacy `audit_logs` table (migration 001).
///
/// **Limitation**: this table has no `tenant_id` column — only
/// `agent_id`, a free-form `VARCHAR(255)` that does not join to
/// identities or tenants. The response is therefore global. The
/// handler still requires an `AdminUser` token (any caller authorized
/// to mint admin tokens can read it), but cannot filter rows by
/// tenant at the SQL level. A future migration that adds a
/// `tenant_id` column should tighten `AuditRepository::list_by_tenants`
/// to `WHERE agent_id = ANY($1)` (after backfilling).
///
/// The handler follows the standard tenant-scope guard pattern
/// (Task 10) for symmetry: super_admin and non-super both end up at
/// the same query because the table can't be filtered, but the
/// AdminUser auth check still holds — non-admin callers cannot reach
/// this branch.
pub async fn list_audit_logs_handler(
    user: AdminUser,
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::AuditLogQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (is_super, allowed) = tenant_filter_for_user(&state, &user).await?;
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let (logs, total) = if is_super {
        let logs = state.audit_repo
            .list_with_filters(
                query.agent_id.as_deref(),
                query.action.as_deref(),
                query.resource_type.as_deref(),
                limit,
                offset,
            )
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        let total = state.audit_repo
            .count_with_filters(
                query.agent_id.as_deref(),
                query.action.as_deref(),
                query.resource_type.as_deref(),
            )
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        (logs, total)
    } else {
        // Legacy `audit_logs` table has no `tenant_id`, so
        // list_by_tenants / count_by_tenants fall back to the
        // unfiltered list. The AdminUser auth check still holds —
        // non-admin callers cannot reach this branch. See the
        // handler doc comment for the full limitation note.
        let logs = state.audit_repo
            .list_by_tenants(
                &allowed,
                query.agent_id.as_deref(),
                query.action.as_deref(),
                query.resource_type.as_deref(),
                limit,
                offset,
            )
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        let total = state.audit_repo
            .count_by_tenants(
                &allowed,
                query.agent_id.as_deref(),
                query.action.as_deref(),
                query.resource_type.as_deref(),
            )
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        (logs, total)
    };

    let data: Vec<_> = logs.into_iter().map(|log| {
        crate::api::models::AuditLogResponse {
            id: log.id.to_string(),
            agent_id: log.agent_id,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            details: log.details,
            timestamp: log.timestamp.to_rfc3339(),
        }
    }).collect();

    Ok((StatusCode::OK, Json(crate::api::models::AuditLogListResponse {
        data,
        total,
        limit,
        offset,
    })))
}

pub async fn list_my_audit_logs_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Query(query): Query<crate::api::models::AuditLogQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let logs = state.audit_repo
        .list_with_filters(Some(&subject), query.action.as_deref(), query.resource_type.as_deref(), limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total = state.audit_repo
        .count_with_filters(Some(&subject), query.action.as_deref(), query.resource_type.as_deref())
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<_> = logs.into_iter().map(|log| {
        crate::api::models::AuditLogResponse {
            id: log.id.to_string(),
            agent_id: log.agent_id,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            details: log.details,
            timestamp: log.timestamp.to_rfc3339(),
        }
    }).collect();

    Ok((StatusCode::OK, Json(crate::api::models::AuditLogListResponse {
        data,
        total,
        limit,
        offset,
    })))
}

pub async fn approve_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo.find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    let reviewer_id = Uuid::parse_str(&agent_context.subject).ok();

    if let (Some(author_id), Some(reviewer_id_val)) = (skill.author_identity_id, reviewer_id) {
        if author_id == reviewer_id_val {
            return Err(ApiError::Forbidden("Cannot approve your own skill submission".to_string()));
        }
    }

    skill_repo.update_status(&skill_id, "published")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve skill: {}", e)))?;

    skill_repo.update_review_status(&skill_id, "approved", reviewer_id, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update review status: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject.clone()),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "approved"}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(crate::api::models::SkillReviewResponse {
        message: "Skill approved successfully".to_string(),
        skill_id,
    })))
}

pub async fn reject_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::RejectSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo.find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    let reviewer_id = Uuid::parse_str(&agent_context.subject).ok();

    if let (Some(author_id), Some(reviewer_id_val)) = (skill.author_identity_id, reviewer_id) {
        if author_id == reviewer_id_val {
            return Err(ApiError::Forbidden("Cannot reject your own skill submission".to_string()));
        }
    }

    skill_repo.update_status(&skill_id, "rejected")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject skill: {}", e)))?;

    skill_repo.update_review_status(&skill_id, "rejected", reviewer_id, body.reason.as_deref())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update review status: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject.clone()),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "rejected", "reason": body.reason}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(crate::api::models::SkillReviewResponse {
        message: "Skill rejected".to_string(),
        skill_id,
    })))
}

pub async fn submit_review_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::SubmitSkillReviewBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    skill_repo.update_status(&skill_id, "in_review")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to submit skill for review: {}", e)))?;

    skill_repo.update_review_status(&skill_id, "pending", None, body.comment.as_deref())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update review status: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_submitted_for_review".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"comment": body.comment}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(crate::api::models::SkillReviewResponse {
        message: "Skill submitted for review".to_string(),
        skill_id,
    })))
}

pub async fn publish_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo.find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.review_status != "approved" {
        return Err(ApiError::BadRequest("Skill must be approved before publishing".to_string()));
    }

    skill_repo.update_status(&skill_id, "published")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to publish skill: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_published".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(crate::api::models::SkillReviewResponse {
        message: "Skill published successfully".to_string(),
        skill_id,
    })))
}

pub async fn approve_org_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo.find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.review_status != "pending" && skill.status != "in_review" {
        return Err(ApiError::BadRequest("Skill must be in pending_review status to approve".to_string()));
    }

    let reviewer_id = Uuid::parse_str(&subject).ok();

    if let (Some(author_id), Some(reviewer_id_val)) = (skill.author_identity_id, reviewer_id) {
        if author_id == reviewer_id_val {
            return Err(ApiError::Forbidden("Cannot approve your own skill submission".to_string()));
        }
    }

    skill_repo.update_status(&skill_id, "published")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve skill: {}", e)))?;

    skill_repo.update_review_status(&skill_id, "approved", reviewer_id, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update review status: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "approved"}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(crate::api::models::SkillReviewResponse {
        message: "Skill approved successfully".to_string(),
        skill_id,
    })))
}

pub async fn reject_org_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::RejectSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo.find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.review_status != "pending" && skill.status != "in_review" {
        return Err(ApiError::BadRequest("Skill must be in pending_review status to reject".to_string()));
    }

    let reviewer_id = Uuid::parse_str(&subject).ok();

    if let (Some(author_id), Some(reviewer_id_val)) = (skill.author_identity_id, reviewer_id) {
        if author_id == reviewer_id_val {
            return Err(ApiError::Forbidden("Cannot reject your own skill submission".to_string()));
        }
    }

    skill_repo.update_status(&skill_id, "rejected")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject skill: {}", e)))?;

    skill_repo.update_review_status(&skill_id, "rejected", reviewer_id, body.reason.as_deref())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update review status: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "rejected", "reason": body.reason}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(crate::api::models::SkillReviewResponse {
        message: "Skill rejected".to_string(),
        skill_id,
    })))
}

pub async fn marketplace_handler(
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::MarketplaceQuery>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let skills = skill_repo.list_by_visibility("marketplace", limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(skills)))
}

pub async fn install_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo.find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    skill_repo.increment_install_count(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to install skill: {}", e)))?;

    Ok((StatusCode::OK, Json(crate::api::models::InstallSkillResponse {
        message: "Skill installed successfully".to_string(),
        skill_id: skill.id.clone(),
        install_count: skill.install_count + 1,
    })))
}

pub async fn list_skill_groups_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupSkillRepository::new(pool);

    let associations = repo.list_by_skill(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let responses: Vec<crate::api::models::SkillGroupResponse> = associations
        .into_iter()
        .map(|a| crate::api::models::SkillGroupResponse {
            skill_id: a.skill_id,
            group_id: a.group_id,
            group_name: String::new(),
            added_at: a.added_at,
        })
        .collect();

    Ok((StatusCode::OK, Json(serde_json::to_value(responses).unwrap())))
}

pub async fn add_skill_to_group_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::AddSkillToGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    use crate::models::group_skill::NewGroupSkill;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupSkillRepository::new(pool);

    repo.associate_skill(NewGroupSkill {
        group_id: body.group_id,
        skill_id: skill_id.clone(),
        added_by: None,
    })
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to add skill to group: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_added_to_group".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"group_id": body.group_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "message": "Skill added to group",
        "skill_id": skill_id,
        "group_id": body.group_id,
    }))))
}

pub async fn remove_skill_from_group_handler(
    State(state): State<ApiState>,
    Path((skill_id, group_id)): Path<(String, Uuid)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupSkillRepository::new(pool);

    repo.dissociate_skill(group_id, &skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove skill from group: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_removed_from_group".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"group_id": group_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Skill removed from group",
        "skill_id": skill_id,
        "group_id": group_id,
    }))))
}

// v0.4 multi-tenant handlers

use uuid::Uuid;

/// Organization handlers

pub async fn create_org_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::CreateOrgBody>,
) -> Result<impl IntoResponse, ApiError> {
    let org = state.organization.create_org(body.name, body.slug, body.display_name, body.description, body.tenant_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::to_value(org).unwrap())))
}

pub async fn get_org_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let org = state.organization.get_org(org_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn list_orgs_handler(
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::ListOrgsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let orgs = if let Some(tenant_id) = query.tenant_id {
        state.organization.list_orgs_by_tenant(tenant_id, limit, offset)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state.organization.list_orgs(limit, offset)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": orgs }))))
}

pub async fn update_org_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateOrgBody>,
) -> Result<impl IntoResponse, ApiError> {
    let org = state.organization.update_org(org_id, body.name, body.display_name, body.description)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn delete_org_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state.organization.delete_org(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": org_id}))))
}

/// Organization member handlers

pub async fn list_org_members_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let agents = state.agent_repo
        .find_by_org(org_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let members: Vec<_> = agents.into_iter().map(|a| {
        crate::api::models::OrgMemberResponse {
            agent_id: a.agent_id,
            name: a.agent_name,
            capabilities: a.capabilities,
            joined_at: a.created_at.to_rfc3339(),
        }
    }).collect();

    Ok((StatusCode::OK, Json(crate::api::models::OrgMemberListResponse { members })))
}

pub async fn add_org_member_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AddOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    use crate::db::repositories::agent::NewAgent;

    let secret = uuid::Uuid::new_v4().to_string();

    let new_agent = NewAgent {
        agent_id: body.agent_id.clone(),
        agent_secret: secret,
        agent_name: body.name.clone(),
        org_id: Some(org_id),
        capabilities: Some(Vec::<String>::new()),
    };

    state.agent_repo
        .create(new_agent)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "message": "Member added successfully",
        "agent_id": body.agent_id
    }))))
}

pub async fn remove_org_member_handler(
    State(state): State<ApiState>,
    Path((_org_id, subject)): Path<(Uuid, String)>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    state.agent_repo
        .update_org(&subject, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"removed": subject}))))
}

pub async fn get_org_stats_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let pool = state.agent_repo.pool();

    let members_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM agents WHERE org_id = $1"
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let skills_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE author_subject IN (SELECT subject FROM agents WHERE org_id = $1)"
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let sessions_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sessions WHERE org_id = $1"
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let tools_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM org_tools WHERE org_id = $1"
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let response = crate::api::models::OrgStatsResponse {
        org_id,
        members_count,
        skills_count,
        sessions_count,
        tools_count,
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_org_by_slug_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = OrganizationRepository::new(pool);

    let org = repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn create_org_skill_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateOrgSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let membership = org_membership_repo.get_member(identity_id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::Unauthorized("Not a member of this organization".to_string()))?;

    let owner_type = body.owner_type.unwrap_or_else(|| "organization".to_string());

    if owner_type == "organization" {
        let role_str = membership.role.to_string();
        if role_str != "owner" && role_str != "admin" && role_str != "developer" {
            return Err(ApiError::Unauthorized("Need developer role to create org skills".to_string()));
        }
    }

    let visibility = body.visibility.as_ref().map(|v| match v.as_str() {
        "private" => crate::models::skill_policy::Visibility::Private,
        "org_visible" => crate::models::skill_policy::Visibility::OrgVisible,
        "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
        _ => crate::models::skill_policy::Visibility::OrgVisible,
    });

    let new_skill = crate::models::NewSkill {
        name: body.name,
        description: body.description,
        tags: body.tags,
        content: body.content,
        version: body.version.unwrap_or_else(|| "1.0.0".to_string()),
        git_url: body.git_url.clone(),
        visibility,
        tools: body.tools.clone(),
        owner_type,
        owner_id: Some(org.id),
    };

    let skill = state.registry.create_skill(new_skill, &subject, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create skill: {}", e)))?;

    let response = crate::api::models::SkillCreatedResponse {
        message: "Skill created successfully".to_string(),
        skill_id: skill.id,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn invite_org_member_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::InviteOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let target_identity = state.identity.get_by_email(&body.email)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User with email '{}' not found", body.email)))?;

    let inviter_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid inviter subject".to_string()))?;

    let org_membership_repo = OrgMembershipRepository::new(pool.clone());
    org_membership_repo.add_member(target_identity.id, org.id, body.role.as_str().into(), Some(inviter_id))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "message": format!("{} added to {}", body.email, slug),
        "organization_id": org.id,
        "identity_id": target_identity.id,
        "role": body.role,
    }))))
}

pub async fn invite_org_member_by_id_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<uuid::Uuid>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::InviteOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo.find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    let target_identity = state.identity.get_by_email(&body.email)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User with email '{}' not found", body.email)))?;

    let inviter_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid inviter subject".to_string()))?;

    let org_membership_repo = OrgMembershipRepository::new(pool.clone());
    org_membership_repo.add_member(target_identity.id, org.id, body.role.as_str().into(), Some(inviter_id))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "message": format!("{} added to organization {}", body.email, org_id),
        "organization_id": org.id,
        "identity_id": target_identity.id,
        "role": body.role,
    }))))
}

pub async fn update_org_member_handler(
    State(state): State<ApiState>,
    Path((slug, username)): Path<(String, String)>,
    Json(body): Json<crate::api::models::UpdateOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let identity = state.identity.get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo.update_role(identity.id, org.id, body.role.as_str().into())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update role: {}", e)))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": format!("{} role updated in {}", username, slug),
        "role": body.role,
    }))))
}

pub async fn remove_org_member_by_slug_handler(
    State(state): State<ApiState>,
    Path((slug, username)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let identity = state.identity.get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo.remove_member(identity.id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": format!("{} removed from {}", username, slug),
    }))))
}

pub async fn update_org_member_by_id_handler(
    State(state): State<ApiState>,
    Path((org_id, username)): Path<(uuid::Uuid, String)>,
    Json(body): Json<crate::api::models::UpdateOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo.find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    let identity = state.identity.get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo.update_role(identity.id, org.id, body.role.as_str().into())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update role: {}", e)))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": format!("{} role updated in {}", username, org_id),
        "role": body.role,
    }))))
}

pub async fn remove_org_member_by_id_handler(
    State(state): State<ApiState>,
    Path((org_id, username)): Path<(uuid::Uuid, String)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo.find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    let identity = state.identity.get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo.remove_member(identity.id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": format!("{} removed from {}", username, org_id),
    }))))
}

pub async fn list_org_members_by_slug_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let members = org_membership_repo.list_members(org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list members: {}", e)))?;

    Ok((StatusCode::OK, Json(members)))
}

pub async fn list_org_members_by_id_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo.find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let members = org_membership_repo.list_members(org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list members: {}", e)))?;

    Ok((StatusCode::OK, Json(members)))
}

pub async fn list_org_skills_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let skill_repo = SkillRepository::new(pool);
    let skills = skill_repo.list_by_org(&org.id.to_string())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list skills: {}", e)))?;

    Ok((StatusCode::OK, Json(skills)))
}

pub async fn list_org_reviews_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let skill_repo = SkillRepository::new(pool);
    let skills = skill_repo.list_by_org(&org.id.to_string())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list skills: {}", e)))?;

    let in_review: Vec<_> = skills.into_iter()
        .filter(|s| s.review_status.as_str() == "pending" || s.status == "in_review")
        .collect();

    Ok((StatusCode::OK, Json(in_review)))
}

/// Session handlers

pub async fn create_session_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::CreateSessionBody>,
) -> Result<impl IntoResponse, ApiError> {
    let session = state.session.create_session(body.agent_id, body.org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::to_value(session).unwrap())))
}

pub async fn get_session_handler(
    State(state): State<ApiState>,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let session = state.session.get_session(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    match session {
        Some(s) => Ok((StatusCode::OK, Json(serde_json::to_value(s).unwrap()))),
        None => Err(ApiError::NotFound(format!("Session {} not found", session_id))),
    }
}

pub async fn list_sessions_handler(
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::ListSessionsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    let status = query.status.as_deref();

    let sessions = state.session.list_sessions(limit, offset, status)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": sessions }))))
}

pub async fn end_session_handler(
    State(state): State<ApiState>,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state.session.end_session(session_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"ended": session_id}))))
}

pub async fn session_declare_handler(
    State(state): State<ApiState>,
    Path(session_id): Path<Uuid>,
    Json(body): Json<crate::api::models::SessionDeclareBody>,
) -> Result<impl IntoResponse, ApiError> {
    let router = state.session.declare_capabilities(session_id, body.capabilities)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(router).unwrap())))
}

/// Org Tool handlers

pub async fn register_org_tool_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::RegisterOrgToolBody>,
) -> Result<impl IntoResponse, ApiError> {
    let tool = state.org_tool.register_tool(
        body.org_id,
        body.tool_id,
        body.name,
        body.description,
        body.schema.unwrap_or(serde_json::json!({})),
        body.implementation.unwrap_or(serde_json::json!({})),
    )
    .await
    .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::to_value(tool).unwrap())))
}

pub async fn list_org_tools_handler(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Query(query): Query<crate::api::models::ListOrgToolsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    if let Some(approved_only) = query.approved_only {
        let tools = if approved_only {
            state.org_tool.list_approved_tools(id).await?
        } else {
            state.org_tool.list_org_tools(id).await?
        };
        Ok((StatusCode::OK, Json(serde_json::json!({ "data": tools }))))
    } else {
        let tool = state.org_tool.get_tool(id).await?;
        match tool {
            Some(t) => Ok((StatusCode::OK, Json(serde_json::json!({ "data": [t] })))),
            None => Err(ApiError::NotFound("Tool not found".to_string())),
        }
    }
}

pub async fn list_all_org_tools_handler(
    State(state): State<ApiState>,
) -> Result<impl IntoResponse, ApiError> {
    let tools = state.org_tool.list_all()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": tools }))))
}

pub async fn approve_org_tool_handler(
    State(state): State<ApiState>,
    Path(tool_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state.org_tool.approve_tool(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"approved": tool_id}))))
}

// Group member management handlers (6.6)

pub async fn list_group_members_handler(
    State(state): State<ApiState>,
    Path(group_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool);

    let members = repo.list_members(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list group members: {}", e)))?;

    let response: Vec<crate::api::models::GroupMemberInfo> = members
        .into_iter()
        .map(|m| crate::api::models::GroupMemberInfo {
            agent_id: m.identity_id.to_string(),
            name: m.identity_name,
            email: m.email,
            username: m.username,
            role: m.role,
            joined_at: m.joined_at,
        })
        .collect();

    Ok((StatusCode::OK, Json(response)))
}

pub async fn add_group_member_handler(
    State(state): State<ApiState>,
    Path(group_id): Path<Uuid>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::AddGroupMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool);

    let target_id = uuid::Uuid::parse_str(&body.agent_id)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let role = body.role.unwrap_or_else(|| "member".to_string());

    repo.add_member(target_id, group_id, &role)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add group member: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_added".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": body.agent_id, "role": role}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "message": "Member added to group",
        "group_id": group_id,
        "member_id": body.agent_id,
    }))))
}

pub async fn update_group_member_handler(
    State(state): State<ApiState>,
    Path((group_id, member_subject)): Path<(Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool);

    let target_id = uuid::Uuid::parse_str(&member_subject)
        .map_err(|_| ApiError::BadRequest("Invalid member subject".to_string()))?;

    repo.add_member(target_id, group_id, &body.role)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update group member: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": member_subject, "role": body.role}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Group member updated",
        "group_id": group_id,
        "member_id": member_subject,
    }))))
}

pub async fn remove_group_member_handler(
    State(state): State<ApiState>,
    Path((group_id, member_subject)): Path<(Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool);

    let target_id = uuid::Uuid::parse_str(&member_subject)
        .map_err(|_| ApiError::BadRequest("Invalid member subject".to_string()))?;

    repo.remove_member(target_id, group_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove group member: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_removed".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": member_subject}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Group member removed",
        "group_id": group_id,
        "member_id": member_subject,
    }))))
}

// Org slug-based Group management (6.6)

pub async fn create_org_group_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool);
    let org = org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let mut new_group: crate::models::group::NewGroup = body.into();
    new_group.organization_id = org.id;

    let group = state.group.create(new_group)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_created".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group.id.to_string()),
            details: serde_json::json!({"org_slug": slug}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::to_value(group).unwrap())))
}

pub async fn list_org_groups_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool);
    let org = org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let groups = state.group.list_by_organization(org.id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": groups }))))
}

pub async fn get_org_group_handler(
    State(state): State<ApiState>,
    Path((slug, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool);
    org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let group = state.group.get(group_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Group not found".to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn update_org_group_handler(
    State(state): State<ApiState>,
    Path((slug, group_id)): Path<(String, Uuid)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool);
    org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let group = state.group.update(group_id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn delete_org_group_handler(
    State(state): State<ApiState>,
    Path((slug, group_id)): Path<(String, Uuid)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool);
    org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    state.group.delete(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_deleted".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": group_id}))))
}

// Org slug-based Group member management (6.6)

pub async fn list_org_group_members_handler(
    State(state): State<ApiState>,
    Path((slug, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let repo = GroupRepository::new(pool);
    let members = repo.list_members(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let member_info: Vec<crate::api::models::GroupMemberInfo> = members
        .into_iter()
        .map(|m| crate::api::models::GroupMemberInfo {
            agent_id: m.identity_id.to_string(),
            name: m.identity_name,
            email: m.email,
            username: m.username,
            role: m.role,
            joined_at: m.joined_at,
        })
        .collect();

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": member_info }))))
}

pub async fn update_org_group_member_handler(
    State(state): State<ApiState>,
    Path((slug, group_id, username)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let target_id = uuid::Uuid::parse_str(&username)
        .map_err(|_| ApiError::BadRequest("Invalid member id".to_string()))?;

    let repo = GroupRepository::new(pool);
    repo.update_member_role(target_id, group_id, &body.role)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update group member: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_role_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "member_id": username, "role": body.role}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Group member role updated",
        "group_id": group_id,
        "member_id": username,
    }))))
}

pub async fn remove_org_group_member_handler(
    State(state): State<ApiState>,
    Path((slug, group_id, username)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let target_id = uuid::Uuid::parse_str(&username)
        .map_err(|_| ApiError::BadRequest("Invalid member id".to_string()))?;

    let repo = GroupRepository::new(pool);
    repo.remove_member(target_id, group_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove group member: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_removed".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "member_id": username}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Group member removed",
        "group_id": group_id,
        "member_id": username,
    }))))
}

// Org slug-based Group-Skill association (6.6)

pub async fn list_org_group_skills_handler(
    State(state): State<ApiState>,
    Path((slug, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let repo = GroupSkillRepository::new(pool);
    let skills = repo.list_by_group(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": skills }))))
}

pub async fn add_org_group_skill_handler(
    State(state): State<ApiState>,
    Path((slug, group_id)): Path<(String, Uuid)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::AddSkillToGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let skill_id = body.skill_id.clone()
        .ok_or_else(|| ApiError::BadRequest("skill_id is required".to_string()))?;

    let repo = GroupSkillRepository::new(pool);
    repo.associate_skill(crate::models::group_skill::NewGroupSkill {
        group_id,
        skill_id: skill_id.clone(),
        added_by: None,
    })
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to associate skill: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_skill_associated".to_string(),
            resource_type: "group_skill".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "skill_id": skill_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "message": "Skill associated to group",
        "group_id": group_id,
        "skill_id": skill_id,
    }))))
}

pub async fn remove_org_group_skill_handler(
    State(state): State<ApiState>,
    Path((slug, group_id, skill_id)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo.find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let repo = GroupSkillRepository::new(pool);
    repo.dissociate_skill(group_id, &skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to dissociate skill: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_skill_dissociated".to_string(),
            resource_type: "group_skill".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "skill_id": skill_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Skill dissociated from group",
        "group_id": group_id,
        "skill_id": skill_id,
    }))))
}

pub async fn reject_org_tool_handler(
    State(state): State<ApiState>,
    Path(tool_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state.org_tool.reject_tool(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"rejected": tool_id}))))
}

pub async fn delete_org_tool_handler(
    State(state): State<ApiState>,
    Path(tool_id): Path<Uuid>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    state.org_tool.delete(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": tool_id}))))
}

pub async fn get_admin_me_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let user = state.identity.get(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    let response = crate::api::models::AdminMeResponse {
        id: user.id.to_string(),
        username: user.username.unwrap_or_else(|| user.name.clone()),
        display_name: user.display_name,
        is_active: user.status == crate::models::identity::IdentityStatus::Active,
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_admin_stats_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let pool = state.agent_repo.pool();

    let total_skills = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM skills")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_agents = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agents")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_organizations = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM organizations")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_evaluations = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM evaluations")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let avg_success_rate = sqlx::query_scalar::<_, f64>(
        "SELECT COALESCE(AVG(CASE WHEN success THEN 1.0 ELSE 0.0 END), 0) FROM evaluations"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0.0);

    let response = crate::api::models::AdminStatsResponse {
        total_skills,
        total_agents,
        total_organizations,
        total_evaluations,
        average_success_rate: avg_success_rate,
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_admin_status_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let pool = state.agent_repo.pool();
    let db_connected = sqlx::query("SELECT 1").execute(pool).await.is_ok();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost:5432/aionhive".to_string());
    let sanitized_url = db_url
        .split('@')
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                if let Some(colon) = part.rfind(':') {
                    format!("{}:****", &part[..colon])
                } else {
                    part.to_string()
                }
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("@");

    let port: u16 = std::env::var("AION_HIVE_HTTP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    let transport_mode = std::env::var("AION_HIVE_TRANSPORT")
        .unwrap_or_else(|_| "http".to_string());

    let data_dir = std::env::var("AION_HIVE_DATA_DIR")
        .unwrap_or_else(|_| "./data".to_string());

    let skills_dir = std::env::var("AION_HIVE_SKILLS_DIR")
        .unwrap_or_else(|_| "./skills".to_string());

    let response = crate::api::models::AdminStatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        transport_mode,
        http_port: port,
        data_dir,
        skills_dir,
        db_connected,
        db_url: sanitized_url,
        jwt_expiry_hours: 24,
    };

    Ok((StatusCode::OK, Json(response)))
}

// Group permission override handlers

pub async fn list_group_default_permissions_handler(
    State(state): State<ApiState>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::role_permission::RolePermissionRepository;

    let pool = state.group_perm_override_repo.pool().clone();
    let role_perm_repo = RolePermissionRepository::new(pool);

    let lead_defaults = role_perm_repo
        .list_by_role("group", "lead")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let member_defaults = role_perm_repo
        .list_by_role("group", "member")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let to_codes = |perms: Vec<crate::models::role_permission::RolePermission>| -> Vec<String> {
        perms.into_iter().map(|p| p.permission_code).collect()
    };

    Ok((StatusCode::OK, Json(serde_json::json!({
        "lead": to_codes(lead_defaults),
        "member": to_codes(member_defaults),
    }))))
}

pub async fn list_group_permissions_handler(
    State(state): State<ApiState>,
    Path(group_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::role_permission::RolePermissionRepository;
    use crate::api::models::GroupPermissionInfo;

    let pool = state.group_perm_override_repo.pool().clone();

    let role_perm_repo = RolePermissionRepository::new(pool.clone());

    let lead_defaults = role_perm_repo
        .list_by_role("group", "lead")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let member_defaults = role_perm_repo
        .list_by_role("group", "member")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let overrides = state.group_perm_override_repo
        .list_by_group(group_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let is_overridden = |perm_code: &str| -> Option<bool> {
        overrides.iter().find(|o| o.permission_code == perm_code).map(|o| o.granted)
    };

    let to_info = |perms: Vec<crate::models::role_permission::RolePermission>| -> Vec<GroupPermissionInfo> {
        perms.into_iter().map(|p| {
            let code = p.permission_code;
            let override_granted = is_overridden(&code);
            GroupPermissionInfo {
                permission_code: code,
                granted: override_granted.unwrap_or(true),
                is_default: override_granted.is_none(),
            }
        }).collect()
    };

    Ok((StatusCode::OK, Json(serde_json::json!({
        "lead": to_info(lead_defaults),
        "member": to_info(member_defaults),
    }))))
}

pub async fn update_group_permission_handler(
    State(state): State<ApiState>,
    Path(group_id): Path<Uuid>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupPermissionBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::models::group_permission_override::NewGroupPermissionOverride;

    let role_name = body.role_name.clone();
    let permission_code = body.permission_code.clone();

    let creator_id = uuid::Uuid::parse_str(&subject).ok();

    state.group_perm_override_repo
        .upsert_override(NewGroupPermissionOverride {
            group_id,
            role_name: body.role_name,
            permission_code: body.permission_code,
            granted: body.granted,
            created_by: creator_id,
        })
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_permission_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({
                "role_name": role_name,
                "permission_code": permission_code,
                "granted": body.granted,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Group permission override updated"
    }))))
}

pub async fn delete_group_permission_handler(
    State(state): State<ApiState>,
    Path((group_id, permission_code)): Path<(Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupPermissionBody>,
) -> Result<impl IntoResponse, ApiError> {
    state.group_perm_override_repo
        .delete_override(group_id, &body.role_name, &permission_code)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let role_name = body.role_name.clone();

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_permission_deleted".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({
                "role_name": role_name,
                "permission_code": permission_code,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Group permission override deleted"
    }))))
}
