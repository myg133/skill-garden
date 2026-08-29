//! 权限检查 handlers

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_admin, ApiState};

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
        tenant_id: body.tenant_id,
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
    require_admin(&state, &agent_context).await?;
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

