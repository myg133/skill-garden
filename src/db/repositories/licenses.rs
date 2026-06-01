//! Licenses Repository

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::license::License;

#[derive(Debug, Clone)]
pub struct LicenseRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LicenseRow {
    pub id: Uuid,
    pub license_key: String,
    pub tenant_id: Uuid,
    pub plan: String,
    pub max_users: i32,
    pub max_organizations: i32,
    pub max_skills: i32,
    pub features: serde_json::Value,
    pub expires_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LicenseRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        tenant_id: Uuid,
        license_key: &str,
        plan: &str,
        max_users: i32,
        max_organizations: i32,
        max_skills: i32,
        features: serde_json::Value,
        expires_at: Option<DateTime<Utc>>,
    ) -> DbResult<License> {
        let license = sqlx::query_as::<_, LicenseRow>(
            r#"
            INSERT INTO licenses (license_key, tenant_id, plan, max_users, max_organizations, max_skills, features, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, license_key, tenant_id, plan, max_users, max_organizations, max_skills, features, expires_at, status, created_at, updated_at
            "#,
        )
        .bind(license_key)
        .bind(tenant_id)
        .bind(plan)
        .bind(max_users)
        .bind(max_organizations)
        .bind(max_skills)
        .bind(&features)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(license.into())
    }

    pub async fn find_by_tenant(&self, tenant_id: Uuid) -> DbResult<Option<License>> {
        let license = sqlx::query_as::<_, LicenseRow>(
            r#"
            SELECT id, license_key, tenant_id, plan, max_users, max_organizations, max_skills, features, expires_at, status, created_at, updated_at
            FROM licenses WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(license.map(|l| l.into()))
    }

    pub async fn find_active_by_tenant(&self, tenant_id: Uuid) -> DbResult<Option<License>> {
        let license = sqlx::query_as::<_, LicenseRow>(
            r#"
            SELECT id, license_key, tenant_id, plan, max_users, max_organizations, max_skills, features, expires_at, status, created_at, updated_at
            FROM licenses
            WHERE tenant_id = $1 AND status = 'active' AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY created_at DESC LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(license.map(|l| l.into()))
    }
}

impl From<LicenseRow> for License {
    fn from(row: LicenseRow) -> Self {
        Self {
            id: row.id,
            license_key: row.license_key,
            tenant_id: row.tenant_id,
            plan: row.plan,
            max_users: row.max_users,
            max_organizations: row.max_organizations,
            max_skills: row.max_skills,
            features: row.features,
            expires_at: row.expires_at,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}