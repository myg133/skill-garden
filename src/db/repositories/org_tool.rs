//! Organization Tool repository

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgTool {
    pub id: Uuid,
    pub tool_id: String,
    pub org_id: Uuid,
    pub name: String,
    pub description: String,
    pub schema: JsonValue,
    pub implementation: JsonValue,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewOrgTool {
    pub tool_id: String,
    pub org_id: Uuid,
    pub name: String,
    pub description: String,
    pub schema: JsonValue,
    pub implementation: JsonValue,
}

#[derive(Clone)]
pub struct OrgToolRepository {
    pool: PgPool,
}

impl OrgToolRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_tool: NewOrgTool) -> DbResult<OrgTool> {
        let tool = sqlx::query_as::<_, OrgToolRow>(
            r#"
            INSERT INTO org_tools (tool_id, org_id, name, description, schema, implementation, status)
            VALUES ($1, $2, $3, $4, $5, $6, 'pending')
            RETURNING id, tool_id, org_id, name, description, schema, implementation, status, created_at
            "#,
        )
        .bind(&new_tool.tool_id)
        .bind(new_tool.org_id)
        .bind(&new_tool.name)
        .bind(&new_tool.description)
        .bind(&new_tool.schema)
        .bind(&new_tool.implementation)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                DbError::AlreadyExists(format!("Tool {} already exists in org", new_tool.tool_id))
            } else {
                DbError::QueryError(e.to_string())
            }
        })?;

        Ok(tool.into())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<OrgTool>> {
        let tool = sqlx::query_as::<_, OrgToolRow>(
            r#"
            SELECT id, tool_id, org_id, name, description, schema, implementation, status, created_at
            FROM org_tools WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tool.map(|t| t.into()))
    }

    pub async fn find_by_org(&self, org_id: Uuid) -> DbResult<Vec<OrgTool>> {
        let tools = sqlx::query_as::<_, OrgToolRow>(
            r#"
            SELECT id, tool_id, org_id, name, description, schema, implementation, status, created_at
            FROM org_tools
            WHERE org_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tools.into_iter().map(|t| t.into()).collect())
    }

    pub async fn find_approved_by_org(&self, org_id: Uuid) -> DbResult<Vec<OrgTool>> {
        let tools = sqlx::query_as::<_, OrgToolRow>(
            r#"
            SELECT id, tool_id, org_id, name, description, schema, implementation, status, created_at
            FROM org_tools
            WHERE org_id = $1 AND status = 'approved'
            ORDER BY created_at DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tools.into_iter().map(|t| t.into()).collect())
    }

    pub async fn find_all(&self) -> DbResult<Vec<OrgTool>> {
        let tools = sqlx::query_as::<_, OrgToolRow>(
            r#"
            SELECT id, tool_id, org_id, name, description, schema, implementation, status, created_at
            FROM org_tools
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tools.into_iter().map(|t| t.into()).collect())
    }

    pub async fn update_status(&self, id: Uuid, status: &str) -> DbResult<()> {
        sqlx::query("UPDATE org_tools SET status = $1 WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn find_by_tool_id(&self, org_id: Uuid, tool_id: &str) -> DbResult<Option<OrgTool>> {
        let tool = sqlx::query_as::<_, OrgToolRow>(
            r#"
            SELECT id, tool_id, org_id, name, description, schema, implementation, status, created_at
            FROM org_tools
            WHERE org_id = $1 AND tool_id = $2
            "#,
        )
        .bind(org_id)
        .bind(tool_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tool.map(|t| t.into()))
    }

    pub async fn delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM org_tools WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct OrgToolRow {
    id: Uuid,
    tool_id: String,
    org_id: Uuid,
    name: String,
    description: String,
    schema: JsonValue,
    implementation: JsonValue,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<OrgToolRow> for OrgTool {
    fn from(row: OrgToolRow) -> Self {
        Self {
            id: row.id,
            tool_id: row.tool_id,
            org_id: row.org_id,
            name: row.name,
            description: row.description,
            schema: row.schema,
            implementation: row.implementation,
            status: row.status,
            created_at: row.created_at,
        }
    }
}
