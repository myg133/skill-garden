use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone, sqlx::FromRow)]
struct SystemRoleAssignmentRow {
    id: Uuid,
    identity_id: Uuid,
    role_name: String,
    assigned_by: Option<Uuid>,
    assigned_at: DateTime<Utc>,
}

impl From<SystemRoleAssignmentRow> for crate::models::SystemRoleAssignment {
    fn from(row: SystemRoleAssignmentRow) -> Self {
        Self {
            id: row.id,
            identity_id: row.identity_id,
            role_name: row.role_name,
            assigned_by: row.assigned_by,
            assigned_at: row.assigned_at,
        }
    }
}

#[derive(Clone)]
pub struct SystemRoleAssignmentRepository {
    pool: PgPool,
}

impl SystemRoleAssignmentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn assign(
        &self,
        identity_id: Uuid,
        role_name: &str,
        assigned_by: Option<Uuid>,
    ) -> DbResult<crate::models::SystemRoleAssignment> {
        let row = sqlx::query_as::<_, SystemRoleAssignmentRow>(
            r#"
            INSERT INTO system_role_assignments (identity_id, role_name, assigned_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (identity_id, role_name) DO NOTHING
            RETURNING id, identity_id, role_name, assigned_by, assigned_at
            "#,
        )
        .bind(identity_id)
        .bind(role_name)
        .bind(assigned_by)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        match row {
            Some(r) => Ok(r.into()),
            None => self
                .find_by_identity_and_role(identity_id, role_name)
                .await?
                .ok_or_else(|| DbError::NotFound("system role assignment".to_string())),
        }
    }

    pub async fn revoke(&self, identity_id: Uuid, role_name: &str) -> DbResult<()> {
        let result = sqlx::query(
            "DELETE FROM system_role_assignments WHERE identity_id = $1 AND role_name = $2",
        )
        .bind(identity_id)
        .bind(role_name)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!(
                "role assignment {} for identity {}",
                role_name, identity_id
            )));
        }
        Ok(())
    }

    pub async fn find_by_identity(
        &self,
        identity_id: Uuid,
    ) -> DbResult<Vec<crate::models::SystemRoleAssignment>> {
        let rows = sqlx::query_as::<_, SystemRoleAssignmentRow>(
            "SELECT id, identity_id, role_name, assigned_by, assigned_at FROM system_role_assignments WHERE identity_id = $1",
        )
        .bind(identity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn find_by_identity_and_role(
        &self,
        identity_id: Uuid,
        role_name: &str,
    ) -> DbResult<Option<crate::models::SystemRoleAssignment>> {
        let row = sqlx::query_as::<_, SystemRoleAssignmentRow>(
            "SELECT id, identity_id, role_name, assigned_by, assigned_at FROM system_role_assignments WHERE identity_id = $1 AND role_name = $2",
        )
        .bind(identity_id)
        .bind(role_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn has_system_role(&self, identity_id: Uuid, role_name: &str) -> DbResult<bool> {
        let row = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM system_role_assignments WHERE identity_id = $1 AND role_name = $2",
        )
        .bind(identity_id)
        .bind(role_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row > 0)
    }

    pub async fn list_by_role(
        &self,
        role_name: &str,
    ) -> DbResult<Vec<crate::models::SystemRoleAssignment>> {
        let rows = sqlx::query_as::<_, SystemRoleAssignmentRow>(
            "SELECT id, identity_id, role_name, assigned_by, assigned_at FROM system_role_assignments WHERE role_name = $1",
        )
        .bind(role_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}
