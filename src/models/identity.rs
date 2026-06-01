//! Identity / User model - unified abstraction for users and agents
//!
//! Maps to the `users` concept in MULTI_TENANT_ADMIN_DESIGN.md.
//! identity_type maps to user_type: human/agent/service + system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: Uuid,
    pub identity_type: IdentityType,
    pub external_id: Option<String>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub password_hash: Option<String>,
    pub is_system_admin: bool,
    pub status: IdentityStatus,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityType {
    User,
    Agent,
    ExternalAgent,
    System,
}

impl Default for IdentityType {
    fn default() -> Self {
        IdentityType::Agent
    }
}

impl From<&str> for IdentityType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "user" | "human" => IdentityType::User,
            "agent" => IdentityType::Agent,
            "external_agent" => IdentityType::ExternalAgent,
            "system" | "service" => IdentityType::System,
            _ => IdentityType::Agent,
        }
    }
}

impl std::fmt::Display for IdentityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentityType::User => write!(f, "user"),
            IdentityType::Agent => write!(f, "agent"),
            IdentityType::ExternalAgent => write!(f, "external_agent"),
            IdentityType::System => write!(f, "system"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IdentityStatus {
    Active,
    Inactive,
    Suspended,
    Deleted,
}

impl Default for IdentityStatus {
    fn default() -> Self {
        IdentityStatus::Active
    }
}

impl std::fmt::Display for IdentityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentityStatus::Active => write!(f, "active"),
            IdentityStatus::Inactive => write!(f, "inactive"),
            IdentityStatus::Suspended => write!(f, "suspended"),
            IdentityStatus::Deleted => write!(f, "deleted"),
        }
    }
}

impl From<&str> for IdentityStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "inactive" => IdentityStatus::Inactive,
            "suspended" => IdentityStatus::Suspended,
            "deleted" => IdentityStatus::Deleted,
            _ => IdentityStatus::Active,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewIdentity {
    pub identity_type: IdentityType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,
    #[serde(default)]
    pub is_system_admin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdentityUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<IdentityStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_system_admin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityWithRoles {
    #[serde(flatten)]
    pub identity: Identity,
    pub roles: Vec<String>,
    pub organizations: Vec<String>,
    pub groups: Vec<String>,
}
