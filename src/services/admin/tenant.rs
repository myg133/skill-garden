//! Tenant Service

use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use crate::db::repositories::TenantRepository;
use crate::models::error::AppError;
use crate::models::tenant::{
    NewTenant, RequestStatus, Tenant, TenantCreationRequest, TenantUpdate,
};

#[derive(Clone)]
pub struct TenantService {
    repo: TenantRepository,
}

impl std::fmt::Debug for TenantService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TenantService").finish()
    }
}

impl TenantService {
    pub fn new(repo: TenantRepository) -> Self {
        Self { repo }
    }

    pub async fn create(&self, new_tenant: NewTenant) -> Result<Tenant, AppError> {
        self.repo
            .create(new_tenant)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<Tenant>, AppError> {
        self.repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_by_slug(&self, slug: &str) -> Result<Option<Tenant>, AppError> {
        self.repo
            .find_by_slug(slug)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 批量查询租户名称（避免 N+1）
    pub async fn get_names_by_ids(&self, ids: &[Uuid]) -> Result<HashMap<Uuid, String>, AppError> {
        self.repo
            .find_names_by_ids(ids)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Tenant>, AppError> {
        self.repo
            .list_all(limit, offset)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn update(&self, id: Uuid, update: TenantUpdate) -> Result<Tenant, AppError> {
        self.repo
            .update(id, update)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        self.repo
            .delete(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    // ===== Tenant Creation Request methods =====

    pub async fn create_tenant_request(
        &self,
        applicant_id: Uuid,
        applicant_name: String,
        applicant_email: String,
        tenant_name: String,
        tenant_slug: String,
        message: Option<String>,
    ) -> Result<TenantCreationRequest, AppError> {
        let request = TenantCreationRequest {
            id: Uuid::new_v4(),
            applicant_id,
            applicant_name,
            applicant_email,
            tenant_name: tenant_name.clone(),
            tenant_slug: tenant_slug.clone(),
            message,
            status: RequestStatus::Pending,
            reviewed_by: None,
            reviewed_at: None,
            review_note: None,
            tenant_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.repo
            .create_tenant_request(request)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_tenant_request(
        &self,
        id: Uuid,
    ) -> Result<Option<TenantCreationRequest>, AppError> {
        self.repo
            .get_tenant_request(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_tenant_requests(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TenantCreationRequest>, AppError> {
        self.repo
            .list_tenant_requests(limit, offset)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_pending_tenant_requests(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TenantCreationRequest>, AppError> {
        self.repo
            .list_pending_tenant_requests(limit, offset)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn count_requests_by_applicant(&self, applicant_id: Uuid) -> Result<i64, AppError> {
        self.repo
            .count_tenant_requests_by_applicant(applicant_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn has_pending_request(&self, applicant_id: Uuid) -> Result<bool, AppError> {
        self.repo
            .check_pending_request_exists(applicant_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn review_tenant_request(
        &self,
        id: Uuid,
        action: &str,
        reviewed_by: Uuid,
        note: Option<String>,
        tenant_id: Option<Uuid>,
    ) -> Result<TenantCreationRequest, AppError> {
        let status = match action.to_lowercase().as_str() {
            "approve" => RequestStatus::Approved,
            "reject" => RequestStatus::Rejected,
            _ => {
                return Err(AppError::ValidationError(
                    "Invalid action. Use 'approve' or 'reject'".to_string(),
                ))
            }
        };

        self.repo
            .update_tenant_request_status(id, status, reviewed_by, note, tenant_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
