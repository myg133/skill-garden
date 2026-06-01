//! Tenant model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: TenantStatus,
    pub billing_plan: Option<String>,
    pub sso_config: Option<serde_json::Value>,
    pub settings: serde_json::Value,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TenantStatus {
    Active,
    Suspended,
    Deleted,
}

impl Default for TenantStatus {
    fn default() -> Self {
        TenantStatus::Active
    }
}

impl std::fmt::Display for TenantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantStatus::Active => write!(f, "active"),
            TenantStatus::Suspended => write!(f, "suspended"),
            TenantStatus::Deleted => write!(f, "deleted"),
        }
    }
}

impl From<&str> for TenantStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "suspended" => TenantStatus::Suspended,
            "deleted" => TenantStatus::Deleted,
            _ => TenantStatus::Active,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewTenant {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub billing_plan: Option<String>,
    #[serde(default)]
    pub sso_config: Option<serde_json::Value>,
    #[serde(default)]
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TenantStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sso_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<serde_json::Value>,
}
