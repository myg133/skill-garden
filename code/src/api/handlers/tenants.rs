//! 租户管理 handlers

use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use crate::services::LicenseService;
use super::helpers::{require_admin, ApiState};

pub async fn list_tenants_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = require_admin(&state, &agent_context).await?;
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);
    let is_super = state
        .permission
        .is_super_admin(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let tenants = if is_super {
        state
            .tenant
            .list(limit, offset)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        let tenant_ids = state
            .permission
            .get_tenant_admin_tenant_ids(identity_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        if tenant_ids.is_empty() {
            Vec::new()
        } else {
            state
                .tenant
                .list(limit, offset)
                .await
                .map_err(|e| ApiError::InternalError(e.to_string()))?
                .into_iter()
                .filter(|t| tenant_ids.contains(&t.id))
                .collect()
        }
    };

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": tenants }))))
}

pub async fn create_tenant_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateTenantBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = require_admin(&state, &agent_context).await?;
    let is_super = state
        .permission
        .is_super_admin(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if !is_super {
        return Err(ApiError::Forbidden("Super admin access required".to_string()));
    }

    // Check license quota if configured
    let license_service = LicenseService::new();
    if license_service.is_configured() {
        let current_count = state
            .tenant
            .count()
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        if let Err(quota_error) = license_service.can_create_tenant(current_count) {
            tracing::warn!(
                "Tenant creation blocked: quota exceeded (max: {}, current: {})",
                quota_error.max_tenants,
                quota_error.current_count
            );
            return Err(ApiError::BadRequest(format!(
                "Tenant quota exceeded: license allows {} tenants, but {} tenants already exist. Please upgrade your license or remove existing tenants.",
                quota_error.max_tenants,
                quota_error.current_count
            )));
        }
    }

    let tenant = state
        .tenant
        .create(body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(tenant).unwrap()),
    ))
}

pub async fn get_tenant_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = require_admin(&state, &agent_context).await?;
    let is_super = state
        .permission
        .is_super_admin(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if !is_super {
        let tenant_ids = state
            .permission
            .get_tenant_admin_tenant_ids(identity_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        if !tenant_ids.contains(&id) {
            return Err(ApiError::NotFound("Tenant not found".to_string()));
        }
    }
    let tenant = state
        .tenant
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Tenant not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(tenant).unwrap())))
}

pub async fn update_tenant_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateTenantBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = require_admin(&state, &agent_context).await?;
    let is_super = state
        .permission
        .is_super_admin(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if !is_super {
        let tenant_ids = state
            .permission
            .get_tenant_admin_tenant_ids(identity_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        if !tenant_ids.contains(&id) {
            return Err(ApiError::NotFound("Tenant not found".to_string()));
        }
    }
    let tenant = state
        .tenant
        .update(id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(tenant).unwrap())))
}

pub async fn delete_tenant_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = require_admin(&state, &agent_context).await?;
    let is_super = state
        .permission
        .is_super_admin(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if !is_super {
        return Err(ApiError::Forbidden("Super admin access required".to_string()));
    }
    state
        .tenant
        .delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}