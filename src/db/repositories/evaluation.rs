//! Evaluation repository

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone)]
pub struct Evaluation {
    pub id: Uuid,
    pub skill_id: String,
    pub agent_id: String,
    pub success: bool,
    pub duration_ms: i64,
    pub error_type: Option<String>,
    pub tags: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

pub struct NewEvaluation {
    pub skill_id: String,
    pub agent_id: String,
    pub success: bool,
    pub duration_ms: i64,
    pub error_type: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SkillStats {
    pub skill_id: String,
    pub success_rate: f64,
    pub avg_duration_ms: i64,
    pub total_evaluations: i32,
    pub unique_agents: i32,
    pub confidence: f64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EvaluationRepository {
    pool: PgPool,
}

impl EvaluationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_eval: NewEvaluation) -> DbResult<Evaluation> {
        let id = Uuid::new_v4();

        let row = sqlx::query_as::<_, EvaluationRow>(
            r#"
            INSERT INTO evaluations (id, skill_id, agent_id, success, duration_ms, error_type, tags)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, skill_id, agent_id, success, duration_ms, error_type, tags, timestamp
            "#,
        )
        .bind(id)
        .bind(&new_eval.skill_id)
        .bind(&new_eval.agent_id)
        .bind(new_eval.success)
        .bind(new_eval.duration_ms)
        .bind(&new_eval.error_type)
        .bind(&new_eval.tags)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.into())
    }

    pub async fn get_stats(&self, skill_id: &str) -> DbResult<SkillStats> {
        let stats_row = sqlx::query_as::<_, StatsRow>(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE success = true) as success_count,
                AVG(duration_ms) as avg_duration,
                COUNT(DISTINCT agent_id) as unique_agents
            FROM evaluations
            WHERE skill_id = $1
            "#,
        )
        .bind(skill_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        let tags = self.get_top_tags(skill_id).await?;

        let success_rate = if stats_row.total > 0 {
            stats_row.success_count as f64 / stats_row.total as f64
        } else {
            0.0
        };

        let avg_duration = stats_row.avg_duration.unwrap_or(0.0) as i64;

        let confidence = self.calculate_confidence(stats_row.total);

        Ok(SkillStats {
            skill_id: skill_id.to_string(),
            success_rate,
            avg_duration_ms: avg_duration,
            total_evaluations: stats_row.total,
            unique_agents: stats_row.unique_agents,
            confidence,
            tags,
        })
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<Evaluation>> {
        let row = sqlx::query_as::<_, EvaluationRow>(
            r#"
            SELECT id, skill_id, agent_id, success, duration_ms, error_type, tags, timestamp
            FROM evaluations
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn delete_by_id(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM evaluations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn list_by_skill(&self, skill_id: &str, limit: i64) -> DbResult<Vec<Evaluation>> {
        let rows = sqlx::query_as::<_, EvaluationRow>(
            r#"
            SELECT id, skill_id, agent_id, success, duration_ms, error_type, tags, timestamp
            FROM evaluations
            WHERE skill_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
        )
        .bind(skill_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_top_tags(&self, skill_id: &str) -> DbResult<Vec<String>> {
        let tags: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT tag, COUNT(*) as cnt
            FROM evaluations, UNEST(evaluations.tags) as tag
            WHERE skill_id = $1
            GROUP BY tag
            ORDER BY cnt DESC
            LIMIT 5
            "#,
        )
        .bind(skill_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tags.into_iter().map(|(t, _)| t).collect())
    }

    fn calculate_confidence(&self, total: i32) -> f64 {
        if total < 3 {
            total as f64 / 3.0
        } else if total > 10 {
            1.0
        } else {
            (total as f64 - 3.0) / 7.0 + 0.5
        }
    }
}

#[derive(sqlx::FromRow)]
struct EvaluationRow {
    id: Uuid,
    skill_id: String,
    agent_id: String,
    success: bool,
    duration_ms: i64,
    error_type: Option<String>,
    tags: Vec<String>,
    timestamp: DateTime<Utc>,
}

impl From<EvaluationRow> for Evaluation {
    fn from(row: EvaluationRow) -> Self {
        Self {
            id: row.id,
            skill_id: row.skill_id,
            agent_id: row.agent_id,
            success: row.success,
            duration_ms: row.duration_ms,
            error_type: row.error_type,
            tags: row.tags,
            timestamp: row.timestamp,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StatsRow {
    total: i32,
    success_count: i32,
    avg_duration: Option<f64>,
    unique_agents: i32,
}
