//! API Route Handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::api::error::ApiError;
use crate::api::http_state::AppRouterState;
use crate::api::jwt::AgentContext;
use crate::models::error::AppError;
use crate::models::evaluation::{ErrorType as EvalErrorType, EvalTag};
use crate::models::{NewSkill, SkillUpdate};

pub type ApiState = Arc<AppRouterState>;

pub async fn health_handler(State(state): State<ApiState>) -> Result<impl IntoResponse, ApiError> {
    let skills_count = state
        .registry
        .count()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let response = crate::api::models::HealthResponse {
        status: "OK".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        skills_count: skills_count as usize,
    };
    Ok((StatusCode::OK, Json(response)))
}

// Tenant handlers

pub async fn list_tenants_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);
    let tenants = state
        .tenant
        .list(limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": tenants }))))
}

pub async fn create_tenant_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateTenantBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let tenant = state
        .tenant
        .create(body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(tenant).unwrap()),
    ))
}

pub async fn get_tenant_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let tenant = state
        .tenant
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Tenant not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(tenant).unwrap())))
}

pub async fn update_tenant_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateTenantBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let tenant = state
        .tenant
        .update(id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(tenant).unwrap())))
}

pub async fn delete_tenant_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    state
        .tenant
        .delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

// Identity handlers

pub async fn list_identities_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);
    let identities = state
        .identity
        .list(limit, offset, None)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": identities })),
    ))
}

pub async fn create_identity_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateIdentityBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let identity = state
        .identity
        .create(body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(identity).unwrap()),
    ))
}

pub async fn get_identity_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let identity = state
        .identity
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Identity not found".to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::to_value(identity).unwrap()),
    ))
}

pub async fn update_identity_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateIdentityBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let identity = state
        .identity
        .update(id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::to_value(identity).unwrap()),
    ))
}

pub async fn delete_identity_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    state
        .identity
        .delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

// Group handlers

pub async fn list_groups_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListGroupsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let org_id = query.organization_id;
    let groups = if let Some(org_id) = org_id {
        state
            .group
            .list_by_organization(org_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state
            .group
            .list()
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": groups }))))
}

pub async fn create_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let subject = agent_context.subject;
    let permission_overrides = body.permission_overrides.clone();
    let new_group: crate::models::group::NewGroup = body.into();
    let group = state
        .group
        .create(new_group)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if let Some(overrides) = permission_overrides {
        let creator_id = uuid::Uuid::parse_str(&subject).ok();
        for ov in overrides {
            state
                .group_perm_override_repo
                .upsert_override(
                    crate::models::group_permission_override::NewGroupPermissionOverride {
                        group_id: group.id,
                        role_name: ov.role_name,
                        permission_code: ov.permission_code,
                        granted: ov.granted,
                        created_by: creator_id,
                    },
                )
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        }
    }

    state
        .audit_repo
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

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(group).unwrap()),
    ))
}

pub async fn get_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let group = state
        .group
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Group not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn update_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let group = state
        .group
        .update(id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn delete_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    state
        .group
        .delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

// Role handlers

pub async fn list_roles_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let roles = state
        .role
        .list()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": roles }))))
}

pub async fn get_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let role = state
        .role
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Role not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(role).unwrap())))
}

// API Key handlers

pub async fn list_api_keys_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListApiKeysQuery>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let identity_id = query.identity_id;
    let org_id = query.organization_id;
    let keys = if let Some(identity_id) = identity_id {
        state
            .api_key
            .list_by_identity(identity_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else if let Some(org_id) = org_id {
        state
            .api_key
            .list_by_organization(org_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state
            .api_key
            .list()
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": keys }))))
}

pub async fn create_api_key_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateApiKeyBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let request: crate::models::api_key::CreateApiKeyRequest = body.into();
    let key = state
        .api_key
        .create(request)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(key).unwrap()),
    ))
}

pub async fn delete_api_key_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    state
        .api_key
        .delete(id)
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

    let keys = state
        .api_key
        .list_by_identity(identity_id)
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
    let key = state
        .api_key
        .create(request)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(key).unwrap()),
    ))
}

pub async fn revoke_my_api_key_handler(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let key = state
        .api_key
        .get(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("API Key not found".to_string()))?;

    if key.identity_id != identity_id {
        return Err(ApiError::Forbidden(
            "Cannot revoke another user's API key".to_string(),
        ));
    }

    state
        .api_key
        .revoke(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"revoked": id}))))
}

// Audit entries handler

pub async fn list_audit_entries_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListAuditEntriesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    let audit_query = crate::models::api_key::AuditLogQuery {
        tenant_id: query.tenant_id,
        organization_id: query.organization_id,
        identity_id: query.identity_id,
        action: query.action,
        resource_type: None,
        limit: Some(limit),
        offset: Some(offset),
    };
    let entries = state
        .audit
        .query(audit_query)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": entries }))))
}

// Role CRUD handlers (C/U/D)

pub async fn create_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let new_role = crate::models::role::NewRole {
        name: body.name,
        role_type: crate::models::role::RoleType::from(body.role_type.as_str()),
        scope_level: crate::models::role::ScopeLevel::from(body.scope_level.as_str()),
        parent_role_id: body.parent_role_id,
        permissions: body.permissions,
        description: body.description,
    };
    let role = state
        .role
        .create(new_role)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(role).unwrap()),
    ))
}

pub async fn update_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let update = crate::models::role::RoleUpdate {
        name: body.name,
        permissions: body.permissions,
        description: body.description,
    };
    let role = state
        .role
        .update(id, update)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(role).unwrap())))
}

pub async fn delete_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    state
        .role
        .delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

// Identity role assignment handlers

pub async fn get_identity_roles_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let roles = state
        .role
        .get_identity_roles(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": roles }))))
}

pub async fn grant_identity_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::GrantRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let admin_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid admin subject".to_string()))?;
    let request = crate::models::role::GrantRoleRequest {
        identity_id: id,
        role_id: body.role_id,
        scope_id: body.scope_id,
        expires_at: body.expires_at,
    };
    let identity_role = state
        .role
        .grant_role(request, admin_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(identity_role).unwrap()),
    ))
}

pub async fn revoke_identity_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((identity_id, role_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<crate::api::models::RevokeRoleQuery>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    state
        .role
        .revoke_role(identity_id, role_id, query.scope_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"revoked": role_id})),
    ))
}

pub async fn get_identity_permissions_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let permissions = state
        .role
        .get_identity_permissions(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": permissions })),
    ))
}

// System role assignment handlers

pub async fn assign_system_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AssignSystemRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let admin_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid admin subject".to_string()))?;
    if !crate::models::system_role_assignment::SystemRole::is_valid(&body.role_name) {
        return Err(ApiError::BadRequest(format!(
            "Invalid system role: {}",
            body.role_name
        )));
    }
    let assignment = state
        .system_role_assignment
        .assign(body.identity_id, &body.role_name, Some(admin_id))
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(assignment).unwrap()),
    ))
}

pub async fn revoke_system_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::RevokeSystemRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    state
        .system_role_assignment
        .revoke(body.identity_id, &body.role_name)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"revoked": true}))))
}

pub async fn list_system_role_assignments_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListSystemRoleAssignmentsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let assignments = if let Some(identity_id) = query.identity_id {
        state
            .system_role_assignment
            .find_by_identity(identity_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else if let Some(role_name) = &query.role_name {
        state
            .system_role_assignment
            .list_by_role(role_name)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        return Err(ApiError::BadRequest(
            "Provide identity_id or role_name query parameter".to_string(),
        ));
    };
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": assignments })),
    ))
}

pub async fn get_identity_system_roles_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let assignments = state
        .system_role_assignment
        .find_by_identity(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": assignments })),
    ))
}

// Role permission management handlers

pub async fn list_role_permissions_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let permissions = if let (Some(role_level), Some(role_name)) =
        (query.get("role_level"), query.get("role_name"))
    {
        state
            .role_permission
            .list_by_role(role_level, role_name)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state
            .role_permission
            .list_all()
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": permissions })),
    ))
}

pub async fn create_role_permission_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateRolePermissionBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let new_perm = crate::models::role_permission::NewRolePermission {
        role_level: body.role_level,
        role_name: body.role_name,
        permission_code: body.permission_code,
        scope_restriction: body.scope_restriction,
    };
    let perm = state
        .role_permission
        .add_permission(new_perm)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(perm).unwrap()),
    ))
}

pub async fn delete_role_permission_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::DeleteRolePermissionQuery>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    state
        .role_permission
        .remove_permission(&query.role_level, &query.role_name, &query.permission_code)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": true}))))
}

// Permission check handlers

pub async fn check_permission_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::PermissionCheckBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;
    let ctx = state
        .permission
        .build_context(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let resource = crate::services::permission::ResourceScope {
        owner_type: body.owner_type,
        owner_id: body.owner_id,
        author_identity_id: body.author_identity_id,
        organization_id: body.organization_id,
        group_id: body.group_id,
    };
    let has_perm = state
        .permission
        .has_permission(&ctx, &body.permission_code, Some(&resource))
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"has_permission": has_perm})),
    ))
}

pub async fn get_permission_context_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;
    let ctx = state
        .permission
        .build_context(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "identity_id": ctx.identity_id,
            "system_roles": ctx.system_roles,
            "org_roles": ctx.org_roles,
            "group_roles": ctx.group_roles,
        })),
    ))
}

/// Sandbox Admin API Handlers

pub async fn list_sandboxes_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let sandboxes = state
        .sandbox
        .list_containers()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": sandboxes })),
    ))
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
        containers: containers
            .into_iter()
            .map(serde_json::to_value)
            .filter_map(|r| r.ok())
            .collect(),
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn execute_tool_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::ExecuteToolBody>,
) -> Result<impl IntoResponse, ApiError> {
    // Ensure the org tool exists and is approved before execution
    let org_id_uuid = Uuid::parse_str(&body.org_id)
        .map_err(|_| ApiError::BadRequest("Invalid org_id".to_string()))?;
    let tool = state
        .org_tool
        .get_tool_by_tool_id(org_id_uuid, &body.tool_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let tool = match tool {
        Some(t) if t.status == "approved" => t,
        Some(_) => {
            return Err(ApiError::Forbidden(
                "Tool must be approved before execution".to_string(),
            ));
        }
        None => {
            return Err(ApiError::NotFound(format!(
                "Tool {} not found in organization {}",
                body.tool_id, body.org_id
            )));
        }
    };

    // Read defaults from stored implementation config; request body can override
    let impl_docker = tool
        .implementation
        .get("docker_image")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let impl_timeout = tool
        .implementation
        .get("timeout_seconds")
        .and_then(|v| v.as_u64());
    let impl_cmd = tool
        .implementation
        .get("cmd")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        });

    let request = crate::services::ToolExecutionRequest {
        tool_id: body.tool_id,
        org_id: body.org_id,
        parameters: body.parameters,
        timeout_seconds: body.timeout_seconds.or(impl_timeout).unwrap_or(30),
        docker_image: body.docker_image.or(impl_docker),
        session_id: None,
        cmd: impl_cmd,
    };

    let result = state
        .sandbox
        .execute_org_tool(request)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "subject": subject,
            "result": result
        })),
    ))
}

pub async fn execute_platform_tool_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::ExecutePlatformToolBody>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .sandbox
        .execute_platform_tool(&body.tool_name, body.parameters, body.timeout_seconds)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "subject": subject,
            "result": result
        })),
    ))
}

pub async fn remove_sandbox_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    state
        .sandbox
        .remove_sandbox(&key)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "removed": key }))))
}

/// Release a sandbox by org_id + tool_id (non-admin, any authenticated user).
pub async fn release_sandbox_handler(
    State(state): State<ApiState>,
    AgentContext { .. }: AgentContext,
    Json(body): Json<crate::api::models::ReleaseSandboxBody>,
) -> Result<impl IntoResponse, ApiError> {
    let released = state
        .sandbox
        .release_sandbox(&body.org_id, &body.tool_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "released": released,
            "org_id": body.org_id,
            "tool_id": body.tool_id
        })),
    ))
}

/// List sandbox status (authenticated users, not admin-only).
pub async fn list_sandbox_status_handler(
    State(state): State<ApiState>,
    AgentContext { .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let now = chrono::Utc::now().timestamp();
    let sandboxes: Vec<crate::api::models::SandboxInfoItem> = state
        .sandbox
        .list_active_sandboxes()
        .into_iter()
        .map(|info| {
            let idle = now - info.last_used.timestamp();
            crate::api::models::SandboxInfoItem {
                key: info.id,
                container_id: info.container_id,
                image: info.image,
                status: info.status.to_string(),
                idle_seconds: idle,
                created_at: info.created_at.to_rfc3339(),
            }
        })
        .collect();

    let status = crate::api::models::SandboxStatusResponse {
        total: sandboxes.len(),
        max: state.sandbox.max_containers(),
        containers: sandboxes,
    };

    Ok((StatusCode::OK, Json(serde_json::json!(status))))
}

/// Git Proxy Admin API Handlers

pub async fn list_git_branches_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(repo_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let branches = state
        .git_proxy
        .list_branches(&repo_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": branches })),
    ))
}

pub async fn get_git_commits_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((repo_id, limit)): Path<(String, u32)>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let commits = state
        .git_proxy
        .get_commits(&repo_id, limit)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": commits }))))
}

pub async fn get_git_file_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((repo_id, path, commit)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let file = state
        .git_proxy
        .get_file_at_commit(&repo_id, &path, &commit)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "path": file.path,
            "content": file.content,
            "size": file.size
        })),
    ))
}

pub async fn get_git_diff_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((repo_id, from, to)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let diff = state
        .git_proxy
        .get_diff(&repo_id, &from, &to)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "from_commit": diff.from_commit,
            "to_commit": diff.to_commit,
            "files_changed": diff.files_changed,
            "additions": diff.additions,
            "deletions": diff.deletions
        })),
    ))
}

pub async fn validate_git_url_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::ValidateGitUrlBody>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let valid = state
        .git_proxy
        .validate_git_url(&body.git_url)
        .await
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
    AgentContext { subject: _, .. }: AgentContext,
    Query(query): Query<crate::api::models::ListSkillsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);

    let skills = state
        .registry
        .list_skills()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

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
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;

    let stats = state.evaluator.get_stats(&skill_id).await.ok();

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

    let skill = state
        .registry
        .create_skill(new_skill, &subject, &state.search)
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

    state
        .registry
        .update_skill(&skill_id, update, &subject, &state.search)
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
    state
        .registry
        .delete_skill(&skill_id, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to delete skill: {}", e)))?;

    let response = crate::api::models::MessageResponse {
        message: "Skill deleted successfully".to_string(),
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_skill_stats_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // 先确认 skill 存在
    state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // 获取统计；如果没有评价数据则返回默认值
    let stats = state
        .evaluator
        .get_stats(&skill_id)
        .await
        .unwrap_or_else(|_| crate::models::evaluation::SkillStats {
            skill_id: skill_id.clone(),
            success_rate: 0.0,
            avg_duration_ms: 0,
            total_evaluations: 0,
            unique_agents: 0,
            confidence: 0.0,
            tags: vec![],
            local_version: None,
            latest_version: "1.0.0".to_string(),
            upgrade_available: false,
        });

    Ok((StatusCode::OK, Json(stats)))
}

/// GET /api/v1/skills/:id/files — 列出 Skill 包中的所有文件
pub async fn list_skill_files_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;

    let files = state
        .skill_git
        .list_files_at_version(&skill.name, &skill.version)
        .unwrap_or_default();

    Ok((StatusCode::OK, Json(serde_json::json!({ "files": files }))))
}

/// GET /api/v1/skills/:id/files/*path — 获取 Skill 包中某个文件的内容
pub async fn get_skill_file_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    axum::extract::Path((skill_id, file_path)): axum::extract::Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;

    let content = state
        .skill_git
        .get_file_at_version(&skill.name, &skill.version, &file_path)
        .map_err(|e| ApiError::NotFound(format!("File '{}' not found: {}", file_path, e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "path": file_path, "content": content })),
    ))
}

pub async fn create_evaluation_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateEvaluationBody>,
) -> Result<
    (
        StatusCode,
        Json<crate::api::models::EvaluationCreatedResponse>,
    ),
    ApiError,
> {
    let error_type = body.error_type.as_ref().and_then(|e| match e.as_str() {
        "timeout" => Some(EvalErrorType::Timeout),
        "crash" => Some(EvalErrorType::Crash),
        "logic_error" => Some(EvalErrorType::LogicError),
        _ => Some(EvalErrorType::Other),
    });

    let tags = body
        .tags
        .iter()
        .filter_map(|t| match t.as_str() {
            "reliable" => Some(EvalTag::Reliable),
            "fast" => Some(EvalTag::Fast),
            "stable" => Some(EvalTag::Stable),
            "experimental" => Some(EvalTag::Experimental),
            _ => None,
        })
        .collect();

    let result = state
        .evaluator
        .add_evaluation(
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

    state
        .agent_repo
        .create(new_agent)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to register agent: {}", e)))?;

    let response = crate::api::models::RegisterAgentResponse {
        agent_id: body.agent_id,
        secret,
        message:
            "Agent registered successfully. Store the secret securely - it will not be shown again."
                .to_string(),
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_token_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::GetTokenBody>,
) -> Result<impl IntoResponse, ApiError> {
    let valid = state
        .agent_repo
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

/// 列出当前用户注册的所有 Agent
pub async fn list_my_agents_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = agent_context.require_identity()?;

    let agents = state
        .agent_repo
        .list_by_identity(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Agent listing error: {}", e)))?;

    let items: Vec<crate::api::models::AgentListItem> = agents
        .into_iter()
        .map(|a| crate::api::models::AgentListItem {
            agent_id: a.agent_id,
            agent_name: a.agent_name,
            agent_description: a.agent_description,
            status: a.status,
            created_at: a.created_at.to_string().into(),
            last_used_at: a.last_used_at.map(|t| t.to_string()),
        })
        .collect();

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": items }))))
}

/// 撤销一个 Agent Token
pub async fn revoke_my_agent_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(agent_id_str): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = agent_context.require_identity()?;

    let agent_id = uuid::Uuid::parse_str(&agent_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid agent ID format".to_string()))?;

    // 查找 agent 并验证归属
    let agent = state
        .agent_repo
        .find_by_uuid(agent_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Agent lookup error: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    if agent.identity_id != Some(identity_id) {
        return Err(ApiError::Forbidden(
            "You can only revoke your own agents".to_string(),
        ));
    }

    state
        .agent_repo
        .revoke(agent_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Agent revoke error: {}", e)))?;

    info!(
        "Agent revoked: agent_id={}, identity_id={}",
        agent_id, identity_id
    );

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "Agent revoked successfully" })),
    ))
}

/// Admin login handler for human administrators
pub async fn admin_login_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::AdminLoginBody>,
) -> Result<impl IntoResponse, ApiError> {
    let rate_key = format!("admin_login:{}", body.username);
    if !state.login_rate_limiter.check(&rate_key).await {
        return Err(ApiError::TooManyRequests(
            "Too many login attempts. Please try again later.".to_string(),
        ));
    }

    tracing::debug!("Admin login attempt for username: {}", body.username);

    let valid = state
        .identity
        .verify_password(&body.username, &body.password)
        .await
        .map_err(|e| {
            tracing::error!("verify_password error: {}", e);
            ApiError::Unauthorized(format!("Authentication error: {}", e))
        })?;

    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let user = state
        .identity
        .get_by_username(&body.username)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Failed to get user: {}", e)))?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    if !user.is_system_admin {
        return Err(ApiError::Unauthorized(
            "Not a system administrator".to_string(),
        ));
    }

    let token = crate::api::jwt::generate_identity_token(user.id, &["admin"], &[])
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
    let rate_key = format!("user_login:{}", body.username);
    if !state.login_rate_limiter.check(&rate_key).await {
        return Err(ApiError::TooManyRequests(
            "Too many login attempts. Please try again later.".to_string(),
        ));
    }

    let valid = state
        .identity
        .verify_password(&body.username, &body.password)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Authentication error: {}", e)))?;

    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let user = state
        .identity
        .get_by_username(&body.username)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Failed to get user: {}", e)))?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    let token = crate::api::jwt::generate_identity_token(user.id, &["user"], &[])
        .map_err(|e| ApiError::Unauthorized(format!("{:?}", e)))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserLoginResponse {
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
        }),
    ))
}

pub async fn user_register_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::UserRegisterBody>,
) -> Result<impl IntoResponse, ApiError> {
    let existing = state
        .identity
        .get_by_username(&body.username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if existing.is_some() {
        return Err(ApiError::BadRequest(format!(
            "Username '{}' already exists",
            body.username
        )));
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

    let user = state
        .identity
        .create(new_identity)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create user: {}", e)))?;

    let token = crate::api::jwt::generate_identity_token(user.id, &["user"], &[])
        .map_err(|e| ApiError::InternalError(format!("{:?}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(crate::api::models::UserLoginResponse {
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
        }),
    ))
}

// Password reset handlers

pub async fn forgot_password_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::ForgotPasswordBody>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state
        .identity
        .get_by_email(&body.email)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("No account found with that email".to_string()))?;

    // Generate short-lived reset token (60 minutes)
    let token =
        crate::api::jwt::generate_short_lived_token(&user.id.to_string(), "password_reset", 60)?;

    // In production, send this token via email
    // For now, return it directly (self-service for MVP)
    Ok((
        StatusCode::OK,
        Json(crate::api::models::ForgotPasswordResponse {
            message: "Password reset token generated. Use this token to reset your password."
                .to_string(),
            reset_token: token,
        }),
    ))
}

pub async fn reset_password_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::ResetPasswordBody>,
) -> Result<impl IntoResponse, ApiError> {
    // Verify the reset token
    let user_id_str = crate::api::jwt::verify_purpose_token(&body.token, "password_reset")?;
    let user_id = uuid::Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid token subject".to_string()))?;

    // Validate new password
    if body.new_password.len() < 8 {
        return Err(ApiError::BadRequest(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    // Hash new password
    let password_hash = bcrypt::hash(&body.new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::BadRequest(format!("Failed to hash password: {}", e)))?;

    // Update password
    let update = crate::models::identity::IdentityUpdate {
        password_hash: Some(password_hash),
        ..Default::default()
    };
    state
        .identity
        .update(user_id, update)
        .await
        .map_err(|_| ApiError::InternalError("Failed to update password".to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::ResetPasswordResponse {
            message: "Password has been reset successfully".to_string(),
        }),
    ))
}

// Account deletion

pub async fn delete_user_me_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    // Soft delete: set status to "deleted"
    let update = crate::models::identity::IdentityUpdate {
        status: Some(crate::models::identity::IdentityStatus::Deleted),
        ..Default::default()
    };
    state
        .identity
        .update(id, update)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to delete account: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"deleted": true, "message": "Account deleted successfully"})),
    ))
}

pub async fn get_user_me_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let user = state
        .identity
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(format!("User not found: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserInfoResponse {
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
        }),
    ))
}

pub async fn update_user_me_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateUserBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let password_hash = match body.password {
        Some(pw) => Some(
            bcrypt::hash(&pw, bcrypt::DEFAULT_COST)
                .map_err(|e| ApiError::BadRequest(format!("Failed to hash password: {}", e)))?,
        ),
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

    let user = state
        .identity
        .update(identity_id, update)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update user: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserInfoResponse {
            id: user.id,
            username: user.username.unwrap_or_else(|| user.name.clone()),
            display_name: user.display_name,
            email: user.email,
            avatar_url: user.avatar_url,
            identity_type: user.identity_type.to_string(),
            created_at: user.created_at,
        }),
    ))
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

    let orgs = org_membership_repo
        .list_user_organizations(identity_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list orgs: {}", e)))?;

    let mut result = Vec::new();
    for (org_id, role) in &orgs {
        let org = org_repo
            .find_by_id(*org_id)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to fetch org: {}", e)))?;

        result.push(crate::api::models::UserOrgResponse {
            id: *org_id,
            name: org
                .as_ref()
                .map(|o| o.name.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            slug: org.and_then(|o| o.slug),
            role: role.clone(),
        });
    }

    Ok((StatusCode::OK, Json(result)))
}

pub async fn get_user_by_username_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::NotFound(format!("User not found: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserInfoResponse {
            id: user.id,
            username: user.username.unwrap_or_else(|| user.name.clone()),
            display_name: user.display_name,
            email: None,
            avatar_url: user.avatar_url,
            identity_type: user.identity_type.to_string(),
            created_at: user.created_at,
        }),
    ))
}

pub async fn list_audit_logs_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::AuditLogQuery>,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let logs = state
        .audit_repo
        .list_with_filters(
            query.agent_id.as_deref(),
            query.action.as_deref(),
            query.resource_type.as_deref(),
            limit,
            offset,
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total = state
        .audit_repo
        .count_with_filters(
            query.agent_id.as_deref(),
            query.action.as_deref(),
            query.resource_type.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<_> = logs
        .into_iter()
        .map(|log| crate::api::models::AuditLogResponse {
            id: log.id.to_string(),
            agent_id: log.agent_id,
            identity_name: log.identity_name,
            identity_type: log.identity_type,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            details: log.details,
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            timestamp: log.timestamp.to_rfc3339(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(crate::api::models::AuditLogListResponse {
            data,
            total,
            limit,
            offset,
        }),
    ))
}

pub async fn list_my_audit_logs_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Query(query): Query<crate::api::models::AuditLogQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let logs = state
        .audit_repo
        .list_with_filters(
            Some(&subject),
            query.action.as_deref(),
            query.resource_type.as_deref(),
            limit,
            offset,
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total = state
        .audit_repo
        .count_with_filters(
            Some(&subject),
            query.action.as_deref(),
            query.resource_type.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<_> = logs
        .into_iter()
        .map(|log| crate::api::models::AuditLogResponse {
            id: log.id.to_string(),
            agent_id: log.agent_id,
            identity_name: log.identity_name,
            identity_type: log.identity_type,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            details: log.details,
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            timestamp: log.timestamp.to_rfc3339(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(crate::api::models::AuditLogListResponse {
            data,
            total,
            limit,
            offset,
        }),
    ))
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

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    let reviewer_id = Uuid::parse_str(&agent_context.subject).ok();

    if let (Some(author_id), Some(reviewer_id_val)) = (skill.author_identity_id, reviewer_id) {
        if author_id == reviewer_id_val {
            return Err(ApiError::Forbidden(
                "Cannot approve your own skill submission".to_string(),
            ));
        }
    }

    skill_repo
        .update_status(&skill_id, "published")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve skill: {}", e)))?;

    skill_repo
        .update_review_status(&skill_id, "approved", reviewer_id, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update review status: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject.clone()),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "approved"}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill approved successfully".to_string(),
            skill_id,
        }),
    ))
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

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    let reviewer_id = Uuid::parse_str(&agent_context.subject).ok();

    if let (Some(author_id), Some(reviewer_id_val)) = (skill.author_identity_id, reviewer_id) {
        if author_id == reviewer_id_val {
            return Err(ApiError::Forbidden(
                "Cannot reject your own skill submission".to_string(),
            ));
        }
    }

    skill_repo
        .update_status(&skill_id, "rejected")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject skill: {}", e)))?;

    skill_repo
        .update_review_status(&skill_id, "rejected", reviewer_id, body.reason.as_deref())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update review status: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject.clone()),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "rejected", "reason": body.reason}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill rejected".to_string(),
            skill_id,
        }),
    ))
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

    skill_repo
        .update_status(&skill_id, "in_review")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to submit skill for review: {}", e)))?;

    skill_repo
        .update_review_status(&skill_id, "pending", None, body.comment.as_deref())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update review status: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_submitted_for_review".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"comment": body.comment}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill submitted for review".to_string(),
            skill_id,
        }),
    ))
}

pub async fn publish_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.review_status != "approved" {
        return Err(ApiError::BadRequest(
            "Skill must be approved before publishing".to_string(),
        ));
    }

    skill_repo
        .update_status(&skill_id, "published")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to publish skill: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_published".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill published successfully".to_string(),
            skill_id,
        }),
    ))
}

pub async fn approve_org_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.review_status != "pending" && skill.status != "in_review" {
        return Err(ApiError::BadRequest(
            "Skill must be in pending_review status to approve".to_string(),
        ));
    }

    let reviewer_id = Uuid::parse_str(&subject).ok();

    if let (Some(author_id), Some(reviewer_id_val)) = (skill.author_identity_id, reviewer_id) {
        if author_id == reviewer_id_val {
            return Err(ApiError::Forbidden(
                "Cannot approve your own skill submission".to_string(),
            ));
        }
    }

    skill_repo
        .update_status(&skill_id, "published")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve skill: {}", e)))?;

    skill_repo
        .update_review_status(&skill_id, "approved", reviewer_id, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update review status: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "approved"}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill approved successfully".to_string(),
            skill_id,
        }),
    ))
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

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.review_status != "pending" && skill.status != "in_review" {
        return Err(ApiError::BadRequest(
            "Skill must be in pending_review status to reject".to_string(),
        ));
    }

    let reviewer_id = Uuid::parse_str(&subject).ok();

    if let (Some(author_id), Some(reviewer_id_val)) = (skill.author_identity_id, reviewer_id) {
        if author_id == reviewer_id_val {
            return Err(ApiError::Forbidden(
                "Cannot reject your own skill submission".to_string(),
            ));
        }
    }

    skill_repo
        .update_status(&skill_id, "rejected")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject skill: {}", e)))?;

    skill_repo
        .update_review_status(&skill_id, "rejected", reviewer_id, body.reason.as_deref())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update review status: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "rejected", "reason": body.reason}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill rejected".to_string(),
            skill_id,
        }),
    ))
}

pub async fn marketplace_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Query(query): Query<crate::api::models::MarketplaceQuery>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let skills = skill_repo
        .list_by_visibility("marketplace", limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(skills)))
}

pub async fn install_skill_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    skill_repo
        .increment_install_count(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to install skill: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::InstallSkillResponse {
            message: "Skill installed successfully".to_string(),
            skill_id: skill.id.clone(),
            install_count: skill.install_count + 1,
        }),
    ))
}

pub async fn list_skill_groups_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupSkillRepository::new(pool);

    let associations = repo
        .list_by_skill(&skill_id)
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

    Ok((
        StatusCode::OK,
        Json(serde_json::to_value(responses).unwrap()),
    ))
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

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_added_to_group".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"group_id": body.group_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Skill added to group",
            "skill_id": skill_id,
            "group_id": body.group_id,
        })),
    ))
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

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_removed_from_group".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"group_id": group_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill removed from group",
            "skill_id": skill_id,
            "group_id": group_id,
        })),
    ))
}

// v0.4 multi-tenant handlers

use uuid::Uuid;

/// Organization handlers

pub async fn create_org_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateOrgBody>,
) -> Result<impl IntoResponse, ApiError> {
    let org = state
        .organization
        .create_org(
            body.name,
            body.slug,
            body.display_name,
            body.description,
            body.tenant_id,
        )
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(org).unwrap()),
    ))
}

pub async fn get_org_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let org = state
        .organization
        .get_org(org_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn list_orgs_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Query(query): Query<crate::api::models::ListOrgsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let orgs = if let Some(tenant_id) = query.tenant_id {
        state
            .organization
            .list_orgs_by_tenant(tenant_id, limit, offset)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state
            .organization
            .list_orgs(limit, offset)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": orgs }))))
}

pub async fn update_org_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(org_id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateOrgBody>,
) -> Result<impl IntoResponse, ApiError> {
    let org = state
        .organization
        .update_org(org_id, body.name, body.display_name, body.description)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn delete_org_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .organization
        .delete_org(org_id)
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

    let agents = state
        .agent_repo
        .find_by_org(org_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let members: Vec<_> = agents
        .into_iter()
        .map(|a| crate::api::models::OrgMemberResponse {
            agent_id: a.agent_id,
            name: a.agent_name,
            capabilities: a.capabilities,
            joined_at: a.created_at.to_rfc3339(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(crate::api::models::OrgMemberListResponse { members }),
    ))
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

    state
        .agent_repo
        .create(new_agent)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Member added successfully",
            "agent_id": body.agent_id
        })),
    ))
}

pub async fn remove_org_member_handler(
    State(state): State<ApiState>,
    Path((_org_id, subject)): Path<(Uuid, String)>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    state
        .agent_repo
        .update_org(&subject, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"removed": subject})),
    ))
}

pub async fn get_org_stats_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let pool = state.agent_repo.pool();

    let members_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agents WHERE org_id = $1")
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

    let sessions_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let tools_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM org_tools WHERE org_id = $1")
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
    AgentContext { subject: _, .. }: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = OrganizationRepository::new(pool);

    let org = repo
        .find_by_slug(&slug)
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
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let membership = org_membership_repo
        .get_member(identity_id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::Unauthorized("Not a member of this organization".to_string()))?;

    let owner_type = body
        .owner_type
        .unwrap_or_else(|| "organization".to_string());

    if owner_type == "organization" {
        let role_str = membership.role.to_string();
        if role_str != "owner" && role_str != "admin" && role_str != "developer" {
            return Err(ApiError::Unauthorized(
                "Need developer role to create org skills".to_string(),
            ));
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

    let skill = state
        .registry
        .create_skill(new_skill, &subject, &state.search)
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
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let target_identity = state
        .identity
        .get_by_email(&body.email)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User with email '{}' not found", body.email)))?;

    let inviter_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid inviter subject".to_string()))?;

    let org_membership_repo = OrgMembershipRepository::new(pool.clone());
    org_membership_repo
        .add_member(
            target_identity.id,
            org.id,
            body.role.as_str().into(),
            Some(inviter_id),
        )
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": format!("{} added to {}", body.email, slug),
            "organization_id": org.id,
            "identity_id": target_identity.id,
            "role": body.role,
        })),
    ))
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
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    let target_identity = state
        .identity
        .get_by_email(&body.email)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User with email '{}' not found", body.email)))?;

    let inviter_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid inviter subject".to_string()))?;

    let org_membership_repo = OrgMembershipRepository::new(pool.clone());
    org_membership_repo
        .add_member(
            target_identity.id,
            org.id,
            body.role.as_str().into(),
            Some(inviter_id),
        )
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": format!("{} added to organization {}", body.email, org_id),
            "organization_id": org.id,
            "identity_id": target_identity.id,
            "role": body.role,
        })),
    ))
}

pub async fn update_org_member_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((slug, username)): Path<(String, String)>,
    Json(body): Json<crate::api::models::UpdateOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .update_role(identity.id, org.id, body.role.as_str().into())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update role: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} role updated in {}", username, slug),
            "role": body.role,
        })),
    ))
}

pub async fn remove_org_member_by_slug_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((slug, username)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .remove_member(identity.id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} removed from {}", username, slug),
        })),
    ))
}

pub async fn update_org_member_by_id_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((org_id, username)): Path<(uuid::Uuid, String)>,
    Json(body): Json<crate::api::models::UpdateOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .update_role(identity.id, org.id, body.role.as_str().into())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update role: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} role updated in {}", username, org_id),
            "role": body.role,
        })),
    ))
}

pub async fn remove_org_member_by_id_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((org_id, username)): Path<(uuid::Uuid, String)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .remove_member(identity.id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} removed from {}", username, org_id),
        })),
    ))
}

pub async fn list_org_members_by_slug_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let members = org_membership_repo
        .list_members(org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list members: {}", e)))?;

    Ok((StatusCode::OK, Json(members)))
}

pub async fn list_org_members_by_id_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(org_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let members = org_membership_repo
        .list_members(org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list members: {}", e)))?;

    Ok((StatusCode::OK, Json(members)))
}

pub async fn list_org_skills_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let skill_repo = SkillRepository::new(pool);
    let skills = skill_repo
        .list_by_org(&org.id.to_string())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list skills: {}", e)))?;

    Ok((StatusCode::OK, Json(skills)))
}

pub async fn list_org_reviews_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let skill_repo = SkillRepository::new(pool);
    let skills = skill_repo
        .list_by_org(&org.id.to_string())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list skills: {}", e)))?;

    let in_review: Vec<_> = skills
        .into_iter()
        .filter(|s| s.review_status.as_str() == "pending" || s.status == "in_review")
        .collect();

    Ok((StatusCode::OK, Json(in_review)))
}

/// Session handlers

pub async fn get_session_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let session = state
        .session
        .get_session(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    match session {
        Some(s) => {
            let enriched = enrich_session_with_meta(&state, s).await?;
            Ok((StatusCode::OK, Json(serde_json::to_value(enriched).unwrap())))
        }
        None => Err(ApiError::NotFound(format!(
            "Session {} not found",
            session_id
        ))),
    }
}

pub async fn list_sessions_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Query(query): Query<crate::api::models::ListSessionsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    let status = query.status.as_deref();

    let sessions = state
        .session
        .list_sessions(limit, offset, status)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Enrich each session with identity & org names (concurrent lookups per session)
    let enriched: Vec<crate::models::session::SessionWithMeta> =
        futures_util::future::join_all(
            sessions
                .into_iter()
                .map(|s| enrich_session_with_meta(&state, s)),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": enriched })),
    ))
}

pub async fn end_session_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .session
        .end_session(session_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"ended": session_id})),
    ))
}

pub async fn session_declare_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(session_id): Path<Uuid>,
    Json(body): Json<crate::api::models::SessionDeclareBody>,
) -> Result<impl IntoResponse, ApiError> {
    let router = state
        .session
        .declare_capabilities(session_id, body.capabilities)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(router).unwrap())))
}

/// Enrich a repo-level Session with identity and org names for admin display.
async fn enrich_session_with_meta(
    state: &AppRouterState,
    session: crate::db::repositories::session::Session,
) -> Result<crate::models::session::SessionWithMeta, ApiError> {
    let (identity_name, identity_display_name) = state
        .identity
        .get(session.identity_id)
        .await
        .ok()
        .flatten()
        .map(|id| (id.name.clone(), id.display_name.clone()))
        .unwrap_or_else(|| (session.identity_id.to_string(), None));

    let (org_name, tenant_name) = state
        .organization
        .get_org(session.org_id)
        .await
        .map(|org| (org.name, org.tenant_name))
        .unwrap_or_else(|_| (session.org_id.to_string(), None));

    Ok(crate::models::session::SessionWithMeta {
        id: session.id,
        identity_id: session.identity_id,
        identity_name,
        identity_display_name,
        org_id: session.org_id,
        org_name,
        tenant_name,
        status: session.status,
        tool_router: session.tool_router,
        capabilities: session.capabilities,
        created_at: session.created_at,
        last_active_at: session.last_active_at,
        ended_at: session.ended_at,
    })
}

/// Org Tool handlers

pub async fn register_org_tool_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Json(body): Json<crate::api::models::RegisterOrgToolBody>,
) -> Result<impl IntoResponse, ApiError> {
    let tool = state
        .org_tool
        .register_tool(
            body.org_id,
            body.tool_id,
            body.name,
            body.description,
            body.schema.unwrap_or(serde_json::json!({})),
            body.implementation.unwrap_or(serde_json::json!({})),
        )
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(tool).unwrap()),
    ))
}

pub async fn list_org_tools_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
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
    AgentContext { subject: _, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let tools = state
        .org_tool
        .list_all()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": tools }))))
}

pub async fn approve_org_tool_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(tool_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .org_tool
        .approve_tool(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"approved": tool_id})),
    ))
}

// Group member management handlers (6.6)

pub async fn list_group_members_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(group_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool);

    let members = repo
        .list_members(group_id)
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

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_added".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": body.agent_id, "role": role}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Member added to group",
            "group_id": group_id,
            "member_id": body.agent_id,
        })),
    ))
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

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": member_subject, "role": body.role}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member updated",
            "group_id": group_id,
            "member_id": member_subject,
        })),
    ))
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

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_removed".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": member_subject}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member removed",
            "group_id": group_id,
            "member_id": member_subject,
        })),
    ))
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
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let mut new_group: crate::models::group::NewGroup = body.into();
    new_group.organization_id = org.id;

    let group = state
        .group
        .create(new_group)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_created".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group.id.to_string()),
            details: serde_json::json!({"org_slug": slug}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(group).unwrap()),
    ))
}

pub async fn list_org_groups_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool);
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let groups = state
        .group
        .list_by_organization(org.id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": groups }))))
}

pub async fn get_org_group_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((slug, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool);
    org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let group = state
        .group
        .get(group_id)
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
    org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let group = state
        .group
        .update(group_id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
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
    org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    state
        .group
        .delete(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_deleted".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"deleted": group_id})),
    ))
}

// Org slug-based Group member management (6.6)

pub async fn list_org_group_members_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((slug, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let repo = GroupRepository::new(pool);
    let members = repo
        .list_members(group_id)
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

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": member_info })),
    ))
}

pub async fn update_org_group_member_handler(
    State(state): State<ApiState>,
    Path((slug, group_id, username)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo
        .find_by_slug(&slug)
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

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member role updated",
            "group_id": group_id,
            "member_id": username,
        })),
    ))
}

pub async fn remove_org_group_member_handler(
    State(state): State<ApiState>,
    Path((slug, group_id, username)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let target_id = uuid::Uuid::parse_str(&username)
        .map_err(|_| ApiError::BadRequest("Invalid member id".to_string()))?;

    let repo = GroupRepository::new(pool);
    repo.remove_member(target_id, group_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove group member: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_removed".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "member_id": username}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member removed",
            "group_id": group_id,
            "member_id": username,
        })),
    ))
}

// Org slug-based Group-Skill association (6.6)

pub async fn list_org_group_skills_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((slug, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let repo = GroupSkillRepository::new(pool);
    let skills = repo
        .list_by_group(group_id)
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
    use crate::db::repositories::group_skill::GroupSkillRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let skill_id = body
        .skill_id
        .clone()
        .ok_or_else(|| ApiError::BadRequest("skill_id is required".to_string()))?;

    let repo = GroupSkillRepository::new(pool);
    repo.associate_skill(crate::models::group_skill::NewGroupSkill {
        group_id,
        skill_id: skill_id.clone(),
        added_by: None,
    })
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to associate skill: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_skill_associated".to_string(),
            resource_type: "group_skill".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "skill_id": skill_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Skill associated to group",
            "group_id": group_id,
            "skill_id": skill_id,
        })),
    ))
}

pub async fn remove_org_group_skill_handler(
    State(state): State<ApiState>,
    Path((slug, group_id, skill_id)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let org_repo = OrganizationRepository::new(pool.clone());
    org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let repo = GroupSkillRepository::new(pool);
    repo.dissociate_skill(group_id, &skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to dissociate skill: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_skill_dissociated".to_string(),
            resource_type: "group_skill".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "skill_id": skill_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill dissociated from group",
            "group_id": group_id,
            "skill_id": skill_id,
        })),
    ))
}

pub async fn reject_org_tool_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(tool_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .org_tool
        .reject_tool(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"rejected": tool_id})),
    ))
}

pub async fn delete_org_tool_handler(
    State(state): State<ApiState>,
    Path(tool_id): Path<Uuid>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    state
        .org_tool
        .delete(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"deleted": tool_id})),
    ))
}

pub async fn get_admin_me_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    agent_context.require_admin()?;

    let id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let user = state
        .identity
        .get(id)
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
        "SELECT COALESCE(AVG(CASE WHEN success THEN 1.0 ELSE 0.0 END), 0) FROM evaluations",
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

    let transport_mode =
        std::env::var("AION_HIVE_TRANSPORT").unwrap_or_else(|_| "http".to_string());

    let data_dir = std::env::var("AION_HIVE_DATA_DIR").unwrap_or_else(|_| "./data".to_string());

    let skills_dir = std::env::var("AION_HIVE_SKILLS_DIR")
        .unwrap_or_else(|_| format!("{}/skills", data_dir));

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
    AgentContext { subject: _, .. }: AgentContext,
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

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "lead": to_codes(lead_defaults),
            "member": to_codes(member_defaults),
        })),
    ))
}

pub async fn list_group_permissions_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(group_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::api::models::GroupPermissionInfo;
    use crate::db::repositories::role_permission::RolePermissionRepository;

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

    let overrides = state
        .group_perm_override_repo
        .list_by_group(group_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let is_overridden = |perm_code: &str| -> Option<bool> {
        overrides
            .iter()
            .find(|o| o.permission_code == perm_code)
            .map(|o| o.granted)
    };

    let to_info =
        |perms: Vec<crate::models::role_permission::RolePermission>| -> Vec<GroupPermissionInfo> {
            perms
                .into_iter()
                .map(|p| {
                    let code = p.permission_code;
                    let override_granted = is_overridden(&code);
                    GroupPermissionInfo {
                        permission_code: code,
                        granted: override_granted.unwrap_or(true),
                        is_default: override_granted.is_none(),
                    }
                })
                .collect()
        };

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "lead": to_info(lead_defaults),
            "member": to_info(member_defaults),
        })),
    ))
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

    state
        .group_perm_override_repo
        .upsert_override(NewGroupPermissionOverride {
            group_id,
            role_name: body.role_name,
            permission_code: body.permission_code,
            granted: body.granted,
            created_by: creator_id,
        })
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
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

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group permission override updated"
        })),
    ))
}

pub async fn delete_group_permission_handler(
    State(state): State<ApiState>,
    Path((group_id, permission_code)): Path<(Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupPermissionBody>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .group_perm_override_repo
        .delete_override(group_id, &body.role_name, &permission_code)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let role_name = body.role_name.clone();

    state
        .audit_repo
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

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group permission override deleted"
        })),
    ))
}

// --- Admin User Management Handlers (Feature #7) ---

pub async fn list_users_handler_admin(
    State(state): State<ApiState>,
    AgentContext { .. }: AgentContext,
    Query(query): Query<crate::api::models::ListUsersQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);
    let identity_type = query.identity_type.as_deref();

    let users = state
        .identity
        .list(limit, offset, identity_type)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<crate::api::models::UserAdminResponse> = users
        .into_iter()
        .map(|u| crate::api::models::UserAdminResponse {
            id: u.id,
            identity_type: u.identity_type.to_string(),
            username: u.username,
            display_name: u.display_name,
            email: u.email,
            avatar_url: u.avatar_url,
            is_system_admin: u.is_system_admin,
            status: u.status.to_string(),
            created_at: u.created_at,
            updated_at: u.updated_at,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": data,
            "total": data.len(),
            "limit": limit,
            "offset": offset,
        })),
    ))
}

pub async fn disable_user_handler_admin(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Path(user_id): Path<uuid::Uuid>,
    Json(body): Json<crate::api::models::DisableUserBody>,
) -> Result<impl IntoResponse, ApiError> {
    let status = if body.disabled { "disabled" } else { "active" };

    let update = crate::models::identity::IdentityUpdate {
        status: Some(status.into()),
        ..Default::default()
    };

    let updated = state
        .identity
        .update(user_id, update)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: if body.disabled {
                "user_disabled".to_string()
            } else {
                "user_enabled".to_string()
            },
            resource_type: "user".to_string(),
            resource_id: Some(user_id.to_string()),
            details: serde_json::json!({
                "username": updated.username,
                "status": status,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("User {} successfully", if body.disabled { "disabled" } else { "enabled" }),
            "user_id": user_id.to_string(),
        })),
    ))
}

pub async fn delete_user_handler_admin(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .identity
        .delete(user_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "user_deleted".to_string(),
            resource_type: "user".to_string(),
            resource_id: Some(user_id.to_string()),
            details: serde_json::json!({}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "User deleted successfully",
            "user_id": user_id.to_string(),
        })),
    ))
}

// --- Evaluation Query/Delete Handlers (Feature #8) ---

pub async fn list_evaluations_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Query(query): Query<crate::api::models::ListEvaluationsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let skill_id = match query.skill_id.as_deref() {
        Some(id) => id,
        None => {
            return Err(ApiError::BadRequest(
                "skill_id query parameter is required".to_string(),
            ))
        }
    };

    let evals = state
        .evaluator
        .list_evaluations(skill_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<crate::api::models::EvaluationItemResponse> = evals
        .into_iter()
        .map(|e| crate::api::models::EvaluationItemResponse {
            id: e.id,
            skill_id: e.skill_id,
            agent_id: e.agent_id,
            success: e.success,
            duration_ms: e.duration_ms,
            error_type: e.error_type.map(|et| format!("{:?}", et)),
            tags: e.tags.iter().map(|t| format!("{:?}", t)).collect(),
            timestamp: e.timestamp.to_rfc3339(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": data,
            "total": data.len(),
        })),
    ))
}

pub async fn get_evaluation_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(eval_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let eval = state
        .evaluator
        .get_evaluation(eval_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Evaluation {} not found", eval_id)))?;

    let response = crate::api::models::EvaluationItemResponse {
        id: eval.id,
        skill_id: eval.skill_id,
        agent_id: eval.agent_id,
        success: eval.success,
        duration_ms: eval.duration_ms,
        error_type: eval.error_type.map(|et| format!("{:?}", et)),
        tags: eval.tags.iter().map(|t| format!("{:?}", t)).collect(),
        timestamp: eval.timestamp.to_rfc3339(),
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn delete_evaluation_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Path(eval_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .evaluator
        .delete_evaluation(eval_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "evaluation_deleted".to_string(),
            resource_type: "evaluation".to_string(),
            resource_id: Some(eval_id.to_string()),
            details: serde_json::json!({}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Evaluation deleted successfully",
            "evaluation_id": eval_id.to_string(),
        })),
    ))
}

// --- Webhook Management Handlers (Feature #11) ---

pub async fn list_webhooks_handler(
    State(state): State<ApiState>,
    AgentContext { .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let urls = state.evaluator.get_webhook_urls();
    let data: Vec<crate::api::models::WebhookItemResponse> = urls
        .iter()
        .enumerate()
        .map(|(index, url)| crate::api::models::WebhookItemResponse {
            index,
            url: url.clone(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": data,
            "total": data.len(),
        })),
    ))
}

pub async fn add_webhook_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::AddWebhookBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.url.is_empty() || !body.url.starts_with("http") {
        return Err(ApiError::BadRequest(
            "Invalid webhook URL. Must be a valid HTTP(S) URL".to_string(),
        ));
    }

    // Note: EvaluatorService webhook management is currently not thread-safe for mutation.
    // This adds to a clone; production should use Arc<RwLock> or a DB-backed store.
    let mut evaluator = state.evaluator.clone();
    evaluator.add_webhook_url_dyn(body.url.clone());

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "webhook_added".to_string(),
            resource_type: "webhook".to_string(),
            resource_id: Some(body.url.clone()),
            details: serde_json::json!({ "url": &body.url }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Webhook URL added successfully",
            "url": body.url,
        })),
    ))
}

pub async fn remove_webhook_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Path(index): Path<usize>,
) -> Result<impl IntoResponse, ApiError> {
    let mut evaluator = state.evaluator.clone();
    evaluator
        .remove_webhook_url(index)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "webhook_removed".to_string(),
            resource_type: "webhook".to_string(),
            resource_id: Some(index.to_string()),
            details: serde_json::json!({}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("Webhook at index {} removed successfully", index),
        })),
    ))
}

// --- Skill Upload & Version Management Handlers (Feature #12) ---

/// Handler for ZIP upload of a skill package
pub async fn upload_skill_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    mut multipart: axum::extract::Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut zip_data: Option<Vec<u8>> = None;
    let mut owner_type = "user".to_string();
    let mut owner_id: Option<uuid::Uuid> = None;
    let mut author_identity_id: Option<uuid::Uuid> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;
                zip_data = Some(data.to_vec());
            }
            "owner_type" => {
                owner_type = field.text().await.unwrap_or_else(|_| "user".to_string());
            }
            "owner_id" => {
                let val = field.text().await.unwrap_or_default();
                if !val.is_empty() {
                    owner_id = uuid::Uuid::parse_str(&val).ok();
                }
            }
            "author_identity_id" => {
                let val = field.text().await.unwrap_or_default();
                if !val.is_empty() {
                    author_identity_id = uuid::Uuid::parse_str(&val).ok();
                }
            }
            _ => {}
        }
    }

    let zip_data = zip_data
        .ok_or_else(|| ApiError::BadRequest("ZIP file is required in 'file' field".to_string()))?;

    let upload_result = state
        .skill_git
        .process_upload(
            &zip_data,
            &subject,
            author_identity_id,
            &owner_type,
            owner_id,
            &state.registry,
            &state.search,
            &state.skill_repo,
            &state.version_repo,
        )
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Audit log
    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_uploaded".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(upload_result.skill_id.clone()),
            details: serde_json::json!({
                "skill_name": upload_result.skill_name,
                "version": upload_result.version,
                "git_commit": upload_result.git_commit,
                "git_tag": upload_result.git_tag,
                "is_new_skill": upload_result.is_new_skill,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let response = crate::api::models::SkillUploadResponse {
        skill_id: upload_result.skill_id,
        skill_name: upload_result.skill_name,
        version: upload_result.version,
        git_commit: upload_result.git_commit,
        git_tag: upload_result.git_tag,
        git_repo_name: upload_result.git_repo_name,
        is_new_skill: upload_result.is_new_skill,
        files: upload_result.files,
        message: "Skill uploaded successfully".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

// --- Skill Upload Preview & Confirm Handlers ---

/// POST /api/v1/skills/upload/preview — 上传 ZIP 仅解压预览，不提交
pub async fn upload_skill_preview_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    mut multipart: axum::extract::Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut zip_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;
            zip_data = Some(data.to_vec());
        }
    }

    let zip_data = zip_data
        .ok_or_else(|| ApiError::BadRequest("ZIP file is required in 'file' field".to_string()))?;

    let preview = state
        .skill_git
        .preview_upload(&zip_data)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let response = crate::api::models::SkillUploadPreviewResponse {
        preview_id: preview.preview_id,
        metadata: crate::api::models::PreviewMetadataResponse {
            name: preview.metadata.name,
            description: preview.metadata.description,
            version: preview.metadata.version.unwrap_or_default(),
            tags: preview.metadata.tags,
            dependencies: preview.metadata.dependencies,
            compatibility: preview.metadata.compatibility,
        },
        files: preview
            .files
            .into_iter()
            .map(|f| crate::api::models::PreviewFileResponse {
                path: f.path,
                size: f.size,
            })
            .collect(),
        total_files: preview.total_files,
        total_size: preview.total_size,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// GET /api/v1/skills/upload/preview/:preview_id/files/*path — 获取预览中文件内容
pub async fn get_preview_file_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    axum::extract::Path((preview_id,)): axum::extract::Path<(String,)>,
    req: axum::http::Request<axum::body::Body>,
) -> Result<impl IntoResponse, ApiError> {
    // Parse file_path from the URL path after /files/
    let uri_path = req.uri().path().to_string();
    let file_marker = "/files/";
    let file_path = match uri_path.find(file_marker) {
        Some(pos) => {
            let raw = &uri_path[pos + file_marker.len()..];
            percent_encoding::percent_decode_str(raw)
                .decode_utf8()
                .map_err(|e| ApiError::BadRequest(format!("Invalid file path encoding: {}", e)))?
                .to_string()
        }
        None => {
            return Err(ApiError::BadRequest(
                "File path not found in URL".to_string(),
            ));
        }
    };

    if file_path.is_empty() {
        return Err(ApiError::BadRequest("File path is required".to_string()));
    }

    let (content, content_type, size) = state
        .skill_git
        .get_preview_file(&preview_id, &file_path)
        .map_err(|e| match e {
        crate::models::error::AppError::FileNotFound(msg) => ApiError::NotFound(msg),
        _ => ApiError::BadRequest(e.to_string()),
    })?;

    let is_binary = content_type == "application/octet-stream";
    let text_content = if is_binary {
        format!("[Binary file: {} bytes, not displayable as text]", size)
    } else {
        String::from_utf8(content)
            .unwrap_or_else(|_| format!("[Cannot decode file as UTF-8: {} bytes]", size))
    };

    let response = crate::api::models::PreviewFileContentResponse {
        path: file_path,
        content: text_content,
        size,
        is_binary,
        content_type,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// POST /api/v1/skills/upload/preview/:preview_id/confirm — 确认上传，提交 Git + DB
pub async fn confirm_skill_upload_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    axum::extract::Path((preview_id,)): axum::extract::Path<(String,)>,
    Json(body): Json<crate::api::models::ConfirmUploadBody>,
) -> Result<impl IntoResponse, ApiError> {
    let owner_type = body.owner_type.unwrap_or_else(|| "user".to_string());
    let author_identity_id = body.author_identity_id;
    let owner_id = body.owner_id;

    let upload_result = state
        .skill_git
        .confirm_upload_from_preview(
            &preview_id,
            &subject,
            author_identity_id,
            &owner_type,
            owner_id,
            &state.registry,
            &state.search,
            &state.skill_repo,
            &state.version_repo,
        )
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Audit log
    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject.clone()),
            action: "skill_uploaded".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(upload_result.skill_id.clone()),
            details: serde_json::json!({
                "skill_name": upload_result.skill_name,
                "version": upload_result.version,
                "git_commit": upload_result.git_commit,
                "git_tag": upload_result.git_tag,
                "is_new_skill": upload_result.is_new_skill,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let response = crate::api::models::SkillUploadResponse {
        skill_id: upload_result.skill_id,
        skill_name: upload_result.skill_name,
        version: upload_result.version,
        git_commit: upload_result.git_commit,
        git_tag: upload_result.git_tag,
        git_repo_name: upload_result.git_repo_name,
        is_new_skill: upload_result.is_new_skill,
        files: upload_result.files,
        message: "Skill uploaded successfully".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/skills/:name/versions — list versions for a skill by name
pub async fn list_skill_versions_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_name): Path<String>,
    Query(query): Query<crate::api::models::ListVersionsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let versions = state
        .version_repo
        .list_by_name(&skill_name, limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<crate::api::models::SkillVersionResponse> = versions
        .into_iter()
        .map(|v| crate::api::models::SkillVersionResponse {
            id: v.id.to_string(),
            skill_name: v.skill_name,
            version: v.version,
            git_commit_hash: v.git_commit_hash,
            git_tag: v.git_tag,
            changelog: v.changelog,
            file_count: v.file_count,
            total_size_bytes: v.total_size_bytes,
            uploaded_by: v.uploaded_by,
            git_remote_url: v.git_remote_url,
            created_at: v.created_at.to_rfc3339(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": data,
            "total": data.len(),
        })),
    ))
}

/// GET /api/v1/skills/:name/versions/diff — diff between two versions
pub async fn get_skill_version_diff_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_name): Path<String>,
    Query(query): Query<crate::api::models::VersionDiffQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let diff = state
        .skill_git
        .get_version_diff(&skill_name, &query.from, &query.to)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "skill_name": skill_name,
            "from_version": query.from,
            "to_version": query.to,
            "diff": diff,
        })),
    ))
}

/// GET /api/v1/skills/:name/tags — list git tags for a skill
pub async fn list_skill_git_tags_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let tags = state
        .skill_git
        .list_git_tags(&skill_name)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "skill_name": skill_name,
            "tags": tags,
        })),
    ))
}

// --- GitLab Remote Sync Handlers ---

/// POST /api/v1/skills/:name/sync — 从 GitLab 拉取最新更新
pub async fn sync_skill_from_gitlab_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .skill_git
        .fetch_from_gitlab(&skill_name)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let tags = state
        .skill_git
        .list_git_tags(&skill_name)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: None,
            action: "skill_gitlab_sync".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_name.clone()),
            details: serde_json::json!({ "skill_name": &skill_name }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("Synced {} from GitLab", skill_name),
            "skill_name": skill_name,
            "tags": tags,
        })),
    ))
}

/// POST /api/v1/skills/:name/clone — 从 GitLab 克隆 skill 仓库到本地
pub async fn clone_skill_from_gitlab_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let repo_path = state
        .skill_git
        .clone_from_gitlab(&skill_name)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: None,
            action: "skill_gitlab_clone".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_name.clone()),
            details: serde_json::json!({
                "skill_name": &skill_name,
                "repo_path": repo_path.to_string_lossy(),
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": format!("Cloned {} from GitLab", skill_name),
            "skill_name": skill_name,
            "repo_path": repo_path.to_string_lossy(),
        })),
    ))
}

/// GET /api/v1/skills/:name/remote — 查看 skill 关联的 GitLab 信息
pub async fn get_skill_remote_info_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let repo_name = format!("skill-{}", skill_name);
    let repo_path = state.skill_git.repo_path(&skill_name);
    let local_repo_exists = repo_path.join(".git").exists();

    let remote_url = state.skill_git.remote_config.remote_url(&repo_name);

    let response = crate::api::models::SkillRemoteInfoResponse {
        skill_name,
        git_remote_url: if local_repo_exists {
            Some(remote_url)
        } else {
            None
        },
        gitlab_group: state.skill_git.remote_config.gitlab_group.clone(),
        gitlab_url: state.skill_git.remote_config.gitlab_url.clone(),
        push_enabled: state.skill_git.remote_config.push_enabled,
        local_repo_exists,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// GET /api/v1/admin/skills/gitlab-sync — 批量同步已配置 remote 的 skills
pub async fn sync_all_skills_from_gitlab_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Query(body): Query<crate::api::models::SkillSyncBody>,
) -> Result<impl IntoResponse, ApiError> {
    let skill_names = if let Some(names) = body.skill_names {
        names
    } else {
        // 获取所有在 skill_versions 中有 git_remote_url 的 skill
        let pool = state.agent_repo.pool();
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT skill_name FROM skill_versions WHERE git_remote_url IS NOT NULL",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
        rows.into_iter().map(|r| r.0).collect()
    };

    let mut results = Vec::new();
    for name in &skill_names {
        let result = state.skill_git.fetch_from_gitlab(name);
        results.push(serde_json::json!({
            "skill_name": name,
            "success": result.is_ok(),
            "error": result.err().map(|e| e.to_string()),
        }));
    }

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: None,
            action: "skills_gitlab_sync_all".to_string(),
            resource_type: "skill".to_string(),
            resource_id: None,
            details: serde_json::json!({ "count": skill_names.len() }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Batch sync completed",
            "results": results,
        })),
    ))
}

/// POST /api/v1/webhooks/gitlab — 接收 GitLab push events
pub async fn gitlab_webhook_handler(
    _state: State<ApiState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<impl IntoResponse, ApiError> {
    // 验证 X-Gitlab-Token
    let expected_token = std::env::var("GITLAB_WEBHOOK_SECRET")
        .unwrap_or_else(|_| "skill-garden-webhook".to_string());
    let token = headers
        .get("X-Gitlab-Token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if expected_token.is_empty() || token != expected_token {
        return Err(ApiError::Unauthorized("Invalid webhook token".to_string()));
    }

    // 解析 event
    let event_type = headers
        .get("X-Gitlab-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // 尝试解析 project name
    let project_name: Option<String> =
        if let Ok(val) = serde_json::from_str::<crate::api::models::GitlabWebhookBody>(&body) {
            val.project.and_then(|p| p.name)
        } else {
            // fallback: 尝试直接解析
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
                val["project"]["name"].as_str().map(|s| s.to_string())
            } else {
                None
            }
        };

    let skill_name = project_name
        .as_deref()
        .and_then(|n| n.strip_prefix("skill-"))
        .unwrap_or("unknown");

    info!(
        "GitLab webhook received: event={}, skill={}",
        event_type, skill_name
    );

    // 仅对 push/tag_push events 做同步
    if event_type == "Push Hook" || event_type == "Tag Push Hook" {
        match _state.skill_git.fetch_from_gitlab(skill_name) {
            Ok(()) => info!("Webhook sync successful for {}", skill_name),
            Err(e) => warn!("Webhook sync failed for {}: {}", skill_name, e),
        }
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "received": true,
            "event": event_type,
        })),
    ))
}
