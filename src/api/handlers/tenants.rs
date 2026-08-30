//! 租户管理 handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use super::helpers::{require_admin, ApiState};
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use crate::TenantMode;

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
        return Err(ApiError::Forbidden(
            "Super admin access required".to_string(),
        ));
    }

    // 企业模式：private_enterprise / internal_delivery 必须指定 admin_email
    let is_enterprise = matches!(
        state.tenant_config.mode,
        TenantMode::PrivateEnterprise | TenantMode::InternalDelivery
    );

    // 提前提取 admin_email 以避免后续借用 body
    let admin_email_opt = body.admin_email.clone();

    if is_enterprise {
        let admin_email = admin_email_opt.as_ref().ok_or_else(|| {
            ApiError::BadRequest("admin_email is required in enterprise mode".to_string())
        })?;

        // 验证用户存在
        let admin_user = state
            .identity
            .get_by_email(admin_email)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
            .ok_or_else(|| {
                ApiError::BadRequest(format!("User with email '{}' not found", admin_email))
            })?;

        // 创建租户
        let tenant = state
            .tenant
            .create(body.into())
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        // 分配 tenant_admin 角色
        state
            .tenant_role_assignment
            .assign(admin_user.id, tenant.id, "tenant_admin", Some(identity_id))
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        tracing::info!(
            "Created enterprise tenant '{}' (id={}) with admin '{}' (id={})",
            tenant.name,
            tenant.id,
            admin_email,
            admin_user.id
        );

        return Ok((
            StatusCode::CREATED,
            Json(serde_json::to_value(tenant).unwrap()),
        ));
    }

    // SaaS 模式：保持原有逻辑（无需 admin_email）
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
        return Err(ApiError::Forbidden(
            "Super admin access required".to_string(),
        ));
    }
    state
        .tenant
        .delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}
