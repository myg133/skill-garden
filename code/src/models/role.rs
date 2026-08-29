//! Role and Permission models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub role_type: RoleType,
    pub scope_level: ScopeLevel,
    pub parent_role_id: Option<Uuid>,
    pub permissions: Vec<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RoleType {
    System,
    Tenant,
    Organization,
    Group,
}

impl Default for RoleType {
    fn default() -> Self {
        RoleType::Organization
    }
}

impl std::fmt::Display for RoleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleType::System => write!(f, "system"),
            RoleType::Tenant => write!(f, "tenant"),
            RoleType::Organization => write!(f, "organization"),
            RoleType::Group => write!(f, "group"),
        }
    }
}

impl From<&str> for RoleType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "system" => RoleType::System,
            "tenant" => RoleType::Tenant,
            "organization" => RoleType::Organization,
            "group" => RoleType::Group,
            _ => RoleType::Organization,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScopeLevel {
    Global,
    Tenant,
    Org,
    Group,
}

impl Default for ScopeLevel {
    fn default() -> Self {
        ScopeLevel::Org
    }
}

impl std::fmt::Display for ScopeLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeLevel::Global => write!(f, "global"),
            ScopeLevel::Tenant => write!(f, "tenant"),
            ScopeLevel::Org => write!(f, "org"),
            ScopeLevel::Group => write!(f, "group"),
        }
    }
}

impl From<&str> for ScopeLevel {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "global" => ScopeLevel::Global,
            "tenant" => ScopeLevel::Tenant,
            "org" => ScopeLevel::Org,
            "group" => ScopeLevel::Group,
            _ => ScopeLevel::Org,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewRole {
    pub name: String,
    pub role_type: RoleType,
    pub scope_level: ScopeLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_role_id: Option<Uuid>,
    pub permissions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoleUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRole {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub role_id: Uuid,
    pub scope_id: Option<Uuid>,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GrantRoleRequest {
    pub identity_id: Uuid,
    pub role_id: Uuid,
    pub scope_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub resource_type: String,
    pub action: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoleWithDetails {
    #[serde(flatten)]
    pub role: Role,
    pub scope_name: Option<String>,
}
