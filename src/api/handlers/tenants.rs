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
use crate::models::tenant::NewTenant;
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

// ===== Tenant Creation Request Handlers =====

/// Helper: Check if self-service tenant approval is required
fn is_self_service_approval_mode(state: &ApiState) -> bool {
    matches!(state.tenant_config.mode, TenantMode::Sas)
        && state.tenant_config.self_service_tenant
        && state.tenant_config.tenant_approval_required
}

/// Helper: Generate slug from name
fn generate_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Create a tenant creation request (self-service workflow)
pub async fn create_tenant_request_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateTenantRequestBody>,
) -> Result<impl IntoResponse, ApiError> {
    // Check if self-service approval mode is enabled
    if !is_self_service_approval_mode(&state) {
        return Err(ApiError::BadRequest(
            "Tenant creation requests are not available in this mode".to_string(),
        ));
    }

    let identity_id = require_admin(&state, &agent_context).await?;

    // Get user info for the request
    let identity = state
        .identity
        .get(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    let applicant_name = identity.display_name.unwrap_or_else(|| {
        identity
            .username
            .clone()
            .unwrap_or_else(|| "Unknown".to_string())
    });
    let applicant_email = identity.email.unwrap_or_default();

    // Check if user already has a pending request
    if state
        .tenant
        .has_pending_request(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
    {
        return Err(ApiError::BadRequest(
            "You already have a pending tenant creation request".to_string(),
        ));
    }

    // Check max tenants per user quota
    let max_tenants = state.tenant_config.max_tenants_per_user;
    if max_tenants > 0 {
        let request_count = state
            .tenant
            .count_requests_by_applicant(identity_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        if request_count >= max_tenants as i64 {
            return Err(ApiError::BadRequest(format!(
                "You have reached the maximum number of tenant creation requests ({})",
                max_tenants
            )));
        }
    }

    // Generate slug from tenant name
    let tenant_slug = generate_slug(&body.tenant_name);

    // Create the request
    let request = state
        .tenant
        .create_tenant_request(
            identity_id,
            applicant_name,
            applicant_email,
            body.tenant_name,
            tenant_slug,
            body.message,
        )
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    tracing::info!(
        "Created tenant creation request '{}' (id={}) for applicant '{}' (id={})",
        request.tenant_name,
        request.id,
        request.applicant_name,
        request.applicant_id
    );

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": request.id,
            "tenant_name": request.tenant_name,
            "status": request.status.to_string(),
            "created_at": request.created_at,
            "message": "Tenant creation request submitted successfully"
        })),
    ))
}

/// List all tenant creation requests (super_admin only)
pub async fn list_tenant_requests_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::PaginationQuery>,
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

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let requests = state
        .tenant
        .list_tenant_requests(limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let response: Vec<crate::api::models::TenantRequestResponse> = requests
        .into_iter()
        .map(|r| crate::api::models::TenantRequestResponse {
            id: r.id,
            applicant_id: r.applicant_id,
            applicant_name: r.applicant_name,
            applicant_email: r.applicant_email,
            tenant_name: r.tenant_name,
            tenant_slug: r.tenant_slug,
            message: r.message,
            status: r.status.to_string(),
            reviewed_by: r.reviewed_by,
            reviewed_at: r.reviewed_at,
            review_note: r.review_note,
            tenant_id: r.tenant_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": response })),
    ))
}

/// Review a tenant creation request (approve/reject) - super_admin only
pub async fn review_tenant_request_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::ReviewTenantRequestBody>,
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

    // Get the request
    let request = state
        .tenant
        .get_tenant_request(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Request not found".to_string()))?;

    // Check if request is still pending
    if !matches!(
        request.status,
        crate::models::tenant::RequestStatus::Pending
    ) {
        return Err(ApiError::BadRequest(format!(
            "Request has already been {}",
            request.status
        )));
    }

    let action = body.action.to_lowercase();
    let mut created_tenant_id: Option<Uuid> = None;

    if action == "approve" {
        // Create the tenant
        let new_tenant = state
            .tenant
            .create(NewTenant {
                name: request.tenant_name.clone(),
                slug: request.tenant_slug.clone(),
                billing_plan: Some("free".to_string()),
                sso_config: None,
                settings: serde_json::json!({}),
            })
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        created_tenant_id = Some(new_tenant.id);

        // Assign tenant_admin role to the applicant
        state
            .tenant_role_assignment
            .assign(
                request.applicant_id,
                new_tenant.id,
                "tenant_admin",
                Some(identity_id),
            )
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        tracing::info!(
            "Approved tenant creation request '{}' - created tenant '{}' (id={}) for applicant '{}' (id={})",
            request.id,
            request.tenant_name,
            new_tenant.id,
            request.applicant_name,
            request.applicant_id
        );
    } else if action != "reject" {
        return Err(ApiError::BadRequest(
            "Invalid action. Use 'approve' or 'reject'".to_string(),
        ));
    }

    // Update request status
    let updated_request = state
        .tenant
        .review_tenant_request(id, &action, identity_id, body.note, created_tenant_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let response = crate::api::models::TenantRequestResponse {
        id: updated_request.id,
        applicant_id: updated_request.applicant_id,
        applicant_name: updated_request.applicant_name,
        applicant_email: updated_request.applicant_email,
        tenant_name: updated_request.tenant_name,
        tenant_slug: updated_request.tenant_slug,
        message: updated_request.message,
        status: updated_request.status.to_string(),
        reviewed_by: updated_request.reviewed_by,
        reviewed_at: updated_request.reviewed_at,
        review_note: updated_request.review_note,
        tenant_id: updated_request.tenant_id,
        created_at: updated_request.created_at,
        updated_at: updated_request.updated_at,
    };

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": response,
            "message": if action == "approve" { "Tenant created successfully" } else { "Request rejected" }
        })),
    ))
}

/// Get a single tenant creation request (super_admin only)
pub async fn get_tenant_request_handler(
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

    let request = state
        .tenant
        .get_tenant_request(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Request not found".to_string()))?;

    let response = crate::api::models::TenantRequestResponse {
        id: request.id,
        applicant_id: request.applicant_id,
        applicant_name: request.applicant_name,
        applicant_email: request.applicant_email,
        tenant_name: request.tenant_name,
        tenant_slug: request.tenant_slug,
        message: request.message,
        status: request.status.to_string(),
        reviewed_by: request.reviewed_by,
        reviewed_at: request.reviewed_at,
        review_note: request.review_note,
        tenant_id: request.tenant_id,
        created_at: request.created_at,
        updated_at: request.updated_at,
    };

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": response })),
    ))
}
