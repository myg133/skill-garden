//! API Key 管理 handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use super::helpers::{require_admin, ApiState};
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;

pub async fn list_api_keys_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    axum::extract::Query(query): axum::extract::Query<crate::api::models::ListApiKeysQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let keys = if let Some(identity_id) = query.identity_id {
        state
            .api_key
            .list_with_names_by_identity(identity_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else if let Some(org_id) = query.organization_id {
        state
            .api_key
            .list_with_names_by_organization(org_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state
            .api_key
            .list_with_names()
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": keys }))))
}

pub async fn create_api_key_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateApiKeyBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let expires_at = body.effective_expires_at();
    let request: crate::models::api_key::CreateApiKeyRequest =
        crate::models::api_key::CreateApiKeyRequest {
            identity_id: body.identity_id,
            organization_id: body.organization_id,
            name: body.name,
            scopes: body.scopes.unwrap_or_default(),
            rate_limit: body.rate_limit.unwrap_or(1000),
            expires_at,
        };
    let key = state
        .api_key
        .create(request)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(key).unwrap()),
    ))
}

pub async fn delete_api_key_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    state
        .api_key
        .delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

pub async fn update_api_key_status_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateApiKeyStatusBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    match body.status.to_lowercase().as_str() {
        "disabled" => {
            state
                .api_key
                .disable(id)
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        }
        "active" => {
            state
                .api_key
                .enable(id)
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        }
        _ => {
            return Err(ApiError::BadRequest(
                "status must be 'disabled' or 'active'".to_string(),
            ));
        }
    }
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"status": body.status})),
    ))
}

// User-facing self-service API Key handlers

pub async fn list_my_api_keys_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let keys = state
        .api_key
        .list_by_identity(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": keys }))))
}

pub async fn create_my_api_key_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateMyApiKeyBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    if let Some(org_id) = body.organization_id {
        let is_member = state
            .permission
            .is_org_member(identity_id, org_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        if !is_member {
            return Err(ApiError::Forbidden(
                "不能为不属于的组织创建 API Key".to_string(),
            ));
        }
    }

    let expires_at = body.effective_expires_at();
    let user_req = crate::models::api_key::UserCreateApiKeyRequest {
        organization_id: body.organization_id,
        name: body.name,
        scopes: body.scopes.unwrap_or_default(),
        rate_limit: body.rate_limit.unwrap_or(1000),
        expires_at,
    };
    let key = state
        .api_key
        .create_user_api_key(identity_id, user_req)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(key).unwrap()),
    ))
}

pub async fn revoke_my_api_key_handler(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let key = state
        .api_key
        .get(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("API Key not found".to_string()))?;

    if key.identity_id != identity_id {
        return Err(ApiError::Forbidden(
            "Cannot revoke another user's API key".to_string(),
        ));
    }

    state
        .api_key
        .revoke(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"revoked": id}))))
}

pub async fn update_my_api_key_status_handler(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateApiKeyStatusBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let key = state
        .api_key
        .get(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("API Key not found".to_string()))?;

    if key.identity_id != identity_id {
        return Err(ApiError::Forbidden(
            "Cannot modify another user's API key".to_string(),
        ));
    }

    match body.status.to_lowercase().as_str() {
        "disabled" => {
            state
                .api_key
                .disable(id)
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        }
        "active" => {
            state
                .api_key
                .enable(id)
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        }
        _ => {
            return Err(ApiError::BadRequest(
                "status must be 'disabled' or 'active'".to_string(),
            ));
        }
    }
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"status": body.status})),
    ))
}
