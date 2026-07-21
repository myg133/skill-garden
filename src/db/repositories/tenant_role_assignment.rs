use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone, sqlx::FromRow)]
struct TenantRoleAssignmentRow {
    id: Uuid,
    identity_id: Uuid,
    tenant_id: Uuid,
    role_name: String,
    assigned_by: Option<Uuid>,
    assigned_at: DateTime<Utc>,
}

impl From<TenantRoleAssignmentRow> for crate::models::TenantRoleAssignment {
    fn from(row: TenantRoleAssignmentRow) -> Self {
        Self {
            id: row.id,
            identity_id: row.identity_id,
            tenant_id: row.tenant_id,
            role_name: row.role_name,
            assigned_by: row.assigned_by,
            assigned_at: row.assigned_at,
        }
    }
}

#[derive(Clone)]
pub struct TenantRoleAssignmentRepository {
    pool: PgPool,
}

impl TenantRoleAssignmentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn assign(
        &self,
        identity_id: Uuid,
        tenant_id: Uuid,
        role_name: &str,
        assigned_by: Option<Uuid>,
    ) -> DbResult<crate::models::TenantRoleAssignment> {
        let row = sqlx::query_as::<_, TenantRoleAssignmentRow>(
            r#"
            INSERT INTO tenant_role_assignments (identity_id, tenant_id, role_name, assigned_by)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (identity_id, tenant_id, role_name) DO NOTHING
            RETURNING id, identity_id, tenant_id, role_name, assigned_by, assigned_at
            "#,
        )
        .bind(identity_id)
        .bind(tenant_id)
        .bind(role_name)
        .bind(assigned_by)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        match row {
            Some(r) => Ok(r.into()),
            None => self
                .find_by_identity_tenant_role(identity_id, tenant_id, role_name)
                .await?
                .ok_or_else(|| DbError::NotFound("tenant role assignment".to_string())),
        }
    }

    pub async fn revoke(
        &self,
        identity_id: Uuid,
        tenant_id: Uuid,
        role_name: &str,
    ) -> DbResult<()> {
        let result = sqlx::query(
            "DELETE FROM tenant_role_assignments WHERE identity_id = $1 AND tenant_id = $2 AND role_name = $3",
        )
        .bind(identity_id)
        .bind(tenant_id)
        .bind(role_name)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!(
                "tenant role assignment {} for identity {} in tenant {}",
                role_name, identity_id, tenant_id
            )));
        }
        Ok(())
    }

    /// 获取用户在指定租户下的所有角色
    pub async fn find_by_identity(
        &self,
        identity_id: Uuid,
    ) -> DbResult<Vec<crate::models::TenantRoleAssignment>> {
        let rows = sqlx::query_as::<_, TenantRoleAssignmentRow>(
            r#"SELECT id, identity_id, tenant_id, role_name, assigned_by, assigned_at
               FROM tenant_role_assignments WHERE identity_id = $1"#,
        )
        .bind(identity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn find_by_identity_and_tenant(
        &self,
        identity_id: Uuid,
        tenant_id: Uuid,
    ) -> DbResult<Vec<crate::models::TenantRoleAssignment>> {
        let rows = sqlx::query_as::<_, TenantRoleAssignmentRow>(
            r#"SELECT id, identity_id, tenant_id, role_name, assigned_by, assigned_at
               FROM tenant_role_assignments WHERE identity_id = $1 AND tenant_id = $2"#,
        )
        .bind(identity_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn find_by_identity_tenant_role(
        &self,
        identity_id: Uuid,
        tenant_id: Uuid,
        role_name: &str,
    ) -> DbResult<Option<crate::models::TenantRoleAssignment>> {
        let row = sqlx::query_as::<_, TenantRoleAssignmentRow>(
            r#"SELECT id, identity_id, tenant_id, role_name, assigned_by, assigned_at
               FROM tenant_role_assignments
               WHERE identity_id = $1 AND tenant_id = $2 AND role_name = $3"#,
        )
        .bind(identity_id)
        .bind(tenant_id)
        .bind(role_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn has_tenant_role(
        &self,
        identity_id: Uuid,
        tenant_id: Uuid,
        role_name: &str,
    ) -> DbResult<bool> {
        let row = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tenant_role_assignments WHERE identity_id = $1 AND tenant_id = $2 AND role_name = $3",
        )
        .bind(identity_id)
        .bind(tenant_id)
        .bind(role_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row > 0)
    }

    pub async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
    ) -> DbResult<Vec<crate::models::TenantRoleAssignment>> {
        let rows = sqlx::query_as::<_, TenantRoleAssignmentRow>(
            r#"SELECT id, identity_id, tenant_id, role_name, assigned_by, assigned_at
               FROM tenant_role_assignments WHERE tenant_id = $1"#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}
