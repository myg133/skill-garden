//! API Key and Audit Repository

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::api_key::{
    ApiKey, ApiKeyStatus, AuditLog, CreateApiKeyRequest, CreateAuditLogRequest,
};

#[derive(Clone)]
pub struct ApiKeyRepository {
    pool: PgPool,
}

impl ApiKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        request: CreateApiKeyRequest,
        key_hash: &str,
        key_prefix: &str,
    ) -> DbResult<ApiKey> {
        let scopes_json = serde_json::to_value(&request.scopes).unwrap_or(serde_json::json!([]));

        let api_key = sqlx::query_as::<_, ApiKeyRow>(
            r#"
            INSERT INTO api_keys (identity_id, organization_id, key_hash, key_prefix, name, scopes, rate_limit, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, identity_id, organization_id, key_hash, key_prefix, name, scopes, rate_limit, status, expires_at, created_at, last_used_at
            "#,
        )
        .bind(request.identity_id)
        .bind(request.organization_id)
        .bind(key_hash)
        .bind(key_prefix)
        .bind(&request.name)
        .bind(&scopes_json)
        .bind(request.rate_limit)
        .bind(&request.expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(api_key.into())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<ApiKey>> {
        let api_key = sqlx::query_as::<_, ApiKeyRow>(
            r#"
            SELECT id, identity_id, organization_id, key_hash, key_prefix, name, scopes, rate_limit, status, expires_at, created_at, last_used_at
            FROM api_keys WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(api_key.map(|k| k.into()))
    }

    pub async fn find_by_key_hash(&self, key_hash: &str) -> DbResult<Option<ApiKey>> {
        let api_key = sqlx::query_as::<_, ApiKeyRow>(
            r#"
            SELECT id, identity_id, organization_id, key_hash, key_prefix, name, scopes, rate_limit, status, expires_at, created_at, last_used_at
            FROM api_keys WHERE key_hash = $1
            "#,
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(api_key.map(|k| k.into()))
    }

    pub async fn list_by_identity(&self, identity_id: Uuid) -> DbResult<Vec<ApiKey>> {
        let keys = sqlx::query_as::<_, ApiKeyRow>(
            r#"
            SELECT id, identity_id, organization_id, key_hash, key_prefix, name, scopes, rate_limit, status, expires_at, created_at, last_used_at
            FROM api_keys
            WHERE identity_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(identity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(keys.into_iter().map(|k| k.into()).collect())
    }

    pub async fn list_by_organization(&self, organization_id: Uuid) -> DbResult<Vec<ApiKey>> {
        let keys = sqlx::query_as::<_, ApiKeyRow>(
            r#"
            SELECT id, identity_id, organization_id, key_hash, key_prefix, name, scopes, rate_limit, status, expires_at, created_at, last_used_at
            FROM api_keys
            WHERE organization_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(keys.into_iter().map(|k| k.into()).collect())
    }

    pub async fn list(&self) -> DbResult<Vec<ApiKey>> {
        let keys = sqlx::query_as::<_, ApiKeyRow>(
            r#"
            SELECT id, identity_id, organization_id, key_hash, key_prefix, name, scopes, rate_limit, status, expires_at, created_at, last_used_at
            FROM api_keys
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(keys.into_iter().map(|k| k.into()).collect())
    }

    pub async fn revoke(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("UPDATE api_keys SET status = 'revoked' WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn update_last_used(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM api_keys WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }
}

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
            INSERT INTO audit_logs (tenant_id, organization_id, identity_id, action, resource_type, resource_id, details, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, tenant_id, organization_id, identity_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at
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
            FROM audit_logs WHERE id = $1
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
            SELECT id, tenant_id, organization_id, identity_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at
            FROM audit_logs
            WHERE ($1::uuid IS NULL OR tenant_id = $1)
              AND ($2::uuid IS NULL OR organization_id = $2)
              AND ($3::uuid IS NULL OR identity_id = $3)
              AND ($4::text IS NULL OR action = $4)
              AND ($5::text IS NULL OR resource_type = $5)
            ORDER BY created_at DESC
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
}

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: Uuid,
    identity_id: Uuid,
    organization_id: Uuid,
    key_hash: String,
    key_prefix: String,
    name: Option<String>,
    scopes: serde_json::Value,
    rate_limit: i32,
    status: String,
    expires_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
    last_used_at: Option<chrono::DateTime<Utc>>,
}

impl From<ApiKeyRow> for ApiKey {
    fn from(row: ApiKeyRow) -> Self {
        let scopes: Vec<String> = serde_json::from_value(row.scopes).unwrap_or_default();
        Self {
            id: row.id,
            identity_id: row.identity_id,
            organization_id: row.organization_id,
            key_hash: row.key_hash,
            key_prefix: row.key_prefix,
            name: row.name,
            scopes,
            rate_limit: row.rate_limit,
            status: ApiKeyStatus::from(row.status.as_str()),
            expires_at: row.expires_at,
            created_at: row.created_at,
            last_used_at: row.last_used_at,
        }
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
        }
    }
}
