//! Audit Service

use crate::db::repositories::AuditLogRepository;
use crate::models::api_key::{AuditLog, AuditLogQuery, CreateAuditLogRequest};
use crate::models::error::AppError;
use uuid::Uuid;

#[derive(Clone)]
pub struct AuditService {
    repo: AuditLogRepository,
}

impl std::fmt::Debug for AuditService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditService").finish()
    }
}

impl AuditService {
    pub fn new(repo: AuditLogRepository) -> Self {
        Self { repo }
    }

    pub async fn create(&self, request: CreateAuditLogRequest) -> Result<AuditLog, AppError> {
        self.repo
            .create(request)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 便捷写入审计日志（不 panic，失败时仅 log warning）
    pub async fn write_entry(
        &self,
        identity_id: Uuid,
        action: &str,
        resource_type: &str,
        resource_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) {
        let req = CreateAuditLogRequest {
            tenant_id: None,
            organization_id: None,
            identity_id,
            action: action.to_string(),
            resource_type: Some(resource_type.to_string()),
            resource_id,
            details,
            ip_address: None,
            user_agent: None,
        };
        if let Err(e) = self.create(req).await {
            tracing::warn!("Failed to write audit entry: {}", e);
        }
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<AuditLog>, AppError> {
        self.repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn query(&self, query: AuditLogQuery) -> Result<Vec<AuditLog>, AppError> {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        self.repo
            .query(
                query.tenant_id,
                query.organization_id,
                query.identity_id,
                query.identity_ids.as_deref(),
                query.action.as_deref(),
                query.resource_type.as_deref(),
                limit,
                offset,
            )
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn count(
        &self,
        tenant_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        identity_id: Option<Uuid>,
        identity_ids: Option<&[Uuid]>,
        action: Option<&str>,
        resource_type: Option<&str>,
    ) -> Result<i64, AppError> {
        self.repo
            .count(
                tenant_id,
                organization_id,
                identity_id,
                identity_ids,
                action,
                resource_type,
            )
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 给定 tenant_ids 列表，返回这些租户下所有 organization 内的 identity_id（去重）。
    pub async fn list_identity_ids_by_tenants(
        &self,
        tenant_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        self.repo
            .list_identity_ids_by_tenants(tenant_ids)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
