//! 角色权限管理 handlers

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use super::helpers::{require_admin, ApiState};
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;

// Role permission management handlers

pub async fn list_role_permissions_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
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
    require_admin(&state, &agent_context).await?;
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
    require_admin(&state, &agent_context).await?;
    state
        .role_permission
        .remove_permission(&query.role_level, &query.role_name, &query.permission_code)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": true}))))
}
