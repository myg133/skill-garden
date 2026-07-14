//! Organization Membership model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMembership {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub organization_id: Uuid,
    pub role: OrgRole,
    pub joined_at: DateTime<Utc>,
    pub invited_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum OrgRole {
    Owner,
    Admin,
    Reviewer,
    Developer,
    Member,
}

impl Default for OrgRole {
    fn default() -> Self {
        OrgRole::Member
    }
}

impl std::fmt::Display for OrgRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrgRole::Owner => write!(f, "owner"),
            OrgRole::Admin => write!(f, "admin"),
            OrgRole::Reviewer => write!(f, "reviewer"),
            OrgRole::Developer => write!(f, "developer"),
            OrgRole::Member => write!(f, "member"),
        }
    }
}

impl From<&str> for OrgRole {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "owner" => OrgRole::Owner,
            "admin" => OrgRole::Admin,
            "reviewer" => OrgRole::Reviewer,
            "developer" => OrgRole::Developer,
            _ => OrgRole::Member,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewOrgMembership {
    pub identity_id: Uuid,
    pub organization_id: Uuid,
    #[serde(default)]
    pub role: OrgRole,
    #[serde(default)]
    pub invited_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrgMembershipUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<OrgRole>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrgMemberInfo {
    pub identity_id: Uuid,
    pub username: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub name: String,
    pub identity_type: String,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}
