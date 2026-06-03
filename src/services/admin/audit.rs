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
                query.action.as_deref(),
                query.resource_type.as_deref(),
                limit,
                offset,
            )
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// Return all audit log entries whose `tenant_id` is in `tenant_ids`.
    /// Used by the tenant-scope guard (Task 10) to filter the
    /// /api/v1/admin/audit-entries endpoint to the caller's accessible
    /// tenants. Returns an empty Vec for an empty slice — the caller
    /// never asks "for an empty tenant set", and avoiding the repository
    /// call also avoids the `tenant_id = ANY('{}')` semantics.
    pub async fn list_by_tenants(
        &self,
        tenant_ids: &[Uuid],
        organization_id: Option<Uuid>,
        identity_id: Option<Uuid>,
        action: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLog>, AppError> {
        if tenant_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.repo
            .list_by_tenants(tenant_ids, organization_id, identity_id, action, limit, offset)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
