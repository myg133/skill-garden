use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePermission {
    pub id: Uuid,
    pub role_level: String,
    pub role_name: String,
    pub permission_code: String,
    pub scope_restriction: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewRolePermission {
    pub role_level: String,
    pub role_name: String,
    pub permission_code: String,
    pub scope_restriction: Option<String>,
}
