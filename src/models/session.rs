//! Session data model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub org_id: Uuid,
    pub status: SessionStatus,
    pub tool_router: JsonValue,
    pub capabilities: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Ended,
}

impl Session {
    pub fn new(agent_id: Uuid, org_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            agent_id,
            org_id,
            status: SessionStatus::Active,
            tool_router: serde_json::json!({}),
            capabilities: Vec::new(),
            created_at: now,
            last_active_at: now,
            ended_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRouter {
    pub routes: HashMap<String, RouteTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouteTarget {
    Local,
    Platform,
    OrgTool(String),
}

impl ToolRouter {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn add_route(&mut self, tool: String, target: RouteTarget) {
        self.routes.insert(tool, target);
    }

    pub fn route(&self, tool_name: &str) -> Option<&RouteTarget> {
        self.routes.get(tool_name)
    }
}

impl Default for ToolRouter {
    fn default() -> Self {
        Self::new()
    }
}
