use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRoleAssignment {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub role_name: String,
    pub assigned_by: Option<Uuid>,
    pub assigned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSystemRoleAssignment {
    pub identity_id: Uuid,
    pub role_name: String,
    pub assigned_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRole {
    pub role_name: String,
}

impl SystemRole {
    pub const SUPER_ADMIN: &'static str = "super_admin";
    pub const MARKETPLACE_ADMIN: &'static str = "marketplace_admin";
    pub const MARKETPLACE_REVIEWER: &'static str = "marketplace_reviewer";

    /// 超管可分配的系统角色（顶级角色）
    pub fn is_valid_super_admin_role(role_name: &str) -> bool {
        matches!(role_name, Self::SUPER_ADMIN | Self::MARKETPLACE_ADMIN)
    }

    /// 所有有效的系统角色
    pub fn is_valid(role_name: &str) -> bool {
        matches!(
            role_name,
            Self::SUPER_ADMIN | Self::MARKETPLACE_ADMIN | Self::MARKETPLACE_REVIEWER
        )
    }
}
