//! 审计日志 handlers

use axum::{extract::{Query, State}, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::ApiState;

/// 审计读路径的角色级 scope
#[derive(Debug, Clone, Default)]
struct AuditScope {
    /// 限制在哪些 tenant 下（None = 无限制；super_admin）
    tenant_ids: Option<Vec<Uuid>>,
    /// 当历史日志的 tenant_id 为 NULL 时，按 identity 反查的兜底集合
    /// （None = 无需反查；Some(_) = 用此列表过滤 identity_id）
    identity_ids: Option<Vec<Uuid>>,
}

/// 计算审计日志查询的角色 scope：
/// - super_admin  →  无限制
/// - tenant_admin →  限制在管租户下，并预先反查出该租户下所有 identity 用于日志 tenant_id 为 NULL 的兜底
/// - 其他         →  403
///
/// 注意：不能仅凭 JWT `roles` 包含 "admin" 就放行，因为登录时
/// `tenant_admin` 也会被授予 "admin" 角色（见 users.rs user_login_handler）。
/// 鉴权必须直接查 DB 角色表，避免"标签越权"。
async fn compute_audit_scope(
    state: &ApiState,
    agent_context: &AgentContext,
) -> Result<AuditScope, ApiError> {
    let identity_id = agent_context.require_identity()?;

    // 1) super_admin：全平台可见
    if state
        .permission
        .is_super_admin(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
    {
        return Ok(AuditScope::default());
    }

    // 2) tenant_admin：仅其管理的租户范围
    let tenant_ids = state
        .permission
        .get_tenant_admin_tenant_ids(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    if tenant_ids.is_empty() {
        return Err(ApiError::Forbidden(
            "Audit log access requires super_admin or tenant_admin role".to_string(),
        ));
    }

    // 兜底：反查这些租户下所有 identity（用于历史日志 tenant_id 为 NULL 的场景）
    let identity_ids = state
        .audit
        .list_identity_ids_by_tenants(&tenant_ids)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(AuditScope {
        tenant_ids: Some(tenant_ids),
        identity_ids: Some(identity_ids),
    })
}

/// 校验客户端传入的 tenant_id 是否在 scope 内
fn ensure_tenant_in_scope(
    requested: Option<Uuid>,
    scope: &AuditScope,
) -> Result<Option<Uuid>, ApiError> {
    match (requested, &scope.tenant_ids) {
        (None, _) => Ok(None),
        (Some(tid), None) => Ok(Some(tid)),
        (Some(tid), Some(allowed)) if allowed.contains(&tid) => Ok(Some(tid)),
        (Some(tid), Some(_)) => Err(ApiError::Forbidden(format!(
            "Tenant {} is outside your audit log scope",
            tid
        ))),
    }
}

pub async fn list_audit_entries_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListAuditEntriesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // 至少需要有审计查看权限（super_admin 或 tenant_admin）
    let scope = compute_audit_scope(&state, &agent_context).await?;
    let tenant_id = ensure_tenant_in_scope(query.tenant_id, &scope)?;
    let organization_id = query.organization_id;
    let identity_id = query.identity_id;
    let identity_ids = scope.identity_ids.clone();

    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    let audit_query = crate::models::api_key::AuditLogQuery {
        tenant_id,
        organization_id,
        identity_id,
        identity_ids,
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

pub async fn list_audit_logs_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::AuditLogQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let scope = compute_audit_scope(&state, &agent_context).await?;
    let tenant_id = ensure_tenant_in_scope(query.tenant_id, &scope)?;
    let identity_ids = scope.identity_ids.clone();

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    // 直接用新的 query / count（支持 tenant_id 过滤），
    // 避免走 audit_repo.list_with_filters 这条不会强制 scope 的旧路径。
    let logs = state
        .audit
        .query(crate::models::api_key::AuditLogQuery {
            tenant_id,
            organization_id: query.organization_id,
            identity_id: query.identity_id,
            identity_ids: identity_ids.clone(),
            action: query.action.clone(),
            resource_type: query.resource_type.clone(),
            limit: Some(limit),
            offset: Some(offset),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total = state
        .audit
        .count(
            tenant_id,
            query.organization_id,
            query.identity_id,
            identity_ids.as_deref(),
            query.action.as_deref(),
            query.resource_type.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<_> = logs
        .into_iter()
        .map(|log| crate::api::models::AuditLogResponse {
            id: log.id.to_string(),
            agent_id: log
                .details
                .as_ref()
                .and_then(|d| d.get("_legacy_agent_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            identity_name: log.identity_name,
            identity_type: log.identity_type,
            action: log.action,
            resource_type: log.resource_type.unwrap_or_default(),
            resource_id: log
                .resource_id
                .map(|id| id.to_string())
                .or_else(|| {
                    log.details
                        .as_ref()
                        .and_then(|d| d.get("_resource_id_str"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                }),
            details: log.details.unwrap_or(serde_json::json!({})),
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            timestamp: log.created_at.to_rfc3339(),
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


