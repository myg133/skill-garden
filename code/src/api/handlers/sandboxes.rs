//! 沙箱管理 handlers

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_admin, ApiState};

pub async fn list_sandboxes_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

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
    require_admin(&state, &agent_context).await?;

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
    require_admin(&state, &agent_context).await?;

    state
        .sandbox
        .remove_sandbox(&key)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "removed": key }))))
}

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