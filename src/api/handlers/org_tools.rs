//! 组织工具管理 handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use super::helpers::{require_admin, require_org_member, ApiState};
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;

/// Org Tool handlers

pub async fn register_org_tool_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::RegisterOrgToolBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_org_member(
        &state,
        &agent_context,
        body.org_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

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
    agent_context: AgentContext,
    Path(tool_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    // Get the tool to find its org_id, then check org membership
    let tool = state
        .org_tool
        .get_tool(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Tool {} not found", tool_id)))?;

    require_org_member(
        &state,
        &agent_context,
        tool.org_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

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
    require_admin(&state, &agent_context).await?;

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
