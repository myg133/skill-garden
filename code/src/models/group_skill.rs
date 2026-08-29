use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSkill {
    pub id: Uuid,
    pub group_id: Uuid,
    pub skill_id: String,
    pub added_by: Option<Uuid>,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewGroupSkill {
    pub group_id: Uuid,
    pub skill_id: String,
    pub added_by: Option<Uuid>,
}
