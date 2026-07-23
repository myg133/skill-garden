//! 管理员 handlers

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_admin, ApiState};

pub async fn get_admin_stats_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let pool = state.agent_repo.pool();

    let total_skills = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM skills")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_agents = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agents")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_organizations = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM organizations")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_evaluations = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM evaluations")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let avg_success_rate = sqlx::query_scalar::<_, f64>(
        "SELECT COALESCE(AVG(CASE WHEN success THEN 1.0 ELSE 0.0 END), 0) FROM evaluations",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0.0);

    let response = crate::api::models::AdminStatsResponse {
        total_skills,
        total_agents,
        total_organizations,
        total_evaluations,
        average_success_rate: avg_success_rate,
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_admin_status_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let pool = state.agent_repo.pool();
    let db_connected = sqlx::query("SELECT 1").execute(pool).await.is_ok();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost:5432/aionhive".to_string());
    let sanitized_url = db_url
        .split('@')
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                if let Some(colon) = part.rfind(':') {
                    format!("{}:****", &part[..colon])
                } else {
                    part.to_string()
                }
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("@");

    let port: u16 = std::env::var("AION_HIVE_HTTP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    let transport_mode =
        std::env::var("AION_HIVE_TRANSPORT").unwrap_or_else(|_| "http".to_string());

    let data_dir = std::env::var("AION_HIVE_DATA_DIR").unwrap_or_else(|_| "./data".to_string());

    let releases_dir = format!("{}/releases", data_dir);

    let response = crate::api::models::AdminStatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        transport_mode,
        http_port: port,
        data_dir,
        skills_dir: releases_dir,
        db_connected,
        db_url: sanitized_url,
        jwt_expiry_hours: 24,
    };

    Ok((StatusCode::OK, Json(response)))
}

