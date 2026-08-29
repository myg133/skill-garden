//! Identity Service

use crate::db::repositories::IdentityRepository;
use crate::models::error::AppError;
use crate::models::identity::{Identity, IdentityUpdate, NewIdentity};
use uuid::Uuid;

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
        self.repo
            .create(new_identity)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<Identity>, AppError> {
        self.repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_by_external_id(
        &self,
        external_id: &str,
    ) -> Result<Option<Identity>, AppError> {
        self.repo
            .find_by_external_id(external_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
        identity_type: Option<&str>,
    ) -> Result<Vec<Identity>, AppError> {
        self.repo
            .list_all(limit, offset, identity_type)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn update(&self, id: Uuid, update: IdentityUpdate) -> Result<Identity, AppError> {
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

    pub async fn exists(&self, id: Uuid) -> Result<bool, AppError> {
        self.repo
            .exists(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_by_username(&self, username: &str) -> Result<Option<Identity>, AppError> {
        self.repo
            .find_by_username(username)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_by_email(&self, email: &str) -> Result<Option<Identity>, AppError> {
        self.repo
            .find_by_email(email)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn verify_password(&self, username: &str, password: &str) -> Result<bool, AppError> {
        let identity = self
            .repo
            .find_by_username(username)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        match identity {
            Some(id) => match &id.password_hash {
                Some(hash) => Ok(bcrypt::verify(password, hash).unwrap_or(false)),
                None => Ok(false),
            },
            None => Ok(false),
        }
    }

    /// 验证密码并返回用户信息（避免一次查询 identity 表后 login handler 再次查询）
    pub async fn verify_password_and_get_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<crate::models::identity::Identity>, AppError> {
        let identity = self
            .repo
            .find_by_username(username)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        match identity {
            Some(user) => match &user.password_hash {
                Some(hash) => {
                    if bcrypt::verify(password, hash).unwrap_or(false) {
                        Ok(Some(user))
                    } else {
                        Ok(None)
                    }
                }
                None => Ok(None),
            },
            None => Ok(None),
        }
    }
}
