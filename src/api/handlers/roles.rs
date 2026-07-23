//! 角色管理 handlers

use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_admin, ApiState};

// Role handlers

pub async fn list_roles_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
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
    require_admin(&state, &agent_context).await?;
    let role = state
        .role
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Role not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(role).unwrap())))
}

// Role CRUD handlers (C/U/D)

pub async fn create_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
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
    require_admin(&state, &agent_context).await?;
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
    require_admin(&state, &agent_context).await?;
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
    require_admin(&state, &agent_context).await?;
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
    require_admin(&state, &agent_context).await?;
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
    require_admin(&state, &agent_context).await?;
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
    require_admin(&state, &agent_context).await?;
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

