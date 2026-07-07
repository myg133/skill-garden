//! Organization repository

use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::organization::{NewOrganization, Organization};

#[derive(Clone)]
pub struct OrganizationRepository {
    pool: PgPool,
}

impl OrganizationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_org: NewOrganization) -> DbResult<Organization> {
        let settings = new_org.settings.unwrap_or_else(|| serde_json::json!({}));

        let org = sqlx::query_as::<_, OrganizationRow>(
            r#"
            INSERT INTO organizations (name, slug, display_name, description, tenant_id, org_type, visibility, avatar_url, settings)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, name, slug, display_name, description, tenant_id,
                      NULL::varchar AS tenant_name,
                      org_type, visibility, avatar_url, status, settings, created_at, updated_at
            "#,
        )
        .bind(&new_org.name)
        .bind(&new_org.slug)
        .bind(&new_org.display_name)
        .bind(&new_org.description)
        .bind(&new_org.tenant_id)
        .bind(&new_org.org_type)
        .bind(&new_org.visibility)
        .bind(&new_org.avatar_url)
        .bind(&settings)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(org.into())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<Organization>> {
        let org = sqlx::query_as::<_, OrganizationRow>(
            r#"
            SELECT o.id, o.name, o.slug, o.display_name, o.description, o.tenant_id,
                   t.name AS tenant_name,
                   o.org_type, o.visibility, o.avatar_url, o.status, o.settings, o.created_at, o.updated_at
            FROM organizations o
            LEFT JOIN tenants t ON o.tenant_id = t.id
            WHERE o.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(org.map(|o| o.into()))
    }

    pub async fn find_by_slug(&self, slug: &str) -> DbResult<Option<Organization>> {
        let org = sqlx::query_as::<_, OrganizationRow>(
            r#"
            SELECT o.id, o.name, o.slug, o.display_name, o.description, o.tenant_id,
                   t.name AS tenant_name,
                   o.org_type, o.visibility, o.avatar_url, o.status, o.settings, o.created_at, o.updated_at
            FROM organizations o
            LEFT JOIN tenants t ON o.tenant_id = t.id
            WHERE o.slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(org.map(|o| o.into()))
    }

    pub async fn list(&self, limit: i64, offset: i64) -> DbResult<Vec<Organization>> {
        let orgs = sqlx::query_as::<_, OrganizationRow>(
            r#"
            SELECT o.id, o.name, o.slug, o.display_name, o.description, o.tenant_id,
                   t.name AS tenant_name,
                   o.org_type, o.visibility, o.avatar_url, o.status, o.settings, o.created_at, o.updated_at
            FROM organizations o
            LEFT JOIN tenants t ON o.tenant_id = t.id
            ORDER BY o.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(orgs.into_iter().map(|o| o.into()).collect())
    }

    pub async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<Organization>> {
        let orgs = sqlx::query_as::<_, OrganizationRow>(
            r#"
            SELECT o.id, o.name, o.slug, o.display_name, o.description, o.tenant_id,
                   t.name AS tenant_name,
                   o.org_type, o.visibility, o.avatar_url, o.status, o.settings, o.created_at, o.updated_at
            FROM organizations o
            LEFT JOIN tenants t ON o.tenant_id = t.id
            WHERE o.tenant_id = $1
            ORDER BY o.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(orgs.into_iter().map(|o| o.into()).collect())
    }

    pub async fn update(
        &self,
        id: Uuid,
        name: String,
        display_name: Option<String>,
        description: Option<String>,
    ) -> DbResult<Organization> {
        sqlx::query(
            "UPDATE organizations SET name = $1, display_name = $2, description = $3, updated_at = NOW() WHERE id = $4",
        )
        .bind(&name)
        .bind(&display_name)
        .bind(&description)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("Organization {} not found", id)))
    }

    pub async fn delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct OrganizationRow {
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
    pub settings: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<OrganizationRow> for Organization {
    fn from(row: OrganizationRow) -> Self {
        Organization {
            id: row.id,
            name: row.name,
            slug: row.slug,
            display_name: row.display_name,
            description: row.description,
            tenant_id: row.tenant_id,
            tenant_name: row.tenant_name,
            org_type: row.org_type,
            visibility: row.visibility,
            avatar_url: row.avatar_url,
            status: row.status,
            settings: row.settings,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
