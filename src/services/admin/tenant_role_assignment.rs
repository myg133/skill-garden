//! Tenant Role Assignment Service

use crate::db::repositories::TenantRoleAssignmentRepository;
use crate::models::error::AppError;
use crate::models::TenantRoleAssignment;
use uuid::Uuid;

#[derive(Clone)]
pub struct TenantRoleAssignmentService {
    repo: TenantRoleAssignmentRepository,
}

impl std::fmt::Debug for TenantRoleAssignmentService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TenantRoleAssignmentService").finish()
    }
}

impl TenantRoleAssignmentService {
    pub fn new(repo: TenantRoleAssignmentRepository) -> Self {
        Self { repo }
    }

    pub async fn assign(
        &self,
        identity_id: Uuid,
        tenant_id: Uuid,
        role_name: &str,
        assigned_by: Option<Uuid>,
    ) -> Result<TenantRoleAssignment, AppError> {
        self.repo
            .assign(identity_id, tenant_id, role_name, assigned_by)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn revoke(
        &self,
        identity_id: Uuid,
        tenant_id: Uuid,
        role_name: &str,
    ) -> Result<(), AppError> {
        self.repo
            .revoke(identity_id, tenant_id, role_name)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn find_by_identity(
        &self,
        identity_id: Uuid,
    ) -> Result<Vec<TenantRoleAssignment>, AppError> {
        self.repo
            .find_by_identity(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn find_by_identity_and_tenant(
        &self,
        identity_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<TenantRoleAssignment>, AppError> {
        self.repo
            .find_by_identity_and_tenant(identity_id, tenant_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<TenantRoleAssignment>, AppError> {
        self.repo
            .list_by_tenant(tenant_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
