//! Organization Tool data model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgTool {
    pub id: Uuid,
    pub tool_id: String,
    pub org_id: Uuid,
    pub name: String,
    pub description: String,
    pub schema: JsonValue,
    pub implementation: ToolImplementation,
    pub status: ToolStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolImplementation {
    pub tool_type: String,
    pub cli_path: String,
    pub docker_image: Option<String>,
    pub timeout_seconds: Option<u32>,
}

impl OrgTool {
    pub fn new(
        tool_id: String,
        org_id: Uuid,
        name: String,
        description: String,
        schema: JsonValue,
        implementation: ToolImplementation,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            tool_id,
            org_id,
            name,
            description,
            schema,
            implementation,
            status: ToolStatus::Pending,
            created_at: Utc::now(),
        }
    }
}
