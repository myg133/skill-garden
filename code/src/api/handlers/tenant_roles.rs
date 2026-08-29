//! 租户角色分配 handlers

use axum::{extract::{Query, State}, http::StatusCode, response::IntoResponse, Json};

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_admin, ApiState};

// Tenant role assignment handlers (tenant_admin assigns org owner/admin)

pub async fn assign_tenant_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AssignTenantRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let admin_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid admin subject".to_string()))?;
    if !matches!(body.role_name.as_str(), "tenant_admin") {
        return Err(ApiError::BadRequest(
            "Only tenant_admin role can be assigned at tenant level".to_string(),
        ));
    }
    let assignment = state
        .tenant_role_assignment
        .assign(body.identity_id, body.tenant_id, &body.role_name, Some(admin_id))
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(assignment).unwrap()),
    ))
}

pub async fn revoke_tenant_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::RevokeTenantRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    state
        .tenant_role_assignment
        .revoke(body.identity_id, body.tenant_id, &body.role_name)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"revoked": true}))))
}

pub async fn list_tenant_role_assignments_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListTenantRoleAssignmentsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let assignments = if let Some(tenant_id) = query.tenant_id {
        state
            .tenant_role_assignment
            .list_by_tenant(tenant_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else if let Some(identity_id) = query.identity_id {
        state
            .tenant_role_assignment
            .find_by_identity(identity_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        return Err(ApiError::BadRequest(
            "Must provide tenant_id or identity_id".to_string(),
        ));
    };
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": assignments })),
    ))
}

