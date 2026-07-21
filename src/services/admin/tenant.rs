//! Tenant Service

use std::collections::HashMap;

use crate::db::repositories::TenantRepository;
use crate::models::error::AppError;
use crate::models::tenant::{NewTenant, Tenant, TenantUpdate};
use uuid::Uuid;

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
}
