//! API Key Service

use crate::db::repositories::ApiKeyRepository;
use crate::models::api_key::{ApiKey, ApiKeyResponse, CreateApiKeyRequest};
use crate::models::error::AppError;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone)]
pub struct ApiKeyService {
    repo: ApiKeyRepository,
}

impl std::fmt::Debug for ApiKeyService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKeyService").finish()
    }
}

impl ApiKeyService {
    pub fn new(repo: ApiKeyRepository) -> Self {
        Self { repo }
    }

    pub async fn create(&self, request: CreateApiKeyRequest) -> Result<ApiKeyResponse, AppError> {
        let (key, key_hash, key_prefix) = self.generate_key();

        let api_key = self
            .repo
            .create(request, &key_hash, &key_prefix)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(ApiKeyResponse::from_api_key(api_key, key))
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<ApiKey>, AppError> {
        self.repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 验证 API Key 明文，返回对应的 ApiKey 记录
    /// 如果 key 无效或已撤销则返回 None
    pub async fn validate(&self, key: &str) -> Result<Option<ApiKey>, AppError> {
        let key_hash = self.hash_key(key);

        self.repo
            .find_by_key_hash(&key_hash)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
            .map(|opt| {
                opt.filter(|k| {
                    // 只有 active 状态的 key 才有效，expired 由 expires_at 自动判断
                    matches!(k.status, crate::models::api_key::ApiKeyStatus::Active)
                })
            })
    }

    /// 更新 API Key 的最后使用时间
    pub async fn mark_used(&self, id: Uuid) -> Result<(), AppError> {
        self.repo
            .update_last_used(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_by_identity(&self, identity_id: Uuid) -> Result<Vec<ApiKey>, AppError> {
        self.repo
            .list_by_identity(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<ApiKey>, AppError> {
        self.repo
            .list_by_organization(organization_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list(&self) -> Result<Vec<ApiKey>, AppError> {
        self.repo
            .list()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn revoke(&self, id: Uuid) -> Result<(), AppError> {
        self.repo
            .revoke(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        self.repo
            .delete(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    fn generate_key(&self) -> (String, String, String) {
        let key = format!("sk_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let key_prefix = key.chars().take(12).collect();
        let key_hash = self.hash_key(&key);
        (key, key_hash, key_prefix)
    }

    /// 使用 SHA-256 + salt 对 API Key 进行安全哈希
    fn hash_key(&self, key: &str) -> String {
        let salt = std::env::var("API_KEY_SALT")
            .unwrap_or_else(|_| "aion_hive_api_key_default_salt".to_string());
        let mut hasher = Sha256::new();
        hasher.update(salt.as_bytes());
        hasher.update(key.as_bytes());
        hex::encode(hasher.finalize())
    }
}
