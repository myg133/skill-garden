//! Agent 注册和管理 handlers

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use tracing::info;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::ApiState;

pub async fn register_agent_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::RegisterAgentBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::agent::NewAgent;

    let secret = uuid::Uuid::new_v4().to_string();

    let new_agent = NewAgent {
        agent_id: body.agent_id.clone(),
        agent_secret: secret.clone(),
        agent_name: body.agent_name.clone(),
        org_id: None,
        capabilities: None,
    };

    state
        .agent_repo
        .create(new_agent)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to register agent: {}", e)))?;

    let response = crate::api::models::RegisterAgentResponse {
        agent_id: body.agent_id,
        secret,
        message:
            "Agent registered successfully. Store the secret securely - it will not be shown again."
                .to_string(),
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_token_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::GetTokenBody>,
) -> Result<impl IntoResponse, ApiError> {
    let valid = state
        .agent_repo
        .verify_secret(&body.agent_id, &body.agent_secret)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Authentication error: {}", e)))?;

    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let token = crate::api::generate_token(&body.agent_id, &[], &[])
        .map_err(|e| ApiError::Unauthorized(format!("{:?}", e)))?;

    let response = crate::api::models::TokenResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: 86400,
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn list_my_agents_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = agent_context.require_identity()?;

    let agents = state
        .agent_repo
        .list_by_identity(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Agent listing error: {}", e)))?;

    let items: Vec<crate::api::models::AgentListItem> = agents
        .into_iter()
        .map(|a| crate::api::models::AgentListItem {
            agent_id: a.agent_id,
            agent_name: a.agent_name,
            agent_description: a.agent_description,
            status: a.status,
            created_at: a.created_at.to_string().into(),
            last_used_at: a.last_used_at.map(|t| t.to_string()),
        })
        .collect();

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": items }))))
}

pub async fn revoke_my_agent_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(agent_id_str): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = agent_context.require_identity()?;

    let agent_id = uuid::Uuid::parse_str(&agent_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid agent ID format".to_string()))?;

    let agent = state
        .agent_repo
        .find_by_uuid(agent_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Agent lookup error: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    if agent.identity_id != Some(identity_id) {
        return Err(ApiError::Forbidden(
            "You can only revoke your own agents".to_string(),
        ));
    }

    state
        .agent_repo
        .revoke(agent_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Agent revoke error: {}", e)))?;

    info!(
        "Agent revoked: agent_id={}, identity_id={}",
        agent_id, identity_id
    );

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "Agent revoked successfully" })),
    ))
}