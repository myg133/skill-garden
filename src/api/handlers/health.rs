//! 健康检查

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use crate::api::error::ApiError;
use super::helpers::ApiState;

pub async fn health_handler(State(state): State<ApiState>) -> Result<impl IntoResponse, ApiError> {
    let skills_count = state
        .registry
        .count()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let response = crate::api::models::HealthResponse {
        status: "OK".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        skills_count: skills_count as usize,
    };
    Ok((StatusCode::OK, Json(response)))
}