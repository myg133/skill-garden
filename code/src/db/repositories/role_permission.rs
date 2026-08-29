use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::role_permission::{NewRolePermission, RolePermission};

#[derive(Clone)]
pub struct RolePermissionRepository {
    pool: PgPool,
}

impl RolePermissionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_by_role(
        &self,
        role_level: &str,
        role_name: &str,
    ) -> DbResult<Vec<RolePermission>> {
        let perms = sqlx::query_as::<_, RolePermissionRow>(
            r#"
            SELECT id, role_level, role_name, permission_code, scope_restriction, created_at
            FROM role_permissions
            WHERE role_level = $1 AND role_name = $2
            ORDER BY permission_code
            "#,
        )
        .bind(role_level)
        .bind(role_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(perms.into_iter().map(|p| p.into()).collect())
    }

    /// 批量查询多个 (role_level, role_name) 组合的权限
    pub async fn list_by_roles_batch(
        &self,
        entries: &[(String, String)],
    ) -> DbResult<Vec<RolePermission>> {
        if entries.is_empty() {
            return Ok(Vec::new());
        }
        let perms = sqlx::query_as::<_, RolePermissionRow>(
            r#"
            SELECT id, role_level, role_name, permission_code, scope_restriction, created_at
            FROM role_permissions
            WHERE (role_level, role_name) IN (
                SELECT * FROM UNNEST($1::text[], $2::text[])
            )
            ORDER BY permission_code
            "#,
        )
        .bind(entries.iter().map(|(l, _)| l.clone()).collect::<Vec<_>>())
        .bind(entries.iter().map(|(_, n)| n.clone()).collect::<Vec<_>>())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(perms.into_iter().map(|p| p.into()).collect())
    }

    pub async fn list_all(&self) -> DbResult<Vec<RolePermission>> {
        let perms = sqlx::query_as::<_, RolePermissionRow>(
            r#"
            SELECT id, role_level, role_name, permission_code, scope_restriction, created_at
            FROM role_permissions
            ORDER BY role_level, role_name, permission_code
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(perms.into_iter().map(|p| p.into()).collect())
    }

    pub async fn add_permission(&self, new_perm: NewRolePermission) -> DbResult<RolePermission> {
        let scope = new_perm
            .scope_restriction
            .unwrap_or_else(|| "none".to_string());

        let perm = sqlx::query_as::<_, RolePermissionRow>(
            r#"
            INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (role_level, role_name, permission_code) DO UPDATE SET scope_restriction = $4
            RETURNING id, role_level, role_name, permission_code, scope_restriction, created_at
            "#,
        )
        .bind(&new_perm.role_level)
        .bind(&new_perm.role_name)
        .bind(&new_perm.permission_code)
        .bind(&scope)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(perm.into())
    }

    pub async fn remove_permission(
        &self,
        role_level: &str,
        role_name: &str,
        permission_code: &str,
    ) -> DbResult<()> {
        sqlx::query(
            "DELETE FROM role_permissions WHERE role_level = $1 AND role_name = $2 AND permission_code = $3",
        )
        .bind(role_level)
        .bind(role_name)
        .bind(permission_code)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct RolePermissionRow {
    id: Uuid,
    role_level: String,
    role_name: String,
    permission_code: String,
    scope_restriction: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<RolePermissionRow> for RolePermission {
    fn from(row: RolePermissionRow) -> Self {
        Self {
            id: row.id,
            role_level: row.role_level,
            role_name: row.role_name,
            permission_code: row.permission_code,
            scope_restriction: row.scope_restriction,
            created_at: row.created_at,
        }
    }
}
