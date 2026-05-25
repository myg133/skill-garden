//! Skill Policy data model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPolicy {
    pub id: Uuid,
    pub org_id: Uuid,
    pub skill_id: Uuid,
    pub visibility: Visibility,
    pub allowed_agents: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Visibility {
    Private,
    OrgVisible,
    Marketplace,
    Shared,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::OrgVisible
    }
}

impl SkillPolicy {
    pub fn new(org_id: Uuid, skill_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            org_id,
            skill_id,
            visibility: Visibility::OrgVisible,
            allowed_agents: Vec::new(),
            created_at: Utc::now(),
        }
    }
}
