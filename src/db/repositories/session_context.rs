//! Session Context Repository - Manages session context data, skill states, and tool execution history

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub id: Uuid,
    pub session_id: Uuid,
    pub context_key: String,
    pub context_value: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSessionContext {
    pub session_id: Uuid,
    pub context_key: String,
    pub context_value: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSkillState {
    pub id: Uuid,
    pub session_id: Uuid,
    pub skill_id: String,
    pub skill_state: JsonValue,
    pub status: String,
    pub loaded_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSessionSkill {
    pub session_id: Uuid,
    pub skill_id: String,
    pub skill_state: JsonValue,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToolExecution {
    pub id: Uuid,
    pub session_id: Uuid,
    pub tool_id: String,
    pub tool_type: String,
    pub parameters: JsonValue,
    pub result: Option<JsonValue>,
    pub success: bool,
    pub execution_time_ms: Option<i32>,
    pub error_message: Option<String>,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewToolExecution {
    pub session_id: Uuid,
    pub tool_id: String,
    pub tool_type: String,
    pub parameters: JsonValue,
    pub result: Option<JsonValue>,
    pub success: bool,
    pub execution_time_ms: Option<i32>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub id: Uuid,
    pub skill_id: String,
    pub dependency_skill_id: String,
    pub version_constraint: String,
    pub is_optional: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewSkillDependency {
    pub skill_id: String,
    pub dependency_skill_id: String,
    pub version_constraint: String,
    pub is_optional: bool,
}

#[derive(Clone)]
pub struct SessionContextRepository {
    pool: PgPool,
}

impl SessionContextRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn create_context(&self, ctx: NewSessionContext) -> DbResult<SessionContext> {
        let row = sqlx::query_as::<_, SessionContextRow>(
            r#"
            INSERT INTO session_context (session_id, context_key, context_value)
            VALUES ($1, $2, $3)
            ON CONFLICT (session_id, context_key) DO UPDATE SET
                context_value = EXCLUDED.context_value,
                updated_at = NOW()
            RETURNING id, session_id, context_key, context_value, created_at, updated_at
            "#,
        )
        .bind(ctx.session_id)
        .bind(&ctx.context_key)
        .bind(&ctx.context_value)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.into())
    }

    pub async fn get_context(&self, session_id: Uuid, context_key: &str) -> DbResult<Option<SessionContext>> {
        let row = sqlx::query_as::<_, SessionContextRow>(
            r#"
            SELECT id, session_id, context_key, context_value, created_at, updated_at
            FROM session_context
            WHERE session_id = $1 AND context_key = $2
            "#,
        )
        .bind(session_id)
        .bind(context_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_contexts(&self, session_id: Uuid) -> DbResult<Vec<SessionContext>> {
        let rows = sqlx::query_as::<_, SessionContextRow>(
            r#"
            SELECT id, session_id, context_key, context_value, created_at, updated_at
            FROM session_context
            WHERE session_id = $1
            ORDER BY created_at
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn delete_context(&self, session_id: Uuid, context_key: &str) -> DbResult<()> {
        sqlx::query(
            r#"
            DELETE FROM session_context WHERE session_id = $1 AND context_key = $2
            "#,
        )
        .bind(session_id)
        .bind(context_key)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn load_skill(&self, session_skill: NewSessionSkill) -> DbResult<SessionSkillState> {
        let row = sqlx::query_as::<_, SessionSkillStateRow>(
            r#"
            INSERT INTO session_skills (session_id, skill_id, skill_state, status)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (session_id, skill_id) DO UPDATE SET
                skill_state = EXCLUDED.skill_state,
                status = EXCLUDED.status,
                last_used_at = NOW()
            RETURNING id, session_id, skill_id, skill_state, status, loaded_at, last_used_at
            "#,
        )
        .bind(session_skill.session_id)
        .bind(&session_skill.skill_id)
        .bind(&session_skill.skill_state)
        .bind(&session_skill.status)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.into())
    }

    pub async fn get_session_skill(&self, session_id: Uuid, skill_id: &str) -> DbResult<Option<SessionSkillState>> {
        let row = sqlx::query_as::<_, SessionSkillStateRow>(
            r#"
            SELECT id, session_id, skill_id, skill_state, status, loaded_at, last_used_at
            FROM session_skills
            WHERE session_id = $1 AND skill_id = $2
            "#,
        )
        .bind(session_id)
        .bind(skill_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_session_skills(&self, session_id: Uuid) -> DbResult<Vec<SessionSkillState>> {
        let rows = sqlx::query_as::<_, SessionSkillStateRow>(
            r#"
            SELECT id, session_id, skill_id, skill_state, status, loaded_at, last_used_at
            FROM session_skills
            WHERE session_id = $1
            ORDER BY loaded_at
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update_skill_state(&self, session_id: Uuid, skill_id: &str, skill_state: JsonValue) -> DbResult<()> {
        sqlx::query(
            r#"
            UPDATE session_skills
            SET skill_state = $1, last_used_at = NOW()
            WHERE session_id = $2 AND skill_id = $3
            "#,
        )
        .bind(&skill_state)
        .bind(session_id)
        .bind(skill_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn unload_skill(&self, session_id: Uuid, skill_id: &str) -> DbResult<()> {
        sqlx::query(
            r#"
            DELETE FROM session_skills WHERE session_id = $1 AND skill_id = $2
            "#,
        )
        .bind(session_id)
        .bind(skill_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn record_tool_execution(&self, execution: NewToolExecution) -> DbResult<SessionToolExecution> {
        let row = sqlx::query_as::<_, SessionToolExecutionRow>(
            r#"
            INSERT INTO session_tool_executions (session_id, tool_id, tool_type, parameters, result, success, execution_time_ms, error_message)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, session_id, tool_id, tool_type, parameters, result, success, execution_time_ms, error_message, executed_at
            "#,
        )
        .bind(execution.session_id)
        .bind(&execution.tool_id)
        .bind(&execution.tool_type)
        .bind(&execution.parameters)
        .bind(&execution.result)
        .bind(execution.success)
        .bind(execution.execution_time_ms)
        .bind(&execution.error_message)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.into())
    }

    pub async fn get_tool_execution_history(&self, session_id: Uuid, limit: i64) -> DbResult<Vec<SessionToolExecution>> {
        let rows = sqlx::query_as::<_, SessionToolExecutionRow>(
            r#"
            SELECT id, session_id, tool_id, tool_type, parameters, result, success, execution_time_ms, error_message, executed_at
            FROM session_tool_executions
            WHERE session_id = $1
            ORDER BY executed_at DESC
            LIMIT $2
            "#,
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn add_skill_dependency(&self, dep: NewSkillDependency) -> DbResult<SkillDependency> {
        let row = sqlx::query_as::<_, SkillDependencyRow>(
            r#"
            INSERT INTO skill_dependencies (skill_id, dependency_skill_id, version_constraint, is_optional)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (skill_id, dependency_skill_id) DO UPDATE SET
                version_constraint = EXCLUDED.version_constraint,
                is_optional = EXCLUDED.is_optional
            RETURNING id, skill_id, dependency_skill_id, version_constraint, is_optional, created_at
            "#,
        )
        .bind(&dep.skill_id)
        .bind(&dep.dependency_skill_id)
        .bind(&dep.version_constraint)
        .bind(dep.is_optional)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.into())
    }

    pub async fn get_skill_dependencies(&self, skill_id: &str) -> DbResult<Vec<SkillDependency>> {
        let rows = sqlx::query_as::<_, SkillDependencyRow>(
            r#"
            SELECT id, skill_id, dependency_skill_id, version_constraint, is_optional, created_at
            FROM skill_dependencies
            WHERE skill_id = $1
            "#,
        )
        .bind(skill_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn resolve_dependencies(&self, skill_ids: Vec<String>) -> DbResult<Vec<String>> {
        let mut resolved = Vec::new();
        let mut to_resolve = skill_ids;

        while !to_resolve.is_empty() {
            let current = to_resolve.remove(0);

            let deps = self.get_skill_dependencies(&current).await?;

            for dep in deps {
                if !resolved.contains(&dep.dependency_skill_id) && !to_resolve.contains(&dep.dependency_skill_id) {
                    to_resolve.push(dep.dependency_skill_id.clone());
                }
            }

            if !resolved.contains(&current) {
                resolved.push(current);
            }
        }

        Ok(resolved)
    }

    pub async fn delete_skill_dependencies(&self, skill_id: &str) -> DbResult<()> {
        sqlx::query("DELETE FROM skill_dependencies WHERE skill_id = $1")
            .bind(skill_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct SessionContextRow {
    id: Uuid,
    session_id: Uuid,
    context_key: String,
    context_value: JsonValue,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SessionContextRow> for SessionContext {
    fn from(row: SessionContextRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            context_key: row.context_key,
            context_value: row.context_value,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SessionSkillStateRow {
    id: Uuid,
    session_id: Uuid,
    skill_id: String,
    skill_state: JsonValue,
    status: String,
    loaded_at: DateTime<Utc>,
    last_used_at: DateTime<Utc>,
}

impl From<SessionSkillStateRow> for SessionSkillState {
    fn from(row: SessionSkillStateRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            skill_id: row.skill_id,
            skill_state: row.skill_state,
            status: row.status,
            loaded_at: row.loaded_at,
            last_used_at: row.last_used_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SessionToolExecutionRow {
    id: Uuid,
    session_id: Uuid,
    tool_id: String,
    tool_type: String,
    parameters: JsonValue,
    result: Option<JsonValue>,
    success: bool,
    execution_time_ms: Option<i32>,
    error_message: Option<String>,
    executed_at: DateTime<Utc>,
}

impl From<SessionToolExecutionRow> for SessionToolExecution {
    fn from(row: SessionToolExecutionRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            tool_id: row.tool_id,
            tool_type: row.tool_type,
            parameters: row.parameters,
            result: row.result,
            success: row.success,
            execution_time_ms: row.execution_time_ms,
            error_message: row.error_message,
            executed_at: row.executed_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SkillDependencyRow {
    id: Uuid,
    skill_id: String,
    dependency_skill_id: String,
    version_constraint: String,
    is_optional: bool,
    created_at: DateTime<Utc>,
}

impl From<SkillDependencyRow> for SkillDependency {
    fn from(row: SkillDependencyRow) -> Self {
        Self {
            id: row.id,
            skill_id: row.skill_id,
            dependency_skill_id: row.dependency_skill_id,
            version_constraint: row.version_constraint,
            is_optional: row.is_optional,
            created_at: row.created_at,
        }
    }
}
