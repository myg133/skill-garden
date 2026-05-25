//! Session repository

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub agent_id: String,
    pub org_id: Uuid,
    pub status: String,
    pub tool_router: JsonValue,
    pub capabilities: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewSession {
    pub agent_id: String,
    pub org_id: Uuid,
}

#[derive(Clone)]
pub struct SessionRepository {
    pool: PgPool,
}

impl SessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_session: NewSession) -> DbResult<Session> {
        let session = sqlx::query_as::<_, SessionRow>(
            r#"
            INSERT INTO sessions (agent_id, org_id, status, tool_router, capabilities, last_active_at)
            VALUES ($1, $2, 'active', '{}', '{}', NOW())
            RETURNING id, agent_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
            "#,
        )
        .bind(&new_session.agent_id)
        .bind(new_session.org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(session.into())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<Session>> {
        let session = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, agent_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
            FROM sessions WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(session.map(|s| s.into()))
    }

    pub async fn find_active_by_agent(&self, agent_id: &str) -> DbResult<Vec<Session>> {
        let sessions = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, agent_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
            FROM sessions
            WHERE agent_id = $1 AND status = 'active'
            ORDER BY created_at DESC
            "#,
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(sessions.into_iter().map(|s| s.into()).collect())
    }

    pub async fn list_all(&self, limit: i64, offset: i64, status: Option<&str>) -> DbResult<Vec<Session>> {
        let sessions = match status {
            Some("active") => {
                sqlx::query_as::<_, SessionRow>(
                    r#"
                    SELECT id, agent_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
                    FROM sessions
                    WHERE status = 'active'
                    ORDER BY last_active_at DESC
                    LIMIT $1 OFFSET $2
                    "#,
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
            Some("ended") => {
                sqlx::query_as::<_, SessionRow>(
                    r#"
                    SELECT id, agent_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
                    FROM sessions
                    WHERE status = 'ended'
                    ORDER BY ended_at DESC
                    LIMIT $1 OFFSET $2
                    "#,
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
            _ => {
                sqlx::query_as::<_, SessionRow>(
                    r#"
                    SELECT id, agent_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
                    FROM sessions
                    ORDER BY last_active_at DESC
                    LIMIT $1 OFFSET $2
                    "#,
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
        }.map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(sessions.into_iter().map(|s| s.into()).collect())
    }

    pub async fn end_session(&self, id: Uuid) -> DbResult<()> {
        sqlx::query(
            r#"
            UPDATE sessions SET status = 'ended', ended_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn update_tool_router(&self, id: Uuid, tool_router: JsonValue) -> DbResult<()> {
        sqlx::query("UPDATE sessions SET tool_router = $1 WHERE id = $2")
            .bind(&tool_router)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    agent_id: String,
    org_id: Uuid,
    status: String,
    tool_router: JsonValue,
    capabilities: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_active_at: chrono::DateTime<chrono::Utc>,
    ended_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<SessionRow> for Session {
    fn from(row: SessionRow) -> Self {
        Self {
            id: row.id,
            agent_id: row.agent_id,
            org_id: row.org_id,
            status: row.status,
            tool_router: row.tool_router,
            capabilities: row.capabilities,
            created_at: row.created_at,
            last_active_at: row.last_active_at,
            ended_at: row.ended_at,
        }
    }
}
