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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrgRole {
    Owner,
    Admin,
    Reviewer,
    Developer,
    Member,
}

/// 手动实现 PartialOrd，确保权限比较：Owner > Admin > Reviewer > Developer > Member
/// 注意：`#[derive(PartialOrd)]` 按声明顺序从上到下递增（Owner=0 < Member=4），
/// 这与权限层级相反，因此必须手动实现。
impl PartialOrd for OrgRole {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrgRole {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        fn rank(role: &OrgRole) -> u8 {
            match role {
                OrgRole::Member => 0,
                OrgRole::Developer => 1,
                OrgRole::Reviewer => 2,
                OrgRole::Admin => 3,
                OrgRole::Owner => 4,
            }
        }
        rank(self).cmp(&rank(other))
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<GroupInfo>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupInfo {
    pub id: Uuid,
    pub name: String,
    pub role: String,
}
