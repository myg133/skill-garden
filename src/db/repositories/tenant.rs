//! Tenant Repository

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::tenant::{NewTenant, Tenant, TenantStatus, TenantUpdate};

#[derive(Clone)]
pub struct TenantRepository {
    pool: PgPool,
}

impl TenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_tenant: NewTenant) -> DbResult<Tenant> {
        let billing_plan = new_tenant.billing_plan.unwrap_or_else(|| "free".to_string());

        let tenant = sqlx::query_as::<_, TenantRow>(
            r#"
            INSERT INTO tenants (name, slug, billing_plan, sso_config, settings)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, slug, status, billing_plan, sso_config, settings, created_by, created_at, updated_at
            "#,
        )
        .bind(&new_tenant.name)
        .bind(&new_tenant.slug)
        .bind(&billing_plan)
        .bind(&new_tenant.sso_config)
        .bind(&new_tenant.settings)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                DbError::AlreadyExists(format!("Tenant with slug '{}' already exists", new_tenant.slug))
            } else {
                DbError::QueryError(e.to_string())
            }
        })?;

        Ok(tenant.into())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<Tenant>> {
        let tenant = sqlx::query_as::<_, TenantRow>(
            r#"SELECT id, name, slug, status, billing_plan, sso_config, settings, created_by, created_at, updated_at FROM tenants WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tenant.map(|t| t.into()))
    }

    pub async fn find_by_slug(&self, slug: &str) -> DbResult<Option<Tenant>> {
        let tenant = sqlx::query_as::<_, TenantRow>(
            r#"SELECT id, name, slug, status, billing_plan, sso_config, settings, created_by, created_at, updated_at FROM tenants WHERE slug = $1"#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tenant.map(|t| t.into()))
    }

    pub async fn list_all(&self, limit: i64, offset: i64) -> DbResult<Vec<Tenant>> {
        let tenants = sqlx::query_as::<_, TenantRow>(
            r#"
            SELECT id, name, slug, status, billing_plan, sso_config, settings, created_by, created_at, updated_at
            FROM tenants
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tenants.into_iter().map(|t| t.into()).collect())
    }

    pub async fn update(&self, id: Uuid, update: TenantUpdate) -> DbResult<Tenant> {
        let current = self.find_by_id(id).await?.ok_or_else(|| DbError::NotFound("Tenant not found".to_string()))?;

        let name = update.name.unwrap_or(current.name);
        let slug = update.slug.unwrap_or(current.slug);
        let status = update.status.unwrap_or(current.status);
        let billing_plan = update.billing_plan.or(current.billing_plan);
        let sso_config = update.sso_config.or(current.sso_config);
        let settings = update.settings.unwrap_or(current.settings);

        let tenant = sqlx::query_as::<_, TenantRow>(
            r#"
            UPDATE tenants
            SET name = $1, slug = $2, status = $3, billing_plan = $4, sso_config = $5, settings = $6, updated_at = NOW()
            WHERE id = $7
            RETURNING id, name, slug, status, billing_plan, sso_config, settings, created_by, created_at, updated_at
            "#,
        )
        .bind(&name)
        .bind(&slug)
        .bind(status.to_string())
        .bind(&billing_plan)
        .bind(&sso_config)
        .bind(&settings)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tenant.into())
    }

    pub async fn delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct TenantRow {
    id: Uuid,
    name: String,
    slug: String,
    status: String,
    billing_plan: Option<String>,
    sso_config: Option<serde_json::Value>,
    settings: serde_json::Value,
    created_by: Option<Uuid>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl From<TenantRow> for Tenant {
    fn from(row: TenantRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            slug: row.slug,
            status: TenantStatus::from(row.status.as_str()),
            billing_plan: row.billing_plan,
            sso_config: row.sso_config,
            settings: row.settings,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
