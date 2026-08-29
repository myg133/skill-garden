//! 市场审核员管理 handlers

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_marketplace_admin, require_marketplace_admin_only, ApiState};

// Marketplace reviewer assignment handlers (marketplace_admin assigns marketplace_reviewer)

pub async fn assign_marketplace_reviewer_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AssignMarketplaceReviewerBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin_only(&state, &agent_context).await?;
    let admin_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid admin subject".to_string()))?;
    // Prevent self-assignment
    if body.identity_id == admin_id {
        return Err(ApiError::BadRequest("Cannot modify your own role".to_string()));
    }
    let role_name = "marketplace_reviewer";
    let assignment = state
        .system_role_assignment
        .assign(body.identity_id, role_name, Some(admin_id))
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(assignment).unwrap()),
    ))
}

pub async fn revoke_marketplace_reviewer_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::RevokeMarketplaceReviewerBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin_only(&state, &agent_context).await?;
    let admin_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid admin subject".to_string()))?;
    if body.identity_id == admin_id {
        return Err(ApiError::BadRequest("Cannot modify your own role".to_string()));
    }
    state
        .system_role_assignment
        .revoke(body.identity_id, "marketplace_reviewer")
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"revoked": true}))))
}

pub async fn list_marketplace_reviewers_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let assignments = state
        .system_role_assignment
        .list_by_role("marketplace_reviewer")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": assignments })),
    ))
}

