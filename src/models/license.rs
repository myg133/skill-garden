use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub id: Uuid,
    pub license_key: String,
    pub tenant_id: Uuid,
    pub plan: String,
    pub max_users: i32,
    pub max_organizations: i32,
    pub max_skills: i32,
    pub features: serde_json::Value,
    pub expires_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewLicense {
    pub license_key: String,
    pub tenant_id: Uuid,
    pub plan: Option<String>,
    pub max_users: Option<i32>,
    pub max_organizations: Option<i32>,
    pub max_skills: Option<i32>,
    pub features: Option<serde_json::Value>,
    pub expires_at: Option<DateTime<Utc>>,
}