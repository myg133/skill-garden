//! API Route Handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::api::error::ApiError;
use crate::api::http_state::AppRouterState;
use crate::api::jwt::AgentContext;
use crate::models::error::AppError;
use crate::models::evaluation::{ErrorType as EvalErrorType, EvalTag};
use crate::models::{NewSkill, SkillUpdate};

pub type ApiState = Arc<AppRouterState>;

/// 辅助函数：从 Skill 模型中提取字段并执行权限校验
async fn check_skill_perm(
    state: &ApiState,
    identity_id: Option<uuid::Uuid>,
    skill: &crate::models::Skill,
    action: crate::services::SkillAction,
) -> Result<(), ApiError> {
    let vis_str = match &skill.visibility {
        crate::models::skill_policy::Visibility::Private => "private",
        crate::models::skill_policy::Visibility::OrgVisible => "org_visible",
        crate::models::skill_policy::Visibility::Marketplace => "marketplace",
        crate::models::skill_policy::Visibility::Shared => "shared",
    };
    check_skill_perm_raw(
        state,
        identity_id,
        &skill.owner_type,
        skill.owner_id,
        skill.author_identity_id,
        &skill.status,
        vis_str,
        skill.marketplace_status.as_deref(),
        action,
    )
    .await
}

/// 辅助函数：使用 DB Skill 类型执行权限校验
async fn check_skill_perm_db(
    state: &ApiState,
    identity_id: Option<uuid::Uuid>,
    skill: &crate::db::repositories::skill::Skill,
    action: crate::services::SkillAction,
) -> Result<(), ApiError> {
    check_skill_perm_raw(
        state,
        identity_id,
        &skill.owner_type,
        skill.owner_id,
        skill.author_identity_id,
        &skill.status,
        &skill.visibility,
        skill.marketplace_status.as_deref(),
        action,
    )
    .await
}

/// 辅助函数：使用原始字段值执行权限校验
async fn check_skill_perm_raw(
    state: &ApiState,
    identity_id: Option<uuid::Uuid>,
    owner_type: &str,
    owner_id: Option<uuid::Uuid>,
    author_identity_id: Option<uuid::Uuid>,
    skill_status: &str,
    visibility: &str,
    marketplace_status: Option<&str>,
    action: crate::services::SkillAction,
) -> Result<(), ApiError> {
    let id_id = identity_id.ok_or_else(|| {
        tracing::warn!(
            "Permission check blocked: no identity_id in token (action={:?}, owner_type={})",
            action,
            owner_type
        );
        ApiError::Forbidden("身份信息缺失，请使用新版 API Key 重新认证".to_string())
    })?;

    state
        .permission
        .check_skill_permission(
            id_id,
            owner_type,
            owner_id,
            author_identity_id,
            skill_status,
            visibility,
            marketplace_status,
            action,
        )
        .await
        .map_err(|e| ApiError::Forbidden(e))?;
    Ok(())
}

/// 辅助函数：批量解析 tenant_roles 中的租户名称（避免 N+1 循环查询）
/// 一次 SQL 查询替代 N 次逐个查询
async fn build_tenant_role_infos(
    state: &ApiState,
    tenant_roles: &[(uuid::Uuid, String)],
) -> Vec<crate::api::models::TenantRoleInfo> {
    if tenant_roles.is_empty() {
        return Vec::new();
    }
    let ids: Vec<uuid::Uuid> = tenant_roles.iter().map(|(id, _)| *id).collect();
    let name_map = state
        .tenant
        .get_names_by_ids(&ids)
        .await
        .unwrap_or_default();

    tenant_roles
        .iter()
        .map(|(tenant_id, role_name)| crate::api::models::TenantRoleInfo {
            tenant_id: *tenant_id,
            tenant_name: name_map
                .get(tenant_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
            role_name: role_name.clone(),
        })
        .collect()
}

// ============================================================
// Phase 1-3: Unified auth entry points - delegate to PermissionService
// ============================================================

/// 统一的管理员权限检查，替代 agent_context.require_admin()
/// 允许 super_admin 以及任意租户的 tenant_admin 访问 /admin/* 路由
/// 快速路径：JWT claims 含 "admin" → 直接通过（向后兼容，无需查 DB）
/// 慢路径：通过 PermissionService 检查管理员角色（单表/双表查询，不做完整 build_context）
async fn require_admin(state: &ApiState, agent_context: &AgentContext) -> Result<uuid::Uuid, ApiError> {
    // Fast path: JWT claim check (backward compatible for existing tokens)
    if agent_context.roles.iter().any(|r| r == "admin") {
        return agent_context.require_identity();
    }

    let identity_id = agent_context.require_identity()?;
    let is_admin = state
        .permission
        .is_any_admin(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    if is_admin {
        return Ok(identity_id);
    }

    Err(ApiError::Forbidden("Admin access required".to_string()))
}

/// 市场管理员权限检查（super_admin / marketplace_admin / marketplace_reviewer）
/// Phase 3: marketplace_admin 可访问市场管理相关路由
/// marketplace_reviewer 拥有审核队列的审批/驳回权限
async fn require_marketplace_admin(state: &ApiState, agent_context: &AgentContext) -> Result<uuid::Uuid, ApiError> {
    // Fast path: JWT claim check (backward compatible for existing tokens)
    if agent_context.roles.iter().any(|r| r == "admin") {
        return agent_context.require_identity();
    }

    let identity_id = agent_context.require_identity()?;
    let has_role = state
        .permission
        .has_any_system_role(identity_id, &["super_admin", "marketplace_admin", "marketplace_reviewer"])
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    if has_role {
        return Ok(identity_id);
    }

    Err(ApiError::Forbidden("Marketplace admin access required".to_string()))
}

/// 市场管理员权限检查（仅 super_admin / marketplace_admin，不含 marketplace_reviewer）
/// 用于 market 上下架/重新上架等需要全权的操作
async fn require_marketplace_admin_only(state: &ApiState, agent_context: &AgentContext) -> Result<uuid::Uuid, ApiError> {
    if agent_context.roles.iter().any(|r| r == "admin") {
        return agent_context.require_identity();
    }

    let identity_id = agent_context.require_identity()?;
    let has_role = state
        .permission
        .has_any_system_role(identity_id, &["super_admin", "marketplace_admin"])
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    if has_role {
        return Ok(identity_id);
    }

    Err(ApiError::Forbidden("Marketplace admin (full) access required".to_string()))
}

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

// Tenant handlers

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
        // tenant_admin: only return tenants they administer
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
    // Only super_admin can create tenants
    let is_super = state
        .permission
        .is_super_admin(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if !is_super {
        return Err(ApiError::Forbidden("Super admin access required".to_string()));
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
    // tenant_admin can only access their own tenants
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
    // tenant_admin can only update their own tenants
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
    // Only super_admin can delete tenants
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

// Identity handlers

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

// Group handlers

pub async fn list_groups_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListGroupsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let org_id = query.organization_id;
    let groups = if let Some(org_id) = org_id {
        state
            .group
            .list_by_organization(org_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state
            .group
            .list()
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": groups }))))
}

pub async fn create_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let subject = agent_context.subject;
    let permission_overrides = body.permission_overrides.clone();
    let new_group: crate::models::group::NewGroup = body.into();
    let group = state
        .group
        .create(new_group)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if let Some(overrides) = permission_overrides {
        let creator_id = uuid::Uuid::parse_str(&subject).ok();
        for ov in overrides {
            state
                .group_perm_override_repo
                .upsert_override(
                    crate::models::group_permission_override::NewGroupPermissionOverride {
                        group_id: group.id,
                        role_name: ov.role_name,
                        permission_code: ov.permission_code,
                        granted: ov.granted,
                        created_by: creator_id,
                    },
                )
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        }
    }

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_created".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group.id.to_string()),
            details: serde_json::json!({
                "group_name": group.name,
                "organization_id": group.organization_id,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(group).unwrap()),
    ))
}

pub async fn get_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let group = state
        .group
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Group not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn update_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let group = state
        .group
        .update(id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn delete_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    state
        .group
        .delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

// Role handlers

pub async fn list_roles_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let roles = state
        .role
        .list()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": roles }))))
}

pub async fn get_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let role = state
        .role
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Role not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(role).unwrap())))
}

// API Key handlers

pub async fn list_api_keys_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListApiKeysQuery>,
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
    let request: crate::models::api_key::CreateApiKeyRequest = crate::models::api_key::CreateApiKeyRequest {
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
    Ok((StatusCode::OK, Json(serde_json::json!({"status": body.status}))))
}

// User-facing self-service API Key handlers (6.5)

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

    // 验证：如果提供了 organization_id，必须是用户所属组织
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
    Ok((StatusCode::OK, Json(serde_json::json!({"status": body.status}))))
}

// Audit entries handler

pub async fn list_audit_entries_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListAuditEntriesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    let audit_query = crate::models::api_key::AuditLogQuery {
        tenant_id: query.tenant_id,
        organization_id: query.organization_id,
        identity_id: query.identity_id,
        action: query.action,
        resource_type: None,
        limit: Some(limit),
        offset: Some(offset),
    };
    let entries = state
        .audit
        .query(audit_query)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": entries }))))
}

// Role CRUD handlers (C/U/D)

pub async fn create_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let new_role = crate::models::role::NewRole {
        name: body.name,
        role_type: crate::models::role::RoleType::from(body.role_type.as_str()),
        scope_level: crate::models::role::ScopeLevel::from(body.scope_level.as_str()),
        parent_role_id: body.parent_role_id,
        permissions: body.permissions,
        description: body.description,
    };
    let role = state
        .role
        .create(new_role)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(role).unwrap()),
    ))
}

pub async fn update_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let update = crate::models::role::RoleUpdate {
        name: body.name,
        permissions: body.permissions,
        description: body.description,
    };
    let role = state
        .role
        .update(id, update)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(role).unwrap())))
}

pub async fn delete_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    state
        .role
        .delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}

// Identity role assignment handlers

pub async fn get_identity_roles_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let roles = state
        .role
        .get_identity_roles(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": roles }))))
}

pub async fn grant_identity_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::GrantRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let admin_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid admin subject".to_string()))?;
    let request = crate::models::role::GrantRoleRequest {
        identity_id: id,
        role_id: body.role_id,
        scope_id: body.scope_id,
        expires_at: body.expires_at,
    };
    let identity_role = state
        .role
        .grant_role(request, admin_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(identity_role).unwrap()),
    ))
}

pub async fn revoke_identity_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((identity_id, role_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<crate::api::models::RevokeRoleQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    state
        .role
        .revoke_role(identity_id, role_id, query.scope_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"revoked": role_id})),
    ))
}

pub async fn get_identity_permissions_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let permissions = state
        .role
        .get_identity_permissions(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": permissions })),
    ))
}

// System role assignment handlers

pub async fn assign_system_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AssignSystemRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let admin_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid admin subject".to_string()))?;
    if !crate::models::system_role_assignment::SystemRole::is_valid_super_admin_role(&body.role_name) {
        return Err(ApiError::BadRequest(format!(
            "Invalid system role for super admin assignment: {}. Only super_admin/marketplace_admin allowed.",
            body.role_name
        )));
    }
    let assignment = state
        .system_role_assignment
        .assign(body.identity_id, &body.role_name, Some(admin_id))
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(assignment).unwrap()),
    ))
}

pub async fn revoke_system_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::RevokeSystemRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    state
        .system_role_assignment
        .revoke(body.identity_id, &body.role_name)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"revoked": true}))))
}

pub async fn list_system_role_assignments_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListSystemRoleAssignmentsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let assignments = if let Some(identity_id) = query.identity_id {
        state
            .system_role_assignment
            .find_by_identity(identity_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else if let Some(role_name) = &query.role_name {
        state
            .system_role_assignment
            .list_by_role(role_name)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state
            .system_role_assignment
            .list_all()
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": assignments })),
    ))
}

pub async fn get_identity_system_roles_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let assignments = state
        .system_role_assignment
        .find_by_identity(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": assignments })),
    ))
}

// Marketplace reviewer assignment handlers (marketplace_admin assigns marketplace_reviewer)

pub async fn assign_marketplace_reviewer_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AssignMarketplaceReviewerBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin_only(&state, &agent_context).await?;
    let admin_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid admin subject".to_string()))?;
    // Prevent self-assignment
    if body.identity_id == admin_id {
        return Err(ApiError::BadRequest("Cannot modify your own role".to_string()));
    }
    let role_name = "marketplace_reviewer";
    let assignment = state
        .system_role_assignment
        .assign(body.identity_id, role_name, Some(admin_id))
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(assignment).unwrap()),
    ))
}

pub async fn revoke_marketplace_reviewer_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::RevokeMarketplaceReviewerBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin_only(&state, &agent_context).await?;
    let admin_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid admin subject".to_string()))?;
    if body.identity_id == admin_id {
        return Err(ApiError::BadRequest("Cannot modify your own role".to_string()));
    }
    state
        .system_role_assignment
        .revoke(body.identity_id, "marketplace_reviewer")
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"revoked": true}))))
}

pub async fn list_marketplace_reviewers_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let assignments = state
        .system_role_assignment
        .list_by_role("marketplace_reviewer")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": assignments })),
    ))
}

// Tenant role assignment handlers (tenant_admin assigns org owner/admin)

pub async fn assign_tenant_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AssignTenantRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let admin_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid admin subject".to_string()))?;
    if !matches!(body.role_name.as_str(), "tenant_admin") {
        return Err(ApiError::BadRequest(
            "Only tenant_admin role can be assigned at tenant level".to_string(),
        ));
    }
    let assignment = state
        .tenant_role_assignment
        .assign(body.identity_id, body.tenant_id, &body.role_name, Some(admin_id))
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(assignment).unwrap()),
    ))
}

pub async fn revoke_tenant_role_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::RevokeTenantRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    state
        .tenant_role_assignment
        .revoke(body.identity_id, body.tenant_id, &body.role_name)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"revoked": true}))))
}

pub async fn list_tenant_role_assignments_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListTenantRoleAssignmentsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let assignments = if let Some(tenant_id) = query.tenant_id {
        state
            .tenant_role_assignment
            .list_by_tenant(tenant_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else if let Some(identity_id) = query.identity_id {
        state
            .tenant_role_assignment
            .find_by_identity(identity_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        return Err(ApiError::BadRequest(
            "Must provide tenant_id or identity_id".to_string(),
        ));
    };
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": assignments })),
    ))
}

// Role permission management handlers

pub async fn list_role_permissions_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let permissions = if let (Some(role_level), Some(role_name)) =
        (query.get("role_level"), query.get("role_name"))
    {
        state
            .role_permission
            .list_by_role(role_level, role_name)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state
            .role_permission
            .list_all()
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": permissions })),
    ))
}

pub async fn create_role_permission_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateRolePermissionBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let new_perm = crate::models::role_permission::NewRolePermission {
        role_level: body.role_level,
        role_name: body.role_name,
        permission_code: body.permission_code,
        scope_restriction: body.scope_restriction,
    };
    let perm = state
        .role_permission
        .add_permission(new_perm)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(perm).unwrap()),
    ))
}

pub async fn delete_role_permission_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::DeleteRolePermissionQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    state
        .role_permission
        .remove_permission(&query.role_level, &query.role_name, &query.permission_code)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": true}))))
}

// Permission check handlers

pub async fn check_permission_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::PermissionCheckBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&agent_context.subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;
    let ctx = state
        .permission
        .build_context(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let resource = crate::services::permission::ResourceScope {
        owner_type: body.owner_type,
        owner_id: body.owner_id,
        author_identity_id: body.author_identity_id,
        tenant_id: body.tenant_id,
        organization_id: body.organization_id,
        group_id: body.group_id,
    };
    let has_perm = state
        .permission
        .has_permission(&ctx, &body.permission_code, Some(&resource))
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"has_permission": has_perm})),
    ))
}

pub async fn get_permission_context_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let ctx = state
        .permission
        .build_context(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "identity_id": ctx.identity_id,
            "system_roles": ctx.system_roles,
            "org_roles": ctx.org_roles,
            "group_roles": ctx.group_roles,
        })),
    ))
}

/// Sandbox Admin API Handlers

pub async fn list_sandboxes_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let sandboxes = state
        .sandbox
        .list_containers()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": sandboxes })),
    ))
}

pub async fn get_sandbox_health_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let docker_healthy = state.sandbox.health_check().await.unwrap_or(false);
    let containers = state.sandbox.list_containers().await.unwrap_or_default();

    let response = crate::api::models::SandboxHealthResponse {
        docker_connected: docker_healthy,
        active_containers: containers.len() as u32,
        containers: containers
            .into_iter()
            .map(serde_json::to_value)
            .filter_map(|r| r.ok())
            .collect(),
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn execute_tool_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::ExecuteToolBody>,
) -> Result<impl IntoResponse, ApiError> {
    // Ensure the org tool exists and is approved before execution
    let org_id_uuid = Uuid::parse_str(&body.org_id)
        .map_err(|_| ApiError::BadRequest("Invalid org_id".to_string()))?;
    let tool = state
        .org_tool
        .get_tool_by_tool_id(org_id_uuid, &body.tool_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let tool = match tool {
        Some(t) if t.status == "approved" => t,
        Some(_) => {
            return Err(ApiError::Forbidden(
                "Tool must be approved before execution".to_string(),
            ));
        }
        None => {
            return Err(ApiError::NotFound(format!(
                "Tool {} not found in organization {}",
                body.tool_id, body.org_id
            )));
        }
    };

    // Read defaults from stored implementation config; request body can override
    let impl_docker = tool
        .implementation
        .get("docker_image")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let impl_timeout = tool
        .implementation
        .get("timeout_seconds")
        .and_then(|v| v.as_u64());
    let impl_cmd = tool
        .implementation
        .get("cmd")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        });

    let request = crate::services::ToolExecutionRequest {
        tool_id: body.tool_id,
        org_id: body.org_id,
        parameters: body.parameters,
        timeout_seconds: body.timeout_seconds.or(impl_timeout).unwrap_or(30),
        docker_image: body.docker_image.or(impl_docker),
        session_id: None,
        cmd: impl_cmd,
    };

    let result = state
        .sandbox
        .execute_org_tool(request)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "subject": subject,
            "result": result
        })),
    ))
}

pub async fn execute_platform_tool_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::ExecutePlatformToolBody>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .sandbox
        .execute_platform_tool(&body.tool_name, body.parameters, body.timeout_seconds)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "subject": subject,
            "result": result
        })),
    ))
}

pub async fn remove_sandbox_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    state
        .sandbox
        .remove_sandbox(&key)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "removed": key }))))
}

/// Release a sandbox by org_id + tool_id (non-admin, any authenticated user).
pub async fn release_sandbox_handler(
    State(state): State<ApiState>,
    AgentContext { .. }: AgentContext,
    Json(body): Json<crate::api::models::ReleaseSandboxBody>,
) -> Result<impl IntoResponse, ApiError> {
    let released = state
        .sandbox
        .release_sandbox(&body.org_id, &body.tool_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "released": released,
            "org_id": body.org_id,
            "tool_id": body.tool_id
        })),
    ))
}

/// List sandbox status (authenticated users, not admin-only).
pub async fn list_sandbox_status_handler(
    State(state): State<ApiState>,
    AgentContext { .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let now = chrono::Utc::now().timestamp();
    let sandboxes: Vec<crate::api::models::SandboxInfoItem> = state
        .sandbox
        .list_active_sandboxes()
        .into_iter()
        .map(|info| {
            let idle = now - info.last_used.timestamp();
            crate::api::models::SandboxInfoItem {
                key: info.id,
                container_id: info.container_id,
                image: info.image,
                status: info.status.to_string(),
                idle_seconds: idle,
                created_at: info.created_at.to_rfc3339(),
            }
        })
        .collect();

    let status = crate::api::models::SandboxStatusResponse {
        total: sandboxes.len(),
        max: state.sandbox.max_containers(),
        containers: sandboxes,
    };

    Ok((StatusCode::OK, Json(serde_json::json!(status))))
}

/// Git Proxy Admin API Handlers

pub async fn list_git_branches_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(repo_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let branches = state
        .git_proxy
        .list_branches(&repo_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": branches })),
    ))
}

pub async fn get_git_commits_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((repo_id, limit)): Path<(String, u32)>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let commits = state
        .git_proxy
        .get_commits(&repo_id, limit)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": commits }))))
}

pub async fn get_git_file_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((repo_id, path, commit)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let file = state
        .git_proxy
        .get_file_at_commit(&repo_id, &path, &commit)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "path": file.path,
            "content": file.content,
            "size": file.size
        })),
    ))
}

pub async fn get_git_diff_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((repo_id, from, to)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let diff = state
        .git_proxy
        .get_diff(&repo_id, &from, &to)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "from_commit": diff.from_commit,
            "to_commit": diff.to_commit,
            "files_changed": diff.files_changed,
            "additions": diff.additions,
            "deletions": diff.deletions
        })),
    ))
}

pub async fn validate_git_url_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::ValidateGitUrlBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let valid = state
        .git_proxy
        .validate_git_url(&body.git_url)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "valid": valid }))))
}

pub async fn get_git_proxy_health_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let healthy = state.git_proxy.health_check().await.unwrap_or(false);

    let response = crate::api::models::GitProxyHealthResponse {
        git_proxy_connected: healthy,
        api_base: std::env::var("GIT_PROXY_API_BASE")
            .unwrap_or_else(|_| "http://localhost:8081".to_string()),
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn list_skills_handler(
    State(state): State<ApiState>,
    AgentContext { identity_id, .. }: AgentContext,
    Query(query): Query<crate::api::models::ListSkillsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);

    let skills = state
        .registry
        .list_skills()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // RBAC: build permission context
    let is_super = if let Some(id_id) = identity_id {
        state
            .permission
            .is_super_admin(id_id)
            .await
            .unwrap_or(false)
    } else {
        false
    };

    // Apply scope filters (Phase 6: role-based views)
    let mut filtered: Vec<_> = if let Some(org_id) = query.org_id {
        // Org member view: only skills owned by this org
        skills
            .into_iter()
            .filter(|s| s.owner_type == "organization" && s.owner_id == Some(org_id))
            .collect()
    } else if let Some(ref mkt_status) = query.marketplace_status {
        // Marketplace admin view: filter by marketplace_status
        skills
            .into_iter()
            .filter(|s| {
                s.marketplace_status
                    .as_ref()
                    .map(|ms| ms == mkt_status)
                    .unwrap_or(false)
            })
            .collect()
    } else if query.scope_personal.unwrap_or(false) {
        // Personal scope view
        skills
            .into_iter()
            .filter(|s| {
                s.owner_type == "user"
                    && identity_id.is_some()
                    && (s.owner_id == identity_id || s.author_identity_id == identity_id)
            })
            .collect()
    } else if is_super {
        skills
    } else {
        // 检查是否为市场管理员（需要看到待审核的市场 Skill）
        let is_market_admin = if let Some(id_id) = identity_id {
            state
                .permission
                .has_any_system_role(id_id, &["super_admin", "marketplace_admin", "marketplace_reviewer"])
                .await
                .unwrap_or(false)
        } else {
            false
        };

        skills
            .into_iter()
            .filter(|s| {
                // Published marketplace skills visible to all
                let is_marketplace_published = s.status == "published"
                    && matches!(
                        s.visibility,
                        crate::models::skill_policy::Visibility::Marketplace
                    );
                // User's own skills
                let is_own = s.owner_type == "user"
                    && identity_id.is_some()
                    && (s.owner_id == identity_id || s.author_identity_id == identity_id);
                // 市场管理员可以看到所有已提交市场的 Skill（任何 marketplace_status）
                let is_market_admin_visible = is_market_admin && s.marketplace_status.is_some();
                is_marketplace_published || is_own || is_market_admin_visible
            })
            .collect()
    };

    if let Some(ref tag) = query.tag {
        filtered.retain(|s| s.tags.iter().any(|t| t == tag));
    }

    if let Some(ref keyword) = query.keyword {
        let keyword_lower = keyword.to_lowercase();
        filtered.retain(|s| {
            s.name.to_lowercase().contains(&keyword_lower)
                || s.description.to_lowercase().contains(&keyword_lower)
        });
    }

    let total = filtered.len();
    let start = (page - 1) * page_size;
    let end = (start + page_size).min(total);

    let page_items: Vec<_> = if start < total {
        filtered[start..end].to_vec()
    } else {
        vec![]
    };

    let response = crate::api::models::ListResponse::new(page_items, total, page, page_size);
    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_skill_handler(
    State(state): State<ApiState>,
    AgentContext { identity_id, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;

    // RBAC 权限校验：需要 Read 权限
    check_skill_perm(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::Read,
    )
    .await?;

    let stats = state.evaluator.get_stats(&skill_id).await.ok();

    let detail = crate::models::SkillDetail {
        metadata: (&skill).into(),
        content: skill.content,
        stats,
    };
    Ok((StatusCode::OK, Json(detail)))
}

pub async fn create_skill_handler(
    State(state): State<ApiState>,
    AgentContext {
        subject,
        identity_id,
        org_id: agent_org_id,
        roles,
        ..
    }: AgentContext,
    Json(body): Json<crate::api::models::CreateSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id =
        identity_id.ok_or_else(|| ApiError::Unauthorized("identity_id required".to_string()))?;

    let is_admin = roles.iter().any(|r| r == "admin");

    // owner_type: body 显式指定 > 自动推断（agent_org_id 存在 → organization，否则 user）
    let effective_owner_type = body.owner_type.as_deref().unwrap_or_else(|| {
        if agent_org_id.is_some() {
            "organization"
        } else {
            "user"
        }
    });

    let (owner_type, owner_id, default_visibility) = if effective_owner_type == "organization" {
        // organization_id: body 显式指定 > 调用者关联的组织
        let org_id = body.organization_id.or(agent_org_id).ok_or_else(|| {
            ApiError::BadRequest(
                "organization_id is required when owner_type is organization".to_string(),
            )
        })?;

        // 验证用户属于该组织（admin 跳过组织成员校验）
        if !is_admin {
            let is_member = state
                .permission
                .is_org_member(identity_id, org_id)
                .await
                .map_err(|e| ApiError::InternalError(e.to_string()))?;
            if !is_member {
                return Err(ApiError::Forbidden(
                    "你不能为不属于的组织创建 Skill".to_string(),
                ));
            }
        }

        (
            "organization".to_string(),
            Some(org_id),
            crate::models::skill_policy::Visibility::OrgVisible,
        )
    } else {
        // 个人用户创建 Skill 时，自动设置为本人所有
        (
            "user".to_string(),
            Some(identity_id),
            crate::models::skill_policy::Visibility::Private,
        )
    };

    // 用户显式指定的 visibility 优先，否则使用按 owner_type 的默认值
    let visibility = match body.visibility.as_deref() {
        Some("private") => crate::models::skill_policy::Visibility::Private,
        Some("org_visible") => crate::models::skill_policy::Visibility::OrgVisible,
        Some("marketplace") => crate::models::skill_policy::Visibility::Marketplace,
        Some("shared") => crate::models::skill_policy::Visibility::Shared,
        _ => default_visibility,
    };

    let new_skill = NewSkill {
        name: body.name,
        description: body.description,
        tags: body.tags,
        content: body.content,
        version: body.version.unwrap_or_else(|| "1.0.0".to_string()),
        git_url: body.git_url.clone(),
        visibility: Some(visibility),
        tools: body.tools.clone(),
        owner_type,
        owner_id,
        author_identity_id: Some(identity_id),
    };

    let skill = state
        .registry
        .create_skill(new_skill, &subject, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create skill: {}", e)))?;

    let response = crate::api::models::SkillCreatedResponse {
        message: "Skill created successfully".to_string(),
        skill_id: skill.id,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn update_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::UpdateSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    let is_market_admin = require_marketplace_admin(&state, &agent_context).await.is_ok();

    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;

    let identity_id = agent_context.require_identity()?;
    let subject = agent_context.subject.clone();

    if !is_market_admin {
        check_skill_perm(
            &state,
            Some(identity_id),
            &skill,
            crate::services::SkillAction::Update,
        )
        .await?;
    }

    // 如果当前是市场已上架状态，编辑内容需要走审核流程
    if skill.marketplace_status.as_deref() == Some("listed")
        || skill.marketplace_status.as_deref() == Some("pending_update")
    {
        use crate::db::repositories::skill::SkillRepository;
        let pool = state.agent_repo.pool().clone();
        let skill_repo = SkillRepository::new(pool);

        // 构建 draft_content
        let mut draft = serde_json::Map::new();
        if let Some(ref desc) = body.description {
            draft.insert("description".to_string(), serde_json::Value::String(desc.clone()));
        }
        if let Some(ref tags) = body.tags {
            draft.insert("tags".to_string(), serde_json::json!(tags));
        }
        if let Some(ref content) = body.content {
            draft.insert("content".to_string(), serde_json::Value::String(content.clone()));
        }

        skill_repo
            .save_draft_content(&skill_id, &serde_json::Value::Object(draft))
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to save draft: {}", e)))?;

        if skill.marketplace_status.as_deref() != Some("pending_update") {
            skill_repo
                .update_marketplace_status(&skill_id, Some("pending_update"))
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to set pending_update: {}", e)))?;
        }

        state
            .audit_repo
            .create(crate::db::repositories::audit::NewAuditLog {
                agent_id: Some(subject),
                action: "marketplace_update_submitted".to_string(),
                resource_type: "skill".to_string(),
                resource_id: Some(skill_id.clone()),
                details: serde_json::json!({
                    "skill_name": skill.name,
                    "marketplace_status": "pending_update",
                }),
            })
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        return Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Update submitted for review",
                "skill_id": skill_id,
                "marketplace_status": "pending_update",
            })),
        ));
    }

    // 非市场 Skill：直接更新
    let visibility = body.visibility.as_ref().map(|v| match v.as_str() {
        "private" => crate::models::skill_policy::Visibility::Private,
        "org_visible" => crate::models::skill_policy::Visibility::OrgVisible,
        "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
        "shared" => crate::models::skill_policy::Visibility::Shared,
        _ => crate::models::skill_policy::Visibility::OrgVisible,
    });

    let update = SkillUpdate {
        description: body.description,
        tags: body.tags,
        content: body.content,
        git_url: body.git_url.clone(),
        visibility,
        tools: body.tools.clone(),
    };

    state
        .registry
        .update_skill(&skill_id, update, &subject, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update skill: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill updated successfully",
        })),
    ))
}

pub async fn delete_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { identity_id, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    // 权限校验：只有所有者和管理员可以删除
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;
    check_skill_perm(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::Delete,
    )
    .await?;

    // 市场 Skill 在上架中或等待下架审核期间不允许删除
    if skill.marketplace_status.as_deref() == Some("listed")
        || skill.marketplace_status.as_deref() == Some("pending_delist")
    {
        return Err(ApiError::BadRequest(
            "Cannot delete this marketplace skill. Please delist it first.".to_string(),
        ));
    }

    state
        .registry
        .delete_skill(&skill_id, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to delete skill: {}", e)))?;

    let response = crate::api::models::MessageResponse {
        message: "Skill deleted successfully".to_string(),
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_skill_stats_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // 先确认 skill 存在
    state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // 获取统计；如果没有评价数据则返回默认值
    let stats = state
        .evaluator
        .get_stats(&skill_id)
        .await
        .unwrap_or_else(|_| crate::models::evaluation::SkillStats {
            skill_id: skill_id.clone(),
            success_rate: 0.0,
            avg_duration_ms: 0,
            total_evaluations: 0,
            unique_agents: 0,
            confidence: 0.0,
            tags: vec![],
            local_version: None,
            latest_version: "1.0.0".to_string(),
            upgrade_available: false,
        });

    Ok((StatusCode::OK, Json(stats)))
}

/// GET /api/v1/skills/:id/files — 列出 Skill 包中的所有文件
pub async fn list_skill_files_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;

    let files = state
        .skill_git
        .list_files_at_version(&skill.name, &skill.version)
        .unwrap_or_default();

    Ok((StatusCode::OK, Json(serde_json::json!({ "files": files }))))
}

/// GET /api/v1/skills/:id/files/*path — 获取 Skill 包中某个文件的内容
pub async fn get_skill_file_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    axum::extract::Path((skill_id, file_path)): axum::extract::Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;

    let content = state
        .skill_git
        .get_file_at_version(&skill.name, &skill.version, &file_path)
        .map_err(|e| ApiError::NotFound(format!("File '{}' not found: {}", file_path, e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "path": file_path, "content": content })),
    ))
}

pub async fn create_evaluation_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateEvaluationBody>,
) -> Result<
    (
        StatusCode,
        Json<crate::api::models::EvaluationCreatedResponse>,
    ),
    ApiError,
> {
    let error_type = body.error_type.as_ref().and_then(|e| match e.as_str() {
        "timeout" => Some(EvalErrorType::Timeout),
        "crash" => Some(EvalErrorType::Crash),
        "logic_error" => Some(EvalErrorType::LogicError),
        _ => Some(EvalErrorType::Other),
    });

    let tags = body
        .tags
        .iter()
        .filter_map(|t| match t.as_str() {
            "reliable" => Some(EvalTag::Reliable),
            "fast" => Some(EvalTag::Fast),
            "stable" => Some(EvalTag::Stable),
            "experimental" => Some(EvalTag::Experimental),
            _ => None,
        })
        .collect();

    let result = state
        .evaluator
        .add_evaluation(
            body.skill_id,
            subject,
            body.success,
            body.duration_ms,
            error_type,
            tags,
        )
        .await
        .map_err(|e: AppError| ApiError::BadRequest(e.to_string()))?;

    let response = crate::api::models::EvaluationCreatedResponse {
        message: "Evaluation recorded successfully".to_string(),
        evaluation_id: result.evaluation_id,
        new_stats: result.new_stats,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

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

/// 列出当前用户注册的所有 Agent
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

/// 撤销一个 Agent Token
pub async fn revoke_my_agent_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(agent_id_str): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = agent_context.require_identity()?;

    let agent_id = uuid::Uuid::parse_str(&agent_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid agent ID format".to_string()))?;

    // 查找 agent 并验证归属
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

pub async fn user_login_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::UserLoginBody>,
) -> Result<impl IntoResponse, ApiError> {
    let rate_key = format!("user_login:{}", body.username);
    if !state.login_rate_limiter.check(&rate_key).await {
        return Err(ApiError::TooManyRequests(
            "Too many login attempts. Please try again later.".to_string(),
        ));
    }

    // 合并 verify + get 为一次 DB 查询
    let user = state
        .identity
        .verify_password_and_get_user(&body.username, &body.password)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Authentication error: {}", e)))?
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;

    // 检查账号状态
    if user.status != crate::models::identity::IdentityStatus::Active {
        return Err(ApiError::Unauthorized(format!(
            "Account is {}. Please contact administrator.",
            user.status
        )));
    }

    // 获取用户所在组织
    let orgs = state
        .permission
        .get_user_orgs(user.id)
        .await
        .unwrap_or_default();
    let organizations: Vec<crate::api::models::UserOrgInfo> = orgs
        .into_iter()
        .map(|o| crate::api::models::UserOrgInfo {
            id: o.id,
            name: o.name,
            slug: o.slug,
            role: o.role,
        })
        .collect();

    // 构建权限上下文，获取 system_roles 和 tenant_roles（必须在 JWT 生成之前，以判断 is_admin）
    let perm_ctx = state
        .permission
        .build_context(user.id)
        .await
        .unwrap_or(crate::services::permission::PermissionContext {
            identity_id: user.id,
            system_roles: std::collections::HashSet::new(),
            tenant_roles: Vec::new(),
            org_roles: Vec::new(),
            group_roles: Vec::new(),
        });

    let system_roles: Vec<String> = perm_ctx.system_roles.into_iter().collect();

    let tenant_roles = build_tenant_role_infos(&state, &perm_ctx.tenant_roles).await;

    // is_admin: 同时检查 is_system_admin 列、system_role_assignments 和 tenant_admin
    let is_admin = user.is_system_admin
        || system_roles.iter().any(|r| r == "super_admin" || r == "marketplace_admin")
        || tenant_roles.iter().any(|r| r.role_name == "tenant_admin");

    // JWT roles: admin 用户需包含 "admin" 角色以启用 require_admin 快速路径
    let jwt_roles: Vec<&str> = if is_admin {
        vec!["user", "admin"]
    } else {
        vec!["user"]
    };
    let token = crate::api::jwt::generate_identity_token(user.id, &jwt_roles, &[])
        .map_err(|e| ApiError::Unauthorized(format!("{:?}", e)))?;

    tracing::info!(
        "Login success for username: {}",
        body.username,
    );

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserLoginResponse {
            token,
            token_type: "Bearer".to_string(),
            expires_in: 86400,
            user: crate::api::models::UserInfoResponse {
                id: user.id,
                username: user.username.unwrap_or_else(|| user.name.clone()),
                display_name: user.display_name,
                email: user.email,
                avatar_url: user.avatar_url,
                identity_type: user.identity_type.to_string(),
                is_admin,
                organizations,
                system_roles,
                tenant_roles,
                created_at: user.created_at,
            },
        }),
    ))
}

pub async fn user_register_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::UserRegisterBody>,
) -> Result<impl IntoResponse, ApiError> {
    let existing = state
        .identity
        .get_by_username(&body.username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if existing.is_some() {
        return Err(ApiError::BadRequest(format!(
            "Username '{}' already exists",
            body.username
        )));
    }

    let password_hash = bcrypt::hash(&body.password, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::BadRequest(format!("Failed to hash password: {}", e)))?;

    let new_identity = crate::models::identity::NewIdentity {
        identity_type: crate::models::identity::IdentityType::User,
        name: body.username.clone(),
        external_id: None,
        username: Some(body.username.clone()),
        display_name: body.display_name.clone().or(Some(body.username.clone())),
        email: body.email,
        avatar_url: None,
        password_hash: Some(password_hash),
        is_system_admin: false,
        metadata: None,
    };

    let user = state
        .identity
        .create(new_identity)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create user: {}", e)))?;

    // 为新注册用户赋予默认 skill_user 角色
    let _ = state
        .system_role_assignment
        .assign(user.id, "skill_user", None)
        .await;

    let token = crate::api::jwt::generate_identity_token(user.id, &["user"], &[])
        .map_err(|e| ApiError::InternalError(format!("{:?}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(crate::api::models::UserLoginResponse {
            token,
            token_type: "Bearer".to_string(),
            expires_in: 86400,
            user: crate::api::models::UserInfoResponse {
                id: user.id,
                username: user.username.unwrap_or_else(|| user.name.clone()),
                display_name: user.display_name,
                email: user.email,
                avatar_url: user.avatar_url,
                identity_type: user.identity_type.to_string(),
                is_admin: false,
                organizations: vec![],
                system_roles: vec![],
                tenant_roles: vec![],
                created_at: user.created_at,
            },
        }),
    ))
}

// Password reset handlers

pub async fn forgot_password_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::ForgotPasswordBody>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state
        .identity
        .get_by_email(&body.email)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("No account found with that email".to_string()))?;

    // Generate short-lived reset token (60 minutes)
    let token =
        crate::api::jwt::generate_short_lived_token(&user.id.to_string(), "password_reset", 60)?;

    // In production, send this token via email
    // For now, return it directly (self-service for MVP)
    Ok((
        StatusCode::OK,
        Json(crate::api::models::ForgotPasswordResponse {
            message: "Password reset token generated. Use this token to reset your password."
                .to_string(),
            reset_token: token,
        }),
    ))
}

pub async fn reset_password_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::ResetPasswordBody>,
) -> Result<impl IntoResponse, ApiError> {
    // Verify the reset token
    let user_id_str = crate::api::jwt::verify_purpose_token(&body.token, "password_reset")?;
    let user_id = uuid::Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid token subject".to_string()))?;

    // Validate new password
    if body.new_password.len() < 8 {
        return Err(ApiError::BadRequest(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    // Hash new password
    let password_hash = bcrypt::hash(&body.new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::BadRequest(format!("Failed to hash password: {}", e)))?;

    // Update password
    let update = crate::models::identity::IdentityUpdate {
        password_hash: Some(password_hash),
        ..Default::default()
    };
    state
        .identity
        .update(user_id, update)
        .await
        .map_err(|_| ApiError::InternalError("Failed to update password".to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::ResetPasswordResponse {
            message: "Password has been reset successfully".to_string(),
        }),
    ))
}

// Account deletion

pub async fn delete_user_me_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    // Soft delete: set status to "deleted"
    let update = crate::models::identity::IdentityUpdate {
        status: Some(crate::models::identity::IdentityStatus::Deleted),
        ..Default::default()
    };
    state
        .identity
        .update(id, update)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to delete account: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"deleted": true, "message": "Account deleted successfully"})),
    ))
}

pub async fn get_user_me_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let user = state
        .identity
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(format!("User not found: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    let orgs = state
        .permission
        .get_user_orgs(user.id)
        .await
        .unwrap_or_default();
    let organizations: Vec<crate::api::models::UserOrgInfo> = orgs
        .into_iter()
        .map(|o| crate::api::models::UserOrgInfo {
            id: o.id,
            name: o.name,
            slug: o.slug,
            role: o.role,
        })
        .collect();

    // 构建权限上下文
    let perm_ctx = state
        .permission
        .build_context(user.id)
        .await
        .unwrap_or(crate::services::permission::PermissionContext {
            identity_id: user.id,
            system_roles: std::collections::HashSet::new(),
            tenant_roles: Vec::new(),
            org_roles: Vec::new(),
            group_roles: Vec::new(),
        });
    let system_roles: Vec<String> = perm_ctx.system_roles.into_iter().collect();
    let tenant_roles = build_tenant_role_infos(&state, &perm_ctx.tenant_roles).await;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserInfoResponse {
            id: user.id,
            username: user.username.unwrap_or_else(|| user.name.clone()),
            display_name: user.display_name,
            email: user.email,
            avatar_url: user.avatar_url,
            identity_type: if user.is_system_admin
                || system_roles.iter().any(|r| r == "super_admin" || r == "marketplace_admin")
                || tenant_roles.iter().any(|r| r.role_name == "tenant_admin")
            {
                "admin".to_string()
            } else {
                user.identity_type.to_string()
            },
            is_admin: user.is_system_admin
                || system_roles.iter().any(|r| r == "super_admin" || r == "marketplace_admin")
                || tenant_roles.iter().any(|r| r.role_name == "tenant_admin"),
            organizations,
            system_roles,
            tenant_roles,
            created_at: user.created_at,
        }),
    ))
}

/// GET /users/me/permissions — 权限刷新端点
/// 返回当前用户在各级别的角色及所有可用权限码
pub async fn get_my_permissions_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let perm_ctx = state
        .permission
        .build_context(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let system_roles: Vec<String> = perm_ctx.system_roles.clone().into_iter().collect();

    // tenant roles with names (batch query)
    let tenant_roles = build_tenant_role_infos(&state, &perm_ctx.tenant_roles).await;

    // org roles (from PermissionContext which already has names from org_membership)
    let org_roles: Vec<crate::api::models::OrgRoleInfo> = perm_ctx
        .org_roles
        .iter()
        .map(|(org_id, role_name)| crate::api::models::OrgRoleInfo {
            org_id: *org_id,
            org_name: String::new(), // org name not in PermissionContext; will be empty here
            role_name: role_name.clone(),
        })
        .collect();

    // group roles
    let group_roles: Vec<crate::api::models::GroupRoleInfo> = perm_ctx
        .group_roles
        .iter()
        .map(|(group_id, role_name)| crate::api::models::GroupRoleInfo {
            group_id: *group_id,
            group_name: String::new(), // group name not in PermissionContext
            role_name: role_name.clone(),
        })
        .collect();

    let permissions = state
        .permission
        .collect_all_permissions(&perm_ctx)
        .await
        .unwrap_or_default();

    Ok((
        StatusCode::OK,
        Json(crate::api::models::MyPermissionsResponse {
            system_roles,
            tenant_roles,
            org_roles,
            group_roles,
            permissions,
        }),
    ))
}

pub async fn update_user_me_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateUserBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let password_hash = match body.password {
        Some(pw) => Some(
            bcrypt::hash(&pw, bcrypt::DEFAULT_COST)
                .map_err(|e| ApiError::BadRequest(format!("Failed to hash password: {}", e)))?,
        ),
        None => None,
    };

    let update = crate::models::identity::IdentityUpdate {
        display_name: body.display_name,
        email: body.email,
        avatar_url: body.avatar_url,
        password_hash,
        name: None,
        status: None,
        is_system_admin: None,
        metadata: None,
    };

    let user = state
        .identity
        .update(identity_id, update)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update user: {}", e)))?;

    let orgs = state
        .permission
        .get_user_orgs(user.id)
        .await
        .unwrap_or_default();
    let organizations: Vec<crate::api::models::UserOrgInfo> = orgs
        .into_iter()
        .map(|o| crate::api::models::UserOrgInfo {
            id: o.id,
            name: o.name,
            slug: o.slug,
            role: o.role,
        })
        .collect();

    // 构建权限上下文
    let perm_ctx = state
        .permission
        .build_context(user.id)
        .await
        .unwrap_or(crate::services::permission::PermissionContext {
            identity_id: user.id,
            system_roles: std::collections::HashSet::new(),
            tenant_roles: Vec::new(),
            org_roles: Vec::new(),
            group_roles: Vec::new(),
        });
    let system_roles: Vec<String> = perm_ctx.system_roles.into_iter().collect();
    let tenant_roles = build_tenant_role_infos(&state, &perm_ctx.tenant_roles).await;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserInfoResponse {
            id: user.id,
            username: user.username.unwrap_or_else(|| user.name.clone()),
            display_name: user.display_name,
            email: user.email,
            avatar_url: user.avatar_url,
            identity_type: user.identity_type.to_string(),
            is_admin: user.is_system_admin
                || system_roles.iter().any(|r| r == "super_admin" || r == "marketplace_admin")
                || tenant_roles.iter().any(|r| r.role_name == "tenant_admin"),
            organizations,
            system_roles,
            tenant_roles,
            created_at: user.created_at,
        }),
    ))
}

pub async fn get_user_orgs_handler(
    State(state): State<ApiState>,
    AgentContext {
        identity_id, ..
    }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let uuid_id = identity_id
        .ok_or_else(|| ApiError::Unauthorized("Identity required".to_string()))?;

    // Build permission context for RBAC-based org visibility
    let perm_ctx = state
        .permission
        .build_context(uuid_id)
        .await
        .unwrap_or_else(|_| crate::services::permission::PermissionContext {
            identity_id: uuid_id,
            system_roles: std::collections::HashSet::new(),
            tenant_roles: Vec::new(),
            org_roles: Vec::new(),
            group_roles: Vec::new(),
        });

    let tenant_admin_ids: Vec<uuid::Uuid> = perm_ctx
        .tenant_roles
        .iter()
        .filter(|(_, role)| role == "tenant_admin")
        .map(|(tid, _)| *tid)
        .collect();

    let mut result_set: std::collections::HashMap<uuid::Uuid, crate::api::models::UserOrgResponse> = std::collections::HashMap::new();

    // Add personal org memberships（所有用户，包括 super_admin，只看自己加入的组织）
    {
        let user_orgs = state
            .permission
            .get_user_orgs(uuid_id)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to list user orgs: {}", e)))?;
        for o in user_orgs {
            result_set.entry(o.id).or_insert(crate::api::models::UserOrgResponse {
                id: o.id,
                name: o.name,
                slug: o.slug,
                role: o.role,
            });
        }

        // If tenant_admin, also add all orgs under the user's tenants
        for tenant_id in tenant_admin_ids {
            let tenant_orgs = state
                .organization
                .list_orgs_by_tenant(tenant_id, 1000, 0)
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to list tenant orgs: {}", e)))?;
            for o in tenant_orgs {
                result_set.entry(o.id).or_insert(crate::api::models::UserOrgResponse {
                    id: o.id,
                    name: o.name,
                    slug: o.slug,
                    role: "admin".to_string(),
                });
            }
        }

    }

    let mut result: Vec<crate::api::models::UserOrgResponse> = result_set.into_values().collect();
    // Sort by name for stable UI
    result.sort_by(|a, b| a.name.cmp(&b.name));

    Ok((StatusCode::OK, Json(result)))
}

pub async fn list_my_skills_handler(
    State(state): State<ApiState>,
    AgentContext { identity_id, .. }: AgentContext,
    Query(_query): Query<crate::api::models::ListSkillsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id =
        identity_id.ok_or_else(|| ApiError::Unauthorized("Identity required".to_string()))?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skills = skill_repo
        .list_by_owner(identity_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list my skills: {}", e)))?;

    Ok((StatusCode::OK, Json(skills)))
}

pub async fn get_user_by_username_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::NotFound(format!("User not found: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let orgs = state
        .permission
        .get_user_orgs(user.id)
        .await
        .unwrap_or_default();
    let organizations: Vec<crate::api::models::UserOrgInfo> = orgs
        .into_iter()
        .map(|o| crate::api::models::UserOrgInfo {
            id: o.id,
            name: o.name,
            slug: o.slug,
            role: o.role,
        })
        .collect();

    // Load system_roles for accurate is_admin determination
    let perm_ctx = state
        .permission
        .build_context(user.id)
        .await
        .unwrap_or(crate::services::permission::PermissionContext {
            identity_id: user.id,
            system_roles: std::collections::HashSet::new(),
            tenant_roles: Vec::new(),
            org_roles: Vec::new(),
            group_roles: Vec::new(),
        });
    let system_roles: Vec<String> = perm_ctx.system_roles.into_iter().collect();
    let tenant_roles = build_tenant_role_infos(&state, &perm_ctx.tenant_roles).await;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserInfoResponse {
            id: user.id,
            username: user.username.unwrap_or_else(|| user.name.clone()),
            display_name: user.display_name,
            email: None,
            avatar_url: user.avatar_url,
            identity_type: user.identity_type.to_string(),
            is_admin: user.is_system_admin
                || system_roles.iter().any(|r| r == "super_admin" || r == "marketplace_admin")
                || tenant_roles.iter().any(|r| r.role_name == "tenant_admin"),
            organizations,
            system_roles,
            tenant_roles,
            created_at: user.created_at,
        }),
    ))
}

pub async fn list_audit_logs_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::AuditLogQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let logs = state
        .audit_repo
        .list_with_filters(
            query.agent_id.as_deref(),
            query.action.as_deref(),
            query.resource_type.as_deref(),
            limit,
            offset,
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total = state
        .audit_repo
        .count_with_filters(
            query.agent_id.as_deref(),
            query.action.as_deref(),
            query.resource_type.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<_> = logs
        .into_iter()
        .map(|log| crate::api::models::AuditLogResponse {
            id: log.id.to_string(),
            agent_id: log.agent_id,
            identity_name: log.identity_name,
            identity_type: log.identity_type,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            details: log.details,
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            timestamp: log.timestamp.to_rfc3339(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(crate::api::models::AuditLogListResponse {
            data,
            total,
            limit,
            offset,
        }),
    ))
}

pub async fn list_my_audit_logs_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Query(query): Query<crate::api::models::AuditLogQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let logs = state
        .audit_repo
        .list_with_filters(
            Some(&subject),
            query.action.as_deref(),
            query.resource_type.as_deref(),
            limit,
            offset,
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total = state
        .audit_repo
        .count_with_filters(
            Some(&subject),
            query.action.as_deref(),
            query.resource_type.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<_> = logs
        .into_iter()
        .map(|log| crate::api::models::AuditLogResponse {
            id: log.id.to_string(),
            agent_id: log.agent_id,
            identity_name: log.identity_name,
            identity_type: log.identity_type,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            details: log.details,
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            timestamp: log.timestamp.to_rfc3339(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(crate::api::models::AuditLogListResponse {
            data,
            total,
            limit,
            offset,
        }),
    ))
}

pub async fn submit_review_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext {
        subject,
        identity_id,
        ..
    }: AgentContext,
    Json(body): Json<crate::api::models::SubmitSkillReviewBody>,
) -> Result<impl IntoResponse, ApiError> {
    // RBAC 权限校验：需要 SubmitReview 权限
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;
    check_skill_perm(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::SubmitReview,
    )
    .await?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    skill_repo
        .update_status(&skill_id, "pending_review", None, body.comment.as_deref())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to submit skill for review: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_submitted_for_review".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"comment": body.comment}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill submitted for review".to_string(),
            skill_id,
        }),
    ))
}

pub async fn publish_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext {
        subject,
        identity_id,
        ..
    }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // RBAC 权限校验：需要 Publish 权限（仅做内部发布）
    check_skill_perm_db(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::Publish,
    )
    .await?;

    if skill.status != "approved" {
        return Err(ApiError::BadRequest(
            "Skill must be approved before publishing".to_string(),
        ));
    }

    // 内部发布：仅设置 status 为 published，不操作 visibility
    // visibility 由创建时决定，提交市场由 skill:publish_to_marketplace 单独控制
    skill_repo
        .update_status(&skill_id, "published", None, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to publish skill: {}", e)))?;

    // 确保 release tarball 存在（审核通过时已生成，此处兜底）
    let release_path = state.skill_git.releases_dir()
        .join(&skill.name)
        .join(format!("v{}.tar.gz", skill.version));
    if !release_path.exists() {
        let _ = state.skill_git.generate_release_tarball(&skill.name, &skill.version);
    }

    // 新版本发布后，旧版本不再对外可见
    let _ = sqlx::query(
        "UPDATE skills SET is_current = false WHERE name = $1 AND is_current = true AND id != $2"
    )
    .bind(&skill.name)
    .bind(&skill.id)
    .execute(state.agent_repo.pool())
    .await;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_published".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"visibility": skill.visibility}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill published successfully".to_string(),
            skill_id,
        }),
    ))
}

/// POST /api/v1/skills/:skill_name/rollback — 版本回退（创建新版本，走审核流程）
/// 权限：skill 的作者或组织成员（与上传新版本一致）
/// 流程：Git checkout 目标 tag → commit（不打 tag）→ 创建 pending_review skill → 等待审核
pub async fn rollback_skill_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    axum::extract::Path((skill_name,)): axum::extract::Path<(String,)>,
    Json(body): Json<crate::api::models::RollbackSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = agent_context.require_identity()?;

    let pool = state.agent_repo.pool().clone();
    let skill_repo = crate::db::repositories::skill::SkillRepository::new(pool.clone());
    let version_repo = crate::db::repositories::version::VersionRepository::new(pool);

    // 1. 获取当前最新版本的 owner 信息，并校验权限（作者或组织成员）
    let skill_list = skill_repo
        .list_by_name(&skill_name)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to lookup skill: {}", e)))?;
    let latest = skill_list
        .first()
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_name)))?;

    // 权限校验：必须是作者或组织成员
    let is_author = latest.author_identity_id == Some(identity_id);
    let is_owner_user = latest.owner_type == "user" && latest.owner_id == Some(identity_id);
    if !is_author && !is_owner_user {
        // 检查是否为组织成员
        let org_ids = state
            .permission
            .get_user_org_ids(identity_id)
            .await
            .unwrap_or_default();
        let is_org_member = latest.owner_type == "organization"
            && latest.owner_id.map_or(false, |oid| org_ids.contains(&oid));
        if !is_org_member {
            return Err(ApiError::Forbidden(
                "Only the skill author or organization member can rollback".to_string(),
            ));
        }
    }

    // 2. 执行 Git 回退：从 tag 恢复文件 + commit only（不打 tag） + 写入 skill_versions
    let result = state
        .skill_git
        .rollback_version_commit_only(&skill_name, &body.version, identity_id, &version_repo)
        .map_err(|e| match e {
            crate::models::error::AppError::SkillNotFound(_) => ApiError::NotFound(format!(
                "Version {} not found for skill {}",
                body.version, skill_name
            )),
            other => ApiError::BadRequest(format!("Rollback failed: {}", other)),
        })?;

    // 3. 读取恢复后的 SKILL.md 获取元数据
    let repo_dir = state.skill_git.repo_path(&skill_name);
    let skill_md_content = std::fs::read_to_string(repo_dir.join("SKILL.md"))
        .map_err(|e| ApiError::InternalError(format!("Failed to read restored SKILL.md: {}", e)))?;
    let meta = crate::services::skill_git::parse_skill_md_frontmatter(&skill_md_content)
        .map_err(|e| ApiError::BadRequest(format!("Failed to parse restored SKILL.md: {}", e)))?;

    // 4. 通过 RegistryService 创建新版本的 skill 记录（状态为 pending_review）
    let new_skill = crate::models::skill::NewSkill {
        name: skill_name.clone(),
        description: meta.description,
        tags: meta.tags,
        content: skill_md_content,
        version: result.new_version.clone(),
        git_url: None,
        visibility: Some(
            skill_list
                .iter()
                .find_map(|s| match s.visibility.as_str() {
                    "private" => Some(crate::models::skill_policy::Visibility::Private),
                    "org_visible" => Some(crate::models::skill_policy::Visibility::OrgVisible),
                    "marketplace" => Some(crate::models::skill_policy::Visibility::Marketplace),
                    "shared" => Some(crate::models::skill_policy::Visibility::Shared),
                    _ => None,
                })
                .unwrap_or(crate::models::skill_policy::Visibility::OrgVisible),
        ),
        tools: None,
        owner_type: latest.owner_type.clone(),
        owner_id: latest.owner_id,
        author_identity_id: Some(identity_id),
    };
    let skill = state
        .registry
        .create_skill(new_skill, &agent_context.subject, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create rolled-back skill: {}", e)))?;

    // 5. 审计日志
    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "skill_rollback".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill.id.clone()),
            details: serde_json::json!({
                "skill_name": skill_name,
                "from_version": result.from_version,
                "target_version": result.target_version,
                "new_version": result.new_version,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!(
                "Skill {} rollback from {} to {} submitted for review (new version: {})",
                skill_name, result.from_version, result.target_version, result.new_version
            ),
            "skill_name": skill_name,
            "from_version": result.from_version,
            "target_version": result.target_version,
            "new_version": result.new_version,
            "skill_id": skill.id,
        })),
    ))
}

/// POST /api/v1/admin/skills/:id/unpublish — 管理员下架已发布的 Skill
/// 新逻辑: 将 marketplace_status 设为 'delisted'，保留 internal status 不变
pub async fn admin_unpublish_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin_only(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("listed") {
        return Err(ApiError::BadRequest(
            "Only listed marketplace skills can be delisted".to_string(),
        ));
    }

    // 下架: 改 marketplace_status 为 delisted，恢复原始 visibility
    let pre_visibility = skill.pre_marketplace_visibility.as_deref().unwrap_or("private");
    skill_repo
        .update_marketplace_status(&skill_id, Some("delisted"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to delist skill: {}", e)))?;

    skill_repo
        .update(&skill_id, None, None, None, Some(pre_visibility))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to restore visibility: {}", e)))?;

    // 向后兼容：同时设置 admin_unpublished 标记
    skill_repo
        .set_admin_unpublished(&skill_id, true)
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set admin unpublished flag: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "skill_unpublished".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "previous_marketplace_status": "listed",
                "new_marketplace_status": "delisted",
                "restored_visibility": pre_visibility,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("Skill {} delisted from marketplace", skill.name),
            "skill_id": skill_id,
            "marketplace_status": "delisted",
        })),
    ))
}

/// POST /api/v1/admin/skills/:id/publish — 管理员直接上架 Skill 到市场（绕过审核）
/// 新逻辑: 使用 marketplace_status 而不是 visibility
pub async fn admin_publish_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin_only(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() == Some("listed") {
        return Err(ApiError::BadRequest(
            "Skill is already listed on marketplace".to_string(),
        ));
    }

    // 只在未 published 时更新状态
    if skill.status != "published" {
        skill_repo
            .update_status(&skill_id, "published", None, None)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to publish skill: {}", e)))?;

        // 新版本发布后，旧版本不再对外可见
        let _ = sqlx::query(
            "UPDATE skills SET is_current = false WHERE name = $1 AND is_current = true AND id != $2"
        )
        .bind(&skill.name)
        .bind(&skill.id)
        .execute(state.agent_repo.pool())
        .await;
    }

    // 如果该 Skill 之前未经过审核流程（没有 Git tag），现在补充版本管理
    {
        let repo_dir = state.skill_git.repo_path(&skill.name);
        let tag_name = format!("v{}", skill.version);
        let tag_exists = std::process::Command::new("git")
            .current_dir(&repo_dir)
            .args(["rev-parse", &tag_name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !tag_exists && repo_dir.join(".git").exists() {
            let tag_msg = format!("v{}: Admin published", skill.version);
            let _ = state.skill_git.git_tag_approved(&repo_dir, &tag_name, &tag_msg);
            let _ = state.skill_git.generate_release_tarball(&skill.name, &skill.version);

            // 统计文件数和总大小（从 git repo 统计）
            let (file_count, total_size_bytes) = state.registry.count_skill_files(&repo_dir)
                .unwrap_or((0, 0));

            use crate::db::repositories::version::{VersionRepository, NewSkillVersion};
            let version_repo = VersionRepository::new(state.agent_repo.pool().clone());
            let _ = version_repo.create(NewSkillVersion {
                skill_name: skill.name.clone(),
                version: skill.version.clone(),
                git_commit_hash: None,
                git_tag: Some(tag_name),
                changelog: Some(tag_msg),
                file_count: file_count as i32,
                total_size_bytes: total_size_bytes as i64,
                uploaded_by: agent_context.require_identity().ok(),
                git_remote_url: None,
            }).await;
        }
    }

    // 保存当前 visibility 作为 pre_marketplace_visibility
    let current_visibility = skill.visibility.clone();
    skill_repo
        .set_pre_marketplace_visibility(&skill_id, Some(&current_visibility))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to save pre-marketplace visibility: {}", e)))?;

    // 设置为 marketplace 可见性
    skill_repo
        .update(&skill_id, None, None, None, Some("marketplace"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set marketplace visibility: {}", e))
        })?;

    // 设置 marketplace_status 为 listed
    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set marketplace status: {}", e))
        })?;

    // 清除 admin_unpublished 标记
    skill_repo
        .set_admin_unpublished(&skill_id, false)
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to clear admin unpublished flag: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "skill_admin_published".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "previous_marketplace_status": skill.marketplace_status,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("Skill {} listed on marketplace", skill.name),
            "skill_id": skill_id,
            "status": "published",
            "marketplace_status": "listed",
        })),
    ))
}

/// POST /api/v1/skills/:id/submit-to-marketplace — 提交已发布 Skill 到市场审核
pub async fn submit_to_marketplace_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext {
        subject,
        identity_id,
        ..
    }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // RBAC 权限校验：需要 PublishToMarketplace 权限
    check_skill_perm_db(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::PublishToMarketplace,
    )
    .await?;

    // 必须是已发布状态
    if skill.status != "published" {
        return Err(ApiError::BadRequest(
            "Only published skills can be submitted to marketplace".to_string(),
        ));
    }

    // 不能重复提交
    if skill.marketplace_status.as_deref() == Some("pending_review") {
        return Err(ApiError::BadRequest(
            "Skill is already pending marketplace review".to_string(),
        ));
    }

    if skill.marketplace_status.as_deref() == Some("listed") {
        return Err(ApiError::BadRequest(
            "Skill is already listed on marketplace".to_string(),
        ));
    }

    // 保存提交前的 visibility
    skill_repo
        .set_pre_marketplace_visibility(&skill_id, Some(&skill.visibility))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to save pre-marketplace visibility: {}", e)))?;

    // 设置 marketplace_status 为 pending_review
    skill_repo
        .update_marketplace_status(&skill_id, Some("pending_review"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to submit to marketplace: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_submitted_to_marketplace".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "previous_visibility": skill.visibility,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill submitted to marketplace review".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "pending_review",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/approve — 市场审核通过
pub async fn marketplace_review_approve_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_review") {
        return Err(ApiError::BadRequest(
            "Skill is not pending marketplace review".to_string(),
        ));
    }

    // 审核通过: 设置 marketplace_status=listed, visibility=marketplace
    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve: {}", e)))?;

    skill_repo
        .update(&skill_id, None, None, None, Some("marketplace"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set marketplace visibility: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_review_approved".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Marketplace review approved".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/reject — 市场审核驳回
pub async fn marketplace_review_reject_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_review") {
        return Err(ApiError::BadRequest(
            "Skill is not pending marketplace review".to_string(),
        ));
    }

    // 驳回: 设置 marketplace_status=rejected, 恢复原始 visibility
    let pre_visibility = skill.pre_marketplace_visibility.as_deref().unwrap_or("private");
    skill_repo
        .update_marketplace_status(&skill_id, Some("rejected"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject: {}", e)))?;

    skill_repo
        .update(&skill_id, None, None, None, Some(pre_visibility))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to restore visibility: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_review_rejected".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "rejected",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Marketplace review rejected".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "rejected",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/relist — 重新上架已下架的 Skill
pub async fn marketplace_relist_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin_only(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("delisted") {
        return Err(ApiError::BadRequest(
            "Only delisted skills can be relisted".to_string(),
        ));
    }

    // 重新上架: 设置 marketplace_status=listed, visibility=marketplace
    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to relist: {}", e)))?;

    skill_repo
        .update(&skill_id, None, None, None, Some("marketplace"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set marketplace visibility: {}", e))
        })?;

    skill_repo
        .set_admin_unpublished(&skill_id, false)
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to clear admin unpublished flag: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_skill_relisted".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill relisted on marketplace".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

pub async fn marketplace_delist_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    // marketplace_admin and marketplace_reviewer can both delist
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("listed") {
        return Err(ApiError::BadRequest(
            "Only currently listed skills can be delisted".to_string(),
        ));
    }

    // 下架: 设置 marketplace_status=delisted, visibility 回退到 pre_marketplace_visibility
    let pre_visibility = skill.pre_marketplace_visibility.as_deref().unwrap_or("private");
    skill_repo
        .update_marketplace_status(&skill_id, Some("delisted"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to delist: {}", e)))?;

    // Revert visibility from marketplace using pre_marketplace_visibility
    skill_repo
        .update(&skill_id, None, None, None, Some(pre_visibility))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to revert visibility: {}", e))
        })?;

    skill_repo
        .set_admin_unpublished(&skill_id, true)
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set admin unpublished flag: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_skill_delisted".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "delisted",
                "restored_visibility": pre_visibility,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill delisted from marketplace".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "delisted",
        })),
    ))
}

pub async fn approve_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext {
        subject,
        identity_id,
        ..
    }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // RBAC 权限校验：需要 Approve 权限（自动校验不能审核自己的 Skill）
    check_skill_perm_db(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::Approve,
    )
    .await?;

    if skill.status != "pending_review" {
        return Err(ApiError::BadRequest(
            "Skill must be in pending_review status to approve".to_string(),
        ));
    }

    let reviewer_id = identity_id;

    skill_repo
        .update_status(&skill_id, "approved", reviewer_id, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve skill: {}", e)))?;

    // Git tag + version_repo 写入 + tarball 生成
    {
        let repo_dir = state.skill_git.repo_path(&skill.name);
        let tag_name = format!("v{}", skill.version);
        let tag_msg = format!("v{}: Approved version", skill.version);

        state.skill_git.git_tag_approved(&repo_dir, &tag_name, &tag_msg)
            .map_err(|e| ApiError::InternalError(format!("Failed to tag version: {}", e)))?;

        // 统计文件数和总大小（从 git repo 统计）
        let (file_count, total_size_bytes) = state.registry.count_skill_files(&repo_dir)
            .unwrap_or((0, 0));

        use crate::db::repositories::version::{VersionRepository, NewSkillVersion};
        let version_repo = VersionRepository::new(state.agent_repo.pool().clone());
        version_repo.create(NewSkillVersion {
            skill_name: skill.name.clone(),
            version: skill.version.clone(),
            git_commit_hash: None,
            git_tag: Some(tag_name),
            changelog: Some(tag_msg),
            file_count: file_count as i32,
            total_size_bytes: total_size_bytes as i64,
            uploaded_by: reviewer_id,
            git_remote_url: None,
        }).await
        .map_err(|e| ApiError::InternalError(format!("Failed to record version: {}", e)))?;

        // 生成 tarball
        let _ = state.skill_git.generate_release_tarball(&skill.name, &skill.version);
    }

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "approved"}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill approved successfully".to_string(),
            skill_id,
        }),
    ))
}

pub async fn reject_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext {
        subject,
        identity_id,
        ..
    }: AgentContext,
    Json(body): Json<crate::api::models::RejectSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // RBAC 权限校验：需要 Reject 权限（自动校验不能审核自己的 Skill）
    check_skill_perm_db(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::Reject,
    )
    .await?;

    if skill.status != "pending_review" {
        return Err(ApiError::BadRequest(
            "Skill must be in pending_review status to reject".to_string(),
        ));
    }

    let reviewer_id = identity_id;

    skill_repo
        .update_status(&skill_id, "rejected", reviewer_id, body.reason.as_deref())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject skill: {}", e)))?;

    // Git reset --soft HEAD~1 撤销审核中的 commit
    {
        let repo_dir = state.skill_git.repo_path(&skill.name);
        if repo_dir.join(".git").exists() {
            let _ = state.skill_git.git_reset_soft_head(&repo_dir);
        }
    }

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "rejected", "reason": body.reason}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill rejected".to_string(),
            skill_id,
        }),
    ))
}

pub async fn marketplace_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Query(query): Query<crate::api::models::MarketplaceQuery>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    // 使用新的双轨模型查询：status=published AND marketplace_status='listed'
    let skills = skill_repo
        .list_marketplace_listed(limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(skills)))
}

/// GET /api/v1/skills/:name/download/:version?token=...
/// 返回 skill 目录的 tar.gz 包
/// token 为 DB 中的不透明 UUID，由 skills.install 生成，5 分钟有效
pub async fn download_skill_handler(
    State(state): State<ApiState>,
    Path((name, version)): Path<(String, String)>,
    Query(query): Query<DownloadSkillQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // 1. 防止路径遍历
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(ApiError::BadRequest("Invalid skill name".to_string()));
    }

    // 2. 从数据库验证并消费下载凭证
    let token_record = state
        .download_token_repo
        .validate_and_consume(&query.token, &name, &version)
        .await
        .map_err(|e| {
            tracing::error!("Download token DB lookup failed: {}", e);
            ApiError::InternalError("Download verification failed".to_string())
        })?
        .ok_or_else(|| {
            ApiError::Unauthorized("Invalid, expired, or already used download token".to_string())
        })?;

    tracing::info!(
        "Skill download: skill={}/v{}, identity={}, api_key={}",
        name,
        version,
        token_record.identity_id,
        token_record.api_key_id
    );

    let filename = format!("{}-{}.tar.gz", name, version);

    // 3. 优先使用预生成的 release tarball（审核通过后 git archive 生成）
    let release_tarball_path = state
        .skill_git
        .releases_dir()
        .join(&name)
        .join(format!("v{}.tar.gz", version));

    if release_tarball_path.exists() {
        let tarball = tokio::fs::read(&release_tarball_path).await.map_err(|e| {
            tracing::error!("Failed to read release tarball: {}", e);
            ApiError::InternalError("Failed to read release tarball".to_string())
        })?;

        tracing::info!(
            "Serving pre-built release tarball: {}",
            release_tarball_path.display()
        );

        let response = axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/gzip")
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", filename),
            )
            .header("Content-Length", tarball.len().to_string())
            .body(axum::body::Body::from(tarball))
            .map_err(|e| ApiError::InternalError(format!("Failed to build response: {}", e)))?;

        return Ok(response);
    }

    // 4. 无预生成 tarball — 实时从 git archive 生成并缓存到 releases
    let _ = state.skill_git.generate_release_tarball(&name, &version);

    // 重新读取刚生成的 tarball
    if release_tarball_path.exists() {
        let tarball = tokio::fs::read(&release_tarball_path).await.map_err(|e| {
            tracing::error!("Failed to read release tarball: {}", e);
            ApiError::InternalError("Failed to read release tarball".to_string())
        })?;

        tracing::info!("Served freshly generated release tarball: {}", release_tarball_path.display());

        let response = axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/gzip")
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", filename),
            )
            .header("Content-Length", tarball.len().to_string())
            .body(axum::body::Body::from(tarball))
            .map_err(|e| ApiError::InternalError(format!("Failed to build response: {}", e)))?;

        return Ok(response);
    }

    return Err(ApiError::NotFound(format!(
        "Release tarball not available for skill '{}' version {}",
        name, version
    )));
}

/// GET /api/v1/cli/download/:version/:target?token=...
/// 返回 CLI 的 tar.gz 包（含 binary + config.toml + install 脚本 + SKILL.md）
/// target 格式：{os}-{arch}，如 linux-x86_64、windows-x86_64
/// token 为 DB 中的不透明 UUID，由 cli.setup MCP 工具生成，5 分钟有效
pub async fn download_cli_handler(
    State(state): State<ApiState>,
    Path((version, target)): Path<(String, String)>,
    Query(query): Query<DownloadSkillQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // 1. 防止路径遍历
    if version.contains("..")
        || version.contains('/')
        || version.contains('\\')
        || target.contains("..")
        || target.contains('/')
        || target.contains('\\')
    {
        return Err(ApiError::BadRequest(
            "Invalid version or target".to_string(),
        ));
    }

    // 2. 验证 CLI 下载凭证
    let token_record = state
        .download_token_repo
        .validate_cli_token(&query.token)
        .await
        .map_err(|e| {
            tracing::error!("CLI download token DB lookup failed: {}", e);
            ApiError::InternalError("Download verification failed".to_string())
        })?
        .ok_or_else(|| {
            ApiError::Unauthorized(
                "Invalid, expired, or already used CLI download token".to_string(),
            )
        })?;

    tracing::info!(
        "CLI download: v{}/{}, identity={}, api_key={}",
        version,
        target,
        token_record.identity_id,
        token_record.api_key_id
    );

    // 3. 找到 CLI 二进制文件
    let is_windows = target.starts_with("windows");
    let binary_name = if is_windows {
        "skill-garden.exe"
    } else {
        "skill-garden"
    };

    let bin_path = std::path::PathBuf::from("cli-dist")
        .join(&version)
        .join(&target)
        .join(binary_name);

    if !bin_path.exists() {
        return Err(ApiError::NotFound(format!(
            "CLI binary v{}/{} not found on server. \
             Build it with: cargo build --release --no-default-features --features cli --bin skill-garden, \
             then place it at cli-dist/{}/{}/{}",
            version, target, version, target, binary_name
        )));
    }

    // 4. 读取预填的 config.toml（cli.setup 时写入 token）
    let config_data = token_record.config_data.unwrap_or_else(|| {
        let server_url = std::env::var("AION_HIVE_PUBLIC_URL").unwrap_or_else(|_| {
            format!(
                "http://localhost:{}",
                std::env::var("AION_HIVE_HTTP_PORT").unwrap_or_else(|_| "8080".to_string())
            )
        });
        format!(
            "server = \"{}\"\ntoken = \"sk_<YOUR_API_KEY>\"\n",
            server_url.trim_end_matches('/')
        )
    });

    let version_clone = version.clone();
    let target_clone = target.clone();

    // Compute display labels for SKILL.md template
    let server_url = std::env::var("AION_HIVE_PUBLIC_URL").unwrap_or_else(|_| {
        format!(
            "http://localhost:{}",
            std::env::var("AION_HIVE_HTTP_PORT").unwrap_or_else(|_| "8080".to_string())
        )
    });
    let os_label = if target.starts_with("linux") {
        "Linux"
    } else if target.starts_with("macos") {
        "macOS"
    } else {
        "Windows"
    };
    let verify_cmd = if is_windows {
        "skill-garden.exe whoami"
    } else {
        "skill-garden whoami"
    };

    // 5. 在 blocking 线程池中生成 tar.gz
    let tarball = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let mut buf = Vec::new();
        let encoder = flate2::write::GzEncoder::new(&mut buf, flate2::Compression::default());
        let mut tar_builder = tar::Builder::new(encoder);

        let prefix = "skill-garden-cli";

        // Helper: add a file from bytes
        fn add_bytes<W: std::io::Write>(
            tar: &mut tar::Builder<W>,
            path: &str,
            data: &[u8],
            mode: u32,
        ) -> Result<(), String> {
            let mut header = tar::Header::new_gnu();
            header
                .set_path(path)
                .map_err(|e| format!("tar path error: {}", e))?;
            header.set_size(data.len() as u64);
            header.set_mode(mode);
            header.set_cksum();
            tar.append_data(&mut header, path, std::io::Cursor::new(data))
                .map_err(|e| format!("tar append error for {}: {}", path, e))?;
            Ok(())
        }

        // 5a. 添加二进制文件
        let bin_bytes = std::fs::read(&bin_path)
            .map_err(|e| format!("Failed to read binary {}: {}", bin_path.display(), e))?;
        let bin_tar_path = format!("{}/{}", prefix, binary_name);
        add_bytes(&mut tar_builder, &bin_tar_path, &bin_bytes, 0o755)?;

        // 5b. 添加 config.toml
        let config_tar_path = format!("{}/config.toml", prefix);
        add_bytes(
            &mut tar_builder,
            &config_tar_path,
            config_data.as_bytes(),
            0o644,
        )?;

        // 5c. 添加 install.sh
        let install_sh =
            include_str!("../../cli-dist/install.sh").replace("{version}", &version_clone);
        let install_sh_path = format!("{}/install.sh", prefix);
        add_bytes(
            &mut tar_builder,
            &install_sh_path,
            install_sh.as_bytes(),
            0o755,
        )?;

        // 5d. 添加 install.ps1
        let install_ps1 =
            include_str!("../../cli-dist/install.ps1").replace("{version}", &version_clone);
        let install_ps1_path = format!("{}/install.ps1", prefix);
        add_bytes(
            &mut tar_builder,
            &install_ps1_path,
            install_ps1.as_bytes(),
            0o644,
        )?;

        // 5e. 添加 skill-garden/SKILL.md（作为独立 Skill 目录，Agent 可直接安装）
        let skill_md = include_str!("../../cli-dist/SKILL.md")
            .replace("{server_url}", &server_url)
            .replace("{os}", os_label)
            .replace("{version}", &version_clone)
            .replace("{verify}", verify_cmd);
        add_bytes(
            &mut tar_builder,
            "skill-garden/SKILL.md",
            skill_md.as_bytes(),
            0o644,
        )?;

        // Finalize tar.gz
        let encoder = tar_builder
            .into_inner()
            .map_err(|e| format!("Failed to finalize tar: {}", e))?;
        encoder
            .finish()
            .map_err(|e| format!("Failed to compress: {}", e))?;

        Ok(buf)
    })
    .await
    .map_err(|e| ApiError::InternalError(format!("Tarball generation failed: {}", e)))?
    .map_err(|e| ApiError::InternalError(e))?;

    // 6. 返回 tar.gz 流
    let archive_name = format!("skill-garden-cli-{}-{}.tar.gz", target_clone, version);
    let response = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/gzip")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", archive_name),
        )
        .header("Content-Length", tarball.len().to_string())
        .body(axum::body::Body::from(tarball))
        .map_err(|e| ApiError::InternalError(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// 下载参数
#[derive(serde::Deserialize)]
pub struct DownloadSkillQuery {
    pub token: String,
}

pub async fn list_skill_groups_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupSkillRepository::new(pool);

    let associations = repo
        .list_by_skill(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let responses: Vec<crate::api::models::SkillGroupResponse> = associations
        .into_iter()
        .map(|a| crate::api::models::SkillGroupResponse {
            skill_id: a.skill_id,
            group_id: a.group_id,
            group_name: String::new(),
            added_at: a.added_at,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::to_value(responses).unwrap()),
    ))
}

pub async fn add_skill_to_group_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AddSkillToGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    use crate::db::repositories::group_skill::GroupSkillRepository;
    use crate::models::group_skill::NewGroupSkill;
    let pool = state.agent_repo.pool().clone();
    let group_repo = GroupRepository::new(pool.clone());

    // Verify group exists and check org membership
    let group = group_repo
        .find_by_id(body.group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Group {} not found", body.group_id)))?;
    require_org_member(&state, &agent_context, group.organization_id, None).await?;

    let repo = GroupSkillRepository::new(pool);
    repo.associate_skill(NewGroupSkill {
        group_id: body.group_id,
        skill_id: skill_id.clone(),
        added_by: None,
    })
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to add skill to group: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "skill_added_to_group".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"group_id": body.group_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Skill added to group",
            "skill_id": skill_id,
            "group_id": body.group_id,
        })),
    ))
}

pub async fn remove_skill_from_group_handler(
    State(state): State<ApiState>,
    Path((skill_id, group_id)): Path<(String, Uuid)>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let group_repo = GroupRepository::new(pool.clone());

    // Verify group exists and check org membership
    let group = group_repo
        .find_by_id(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Group {} not found", group_id)))?;
    require_org_member(&state, &agent_context, group.organization_id, None).await?;

    let repo = GroupSkillRepository::new(pool);
    repo.dissociate_skill(group_id, &skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove skill from group: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "skill_removed_from_group".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"group_id": group_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill removed from group",
            "skill_id": skill_id,
            "group_id": group_id,
        })),
    ))
}

// v0.4 multi-tenant handlers

use uuid::Uuid;

/// Organization handlers

/// 验证当前用户是指定组织的成员，可选最低角色要求
/// super_admin 全局通过；tenant_admin 对其租户下所有组织通过。
/// 非管理员用户回退到 build_context 的 org_roles 检查。
async fn require_org_member(
    state: &ApiState,
    agent_context: &AgentContext,
    org_id: uuid::Uuid,
    min_role: Option<crate::models::org_membership::OrgRole>,
) -> Result<uuid::Uuid, ApiError> {
    let identity_id = agent_context.require_identity()?;

    // 1) Fast path: super_admin 拥有所有组织权限
    let is_super = state
        .permission
        .is_super_admin(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if is_super {
        return Ok(identity_id);
    }

    // 2) Fast path: tenant_admin 对其租户下所有组织有完全管理权
    //    先查出组织所属租户，再检查用户是否是该租户的 tenant_admin
    let tenant_ids = state
        .permission
        .get_tenant_admin_tenant_ids(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if !tenant_ids.is_empty() {
        use crate::db::repositories::organization::OrganizationRepository;
        let pool = state.agent_repo.pool().clone();
        let org_repo = OrganizationRepository::new(pool);
        if let Ok(Some(org)) = org_repo.find_by_id(org_id).await {
            if let Some(tid) = org.tenant_id {
                if tenant_ids.contains(&tid) {
                    return Ok(identity_id);
                }
            }
        }
    }

    // 3) 普通用户：通过 build_context 检查是否是该组织成员
    let ctx = state
        .permission
        .build_context(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let role_str = ctx
        .org_roles
        .iter()
        .find(|(id, _)| *id == org_id)
        .map(|(_, role)| role.clone())
        .ok_or_else(|| ApiError::Forbidden("Not a member of this organization".to_string()))?;

    let role = crate::models::org_membership::OrgRole::from(role_str.as_str());

    if let Some(min_role) = min_role {
        if role < min_role {
            return Err(ApiError::Forbidden(format!(
                "Requires at least {} role in this organization",
                min_role
            )));
        }
    }

    Ok(identity_id)
}

/// 通过 slug 或 UUID 解析组织 ID，返回 (org_id, org_slug)
async fn resolve_org_id(pool: &sqlx::PgPool, slug_or_id: &str) -> Result<(uuid::Uuid, String), ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let org_repo = OrganizationRepository::new(pool.clone());

    // 先尝试 slug 查找
    if let Ok(Some(org)) = org_repo.find_by_slug(slug_or_id).await {
        let slug = org.slug.unwrap_or_else(|| org.id.to_string());
        return Ok((org.id, slug));
    }

    // 尝试解析为 UUID 并按 ID 查找
    if let Ok(id) = uuid::Uuid::parse_str(slug_or_id) {
        if let Ok(Some(org)) = org_repo.find_by_id(id).await {
            let slug = org.slug.unwrap_or_else(|| org.id.to_string());
            return Ok((org.id, slug));
        }
    }

    Err(ApiError::NotFound(format!("Organization '{}' not found", slug_or_id)))
}

pub async fn create_org_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateOrgBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let org = state
        .organization
        .create_org(
            body.name,
            body.slug,
            body.display_name,
            body.description,
            body.tenant_id,
        )
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(org).unwrap()),
    ))
}

pub async fn get_org_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let org = state
        .organization
        .get_org(org_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn list_orgs_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Query(query): Query<crate::api::models::ListOrgsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let orgs = if let Some(tenant_id) = query.tenant_id {
        state
            .organization
            .list_orgs_by_tenant(tenant_id, limit, offset)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state
            .organization
            .list_orgs(limit, offset)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": orgs }))))
}

pub async fn update_org_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(org_id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateOrgBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_org_member(
        &state,
        &agent_context,
        org_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let org = state
        .organization
        .update_org(org_id, body.name, body.display_name, body.description)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn delete_org_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    state
        .organization
        .delete_org(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": org_id}))))
}

/// Organization member handlers

pub async fn list_org_members_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let agents = state
        .agent_repo
        .find_by_org(org_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let members: Vec<_> = agents
        .into_iter()
        .map(|a| crate::api::models::OrgMemberResponse {
            agent_id: a.agent_id,
            name: a.agent_name,
            capabilities: a.capabilities,
            joined_at: a.created_at.to_rfc3339(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(crate::api::models::OrgMemberListResponse { members }),
    ))
}

pub async fn add_org_member_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AddOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    use crate::db::repositories::agent::NewAgent;

    let secret = uuid::Uuid::new_v4().to_string();

    let new_agent = NewAgent {
        agent_id: body.agent_id.clone(),
        agent_secret: secret,
        agent_name: body.name.clone(),
        org_id: Some(org_id),
        capabilities: Some(Vec::<String>::new()),
    };

    state
        .agent_repo
        .create(new_agent)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Member added successfully",
            "agent_id": body.agent_id
        })),
    ))
}

pub async fn remove_org_member_handler(
    State(state): State<ApiState>,
    Path((_org_id, subject)): Path<(Uuid, String)>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    state
        .agent_repo
        .update_org(&subject, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"removed": subject})),
    ))
}

pub async fn get_org_stats_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let pool = state.agent_repo.pool();

    let members_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agents WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let skills_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE author_subject IN (SELECT subject FROM agents WHERE org_id = $1)"
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let sessions_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let tools_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM org_tools WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let response = crate::api::models::OrgStatsResponse {
        org_id,
        members_count,
        skills_count,
        sessions_count,
        tools_count,
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_org_by_slug_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = OrganizationRepository::new(pool);

    let org = repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn create_org_skill_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateOrgSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let membership = org_membership_repo
        .get_member(identity_id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("Not a member of this organization".to_string()))?;

    let owner_type = body
        .owner_type
        .unwrap_or_else(|| "organization".to_string());

    if owner_type == "organization" {
        let role_str = membership.role.to_string();
        if role_str != "owner" && role_str != "admin" && role_str != "developer" {
            return Err(ApiError::Unauthorized(
                "Need developer role to create org skills".to_string(),
            ));
        }
    }

    let visibility = body.visibility.as_ref().map(|v| match v.as_str() {
        "private" => crate::models::skill_policy::Visibility::Private,
        "org_visible" => crate::models::skill_policy::Visibility::OrgVisible,
        "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
        _ => crate::models::skill_policy::Visibility::OrgVisible,
    });

    let new_skill = crate::models::NewSkill {
        name: body.name,
        description: body.description,
        tags: body.tags,
        content: body.content,
        version: body.version.unwrap_or_else(|| "1.0.0".to_string()),
        git_url: body.git_url.clone(),
        visibility,
        tools: body.tools.clone(),
        owner_type,
        owner_id: Some(org.id),
        author_identity_id: Some(identity_id),
    };

    let skill = state
        .registry
        .create_skill(new_skill, &subject, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create skill: {}", e)))?;

    let response = crate::api::models::SkillCreatedResponse {
        message: "Skill created successfully".to_string(),
        skill_id: skill.id,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn invite_org_member_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::InviteOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let target_identity = state
        .identity
        .get_by_email(&body.email)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User with email '{}' not found", body.email)))?;

    let inviter_id = agent_context.require_identity()?;

    let org_membership_repo = OrgMembershipRepository::new(pool.clone());
    org_membership_repo
        .add_member(
            target_identity.id,
            org.id,
            body.role.as_str().into(),
            Some(inviter_id),
        )
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": format!("{} added to {}", body.email, slug),
            "organization_id": org.id,
            "identity_id": target_identity.id,
            "role": body.role,
        })),
    ))
}

pub async fn invite_org_member_by_id_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<uuid::Uuid>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::InviteOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let target_identity = state
        .identity
        .get_by_email(&body.email)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User with email '{}' not found", body.email)))?;

    let inviter_id = agent_context.require_identity()?;

    let org_membership_repo = OrgMembershipRepository::new(pool.clone());
    org_membership_repo
        .add_member(
            target_identity.id,
            org.id,
            body.role.as_str().into(),
            Some(inviter_id),
        )
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": format!("{} added to organization {}", body.email, org_id),
            "organization_id": org.id,
            "identity_id": target_identity.id,
            "role": body.role,
        })),
    ))
}

pub async fn update_org_member_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((slug, username)): Path<(String, String)>,
    Json(body): Json<crate::api::models::UpdateOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .update_role(identity.id, org.id, body.role.as_str().into())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update role: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} role updated in {}", username, slug),
            "role": body.role,
        })),
    ))
}

pub async fn remove_org_member_by_slug_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((slug, username)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .remove_member(identity.id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} removed from {}", username, slug),
        })),
    ))
}

pub async fn update_org_member_by_id_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((org_id, username)): Path<(uuid::Uuid, String)>,
    Json(body): Json<crate::api::models::UpdateOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .update_role(identity.id, org.id, body.role.as_str().into())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update role: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} role updated in {}", username, org_id),
            "role": body.role,
        })),
    ))
}

pub async fn remove_org_member_by_id_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((org_id, username)): Path<(uuid::Uuid, String)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .remove_member(identity.id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} removed from {}", username, org_id),
        })),
    ))
}

pub async fn list_org_members_by_slug_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(&state, &agent_context, org.id, None).await?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let members = org_membership_repo
        .list_members(org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list members: {}", e)))?;

    Ok((StatusCode::OK, Json(members)))
}

pub async fn list_org_members_by_id_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(org_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    require_org_member(&state, &agent_context, org.id, None).await?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let members = org_membership_repo
        .list_members(org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list members: {}", e)))?;

    Ok((StatusCode::OK, Json(members)))
}

pub async fn list_org_skills_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(&state, &agent_context, org.id, None).await?;

    let skill_repo = SkillRepository::new(pool);
    let skills = skill_repo
        .list_by_org(&org.id.to_string())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list skills: {}", e)))?;

    Ok((StatusCode::OK, Json(skills)))
}

pub async fn list_org_reviews_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(&state, &agent_context, org.id, None).await?;

    let skill_repo = SkillRepository::new(pool);
    let skills = skill_repo
        .list_by_org(&org.id.to_string())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list skills: {}", e)))?;

    let in_review: Vec<_> = skills
        .into_iter()
        .filter(|s| s.status == "pending_review")
        .collect();

    Ok((StatusCode::OK, Json(in_review)))
}

/// Session handlers

pub async fn get_session_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let session = state
        .session
        .get_session(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    match session {
        Some(s) => {
            // Check ownership: only the session owner or admin can view
            let is_admin = require_admin(&state, &agent_context).await.is_ok();
            if !is_admin {
                let identity_id = agent_context.require_identity()?;
                if s.identity_id != identity_id {
                    return Err(ApiError::Unauthorized(
                        "Not authorized to view this session".to_string(),
                    ));
                }
            }

            let enriched = enrich_session_with_meta(&state, s).await?;
            Ok((
                StatusCode::OK,
                Json(serde_json::to_value(enriched).unwrap()),
            ))
        }
        None => Err(ApiError::NotFound(format!(
            "Session {} not found",
            session_id
        ))),
    }
}

pub async fn list_sessions_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListSessionsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    let status = query.status.as_deref();

    let is_admin = require_admin(&state, &agent_context).await.is_ok();
    let own_identity_id = if !is_admin {
        Some(agent_context.require_identity()?)
    } else {
        None
    };

    let sessions = state
        .session
        .list_sessions(limit, offset, status)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Non-admin users can only see their own sessions
    let filtered: Vec<_> = if let Some(identity_id) = own_identity_id {
        sessions
            .into_iter()
            .filter(|s| s.identity_id == identity_id)
            .collect()
    } else {
        sessions
    };

    // Enrich each session with identity & org names (concurrent lookups per session)
    let enriched: Vec<crate::models::session::SessionWithMeta> = futures_util::future::join_all(
        filtered
            .into_iter()
            .map(|s| enrich_session_with_meta(&state, s)),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": enriched })),
    ))
}

pub async fn end_session_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    // Check ownership: only session owner or admin can end a session
    let session = state
        .session
        .get_session(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Session {} not found", session_id)))?;

    let is_admin = require_admin(&state, &agent_context).await.is_ok();
    if !is_admin {
        let identity_id = agent_context.require_identity()?;
        if session.identity_id != identity_id {
            return Err(ApiError::Unauthorized(
                "Not authorized to end this session".to_string(),
            ));
        }
    }

    state
        .session
        .end_session(session_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"ended": session_id})),
    ))
}

pub async fn session_declare_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(session_id): Path<Uuid>,
    Json(body): Json<crate::api::models::SessionDeclareBody>,
) -> Result<impl IntoResponse, ApiError> {
    let router = state
        .session
        .declare_capabilities(session_id, body.capabilities)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(router).unwrap())))
}

/// Enrich a repo-level Session with identity and org names for admin display.
async fn enrich_session_with_meta(
    state: &AppRouterState,
    session: crate::db::repositories::session::Session,
) -> Result<crate::models::session::SessionWithMeta, ApiError> {
    let (identity_name, identity_display_name) = state
        .identity
        .get(session.identity_id)
        .await
        .ok()
        .flatten()
        .map(|id| (id.name.clone(), id.display_name.clone()))
        .unwrap_or_else(|| (session.identity_id.to_string(), None));

    let (org_name, tenant_name) = state
        .organization
        .get_org(session.org_id)
        .await
        .map(|org| (org.name, org.tenant_name))
        .unwrap_or_else(|_| (session.org_id.to_string(), None));

    Ok(crate::models::session::SessionWithMeta {
        id: session.id,
        identity_id: session.identity_id,
        identity_name,
        identity_display_name,
        org_id: session.org_id,
        org_name,
        tenant_name,
        status: session.status,
        tool_router: session.tool_router,
        capabilities: session.capabilities,
        created_at: session.created_at,
        last_active_at: session.last_active_at,
        ended_at: session.ended_at,
    })
}

/// Org Tool handlers

pub async fn register_org_tool_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::RegisterOrgToolBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_org_member(
        &state,
        &agent_context,
        body.org_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let tool = state
        .org_tool
        .register_tool(
            body.org_id,
            body.tool_id,
            body.name,
            body.description,
            body.schema.unwrap_or(serde_json::json!({})),
            body.implementation.unwrap_or(serde_json::json!({})),
        )
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(tool).unwrap()),
    ))
}

pub async fn list_org_tools_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(id): Path<Uuid>,
    Query(query): Query<crate::api::models::ListOrgToolsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    if let Some(approved_only) = query.approved_only {
        let tools = if approved_only {
            state.org_tool.list_approved_tools(id).await?
        } else {
            state.org_tool.list_org_tools(id).await?
        };
        Ok((StatusCode::OK, Json(serde_json::json!({ "data": tools }))))
    } else {
        let tool = state.org_tool.get_tool(id).await?;
        match tool {
            Some(t) => Ok((StatusCode::OK, Json(serde_json::json!({ "data": [t] })))),
            None => Err(ApiError::NotFound("Tool not found".to_string())),
        }
    }
}

pub async fn list_all_org_tools_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let tools = state
        .org_tool
        .list_all()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": tools }))))
}

pub async fn approve_org_tool_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(tool_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    // Get the tool to find its org_id, then check org membership
    let tool = state
        .org_tool
        .get_tool(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Tool {} not found", tool_id)))?;

    require_org_member(
        &state,
        &agent_context,
        tool.org_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    state
        .org_tool
        .approve_tool(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"approved": tool_id})),
    ))
}

// Group member management handlers (6.6)

pub async fn list_group_members_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(group_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool);

    let members = repo
        .list_members(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list group members: {}", e)))?;

    let response: Vec<crate::api::models::GroupMemberInfo> = members
        .into_iter()
        .map(|m| crate::api::models::GroupMemberInfo {
            agent_id: m.identity_id.to_string(),
            name: m.identity_name,
            email: m.email,
            username: m.username,
            role: m.role,
            joined_at: m.joined_at,
        })
        .collect();

    Ok((StatusCode::OK, Json(response)))
}

pub async fn add_group_member_handler(
    State(state): State<ApiState>,
    Path(group_id): Path<Uuid>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AddGroupMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool.clone());

    // Verify group exists and check org membership
    let group = repo
        .find_by_id(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Group {} not found", group_id)))?;
    require_org_member(
        &state,
        &agent_context,
        group.organization_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let target_id = uuid::Uuid::parse_str(&body.agent_id)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let role = body.role.unwrap_or_else(|| "member".to_string());

    repo.add_member(target_id, group_id, &role)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add group member: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "group_member_added".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": body.agent_id, "role": role}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Member added to group",
            "group_id": group_id,
            "member_id": body.agent_id,
        })),
    ))
}

pub async fn update_group_member_handler(
    State(state): State<ApiState>,
    Path((group_id, member_subject)): Path<(Uuid, String)>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool.clone());

    // Verify group exists and check org membership
    let group = repo
        .find_by_id(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Group {} not found", group_id)))?;
    require_org_member(
        &state,
        &agent_context,
        group.organization_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let target_id = uuid::Uuid::parse_str(&member_subject)
        .map_err(|_| ApiError::BadRequest("Invalid member subject".to_string()))?;

    repo.add_member(target_id, group_id, &body.role)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update group member: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "group_member_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": member_subject, "role": body.role}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member updated",
            "group_id": group_id,
            "member_id": member_subject,
        })),
    ))
}

pub async fn remove_group_member_handler(
    State(state): State<ApiState>,
    Path((group_id, member_subject)): Path<(Uuid, String)>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool.clone());

    // Verify group exists and check org membership
    let group = repo
        .find_by_id(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Group {} not found", group_id)))?;
    require_org_member(
        &state,
        &agent_context,
        group.organization_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let target_id = uuid::Uuid::parse_str(&member_subject)
        .map_err(|_| ApiError::BadRequest("Invalid member subject".to_string()))?;

    repo.remove_member(target_id, group_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove group member: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "group_member_removed".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": member_subject}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member removed",
            "group_id": group_id,
            "member_id": member_subject,
        })),
    ))
}

// Org slug-based Group management (6.6)

pub async fn create_org_group_handler(
    State(state): State<ApiState>,
    Path(slug_or_id): Path<String>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    let pool = state.agent_repo.pool().clone();
    let (org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    require_org_member(
        &state,
        &agent_context,
        org_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let mut new_group: crate::models::group::NewGroup = body.into();
    new_group.organization_id = org_id;

    let group = state
        .group
        .create(new_group)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "group_created".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group.id.to_string()),
            details: serde_json::json!({"org_slug": slug}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(group).unwrap()),
    ))
}

pub async fn list_org_groups_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(slug_or_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let pool = state.agent_repo.pool().clone();
    let (org_id, _slug) = resolve_org_id(&pool, &slug_or_id).await?;

    require_org_member(&state, &agent_context, org_id, None).await?;

    let groups = state
        .group
        .list_by_organization(org_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": groups }))))
}

pub async fn get_org_group_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let pool = state.agent_repo.pool().clone();
    let (_org_id, _slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let group = state
        .group
        .get(group_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Group not found".to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn update_org_group_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let group = state
        .group
        .update(group_id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn delete_org_group_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    state
        .group
        .delete(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_deleted".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"deleted": group_id})),
    ))
}

// Org slug/id-based Group member management (6.6)

pub async fn list_org_group_members_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, _slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let repo = GroupRepository::new(pool);
    let members = repo
        .list_members(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let member_info: Vec<crate::api::models::GroupMemberInfo> = members
        .into_iter()
        .map(|m| crate::api::models::GroupMemberInfo {
            agent_id: m.identity_id.to_string(),
            name: m.identity_name,
            email: m.email,
            username: m.username,
            role: m.role,
            joined_at: m.joined_at,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": member_info })),
    ))
}

pub async fn update_org_group_member_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id, username)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let target_id = uuid::Uuid::parse_str(&username)
        .map_err(|_| ApiError::BadRequest("Invalid member id".to_string()))?;

    let repo = GroupRepository::new(pool);
    repo.update_member_role(target_id, group_id, &body.role)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update group member: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_role_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "member_id": username, "role": body.role}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member role updated",
            "group_id": group_id,
            "member_id": username,
        })),
    ))
}

pub async fn remove_org_group_member_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id, username)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let target_id = uuid::Uuid::parse_str(&username)
        .map_err(|_| ApiError::BadRequest("Invalid member id".to_string()))?;

    let repo = GroupRepository::new(pool);
    repo.remove_member(target_id, group_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove group member: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_removed".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "member_id": username}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member removed",
            "group_id": group_id,
            "member_id": username,
        })),
    ))
}

// Org slug/id-based Group-Skill association (6.6)

pub async fn list_org_group_skills_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, _slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let repo = GroupSkillRepository::new(pool);
    let skills = repo
        .list_by_group(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": skills }))))
}

pub async fn add_org_group_skill_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::AddSkillToGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let skill_id = body
        .skill_id
        .clone()
        .ok_or_else(|| ApiError::BadRequest("skill_id is required".to_string()))?;

    let repo = GroupSkillRepository::new(pool);
    repo.associate_skill(crate::models::group_skill::NewGroupSkill {
        group_id,
        skill_id: skill_id.clone(),
        added_by: None,
    })
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to associate skill: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_skill_associated".to_string(),
            resource_type: "group_skill".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "skill_id": skill_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Skill associated to group",
            "group_id": group_id,
            "skill_id": skill_id,
        })),
    ))
}

pub async fn remove_org_group_skill_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id, skill_id)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let repo = GroupSkillRepository::new(pool);
    repo.dissociate_skill(group_id, &skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to dissociate skill: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_skill_dissociated".to_string(),
            resource_type: "group_skill".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "skill_id": skill_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill dissociated from group",
            "group_id": group_id,
            "skill_id": skill_id,
        })),
    ))
}

pub async fn reject_org_tool_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(tool_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .org_tool
        .reject_tool(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"rejected": tool_id})),
    ))
}

pub async fn delete_org_tool_handler(
    State(state): State<ApiState>,
    Path(tool_id): Path<Uuid>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    state
        .org_tool
        .delete(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"deleted": tool_id})),
    ))
}

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

// Group permission override handlers

pub async fn list_group_default_permissions_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::role_permission::RolePermissionRepository;

    let pool = state.group_perm_override_repo.pool().clone();
    let role_perm_repo = RolePermissionRepository::new(pool);

    let lead_defaults = role_perm_repo
        .list_by_role("group", "lead")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let member_defaults = role_perm_repo
        .list_by_role("group", "member")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let to_codes = |perms: Vec<crate::models::role_permission::RolePermission>| -> Vec<String> {
        perms.into_iter().map(|p| p.permission_code).collect()
    };

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "lead": to_codes(lead_defaults),
            "member": to_codes(member_defaults),
        })),
    ))
}

pub async fn list_group_permissions_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(group_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::api::models::GroupPermissionInfo;
    use crate::db::repositories::role_permission::RolePermissionRepository;

    let pool = state.group_perm_override_repo.pool().clone();

    let role_perm_repo = RolePermissionRepository::new(pool.clone());

    let lead_defaults = role_perm_repo
        .list_by_role("group", "lead")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let member_defaults = role_perm_repo
        .list_by_role("group", "member")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let overrides = state
        .group_perm_override_repo
        .list_by_group(group_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let is_overridden = |perm_code: &str| -> Option<bool> {
        overrides
            .iter()
            .find(|o| o.permission_code == perm_code)
            .map(|o| o.granted)
    };

    let to_info =
        |perms: Vec<crate::models::role_permission::RolePermission>| -> Vec<GroupPermissionInfo> {
            perms
                .into_iter()
                .map(|p| {
                    let code = p.permission_code;
                    let override_granted = is_overridden(&code);
                    GroupPermissionInfo {
                        permission_code: code,
                        granted: override_granted.unwrap_or(true),
                        is_default: override_granted.is_none(),
                    }
                })
                .collect()
        };

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "lead": to_info(lead_defaults),
            "member": to_info(member_defaults),
        })),
    ))
}

pub async fn update_group_permission_handler(
    State(state): State<ApiState>,
    Path(group_id): Path<Uuid>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupPermissionBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::models::group_permission_override::NewGroupPermissionOverride;

    let role_name = body.role_name.clone();
    let permission_code = body.permission_code.clone();

    let creator_id = uuid::Uuid::parse_str(&subject).ok();

    state
        .group_perm_override_repo
        .upsert_override(NewGroupPermissionOverride {
            group_id,
            role_name: body.role_name,
            permission_code: body.permission_code,
            granted: body.granted,
            created_by: creator_id,
        })
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_permission_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({
                "role_name": role_name,
                "permission_code": permission_code,
                "granted": body.granted,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group permission override updated"
        })),
    ))
}

pub async fn delete_group_permission_handler(
    State(state): State<ApiState>,
    Path((group_id, permission_code)): Path<(Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupPermissionBody>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .group_perm_override_repo
        .delete_override(group_id, &body.role_name, &permission_code)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let role_name = body.role_name.clone();

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_permission_deleted".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({
                "role_name": role_name,
                "permission_code": permission_code,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group permission override deleted"
        })),
    ))
}

// --- Admin User Management Handlers (Feature #7) ---

pub async fn list_users_handler_admin(
    State(state): State<ApiState>,
    AgentContext { .. }: AgentContext,
    Query(query): Query<crate::api::models::ListUsersQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);
    let identity_type = query.identity_type.as_deref();

    let users = state
        .identity
        .list(limit, offset, identity_type)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<crate::api::models::UserAdminResponse> = users
        .into_iter()
        .map(|u| crate::api::models::UserAdminResponse {
            id: u.id,
            identity_type: u.identity_type.to_string(),
            username: u.username,
            display_name: u.display_name,
            email: u.email,
            avatar_url: u.avatar_url,
            is_system_admin: u.is_system_admin,
            status: u.status.to_string(),
            created_at: u.created_at,
            updated_at: u.updated_at,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": data,
            "total": data.len(),
            "limit": limit,
            "offset": offset,
        })),
    ))
}

pub async fn disable_user_handler_admin(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Path(user_id): Path<uuid::Uuid>,
    Json(body): Json<crate::api::models::DisableUserBody>,
) -> Result<impl IntoResponse, ApiError> {
    let status = if body.disabled { "disabled" } else { "active" };

    let update = crate::models::identity::IdentityUpdate {
        status: Some(status.into()),
        ..Default::default()
    };

    let updated = state
        .identity
        .update(user_id, update)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: if body.disabled {
                "user_disabled".to_string()
            } else {
                "user_enabled".to_string()
            },
            resource_type: "user".to_string(),
            resource_id: Some(user_id.to_string()),
            details: serde_json::json!({
                "username": updated.username,
                "status": status,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("User {} successfully", if body.disabled { "disabled" } else { "enabled" }),
            "user_id": user_id.to_string(),
        })),
    ))
}

pub async fn delete_user_handler_admin(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .identity
        .delete(user_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "user_deleted".to_string(),
            resource_type: "user".to_string(),
            resource_id: Some(user_id.to_string()),
            details: serde_json::json!({}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "User deleted successfully",
            "user_id": user_id.to_string(),
        })),
    ))
}

// --- Evaluation Query/Delete Handlers (Feature #8) ---

pub async fn list_evaluations_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Query(query): Query<crate::api::models::ListEvaluationsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let skill_id = match query.skill_id.as_deref() {
        Some(id) => id,
        None => {
            return Err(ApiError::BadRequest(
                "skill_id query parameter is required".to_string(),
            ))
        }
    };

    let evals = state
        .evaluator
        .list_evaluations(skill_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<crate::api::models::EvaluationItemResponse> = evals
        .into_iter()
        .map(|e| crate::api::models::EvaluationItemResponse {
            id: e.id,
            skill_id: e.skill_id,
            agent_id: e.agent_id,
            success: e.success,
            duration_ms: e.duration_ms,
            error_type: e.error_type.map(|et| format!("{:?}", et)),
            tags: e.tags.iter().map(|t| format!("{:?}", t)).collect(),
            timestamp: e.timestamp.to_rfc3339(),
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

pub async fn get_evaluation_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(eval_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let eval = state
        .evaluator
        .get_evaluation(eval_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Evaluation {} not found", eval_id)))?;

    let response = crate::api::models::EvaluationItemResponse {
        id: eval.id,
        skill_id: eval.skill_id,
        agent_id: eval.agent_id,
        success: eval.success,
        duration_ms: eval.duration_ms,
        error_type: eval.error_type.map(|et| format!("{:?}", et)),
        tags: eval.tags.iter().map(|t| format!("{:?}", t)).collect(),
        timestamp: eval.timestamp.to_rfc3339(),
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn delete_evaluation_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(eval_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    // Check ownership: only evaluation creator or admin can delete
    let eval = state
        .evaluator
        .get_evaluation(eval_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Evaluation {} not found", eval_id)))?;

    let is_admin = require_admin(&state, &agent_context).await.is_ok();
    if !is_admin {
        let identity_id = agent_context.require_identity()?;
        let subject_str = identity_id.to_string();
        let agent_str = agent_context
            .agent_id
            .map(|id| id.to_string())
            .unwrap_or_default();
        if eval.agent_id != subject_str && eval.agent_id != agent_str {
            return Err(ApiError::Unauthorized(
                "Not authorized to delete this evaluation".to_string(),
            ));
        }
    }

    state
        .evaluator
        .delete_evaluation(eval_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "evaluation_deleted".to_string(),
            resource_type: "evaluation".to_string(),
            resource_id: Some(eval_id.to_string()),
            details: serde_json::json!({}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Evaluation deleted successfully",
            "evaluation_id": eval_id.to_string(),
        })),
    ))
}

// --- Webhook Management Handlers (Feature #11) ---

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

// --- Skill Upload & Version Management Handlers (Feature #12) ---

/// Handler for ZIP upload of a skill package
pub async fn upload_skill_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    mut multipart: axum::extract::Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut zip_data: Option<Vec<u8>> = None;
    let mut owner_type = "user".to_string();
    let mut owner_id: Option<uuid::Uuid> = None;
    let mut author_identity_id: Option<uuid::Uuid> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;
                zip_data = Some(data.to_vec());
            }
            "owner_type" => {
                owner_type = field.text().await.unwrap_or_else(|_| "user".to_string());
            }
            "owner_id" => {
                let val = field.text().await.unwrap_or_default();
                if !val.is_empty() {
                    owner_id = uuid::Uuid::parse_str(&val).ok();
                }
            }
            "author_identity_id" => {
                let val = field.text().await.unwrap_or_default();
                if !val.is_empty() {
                    author_identity_id = uuid::Uuid::parse_str(&val).ok();
                }
            }
            _ => {}
        }
    }

    let zip_data = zip_data
        .ok_or_else(|| ApiError::BadRequest("ZIP file is required in 'file' field".to_string()))?;

    let upload_result = state
        .skill_git
        .process_upload(
            &zip_data,
            &subject,
            author_identity_id,
            &owner_type,
            owner_id,
            &state.registry,
            &state.search,
            &state.skill_repo,
            &state.version_repo,
        )
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Audit log
    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_uploaded".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(upload_result.skill_id.clone()),
            details: serde_json::json!({
                "skill_name": upload_result.skill_name,
                "version": upload_result.version,
                "git_commit": upload_result.git_commit,
                "git_tag": upload_result.git_tag,
                "is_new_skill": upload_result.is_new_skill,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let response = crate::api::models::SkillUploadResponse {
        skill_id: upload_result.skill_id,
        skill_name: upload_result.skill_name,
        version: upload_result.version,
        git_commit: upload_result.git_commit,
        git_tag: upload_result.git_tag,
        git_repo_name: upload_result.git_repo_name,
        is_new_skill: upload_result.is_new_skill,
        files: upload_result.files,
        message: "Skill uploaded successfully".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

// --- Skill Upload Preview & Confirm Handlers ---

/// POST /api/v1/skills/upload/preview — 上传 ZIP 仅解压预览，不提交
pub async fn upload_skill_preview_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    mut multipart: axum::extract::Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut zip_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;
            zip_data = Some(data.to_vec());
        }
    }

    let zip_data = zip_data
        .ok_or_else(|| ApiError::BadRequest("ZIP file is required in 'file' field".to_string()))?;

    let preview = state
        .skill_git
        .preview_upload(&zip_data)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let response = crate::api::models::SkillUploadPreviewResponse {
        preview_id: preview.preview_id,
        metadata: crate::api::models::PreviewMetadataResponse {
            name: preview.metadata.name,
            description: preview.metadata.description,
            version: preview.metadata.version.unwrap_or_default(),
            tags: preview.metadata.tags,
            dependencies: preview.metadata.dependencies,
            compatibility: preview.metadata.compatibility,
        },
        files: preview
            .files
            .into_iter()
            .map(|f| crate::api::models::PreviewFileResponse {
                path: f.path,
                size: f.size,
            })
            .collect(),
        total_files: preview.total_files,
        total_size: preview.total_size,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// GET /api/v1/skills/upload/preview/:preview_id/files/*path — 获取预览中文件内容
pub async fn get_preview_file_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    axum::extract::Path((preview_id,)): axum::extract::Path<(String,)>,
    req: axum::http::Request<axum::body::Body>,
) -> Result<impl IntoResponse, ApiError> {
    // Parse file_path from the URL path after /files/
    let uri_path = req.uri().path().to_string();
    let file_marker = "/files/";
    let file_path = match uri_path.find(file_marker) {
        Some(pos) => {
            let raw = &uri_path[pos + file_marker.len()..];
            percent_encoding::percent_decode_str(raw)
                .decode_utf8()
                .map_err(|e| ApiError::BadRequest(format!("Invalid file path encoding: {}", e)))?
                .to_string()
        }
        None => {
            return Err(ApiError::BadRequest(
                "File path not found in URL".to_string(),
            ));
        }
    };

    if file_path.is_empty() {
        return Err(ApiError::BadRequest("File path is required".to_string()));
    }

    let (content, content_type, size) = state
        .skill_git
        .get_preview_file(&preview_id, &file_path)
        .map_err(|e| match e {
        crate::models::error::AppError::FileNotFound(msg) => ApiError::NotFound(msg),
        _ => ApiError::BadRequest(e.to_string()),
    })?;

    let is_binary = content_type == "application/octet-stream";
    let text_content = if is_binary {
        format!("[Binary file: {} bytes, not displayable as text]", size)
    } else {
        String::from_utf8(content)
            .unwrap_or_else(|_| format!("[Cannot decode file as UTF-8: {} bytes]", size))
    };

    let response = crate::api::models::PreviewFileContentResponse {
        path: file_path,
        content: text_content,
        size,
        is_binary,
        content_type,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// POST /api/v1/skills/upload/preview/:preview_id/confirm — 确认上传，提交 Git + DB
pub async fn confirm_skill_upload_handler(
    State(state): State<ApiState>,
    AgentContext {
        subject,
        identity_id,
        org_id: agent_org_id,
        roles,
        ..
    }: AgentContext,
    axum::extract::Path((preview_id,)): axum::extract::Path<(String,)>,
    Json(body): Json<crate::api::models::ConfirmUploadBody>,
) -> Result<impl IntoResponse, ApiError> {
    let _identity_id =
        identity_id.ok_or_else(|| ApiError::Unauthorized("identity_id required".to_string()))?;

    let is_admin = roles.iter().any(|r| r == "admin");

    // 推断 owner_type：body 显式 → 自动（有 agent_org_id → organization，否则 user）
    let effective_owner_type = body.owner_type.as_deref().unwrap_or_else(|| {
        if agent_org_id.is_some() {
            "organization"
        } else {
            "user"
        }
    });

    let (owner_type, owner_id) = if effective_owner_type == "organization" {
        let org_id = body
            .organization_id
            .or(body.owner_id)
            .or(agent_org_id)
            .ok_or_else(|| {
                ApiError::BadRequest(
                    "organization_id is required when owner_type is organization".to_string(),
                )
            })?;

        // 验证用户属于该组织（admin 跳过组织成员校验）
        if !is_admin {
            let is_member = state
                .permission
                .is_org_member(_identity_id, org_id)
                .await
                .map_err(|e| ApiError::InternalError(e.to_string()))?;
            if !is_member {
                return Err(ApiError::Forbidden(
                    "你不能为不属于的组织创建 Skill".to_string(),
                ));
            }
        }

        ("organization".to_string(), Some(org_id))
    } else {
        // 个人用户创建
        ("user".to_string(), Some(_identity_id))
    };

    let author_identity_id = body.author_identity_id.or(Some(_identity_id));

    let upload_result = state
        .skill_git
        .confirm_upload_from_preview(
            &preview_id,
            &subject,
            author_identity_id,
            &owner_type,
            owner_id,
            &state.registry,
            &state.search,
            &state.skill_repo,
            &state.version_repo,
        )
        .await
        .map_err(|e| match e {
            crate::models::error::AppError::ValidationError(ref msg)
                if msg.contains("完全相同") =>
            {
                ApiError::BadRequest(msg.clone())
            }
            other => ApiError::BadRequest(other.to_string()),
        })?;

    // Audit log
    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject.clone()),
            action: "skill_uploaded".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(upload_result.skill_id.clone()),
            details: serde_json::json!({
                "skill_name": upload_result.skill_name,
                "version": upload_result.version,
                "git_commit": upload_result.git_commit,
                "git_tag": upload_result.git_tag,
                "is_new_skill": upload_result.is_new_skill,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let response = crate::api::models::SkillUploadResponse {
        skill_id: upload_result.skill_id,
        skill_name: upload_result.skill_name,
        version: upload_result.version,
        git_commit: upload_result.git_commit,
        git_tag: upload_result.git_tag,
        git_repo_name: upload_result.git_repo_name,
        is_new_skill: upload_result.is_new_skill,
        files: upload_result.files,
        message: "Skill uploaded successfully".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/skills/:name/versions — list versions for a skill by name
pub async fn list_skill_versions_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_name): Path<String>,
    Query(query): Query<crate::api::models::ListVersionsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let versions = state
        .version_repo
        .list_by_name(&skill_name, limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<crate::api::models::SkillVersionResponse> = versions
        .into_iter()
        .map(|v| crate::api::models::SkillVersionResponse {
            id: v.id.to_string(),
            skill_name: v.skill_name,
            version: v.version,
            git_commit_hash: v.git_commit_hash,
            git_tag: v.git_tag,
            changelog: v.changelog,
            file_count: v.file_count,
            total_size_bytes: v.total_size_bytes,
            uploaded_by: v.uploaded_by,
            git_remote_url: v.git_remote_url,
            created_at: v.created_at.to_rfc3339(),
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

/// GET /api/v1/skills/:name/versions/diff — diff between two versions
pub async fn get_skill_version_diff_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_name): Path<String>,
    Query(query): Query<crate::api::models::VersionDiffQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let diff = state
        .skill_git
        .get_version_diff(&skill_name, &query.from, &query.to)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "skill_name": skill_name,
            "from_version": query.from,
            "to_version": query.to,
            "diff": diff,
        })),
    ))
}

/// GET /api/v1/skills/:name/tags — list git tags for a skill
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

/// POST /api/v1/skills/:name/sync — 从 GitLab 拉取最新更新
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

/// POST /api/v1/skills/:name/clone — 从 GitLab 克隆 skill 仓库到本地
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

/// GET /api/v1/skills/:name/remote — 查看 skill 关联的 GitLab 信息
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

/// GET /api/v1/admin/skills/gitlab-sync — 批量同步已配置 remote 的 skills
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

/// POST /api/v1/webhooks/gitlab — 接收 GitLab push events
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
            // fallback: 尝试直接解析
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

    // 仅对 push/tag_push events 做同步
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

// ============================================================================
// Marketplace Delist Request Workflow (Author → Admin/Reviewer)
// ============================================================================

/// POST /api/v1/skills/:id/request-delist — 作者申请下架市场上架的 Skill
pub async fn request_marketplace_delist_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::RequestDelistBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let identity_id = agent_context.require_identity()?;

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // 只有 Skill 的作者（owner）或组织 Admin 可以申请下架
    let mut is_owner = false;

    if skill.owner_type == "user" {
        // 个人 Skill：作者本人可申请下架
        is_owner = skill.owner_id == Some(identity_id)
            || skill.author_identity_id == Some(identity_id);
    } else if skill.owner_type == "organization" {
        // 组织 Skill：组织 Admin 及以上可申请下架
        if let Some(org_id) = skill.owner_id {
            match state.permission.get_org_role(identity_id, org_id).await {
                Ok(Some(role)) if role >= crate::models::org_membership::OrgRole::Admin => {
                    is_owner = true;
                }
                _ => {}
            }
        }
    }

    if !is_owner {
        return Err(ApiError::Unauthorized(
            "Only the skill owner can request delisting".to_string(),
        ));
    }

    // 只有上架中的 Skill 可以申请下架
    if skill.marketplace_status.as_deref() != Some("listed") {
        return Err(ApiError::BadRequest(
            "Only listed marketplace skills can request delisting".to_string(),
        ));
    }

    // 设置 marketplace_status = pending_delist，同时保存下架原因到 review_comment
    skill_repo
        .update_marketplace_status(&skill_id, Some("pending_delist"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to request delist: {}", e))
        })?;

    if let Some(ref reason) = body.reason {
        skill_repo
            .update_status(&skill_id, &skill.status, None, Some(reason))
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to save delist reason: {}", e)))?;
    }

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_delist_requested".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "reason": body.reason,
                "new_marketplace_status": "pending_delist",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Delist request submitted for review".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "pending_delist",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/approve-delist — 市场管理员批准下架申请
pub async fn marketplace_approve_delist_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_delist") {
        return Err(ApiError::BadRequest(
            "Skill is not pending delist review".to_string(),
        ));
    }

    // 批准下架：设置 marketplace_status=delisted，恢复 pre_marketplace_visibility
    let pre_visibility = skill.pre_marketplace_visibility.as_deref().unwrap_or("private");
    skill_repo
        .update_marketplace_status(&skill_id, Some("delisted"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve delist: {}", e)))?;

    skill_repo
        .update(&skill_id, None, None, None, Some(pre_visibility))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to revert visibility: {}", e))
        })?;

    skill_repo
        .set_admin_unpublished(&skill_id, true)
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set admin unpublished flag: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_delist_approved".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "delisted",
                "restored_visibility": pre_visibility,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Delist request approved, skill delisted from marketplace".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "delisted",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/reject-delist — 市场管理员驳回下架申请
pub async fn marketplace_reject_delist_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_delist") {
        return Err(ApiError::BadRequest(
            "Skill is not pending delist review".to_string(),
        ));
    }

    // 驳回下架申请：恢复 marketplace_status = listed
    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject delist: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_delist_rejected".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Delist request rejected, skill remains listed".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

// ============================================================================
// Marketplace Update Review Workflow (pending_update)
// ============================================================================

/// POST /api/v1/admin/marketplace/:id/approve-update — 批准内容更新
pub async fn marketplace_approve_update_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_update") {
        return Err(ApiError::BadRequest(
            "Skill is not pending update review".to_string(),
        ));
    }

    let draft = skill.draft_content.ok_or_else(|| {
        ApiError::BadRequest("No draft content to apply".to_string())
    })?;

    // 应用 draft 到主字段
    skill_repo
        .apply_draft_content(&skill_id, &draft)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to apply update: {}", e)))?;

    // 恢复 marketplace_status = listed
    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to restore listed status: {}", e)))?;

    // 更新搜索索引
    if let Ok(updated) = state.registry.get_skill(&skill_id).await {
        if let Err(e) = state.search.update_skill(&updated) {
            tracing::warn!("Failed to update search index for {}: {}", skill_id, e);
        }
    }

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_update_approved".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Update approved and applied".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/reject-update — 驳回内容更新
pub async fn marketplace_reject_update_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_update") {
        return Err(ApiError::BadRequest(
            "Skill is not pending update review".to_string(),
        ));
    }

    // 清空 draft，恢复 listed
    skill_repo
        .clear_draft_content(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to clear draft: {}", e)))?;

    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to restore listed status: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_update_rejected".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Update rejected, skill remains listed".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

/// POST /api/v1/skills/:id/cancel-update — 作者取消更新草稿
pub async fn cancel_update_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let identity_id = agent_context.require_identity()?;

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // 只有作者可以取消
    let is_owner = skill.owner_type == "user"
        && (skill.owner_id == Some(identity_id)
            || skill.author_identity_id == Some(identity_id));
    if !is_owner && skill.owner_type == "organization" {
        // 组织成员检查稍后在下面
    }
    if !is_owner && skill.owner_type == "user" {
        return Err(ApiError::Unauthorized(
            "Only the skill owner can cancel updates".to_string(),
        ));
    }

    if skill.marketplace_status.as_deref() != Some("pending_update") {
        return Err(ApiError::BadRequest(
            "Skill is not pending update review".to_string(),
        ));
    }

    skill_repo
        .clear_draft_content(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to clear draft: {}", e)))?;

    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to restore listed status: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_update_cancelled".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Update cancelled, skill remains listed".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

/// GET /api/v1/admin/marketplace/stats — 市场统计（市场管理员/审核员可访问）
pub async fn marketplace_stats_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;

    let pool = state.agent_repo.pool();

    let listed = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE marketplace_status = 'listed'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let pending_review = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE marketplace_status = 'pending_review'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let pending_update = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE marketplace_status = 'pending_update'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let pending_delist = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE marketplace_status = 'pending_delist'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 本月新增（listed 且 created_at 在本月）
    let new_this_month = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE marketplace_status = 'listed' AND created_at >= date_trunc('month', NOW())",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 总安装次数（listed 的 skill）
    let total_installs = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(install_count), 0) FROM skills WHERE marketplace_status = 'listed'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "listed": listed,
            "pending_review": pending_review,
            "pending_update": pending_update,
            "pending_delist": pending_delist,
            "new_this_month": new_this_month,
            "total_installs": total_installs,
        })),
    ))
}
