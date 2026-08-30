//! Webhook 管理 handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use super::helpers::ApiState;
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;

pub async fn list_webhooks_handler(
    State(state): State<ApiState>,
    AgentContext { .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let urls = state.evaluator.get_webhook_urls();
    let data: Vec<crate::api::models::WebhookItemResponse> = urls
        .iter()
        .enumerate()
        .map(|(index, url)| crate::api::models::WebhookItemResponse {
            index,
            url: url.clone(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": data,
            "total": data.len(),
        })),
    ))
}

pub async fn add_webhook_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::AddWebhookBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.url.is_empty() || !body.url.starts_with("http") {
        return Err(ApiError::BadRequest(
            "Invalid webhook URL. Must be a valid HTTP(S) URL".to_string(),
        ));
    }

    // Note: EvaluatorService webhook management is currently not thread-safe for mutation.
    // This adds to a clone; production should use Arc<RwLock> or a DB-backed store.
    let mut evaluator = state.evaluator.clone();
    evaluator.add_webhook_url_dyn(body.url.clone());

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "webhook_added".to_string(),
            resource_type: "webhook".to_string(),
            resource_id: Some(body.url.clone()),
            details: serde_json::json!({ "url": &body.url }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Webhook URL added successfully",
            "url": body.url,
        })),
    ))
}

pub async fn remove_webhook_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Path(index): Path<usize>,
) -> Result<impl IntoResponse, ApiError> {
    let mut evaluator = state.evaluator.clone();
    evaluator
        .remove_webhook_url(index)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "webhook_removed".to_string(),
            resource_type: "webhook".to_string(),
            resource_id: Some(index.to_string()),
            details: serde_json::json!({}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("Webhook at index {} removed successfully", index),
        })),
    ))
}
