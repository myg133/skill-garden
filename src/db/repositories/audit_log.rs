//! Audit Log Repository (new system — audit_log_entries table)

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::api_key::{AuditLog, CreateAuditLogRequest};

#[derive(Clone)]
pub struct AuditLogRepository {
    pool: PgPool,
}

impl AuditLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, request: CreateAuditLogRequest) -> DbResult<AuditLog> {
        let log = sqlx::query_as::<_, AuditLogRow>(
            r#"
            INSERT INTO audit_log_entries (tenant_id, organization_id, identity_id, action, resource_type, resource_id, details, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::INET, $9)
            RETURNING id, tenant_id, organization_id, identity_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at, NULL::text AS identity_name, NULL::text AS identity_type
            "#,
        )
        .bind(&request.tenant_id)
        .bind(&request.organization_id)
        .bind(request.identity_id)
        .bind(&request.action)
        .bind(&request.resource_type)
        .bind(&request.resource_id)
        .bind(&request.details)
        .bind(&request.ip_address)
        .bind(&request.user_agent)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(log.into())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<AuditLog>> {
        let log = sqlx::query_as::<_, AuditLogRow>(
            r#"
            SELECT id, tenant_id, organization_id, identity_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at
            FROM audit_log_entries WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(log.map(|l| l.into()))
    }

    pub async fn query(
        &self,
        tenant_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        identity_id: Option<Uuid>,
        action: Option<&str>,
        resource_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<AuditLog>> {
        let logs = sqlx::query_as::<_, AuditLogRow>(
            r#"
            SELECT
                ale.id, ale.tenant_id, ale.organization_id, ale.identity_id,
                ale.action, ale.resource_type, ale.resource_id, ale.details,
                ale.ip_address, ale.user_agent, ale.created_at,
                COALESCE(i.display_name, i.username, i.name) AS identity_name,
                i.identity_type
            FROM audit_log_entries ale
            LEFT JOIN identities i ON i.id = ale.identity_id
            WHERE ($1::uuid IS NULL OR ale.tenant_id = $1)
              AND ($2::uuid IS NULL OR ale.organization_id = $2)
              AND ($3::uuid IS NULL OR ale.identity_id = $3)
              AND ($4::text IS NULL OR ale.action = $4)
              AND ($5::text IS NULL OR ale.resource_type = $5)
            ORDER BY ale.created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(&tenant_id)
        .bind(&organization_id)
        .bind(&identity_id)
        .bind(&action)
        .bind(&resource_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(logs.into_iter().map(|l| l.into()).collect())
    }

    pub async fn count(
        &self,
        tenant_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        identity_id: Option<Uuid>,
        action: Option<&str>,
        resource_type: Option<&str>,
    ) -> DbResult<i64> {
        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            SELECT COUNT(*) FROM audit_log_entries
            WHERE ($1::uuid IS NULL OR tenant_id = $1)
              AND ($2::uuid IS NULL OR organization_id = $2)
              AND ($3::uuid IS NULL OR identity_id = $3)
              AND ($4::text IS NULL OR action = $4)
              AND ($5::text IS NULL OR resource_type = $5)
            "#,
        )
        .bind(&tenant_id)
        .bind(&organization_id)
        .bind(&identity_id)
        .bind(&action)
        .bind(&resource_type)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.0)
    }
}

#[derive(sqlx::FromRow)]
struct AuditLogRow {
    id: Uuid,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    identity_id: Uuid,
    action: String,
    resource_type: Option<String>,
    resource_id: Option<Uuid>,
    details: Option<serde_json::Value>,
    ip_address: Option<String>,
    user_agent: Option<String>,
    created_at: chrono::DateTime<Utc>,
    identity_name: Option<String>,
    identity_type: Option<String>,
}

impl From<AuditLogRow> for AuditLog {
    fn from(row: AuditLogRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            organization_id: row.organization_id,
            identity_id: row.identity_id,
            action: row.action,
            resource_type: row.resource_type,
            resource_id: row.resource_id,
            details: row.details,
            ip_address: row.ip_address,
            user_agent: row.user_agent,
            created_at: row.created_at,
            identity_name: row.identity_name,
            identity_type: row.identity_type,
        }
    }
}
