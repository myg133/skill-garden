//! Organization data model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub settings: JsonValue,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewOrganization {
    pub name: String,
    pub settings: Option<JsonValue>,
}

impl Organization {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            settings: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }
}
