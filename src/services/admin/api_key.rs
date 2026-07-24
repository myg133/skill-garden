//! API Key Service

use crate::db::repositories::ApiKeyRepository;
use crate::models::api_key::{
    ApiKey, ApiKeyListItem, ApiKeyResponse, ApiKeyStatus, CreateApiKeyRequest,
    UserCreateApiKeyRequest,
};
use crate::models::error::AppError;
use crate::models::identity::IdentityStatus;
use crate::services::admin::identity::IdentityService;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone)]
pub struct ApiKeyService {
    repo: ApiKeyRepository,
    identity: IdentityService,
}

impl std::fmt::Debug for ApiKeyService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKeyService").finish()
    }
}

impl ApiKeyService {
    pub fn new(repo: ApiKeyRepository, identity: IdentityService) -> Self {
        Self { repo, identity }
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

    /// 用户自服务创建 API Key（identity_id 从认证上下文来，org 可选且需校验归属）
    pub async fn create_user_api_key(
        &self,
        identity_id: Uuid,
        req: UserCreateApiKeyRequest,
    ) -> Result<ApiKeyResponse, AppError> {
        let request = CreateApiKeyRequest {
            identity_id,
            organization_id: req.organization_id,
            name: req.name,
            scopes: req.scopes,
            rate_limit: req.rate_limit,
            expires_at: req.expires_at,
        };
        self.create(request).await
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<ApiKey>, AppError> {
        self.repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 验证 API Key 明文，返回对应的 ApiKey 记录
    /// 如果 key 无效、已禁用、已撤销、已过期，或关联 identity 被禁用则返回 None
    pub async fn validate(&self, key: &str) -> Result<Option<ApiKey>, AppError> {
        let key_hash = self.hash_key(key);
        let now = chrono::Utc::now();

        let result = self
            .repo
            .find_by_key_hash(&key_hash)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
            .filter(|k| {
                if !matches!(k.status, crate::models::api_key::ApiKeyStatus::Active) {
                    return false;
                }
                if let Some(ref expires_at) = k.expires_at {
                    if *expires_at < now {
                        return false;
                    }
                }
                true
            });

        // 检查关联 identity 的状态
        if let Some(ref api_key) = result {
            let identity = self.identity.get(api_key.identity_id).await
                .map_err(|e| AppError::InternalError(e.to_string()))?;
            if let Some(id) = identity {
                if id.status != IdentityStatus::Active {
                    return Ok(None);
                }
            }
        }

        Ok(result)
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
            .map(|keys| {
                keys.into_iter()
                    .map(|mut k| {
                        k.status = k.effective_status();
                        k
                    })
                    .collect()
            })
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<ApiKey>, AppError> {
        self.repo
            .list_by_organization(organization_id)
            .await
            .map(|keys| {
                keys.into_iter()
                    .map(|mut k| {
                        k.status = k.effective_status();
                        k
                    })
                    .collect()
            })
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list(&self) -> Result<Vec<ApiKey>, AppError> {
        self.repo
            .list()
            .await
            .map(|keys| {
                keys.into_iter()
                    .map(|mut k| {
                        k.status = k.effective_status();
                        k
                    })
                    .collect()
            })
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_with_names(&self) -> Result<Vec<ApiKeyListItem>, AppError> {
        self.repo
            .list_with_names()
            .await
            .map(|keys| {
                keys.into_iter()
                    .map(|mut k| {
                        k.status = ApiKeyStatus::compute_effective(k.status, k.expires_at);
                        k
                    })
                    .collect()
            })
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_with_names_by_identity(
        &self,
        identity_id: Uuid,
    ) -> Result<Vec<ApiKeyListItem>, AppError> {
        self.repo
            .list_with_names_by_identity(identity_id)
            .await
            .map(|keys| {
                keys.into_iter()
                    .map(|mut k| {
                        k.status = ApiKeyStatus::compute_effective(k.status, k.expires_at);
                        k
                    })
                    .collect()
            })
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_with_names_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<ApiKeyListItem>, AppError> {
        self.repo
            .list_with_names_by_organization(organization_id)
            .await
            .map(|keys| {
                keys.into_iter()
                    .map(|mut k| {
                        k.status = ApiKeyStatus::compute_effective(k.status, k.expires_at);
                        k
                    })
                    .collect()
            })
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn revoke(&self, id: Uuid) -> Result<(), AppError> {
        self.repo
            .revoke(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn disable(&self, id: Uuid) -> Result<(), AppError> {
        self.repo
            .disable(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn enable(&self, id: Uuid) -> Result<(), AppError> {
        self.repo
            .enable(id)
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
