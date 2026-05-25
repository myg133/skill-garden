//! Organization Service

use uuid::Uuid;
use crate::db::repositories::organization::{OrganizationRepository, NewOrganization, Organization as OrgRepo};
use crate::models::error::AppError;

#[derive(Clone)]
pub struct OrganizationService {
    org_repo: OrganizationRepository,
}

impl std::fmt::Debug for OrganizationService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationService").finish()
    }
}

impl OrganizationService {
    pub fn new(org_repo: OrganizationRepository) -> Self {
        Self { org_repo }
    }

    pub async fn create_org(&self, name: String) -> Result<OrgRepo, AppError> {
        let new_org = NewOrganization {
            name: name.clone(),
            settings: None,
        };

        self.org_repo.create(new_org)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_org(&self, id: Uuid) -> Result<OrgRepo, AppError> {
        self.org_repo.find_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
            .ok_or_else(|| AppError::ValidationError(format!("Organization {} not found", id)))
    }

    pub async fn list_orgs(&self, limit: i64, offset: i64) -> Result<Vec<OrgRepo>, AppError> {
        self.org_repo.list(limit, offset)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn update_org(&self, id: Uuid, name: String) -> Result<OrgRepo, AppError> {
        self.org_repo.update(id, name)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn delete_org(&self, id: Uuid) -> Result<(), AppError> {
        self.org_repo.delete(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
