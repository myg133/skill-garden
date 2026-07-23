//! Git 代理和远程同步 handlers

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_admin, ApiState};

pub async fn list_git_branches_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(repo_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

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
    require_admin(&state, &agent_context).await?;

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
    require_admin(&state, &agent_context).await?;

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
    require_admin(&state, &agent_context).await?;

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
    require_admin(&state, &agent_context).await?;

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
    require_admin(&state, &agent_context).await?;

    let healthy = state.git_proxy.health_check().await.unwrap_or(false);

    let response = crate::api::models::GitProxyHealthResponse {
        git_proxy_connected: healthy,
        api_base: std::env::var("GIT_PROXY_API_BASE")
            .unwrap_or_else(|_| "http://localhost:8081".to_string()),
    };

    Ok((StatusCode::OK, Json(response)))
}