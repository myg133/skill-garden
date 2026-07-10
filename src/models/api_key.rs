//! API Key model for external agent access

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub organization_id: Uuid,
    pub key_hash: String,
    pub key_prefix: String,
    pub name: Option<String>,
    pub scopes: Vec<String>,
    pub rate_limit: i32,
    pub status: ApiKeyStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyStatus {
    Active,
    Expired,
    Revoked,
}

impl Default for ApiKeyStatus {
    fn default() -> Self {
        ApiKeyStatus::Active
    }
}

impl From<&str> for ApiKeyStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "expired" => ApiKeyStatus::Expired,
            "revoked" => ApiKeyStatus::Revoked,
            _ => ApiKeyStatus::Active,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub identity_id: Uuid,
    pub organization_id: Uuid,
    pub name: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default = "default_rate_limit")]
    pub rate_limit: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

fn default_rate_limit() -> i32 {
    1000
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub organization_id: Uuid,
    pub key: String,
    pub key_prefix: String,
    pub name: Option<String>,
    pub scopes: Vec<String>,
    pub rate_limit: i32,
    pub status: ApiKeyStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ApiKeyResponse {
    pub fn from_api_key(key: ApiKey, raw_key: String) -> Self {
        Self {
            id: key.id,
            identity_id: key.identity_id,
            organization_id: key.organization_id,
            key: raw_key,
            key_prefix: key.key_prefix,
            name: key.name,
            scopes: key.scopes,
            rate_limit: key.rate_limit,
            status: key.status,
            expires_at: key.expires_at,
            created_at: key.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub identity_id: Uuid,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub identity_name: Option<String>,
    pub identity_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditLogQuery {
    #[serde(default)]
    pub tenant_id: Option<Uuid>,
    #[serde(default)]
    pub organization_id: Option<Uuid>,
    #[serde(default)]
    pub identity_id: Option<Uuid>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub resource_type: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAuditLogRequest {
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub identity_id: Uuid,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}
