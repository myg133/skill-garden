//! Audit log repository

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone)]
pub struct AuditLog {
    pub id: Uuid,
    pub agent_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Value,
    pub timestamp: DateTime<Utc>,
}

pub struct NewAuditLog {
    pub agent_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Value,
}

#[derive(Clone)]
pub struct AuditRepository {
    pool: PgPool,
}

impl AuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_log: NewAuditLog) -> DbResult<AuditLog> {
        let id = Uuid::new_v4();

        let row = sqlx::query_as::<_, AuditLogRow>(
            r#"
            INSERT INTO audit_logs (id, agent_id, action, resource_type, resource_id, details)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, agent_id, action, resource_type, resource_id, details, timestamp
            "#,
        )
        .bind(id)
        .bind(&new_log.agent_id)
        .bind(&new_log.action)
        .bind(&new_log.resource_type)
        .bind(&new_log.resource_id)
        .bind(&new_log.details)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.into())
    }

    pub async fn list_by_agent(&self, agent_id: &str, limit: i64) -> DbResult<Vec<AuditLog>> {
        let rows = sqlx::query_as::<_, AuditLogRow>(
            r#"
            SELECT id, agent_id, action, resource_type, resource_id, details, timestamp
            FROM audit_logs
            WHERE agent_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn list_by_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
        limit: i64,
    ) -> DbResult<Vec<AuditLog>> {
        let rows = sqlx::query_as::<_, AuditLogRow>(
            r#"
            SELECT id, agent_id, action, resource_type, resource_id, details, timestamp
            FROM audit_logs
            WHERE resource_type = $1 AND resource_id = $2
            ORDER BY timestamp DESC
            LIMIT $3
            "#,
        )
        .bind(resource_type)
        .bind(resource_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn list_with_filters(
        &self,
        agent_id: Option<&str>,
        action: Option<&str>,
        resource_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<AuditLog>> {
        let mut query = "SELECT id, agent_id, action, resource_type, resource_id, details, timestamp FROM audit_logs WHERE 1=1".to_string();
        let mut param_count = 0;

        if agent_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND agent_id = ${}", param_count));
        }
        if action.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND action = ${}", param_count));
        }
        if resource_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND resource_type = ${}", param_count));
        }

        param_count += 1;
        query.push_str(&format!(" ORDER BY timestamp DESC LIMIT ${}", param_count));
        param_count += 1;
        query.push_str(&format!(" OFFSET ${}", param_count));

        let mut q = sqlx::query_as::<_, AuditLogRow>(&query);

        if let Some(aid) = agent_id {
            q = q.bind(aid);
        }
        if let Some(act) = action {
            q = q.bind(act);
        }
        if let Some(rt) = resource_type {
            q = q.bind(rt);
        }
        q = q.bind(limit).bind(offset);

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn count_with_filters(
        &self,
        agent_id: Option<&str>,
        action: Option<&str>,
        resource_type: Option<&str>,
    ) -> DbResult<i64> {
        let mut query = "SELECT COUNT(*) FROM audit_logs WHERE 1=1".to_string();
        let mut param_count = 0;

        if agent_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND agent_id = ${}", param_count));
        }
        if action.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND action = ${}", param_count));
        }
        if resource_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND resource_type = ${}", param_count));
        }

        let mut q = sqlx::query_as::<_, (i64,)>(&query);

        if let Some(aid) = agent_id {
            q = q.bind(aid);
        }
        if let Some(act) = action {
            q = q.bind(act);
        }
        if let Some(rt) = resource_type {
            q = q.bind(rt);
        }

        let row = q
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.0)
    }

    /// Tenant-scoped variant of `list_with_filters`.
    ///
    /// **Limitation**: the legacy `audit_logs` table (created in
    /// migration 001) has no `tenant_id` column — only `agent_id`, a
    /// free-form `VARCHAR(255)` that does not join to identities or
    /// tenants. The data is therefore not tenant-scoped at the SQL
    /// level. We accept the `tenant_ids` parameter for symmetry with
    /// other `list_by_tenants` methods, but ignore it: the caller
    /// still requires an `AdminUser` token and is expected to be a
    /// super admin who needs global visibility into legacy audit
    /// events. A future migration that adds a `tenant_id` column
    /// should tighten the WHERE clause here.
    pub async fn list_by_tenants(
        &self,
        _tenant_ids: &[Uuid],
        agent_id: Option<&str>,
        action: Option<&str>,
        resource_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<AuditLog>> {
        self.list_with_filters(agent_id, action, resource_type, limit, offset)
            .await
    }

    /// Tenant-scoped variant of `count_with_filters`. See
    /// `list_by_tenants` for the no-tenant_id limitation of the
    /// legacy `audit_logs` table.
    pub async fn count_by_tenants(
        &self,
        _tenant_ids: &[Uuid],
        agent_id: Option<&str>,
        action: Option<&str>,
        resource_type: Option<&str>,
    ) -> DbResult<i64> {
        self.count_with_filters(agent_id, action, resource_type)
            .await
    }
}

#[derive(sqlx::FromRow)]
struct AuditLogRow {
    id: Uuid,
    agent_id: Option<String>,
    action: String,
    resource_type: String,
    resource_id: Option<String>,
    details: Value,
    timestamp: DateTime<Utc>,
}

impl From<AuditLogRow> for AuditLog {
    fn from(row: AuditLogRow) -> Self {
        Self {
            id: row.id,
            agent_id: row.agent_id,
            action: row.action,
            resource_type: row.resource_type,
            resource_id: row.resource_id,
            details: row.details,
            timestamp: row.timestamp,
        }
    }
}
