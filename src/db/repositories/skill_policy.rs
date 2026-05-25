//! Skill Policy repository

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone)]
pub struct SkillPolicy {
    pub id: Uuid,
    pub org_id: Uuid,
    pub skill_id: Uuid,
    pub visibility: String,
    pub allowed_agents: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewSkillPolicy {
    pub org_id: Uuid,
    pub skill_id: Uuid,
    pub visibility: String,
    pub allowed_agents: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct SkillPolicyRepository {
    pool: PgPool,
}

impl SkillPolicyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_policy: NewSkillPolicy) -> DbResult<SkillPolicy> {
        let allowed_agents = new_policy.allowed_agents.unwrap_or_default();
        let visibility = if new_policy.visibility.is_empty() {
            "org_visible".to_string()
        } else {
            new_policy.visibility
        };

        let policy = sqlx::query_as::<_, SkillPolicyRow>(
            r#"
            INSERT INTO skill_policies (org_id, skill_id, visibility, allowed_agents)
            VALUES ($1, $2, $3, $4)
            RETURNING id, org_id, skill_id, visibility, allowed_agents, created_at
            "#,
        )
        .bind(new_policy.org_id)
        .bind(new_policy.skill_id.to_string())
        .bind(&visibility)
        .bind(&allowed_agents)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(policy.into())
    }

    pub async fn find_by_org_and_skill(&self, org_id: Uuid, skill_id: Uuid) -> DbResult<Option<SkillPolicy>> {
        let policy = sqlx::query_as::<_, SkillPolicyRow>(
            r#"
            SELECT id, org_id, skill_id, visibility, allowed_agents, created_at
            FROM skill_policies
            WHERE org_id = $1 AND skill_id = $2
            "#,
        )
        .bind(org_id)
        .bind(skill_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(policy.map(|p| p.into()))
    }

    pub async fn list_by_org(&self, org_id: Uuid) -> DbResult<Vec<SkillPolicy>> {
        let policies = sqlx::query_as::<_, SkillPolicyRow>(
            r#"
            SELECT id, org_id, skill_id, visibility, allowed_agents, created_at
            FROM skill_policies
            WHERE org_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(policies.into_iter().map(|p| p.into()).collect())
    }

    pub async fn update_visibility(&self, id: Uuid, visibility: &str) -> DbResult<()> {
        sqlx::query("UPDATE skill_policies SET visibility = $1 WHERE id = $2")
            .bind(visibility)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn update_allowed_agents(&self, id: Uuid, allowed_agents: Vec<String>) -> DbResult<()> {
        sqlx::query("UPDATE skill_policies SET allowed_agents = $1 WHERE id = $2")
            .bind(&allowed_agents)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM skill_policies WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct SkillPolicyRow {
    id: Uuid,
    org_id: Uuid,
    skill_id: String,
    visibility: String,
    allowed_agents: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<SkillPolicyRow> for SkillPolicy {
    fn from(row: SkillPolicyRow) -> Self {
        Self {
            id: row.id,
            org_id: row.org_id,
            skill_id: Uuid::parse_str(&row.skill_id).unwrap_or_default(),
            visibility: row.visibility,
            allowed_agents: row.allowed_agents,
            created_at: row.created_at,
        }
    }
}
