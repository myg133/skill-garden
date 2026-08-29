//! Role Service

use crate::db::repositories::RoleRepository;
use crate::models::error::AppError;
use crate::models::role::{GrantRoleRequest, IdentityRole, NewRole, Role, RoleType, RoleUpdate};
use uuid::Uuid;

#[derive(Clone)]
pub struct RoleService {
    repo: RoleRepository,
}

impl std::fmt::Debug for RoleService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoleService").finish()
    }
}

impl RoleService {
    pub fn new(repo: RoleRepository) -> Self {
        Self { repo }
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<Role>, AppError> {
        self.repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_by_name(
        &self,
        name: &str,
        role_type: RoleType,
    ) -> Result<Option<Role>, AppError> {
        self.repo
            .find_by_name(name, role_type)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list(&self) -> Result<Vec<Role>, AppError> {
        self.repo
            .list_all()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_by_type(&self, role_type: RoleType) -> Result<Vec<Role>, AppError> {
        self.repo
            .list_by_type(role_type)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn create(&self, new_role: NewRole) -> Result<Role, AppError> {
        self.repo
            .create(new_role)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn update(&self, id: Uuid, update: RoleUpdate) -> Result<Role, AppError> {
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

    pub async fn grant_role(
        &self,
        request: GrantRoleRequest,
        granted_by: Uuid,
    ) -> Result<IdentityRole, AppError> {
        self.repo
            .grant_role(request, granted_by)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn revoke_role(
        &self,
        identity_id: Uuid,
        role_id: Uuid,
        scope_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        self.repo
            .revoke_role(identity_id, role_id, scope_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_identity_roles(
        &self,
        identity_id: Uuid,
    ) -> Result<Vec<IdentityRole>, AppError> {
        self.repo
            .get_identity_roles(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_identity_permissions(
        &self,
        identity_id: Uuid,
    ) -> Result<Vec<String>, AppError> {
        self.repo
            .get_identity_permissions(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn has_permission(
        &self,
        identity_id: Uuid,
        permission: &str,
    ) -> Result<bool, AppError> {
        self.repo
            .has_permission(identity_id, permission)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
