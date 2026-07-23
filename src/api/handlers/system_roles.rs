//! 系统角色分配 handlers

use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_admin, ApiState};

// System role assignment handlers

pub async fn assign_system_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AssignSystemRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let admin_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid admin subject".to_string()))?;
    if !crate::models::system_role_assignment::SystemRole::is_valid_super_admin_role(&body.role_name) {
        return Err(ApiError::BadRequest(format!(
            "Invalid system role for super admin assignment: {}. Only super_admin/marketplace_admin allowed.",
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
    require_admin(&state, &agent_context).await?;
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
    require_admin(&state, &agent_context).await?;
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
        state
            .system_role_assignment
            .list_all()
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
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
    require_admin(&state, &agent_context).await?;
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

