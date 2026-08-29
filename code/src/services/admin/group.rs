//! Group Service

use crate::db::repositories::GroupRepository;
use crate::models::error::AppError;
use crate::models::group::{Group, GroupMember, GroupUpdate, Membership, NewGroup};
use uuid::Uuid;

#[derive(Clone)]
pub struct GroupService {
    repo: GroupRepository,
}

impl std::fmt::Debug for GroupService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupService").finish()
    }
}

impl GroupService {
    pub fn new(repo: GroupRepository) -> Self {
        Self { repo }
    }

    pub async fn create(&self, new_group: NewGroup) -> Result<Group, AppError> {
        self.repo
            .create(new_group)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<Group>, AppError> {
        self.repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<Group>, AppError> {
        self.repo
            .list_by_organization(organization_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list(&self) -> Result<Vec<Group>, AppError> {
        self.repo
            .list()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn update(&self, id: Uuid, update: GroupUpdate) -> Result<Group, AppError> {
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

    pub async fn add_member(
        &self,
        identity_id: Uuid,
        group_id: Uuid,
        role: String,
    ) -> Result<Membership, AppError> {
        self.repo
            .add_member(identity_id, group_id, &role)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn remove_member(&self, identity_id: Uuid, group_id: Uuid) -> Result<(), AppError> {
        self.repo
            .remove_member(identity_id, group_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_members(&self, group_id: Uuid) -> Result<Vec<GroupMember>, AppError> {
        self.repo
            .list_members(group_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_identity_groups(&self, identity_id: Uuid) -> Result<Vec<Group>, AppError> {
        self.repo
            .get_identity_groups(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
