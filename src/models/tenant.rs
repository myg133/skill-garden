//! Tenant model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a tenant creation request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RequestStatus {
    Pending,
    Approved,
    Rejected,
}

impl Default for RequestStatus {
    fn default() -> Self {
        RequestStatus::Pending
    }
}

impl std::fmt::Display for RequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestStatus::Pending => write!(f, "pending"),
            RequestStatus::Approved => write!(f, "approved"),
            RequestStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl From<&str> for RequestStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "approved" => RequestStatus::Approved,
            "rejected" => RequestStatus::Rejected,
            _ => RequestStatus::Pending,
        }
    }
}

/// Tenant creation request (self-service workflow)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantCreationRequest {
    pub id: Uuid,
    pub applicant_id: Uuid,
    pub applicant_name: String,
    pub applicant_email: String,
    pub tenant_name: String,
    pub tenant_slug: String,
    pub message: Option<String>,
    pub status: RequestStatus,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_note: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// New tenant creation request input
#[derive(Debug, Clone, Deserialize)]
pub struct NewTenantCreationRequest {
    pub tenant_name: String,
    pub message: Option<String>,
}

/// Tenant creation request review input
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewTenantCreationRequest {
    pub action: String, // "approve" or "reject"
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: TenantStatus,
    pub billing_plan: Option<String>,
    pub sso_config: Option<serde_json::Value>,
    pub settings: serde_json::Value,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TenantStatus {
    Active,
    Suspended,
    Deleted,
}

impl Default for TenantStatus {
    fn default() -> Self {
        TenantStatus::Active
    }
}

impl std::fmt::Display for TenantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantStatus::Active => write!(f, "active"),
            TenantStatus::Suspended => write!(f, "suspended"),
            TenantStatus::Deleted => write!(f, "deleted"),
        }
    }
}

impl From<&str> for TenantStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "suspended" => TenantStatus::Suspended,
            "deleted" => TenantStatus::Deleted,
            _ => TenantStatus::Active,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewTenant {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub billing_plan: Option<String>,
    #[serde(default)]
    pub sso_config: Option<serde_json::Value>,
    #[serde(default)]
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TenantStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sso_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<serde_json::Value>,
}

/// Include applicant info in request response
#[derive(Debug, Clone, Serialize)]
pub struct TenantCreationRequestWithApplicant {
    pub id: Uuid,
    pub applicant_id: Uuid,
    pub applicant_name: String,
    pub applicant_email: String,
    pub tenant_name: String,
    pub tenant_slug: String,
    pub message: Option<String>,
    pub status: RequestStatus,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_note: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
