//! Audit repository compatibility adapter
//!
//! Wraps AuditLogRepository (new system → audit_log_entries table) with
//! the old NewAuditLog interface so that existing handler code can be
//! migrated transparently without touching 28+ call sites.
//!
//! This adapter will be removed once all handlers are migrated to use
//! AuditService::write_entry() directly.

use crate::db::error::DbResult;
use crate::db::repositories::AuditLogRepository;
use crate::models::api_key::CreateAuditLogRequest;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

// ── Re-exported old-style types for backward compat ──

/// Mirror of the old `NewAuditLog` from `src/db/repositories/audit.rs`
/// Used by the adapter to accept legacy-style audit writes.
#[derive(Debug, Clone)]
pub struct NewAuditLog {
    pub agent_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Value,
}

/// Mirror of the old `AuditLog` for backward-compatible response
#[derive(Debug, Clone)]
pub struct AuditLog {
    pub id: Uuid,
    pub agent_id: Option<String>,
    pub identity_name: Option<String>,
    pub identity_type: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// ── Compatibility Adapter ──

#[derive(Clone)]
pub struct AuditRepositoryCompat {
    new_repo: AuditLogRepository,
}

impl AuditRepositoryCompat {
    pub fn new(new_repo: AuditLogRepository) -> Self {
        Self { new_repo }
    }

    /// Accept old-style NewAuditLog and write to new audit_log_entries table.
    ///
    /// Mapping rules:
    /// - `agent_id` → stored in `details._legacy_agent_id` if not a UUID
    /// - `agent_id` → `identity_id` if it parses as a UUID
    /// - Otherwise: fallback to a zero UUID ("00000000-0000-0000-0000-000000000000")
    ///   and store agent_id in details
    /// - `resource_id` (String) → `resource_id` (UUID) if parseable, else stored in `details._resource_id_str`
    pub async fn create(&self, new_log: NewAuditLog) -> DbResult<AuditLog> {
        let mut details = new_log.details;

        // Try to parse agent_id as identity_id UUID
        let identity_id: Uuid = new_log
            .agent_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(|| {
                // Not a UUID — store agent_id in details for traceability
                if let Some(ref agent_id) = new_log.agent_id {
                    if let Some(ref mut map) = details.as_object_mut() {
                        map.insert(
                            "_legacy_agent_id".to_string(),
                            Value::String(agent_id.clone()),
                        );
                    } else {
                        details = serde_json::json!({ "_legacy_agent_id": agent_id });
                    }
                }
                // Use nil UUID as placeholder for unmapped legacy entries
                Uuid::nil()
            });

        // Try to parse resource_id as UUID
        let resource_id: Option<Uuid> = new_log
            .resource_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok());

        // If resource_id was a string but not a UUID, store it in details
        if resource_id.is_none() {
            if let Some(ref rid) = new_log.resource_id {
                if let Some(ref mut map) = details.as_object_mut() {
                    map.insert("_resource_id_str".to_string(), Value::String(rid.clone()));
                } else {
                    details = serde_json::json!({ "_resource_id_str": rid });
                }
            }
        }

        let request = CreateAuditLogRequest {
            tenant_id: None,
            organization_id: None,
            identity_id,
            action: new_log.action,
            resource_type: Some(new_log.resource_type),
            resource_id,
            details: Some(details),
            ip_address: None,
            user_agent: None,
        };

        let entry = self.new_repo.create(request).await?;

        // Convert back to old-style AuditLog for API compat
        Ok(AuditLog {
            id: entry.id,
            agent_id: new_log.agent_id,
            identity_name: entry.identity_name,
            identity_type: entry.identity_type,
            action: entry.action,
            resource_type: entry.resource_type.unwrap_or_default(),
            resource_id: new_log.resource_id,
            details: entry.details.unwrap_or(serde_json::json!({})),
            ip_address: entry.ip_address,
            user_agent: entry.user_agent,
            timestamp: entry.created_at,
        })
    }

    /// Query with old-style filters (uses agent_id stored in details._legacy_agent_id)
    pub async fn list_with_filters(
        &self,
        agent_id: Option<&str>,
        action: Option<&str>,
        resource_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<AuditLog>> {
        // For backward compat: query new table with filters
        let entries = self
            .new_repo
            .query(
                None, // tenant_id
                None, // organization_id
                None, // identity_id — can't filter by identity_id in old API
                action,
                resource_type,
                limit,
                offset,
            )
            .await?;

        let logs: Vec<AuditLog> = entries
            .into_iter()
            .map(|entry| {
                let agent_id = entry
                    .details
                    .as_ref()
                    .and_then(|d| d.get("_legacy_agent_id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                AuditLog {
                    id: entry.id,
                    agent_id,
                    identity_name: entry.identity_name,
                    identity_type: entry.identity_type,
                    action: entry.action,
                    resource_type: entry.resource_type.unwrap_or_default(),
                    resource_id: entry.resource_id.map(|id| id.to_string()).or_else(|| {
                        entry
                            .details
                            .as_ref()
                            .and_then(|d| d.get("_resource_id_str"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    }),
                    details: entry.details.unwrap_or(serde_json::json!({})),
                    ip_address: entry.ip_address,
                    user_agent: entry.user_agent,
                    timestamp: entry.created_at,
                }
            })
            .collect();

        // Post-filter by agent_id if specified
        let result: Vec<AuditLog> = if let Some(filter_agent_id) = agent_id {
            logs.into_iter()
                .filter(|log| {
                    log.agent_id
                        .as_deref()
                        .map(|a| a == filter_agent_id)
                        .unwrap_or(false)
                })
                .collect()
        } else {
            logs
        };

        Ok(result)
    }

    /// Count with old-style filters
    pub async fn count_with_filters(
        &self,
        agent_id: Option<&str>,
        action: Option<&str>,
        resource_type: Option<&str>,
    ) -> DbResult<i64> {
        // Count from new table
        let total = self
            .new_repo
            .count(None, None, None, action, resource_type)
            .await?;

        // If agent_id filter, we need to query all and count (inefficient but backward compat)
        if agent_id.is_some() {
            let logs = self
                .list_with_filters(agent_id, action, resource_type, i64::MAX, 0)
                .await?;
            return Ok(logs.len() as i64);
        }

        Ok(total)
    }

    /// List by agent (legacy API)
    pub async fn list_by_agent(&self, agent_id: &str, limit: i64) -> DbResult<Vec<AuditLog>> {
        self.list_with_filters(Some(agent_id), None, None, limit, 0)
            .await
    }
}
