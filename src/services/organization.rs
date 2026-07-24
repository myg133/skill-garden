//! Organization Service

use crate::db::repositories::organization::OrganizationRepository;
use crate::models::error::AppError;
use crate::models::organization::{NewOrganization, Organization};
use uuid::Uuid;

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

    pub async fn create_org(
        &self,
        name: String,
        slug: Option<String>,
        display_name: Option<String>,
        description: Option<String>,
        tenant_id: Option<Uuid>,
    ) -> Result<Organization, AppError> {
        let new_org = NewOrganization {
            name: name.clone(),
            slug,
            display_name,
            description,
            tenant_id,
            org_type: None,
            visibility: Some("public".to_string()),
            avatar_url: None,
            settings: None,
        };

        self.org_repo
            .create(new_org)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_org(&self, id: Uuid) -> Result<Organization, AppError> {
        self.org_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
            .ok_or_else(|| AppError::ValidationError(format!("Organization {} not found", id)))
    }

    pub async fn list_orgs(&self, limit: i64, offset: i64) -> Result<Vec<Organization>, AppError> {
        self.org_repo
            .list(limit, offset)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_orgs_by_tenant(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Organization>, AppError> {
        self.org_repo
            .list_by_tenant(tenant_id, limit, offset)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn update_org(
        &self,
        id: Uuid,
        name: String,
        display_name: Option<String>,
        description: Option<String>,
    ) -> Result<Organization, AppError> {
        self.org_repo
            .update(id, name, display_name, description)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn delete_org(&self, id: Uuid) -> Result<(), AppError> {
        self.org_repo
            .delete(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
