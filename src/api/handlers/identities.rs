//! 身份管理 handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use super::helpers::{require_admin, require_auth, ApiState};
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;

pub async fn list_identities_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);
    let identities = state
        .identity
        .list(limit, offset, None)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": identities })),
    ))
}

pub async fn create_identity_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateIdentityBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let identity = state
        .identity
        .create(body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(identity).unwrap()),
    ))
}

pub async fn get_identity_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let identity = state
        .identity
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Identity not found".to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::to_value(identity).unwrap()),
    ))
}

pub async fn update_identity_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateIdentityBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let identity = state
        .identity
        .update(id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::to_value(identity).unwrap()),
    ))
}

pub async fn delete_identity_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    state
        .identity
        .delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

/// Search identities by name/email/username
/// GET /identities/search?q={query}&limit={limit}
pub async fn search_identities_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<SearchIdentitiesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // Require authentication - any logged in user can search
    let _identity_id = require_auth(&agent_context)
        .await
        .map_err(|e| ApiError::Unauthorized(e.to_string()))?;

    let query_str = query.q.trim();
    if query_str.is_empty() {
        return Err(ApiError::BadRequest(
            "Query parameter 'q' is required".to_string(),
        ));
    }

    let limit = query.limit.unwrap_or(10).min(50);
    let identities = state
        .identity
        .search(query_str, limit)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Convert to search result format (without sensitive fields)
    let results: Vec<serde_json::Value> = identities
        .into_iter()
        .map(|i| {
            serde_json::json!({
                "id": i.id,
                "name": i.name,
                "username": i.username,
                "display_name": i.display_name,
                "email": i.email,
                "avatar_url": i.avatar_url,
                "identity_type": i.identity_type.to_string(),
            })
        })
        .collect();

    Ok((StatusCode::OK, Json(serde_json::json!({"data": results}))))
}

#[derive(serde::Deserialize)]
pub struct SearchIdentitiesQuery {
    pub q: String,
    #[serde(default)]
    pub limit: Option<i64>,
}
