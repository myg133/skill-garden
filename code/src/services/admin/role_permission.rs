//! Role Permission Service

use crate::db::repositories::RolePermissionRepository;
use crate::models::error::AppError;
use crate::models::role_permission::{NewRolePermission, RolePermission};

#[derive(Clone)]
pub struct RolePermissionService {
    repo: RolePermissionRepository,
}

impl std::fmt::Debug for RolePermissionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RolePermissionService").finish()
    }
}

impl RolePermissionService {
    pub fn new(repo: RolePermissionRepository) -> Self {
        Self { repo }
    }

    pub async fn list_all(&self) -> Result<Vec<RolePermission>, AppError> {
        self.repo
            .list_all()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_by_role(
        &self,
        role_level: &str,
        role_name: &str,
    ) -> Result<Vec<RolePermission>, AppError> {
        self.repo
            .list_by_role(role_level, role_name)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn add_permission(
        &self,
        new_perm: NewRolePermission,
    ) -> Result<RolePermission, AppError> {
        self.repo
            .add_permission(new_perm)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn remove_permission(
        &self,
        role_level: &str,
        role_name: &str,
        permission_code: &str,
    ) -> Result<(), AppError> {
        self.repo
            .remove_permission(role_level, role_name, permission_code)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
