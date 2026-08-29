//! Session repository

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub identity_id: Uuid,
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
    pub identity_id: Uuid,
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
            INSERT INTO sessions (identity_id, org_id, status, tool_router, capabilities, last_active_at)
            VALUES ($1, $2, 'active', '{}', '{}', NOW())
            RETURNING id, identity_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
            "#,
        )
        .bind(new_session.identity_id)
        .bind(new_session.org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(session.into())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<Session>> {
        let session = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, identity_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
            FROM sessions WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(session.map(|s| s.into()))
    }

    pub async fn find_active_by_identity(&self, identity_id: Uuid) -> DbResult<Vec<Session>> {
        let sessions = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, identity_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
            FROM sessions
            WHERE identity_id = $1 AND status = 'active'
            ORDER BY created_at DESC
            "#,
        )
        .bind(identity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(sessions.into_iter().map(|s| s.into()).collect())
    }

    pub async fn list_all(
        &self,
        limit: i64,
        offset: i64,
        status: Option<&str>,
    ) -> DbResult<Vec<Session>> {
        let sessions = match status {
            Some("active") => {
                sqlx::query_as::<_, SessionRow>(
                    r#"
                    SELECT id, identity_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
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
                    SELECT id, identity_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
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
                    SELECT id, identity_id, org_id, status, tool_router, capabilities, created_at, last_active_at, ended_at
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

    /// Touch a session — update last_active_at to NOW(). Used on each MCP request.
    pub async fn touch(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("UPDATE sessions SET last_active_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    /// End all active sessions that have been idle for more than `idle_secs` seconds.
    /// Returns the number of sessions ended.
    pub async fn end_idle_sessions(&self, idle_secs: i64) -> DbResult<usize> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET status = 'ended', ended_at = NOW()
            WHERE status = 'active'
              AND last_active_at < NOW() - ($1 || ' seconds')::INTERVAL
            "#,
        )
        .bind(idle_secs)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(result.rows_affected() as usize)
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    identity_id: Uuid,
    org_id: Uuid,
    status: String,
    tool_router: JsonValue,
    capabilities: JsonValue,
    created_at: chrono::DateTime<chrono::Utc>,
    last_active_at: chrono::DateTime<chrono::Utc>,
    ended_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<SessionRow> for Session {
    fn from(row: SessionRow) -> Self {
        let capabilities = row
            .capabilities
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            id: row.id,
            identity_id: row.identity_id,
            org_id: row.org_id,
            status: row.status,
            tool_router: row.tool_router,
            capabilities,
            created_at: row.created_at,
            last_active_at: row.last_active_at,
            ended_at: row.ended_at,
        }
    }
}
