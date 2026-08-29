//! Group model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub group_type: GroupType,
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupType {
    Team,
    Project,
    Department,
}

impl Default for GroupType {
    fn default() -> Self {
        GroupType::Team
    }
}

impl From<&str> for GroupType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "project" => GroupType::Project,
            "department" => GroupType::Department,
            _ => GroupType::Team,
        }
    }
}

impl std::fmt::Display for GroupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupType::Team => write!(f, "team"),
            GroupType::Project => write!(f, "project"),
            GroupType::Department => write!(f, "department"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewGroup {
    pub organization_id: Uuid,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub group_type: GroupType,
    #[serde(default)]
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_type: Option<GroupType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Membership {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub group_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddMemberRequest {
    pub identity_id: Uuid,
    #[serde(default = "default_member_role")]
    pub role: String,
}

fn default_member_role() -> String {
    "member".to_string()
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupMember {
    pub identity_id: Uuid,
    pub identity_name: String,
    pub identity_type: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}
