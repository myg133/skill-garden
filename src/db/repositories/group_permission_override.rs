use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::group_permission_override::{
    GroupPermissionOverride, NewGroupPermissionOverride,
};

#[derive(Clone)]
pub struct GroupPermissionOverrideRepository {
    pool: PgPool,
}

impl GroupPermissionOverrideRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn list_by_group(&self, group_id: Uuid) -> DbResult<Vec<GroupPermissionOverride>> {
        let overrides = sqlx::query_as::<_, GroupPermissionOverrideRow>(
            r#"
            SELECT id, group_id, role_name, permission_code, granted, created_by, created_at
            FROM group_permission_overrides
            WHERE group_id = $1
            ORDER BY role_name, permission_code
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(overrides.into_iter().map(|o| o.into()).collect())
    }

    pub async fn list_by_group_and_role(
        &self,
        group_id: Uuid,
        role_name: &str,
    ) -> DbResult<Vec<GroupPermissionOverride>> {
        let overrides = sqlx::query_as::<_, GroupPermissionOverrideRow>(
            r#"
            SELECT id, group_id, role_name, permission_code, granted, created_by, created_at
            FROM group_permission_overrides
            WHERE group_id = $1 AND role_name = $2
            ORDER BY permission_code
            "#,
        )
        .bind(group_id)
        .bind(role_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(overrides.into_iter().map(|o| o.into()).collect())
    }

    pub async fn find_by_group_role_permission(
        &self,
        group_id: Uuid,
        role_name: &str,
        permission_code: &str,
    ) -> DbResult<Option<GroupPermissionOverride>> {
        let row = sqlx::query_as::<_, GroupPermissionOverrideRow>(
            r#"
            SELECT id, group_id, role_name, permission_code, granted, created_by, created_at
            FROM group_permission_overrides
            WHERE group_id = $1 AND role_name = $2 AND permission_code = $3
            "#,
        )
        .bind(group_id)
        .bind(role_name)
        .bind(permission_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.map(|o| o.into()))
    }

    pub async fn upsert_override(
        &self,
        new_override: NewGroupPermissionOverride,
    ) -> DbResult<GroupPermissionOverride> {
        let row = sqlx::query_as::<_, GroupPermissionOverrideRow>(
            r#"
            INSERT INTO group_permission_overrides (group_id, role_name, permission_code, granted, created_by)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (group_id, role_name, permission_code) DO UPDATE SET granted = $4
            RETURNING id, group_id, role_name, permission_code, granted, created_by, created_at
            "#,
        )
        .bind(new_override.group_id)
        .bind(&new_override.role_name)
        .bind(&new_override.permission_code)
        .bind(new_override.granted)
        .bind(new_override.created_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.into())
    }

    pub async fn delete_override(
        &self,
        group_id: Uuid,
        role_name: &str,
        permission_code: &str,
    ) -> DbResult<()> {
        sqlx::query(
            "DELETE FROM group_permission_overrides WHERE group_id = $1 AND role_name = $2 AND permission_code = $3",
        )
        .bind(group_id)
        .bind(role_name)
        .bind(permission_code)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct GroupPermissionOverrideRow {
    id: Uuid,
    group_id: Uuid,
    role_name: String,
    permission_code: String,
    granted: bool,
    created_by: Option<Uuid>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<GroupPermissionOverrideRow> for GroupPermissionOverride {
    fn from(row: GroupPermissionOverrideRow) -> Self {
        Self {
            id: row.id,
            group_id: row.group_id,
            role_name: row.role_name,
            permission_code: row.permission_code,
            granted: row.granted,
            created_by: row.created_by,
            created_at: row.created_at,
        }
    }
}
