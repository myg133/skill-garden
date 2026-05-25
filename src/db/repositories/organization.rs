//! Organization repository

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub settings: JsonValue,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewOrganization {
    pub name: String,
    pub settings: Option<JsonValue>,
}

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
            INSERT INTO organizations (name, settings)
            VALUES ($1, $2)
            RETURNING id, name, settings, created_at
            "#,
        )
        .bind(&new_org.name)
        .bind(&settings)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(org.into())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<Organization>> {
        let org = sqlx::query_as::<_, OrganizationRow>(
            r#"
            SELECT id, name, settings, created_at
            FROM organizations WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(org.map(|o| o.into()))
    }

    pub async fn list(&self, limit: i64, offset: i64) -> DbResult<Vec<Organization>> {
        let orgs = sqlx::query_as::<_, OrganizationRow>(
            r#"
            SELECT id, name, settings, created_at
            FROM organizations
            ORDER BY created_at DESC
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

    pub async fn update(&self, id: Uuid, name: String) -> DbResult<Organization> {
        let org = sqlx::query_as::<_, OrganizationRow>(
            r#"
            UPDATE organizations SET name = $1
            WHERE id = $2
            RETURNING id, name, settings, created_at
            "#,
        )
        .bind(&name)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(org.into())
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

#[derive(sqlx::FromRow)]
struct OrganizationRow {
    id: Uuid,
    name: String,
    settings: JsonValue,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<OrganizationRow> for Organization {
    fn from(row: OrganizationRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            settings: row.settings,
            created_at: row.created_at,
        }
    }
}
