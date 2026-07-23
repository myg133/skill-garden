//! GitLab 杩滅▼鍚屾 handlers

use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Json};

use tracing::{info, warn};
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::ApiState;

/// GET /api/v1/skills/:name/tags - list git tags for a skill
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

/// POST /api/v1/skills/:name/sync - sync skill from GitLab repository
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

/// POST /api/v1/skills/:name/clone - 从 GitLab 克隆 skill 仓库到本地
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

/// GET /api/v1/skills/:name/remote - get remote info for a skill
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

/// GET /api/v1/admin/skills/gitlab-sync - sync all skills with remote config
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

/// POST /api/v1/webhooks/gitlab - handle GitLab push events
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
            // fallback: 解析 JSON Value 兜底
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


