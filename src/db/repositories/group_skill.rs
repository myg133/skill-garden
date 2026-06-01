use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::group_skill::{GroupSkill, NewGroupSkill};

#[derive(Clone)]
pub struct GroupSkillRepository {
    pool: PgPool,
}

impl GroupSkillRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_by_group(&self, group_id: Uuid) -> DbResult<Vec<GroupSkill>> {
        let rows = sqlx::query_as::<_, GroupSkillRow>(
            r#"
            SELECT id, group_id, skill_id, added_by, added_at
            FROM group_skills
            WHERE group_id = $1
            ORDER BY added_at DESC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn list_by_skill(&self, skill_id: &str) -> DbResult<Vec<GroupSkill>> {
        let rows = sqlx::query_as::<_, GroupSkillRow>(
            r#"
            SELECT id, group_id, skill_id, added_by, added_at
            FROM group_skills
            WHERE skill_id = $1
            ORDER BY added_at DESC
            "#,
        )
        .bind(skill_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn associate_skill(
        &self,
        new_gs: NewGroupSkill,
    ) -> DbResult<GroupSkill> {
        let row = sqlx::query_as::<_, GroupSkillRow>(
            r#"
            INSERT INTO group_skills (group_id, skill_id, added_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (group_id, skill_id) DO NOTHING
            RETURNING id, group_id, skill_id, added_by, added_at
            "#,
        )
        .bind(new_gs.group_id)
        .bind(&new_gs.skill_id)
        .bind(new_gs.added_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.into())
    }

    pub async fn dissociate_skill(&self, group_id: Uuid, skill_id: &str) -> DbResult<()> {
        sqlx::query("DELETE FROM group_skills WHERE group_id = $1 AND skill_id = $2")
            .bind(group_id)
            .bind(skill_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(())
    }

    pub async fn is_skill_in_group(&self, group_id: Uuid, skill_id: &str) -> DbResult<bool> {
        let row = sqlx::query_as::<_, GroupSkillRow>(
            r#"
            SELECT id, group_id, skill_id, added_by, added_at
            FROM group_skills
            WHERE group_id = $1 AND skill_id = $2
            "#,
        )
        .bind(group_id)
        .bind(skill_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.is_some())
    }
}

#[derive(sqlx::FromRow)]
struct GroupSkillRow {
    id: Uuid,
    group_id: Uuid,
    skill_id: String,
    added_by: Option<Uuid>,
    added_at: chrono::DateTime<chrono::Utc>,
}

impl From<GroupSkillRow> for GroupSkill {
    fn from(row: GroupSkillRow) -> Self {
        Self {
            id: row.id,
            group_id: row.group_id,
            skill_id: row.skill_id,
            added_by: row.added_by,
            added_at: row.added_at,
        }
    }
}