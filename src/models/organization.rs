//! Organization data model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub tenant_name: Option<String>,
    pub org_type: Option<String>,
    pub visibility: Option<String>,
    pub avatar_url: Option<String>,
    pub status: Option<String>,
    pub join_policy: Option<String>,
    pub settings: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewOrganization {
    pub name: String,
    pub slug: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub org_type: Option<String>,
    pub visibility: Option<String>,
    pub avatar_url: Option<String>,
    pub settings: Option<JsonValue>,
}

impl Organization {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            slug: None,
            display_name: None,
            description: None,
            tenant_id: None,
            tenant_name: None,
            org_type: None,
            visibility: Some("public".to_string()),
            avatar_url: None,
            status: Some("active".to_string()),
            join_policy: Some("approval_required".to_string()),
            settings: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: None,
        }
    }
}
