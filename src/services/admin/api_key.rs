//! API Key Service

use crate::db::repositories::ApiKeyRepository;
use crate::models::api_key::{ApiKey, ApiKeyResponse, CreateApiKeyRequest};
use crate::models::error::AppError;
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

    pub async fn validate(&self, key: &str) -> Result<Option<ApiKey>, AppError> {
        let key_hash = self.hash_key(key);

        self.repo
            .find_by_key_hash(&key_hash)
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

    /// Used by the tenant-scope guard (Task 9) to filter the api-keys
    /// list endpoint to the caller's accessible tenants. Returns an
    /// empty Vec for an empty slice — the caller never asks "for an
    /// empty tenant set", and avoiding the repository call also avoids
    /// the `tenant_id = ANY('{}')` semantics.
    pub async fn list_by_tenants(
        &self,
        tenant_ids: &[Uuid],
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ApiKey>, AppError> {
        if tenant_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.repo
            .list_by_tenants(tenant_ids, limit, offset)
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

    fn hash_key(&self, key: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
