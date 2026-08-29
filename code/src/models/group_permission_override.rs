use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupPermissionOverride {
    pub id: Uuid,
    pub group_id: Uuid,
    pub role_name: String,
    pub permission_code: String,
    pub granted: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewGroupPermissionOverride {
    pub group_id: Uuid,
    pub role_name: String,
    pub permission_code: String,
    pub granted: bool,
    pub created_by: Option<Uuid>,
}
