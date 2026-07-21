//! API Key model for external agent access

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub organization_id: Option<Uuid>,
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

impl ApiKey {
    /// 计算有效状态：如果 DB 中为 Active 但已过期，返回 Expired
    pub fn effective_status(&self) -> ApiKeyStatus {
        ApiKeyStatus::compute_effective(self.status, self.expires_at)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyStatus {
    Active,
    Disabled,
    Expired,
    Revoked,
}

impl ApiKeyStatus {
    /// 根据 DB 状态和 expires_at 计算有效的展示状态
    /// 优先级：Revoked > Disabled > Expired > Active
    pub fn compute_effective(db_status: ApiKeyStatus, expires_at: Option<DateTime<Utc>>) -> ApiKeyStatus {
        // Revoked / Disabled 等显式管理状态原样保留
        if db_status == ApiKeyStatus::Revoked {
            return ApiKeyStatus::Revoked;
        }
        if db_status == ApiKeyStatus::Disabled {
            return ApiKeyStatus::Disabled;
        }
        if let Some(ref expires_at) = expires_at {
            if *expires_at < chrono::Utc::now() {
                return ApiKeyStatus::Expired;
            }
        }
        ApiKeyStatus::Active
    }
}

impl Default for ApiKeyStatus {
    fn default() -> Self {
        ApiKeyStatus::Active
    }
}

impl From<&str> for ApiKeyStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "disabled" => ApiKeyStatus::Disabled,
            "expired" => ApiKeyStatus::Expired,
            "revoked" => ApiKeyStatus::Revoked,
            _ => ApiKeyStatus::Active,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub identity_id: Uuid,
    pub organization_id: Option<Uuid>,
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

/// 用户自服务创建 API Key 的请求（区别于管理员创建，identity_id 从认证上下文获取）
#[derive(Debug, Clone, Deserialize)]
pub struct UserCreateApiKeyRequest {
    /// 组织 ID，可为空（个人用户不选组织时）
    pub organization_id: Option<Uuid>,
    pub name: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default = "default_rate_limit")]
    pub rate_limit: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub organization_id: Option<Uuid>,
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

/// 列表项，附带 identity / organization 的显示名称，避免前端二次查询。
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyListItem {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub identity_name: Option<String>,
    pub organization_id: Option<Uuid>,
    pub organization_name: Option<String>,
    pub key_prefix: String,
    pub name: Option<String>,
    pub scopes: Vec<String>,
    pub rate_limit: i32,
    pub status: ApiKeyStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
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
