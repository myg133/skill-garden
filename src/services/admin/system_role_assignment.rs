//! System Role Assignment Service

use crate::db::repositories::SystemRoleAssignmentRepository;
use crate::models::error::AppError;
use crate::models::SystemRoleAssignment;
use uuid::Uuid;

#[derive(Clone)]
pub struct SystemRoleAssignmentService {
    repo: SystemRoleAssignmentRepository,
}

impl std::fmt::Debug for SystemRoleAssignmentService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemRoleAssignmentService").finish()
    }
}

impl SystemRoleAssignmentService {
    pub fn new(repo: SystemRoleAssignmentRepository) -> Self {
        Self { repo }
    }

    pub async fn assign(
        &self,
        identity_id: Uuid,
        role_name: &str,
        assigned_by: Option<Uuid>,
    ) -> Result<SystemRoleAssignment, AppError> {
        self.repo
            .assign(identity_id, role_name, assigned_by)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn revoke(&self, identity_id: Uuid, role_name: &str) -> Result<(), AppError> {
        self.repo
            .revoke(identity_id, role_name)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn find_by_identity(
        &self,
        identity_id: Uuid,
    ) -> Result<Vec<SystemRoleAssignment>, AppError> {
        self.repo
            .find_by_identity(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn has_system_role(
        &self,
        identity_id: Uuid,
        role_name: &str,
    ) -> Result<bool, AppError> {
        self.repo
            .has_system_role(identity_id, role_name)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_by_role(
        &self,
        role_name: &str,
    ) -> Result<Vec<SystemRoleAssignment>, AppError> {
        self.repo
            .list_by_role(role_name)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
