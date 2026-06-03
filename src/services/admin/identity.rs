//! Identity Service

use uuid::Uuid;
use crate::db::repositories::IdentityRepository;
use crate::models::identity::{Identity, NewIdentity, IdentityUpdate};
use crate::models::error::AppError;

#[derive(Clone)]
pub struct IdentityService {
    repo: IdentityRepository,
}

impl std::fmt::Debug for IdentityService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityService").finish()
    }
}

impl IdentityService {
    pub fn new(repo: IdentityRepository) -> Self {
        Self { repo }
    }

    pub async fn create(&self, new_identity: NewIdentity) -> Result<Identity, AppError> {
        self.repo.create(new_identity)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<Identity>, AppError> {
        self.repo.find_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_by_external_id(&self, external_id: &str) -> Result<Option<Identity>, AppError> {
        self.repo.find_by_external_id(external_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list(&self, limit: i64, offset: i64, identity_type: Option<&str>) -> Result<Vec<Identity>, AppError> {
        self.repo.list_all(limit, offset, identity_type)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// Return all identities that are members of at least one organization
    /// whose tenant_id is in `tenant_ids`. Used by the tenant-scope guard
    /// (Task 7) to filter the identities list endpoint to the caller's
    /// accessible tenants. Returns an empty Vec for an empty slice — the
    /// caller never asks "for an empty tenant set", and avoiding the
    /// repository call also avoids the `tenant_id = ANY('{}')` semantics.
    pub async fn list_by_tenants(
        &self,
        tenant_ids: &[Uuid],
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Identity>, AppError> {
        if tenant_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.repo
            .list_by_tenants(tenant_ids, limit, offset)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn update(&self, id: Uuid, update: IdentityUpdate) -> Result<Identity, AppError> {
        self.repo.update(id, update)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        self.repo.delete(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn exists(&self, id: Uuid) -> Result<bool, AppError> {
        self.repo.exists(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_by_username(&self, username: &str) -> Result<Option<Identity>, AppError> {
        self.repo.find_by_username(username)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_by_email(&self, email: &str) -> Result<Option<Identity>, AppError> {
        self.repo.find_by_email(email)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn verify_password(&self, username: &str, password: &str) -> Result<bool, AppError> {
        let identity = self.repo.find_by_username(username)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        match identity {
            Some(id) => {
                match &id.password_hash {
                    Some(hash) => Ok(bcrypt::verify(password, hash).unwrap_or(false)),
                    None => Ok(false),
                }
            }
            None => Ok(false),
        }
    }
}
