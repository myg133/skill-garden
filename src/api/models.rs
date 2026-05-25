//! API Request/Response Models

use serde::{Deserialize, Serialize};
use crate::models::{SkillStats};

#[derive(Debug, Serialize)]
pub struct ListResponse<T> {
    pub data: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

impl<T> ListResponse<T> {
    pub fn new(data: Vec<T>, total: usize, page: usize, page_size: usize) -> Self {
        Self {
            data,
            total,
            page,
            page_size,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub skills_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct ListSkillsQuery {
    pub tag: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSkillBody {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub content: String,
    pub version: Option<String>,
    #[serde(default)]
    pub git_url: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSkillBody {
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub content: Option<String>,
    #[serde(default)]
    pub git_url: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEvaluationBody {
    pub skill_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error_type: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SkillCreatedResponse {
    pub message: String,
    pub skill_id: String,
}

#[derive(Debug, Serialize)]
pub struct EvaluationCreatedResponse {
    pub message: String,
    pub evaluation_id: String,
    pub new_stats: SkillStats,
}

#[derive(Debug, Deserialize)]
pub struct RegisterAgentBody {
    pub agent_id: String,
    pub agent_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterAgentResponse {
    pub agent_id: String,
    pub secret: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GetTokenBody {
    pub agent_id: String,
    pub agent_secret: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub agent_id: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub id: String,
    pub agent_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: serde_json::Value,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct AuditLogListResponse {
    pub data: Vec<AuditLogResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct RejectSkillBody {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SkillReviewResponse {
    pub message: String,
    pub skill_id: String,
}

// v0.4 multi-tenant API models

use uuid::Uuid;

/// Organization models

#[derive(Debug, Deserialize)]
pub struct CreateOrgBody {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrgBody {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ListOrgsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Session models

#[derive(Debug, Deserialize)]
pub struct CreateSessionBody {
    pub agent_id: String,
    pub org_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SessionDeclareBody {
    pub capabilities: Vec<String>,
}

/// Org Tool models

#[derive(Debug, Deserialize)]
pub struct RegisterOrgToolBody {
    pub org_id: Uuid,
    pub tool_id: String,
    pub name: String,
    pub description: String,
    pub schema: Option<serde_json::Value>,
    pub implementation: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListOrgToolsQuery {
    pub approved_only: Option<bool>,
}

/// Admin user login models

#[derive(Debug, Deserialize)]
pub struct AdminLoginBody {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AdminLoginResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: AdminUserInfo,
}

#[derive(Debug, Serialize)]
pub struct AdminUserInfo {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
}
