//! Role Repository

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::role::{GrantRoleRequest, IdentityRole, NewRole, Role, RoleType, RoleUpdate};

#[derive(Clone)]
pub struct RoleRepository {
    pool: PgPool,
}

impl RoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<Role>> {
        let role = sqlx::query_as::<_, RoleRow>(
            r#"
            SELECT id, name, role_type, scope_level, parent_role_id, permissions, description, created_at
            FROM roles WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(role.map(|r| r.into()))
    }

    pub async fn find_by_name(&self, name: &str, role_type: RoleType) -> DbResult<Option<Role>> {
        let role = sqlx::query_as::<_, RoleRow>(
            r#"
            SELECT id, name, role_type, scope_level, parent_role_id, permissions, description, created_at
            FROM roles WHERE name = $1 AND role_type = $2
            "#,
        )
        .bind(name)
        .bind(role_type.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(role.map(|r| r.into()))
    }

    pub async fn list_by_type(&self, role_type: RoleType) -> DbResult<Vec<Role>> {
        let roles = sqlx::query_as::<_, RoleRow>(
            r#"
            SELECT id, name, role_type, scope_level, parent_role_id, permissions, description, created_at
            FROM roles WHERE role_type = $1
            ORDER BY name
            "#,
        )
        .bind(role_type.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(roles.into_iter().map(|r| r.into()).collect())
    }

    pub async fn list_all(&self) -> DbResult<Vec<Role>> {
        let roles = sqlx::query_as::<_, RoleRow>(
            r#"
            SELECT id, name, role_type, scope_level, parent_role_id, permissions, description, created_at
            FROM roles
            ORDER BY role_type, name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(roles.into_iter().map(|r| r.into()).collect())
    }

    pub async fn create(&self, new_role: NewRole) -> DbResult<Role> {
        let permissions_json =
            serde_json::to_value(&new_role.permissions).unwrap_or(serde_json::json!([]));

        let role = sqlx::query_as::<_, RoleRow>(
            r#"
            INSERT INTO roles (name, role_type, scope_level, parent_role_id, permissions, description)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, name, role_type, scope_level, parent_role_id, permissions, description, created_at
            "#,
        )
        .bind(&new_role.name)
        .bind(new_role.role_type.to_string())
        .bind(new_role.scope_level.to_string())
        .bind(&new_role.parent_role_id)
        .bind(&permissions_json)
        .bind(&new_role.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(role.into())
    }

    pub async fn update(&self, id: Uuid, update: RoleUpdate) -> DbResult<Role> {
        let current = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| DbError::NotFound("Role not found".to_string()))?;

        let name = update.name.unwrap_or(current.name.clone());
        let permissions = update.permissions.unwrap_or(current.permissions);
        let description = update.description.or(current.description);

        let permissions_json = serde_json::to_value(&permissions).unwrap_or(serde_json::json!([]));

        let role = sqlx::query_as::<_, RoleRow>(
            r#"
            UPDATE roles
            SET name = $1, permissions = $2, description = $3
            WHERE id = $4
            RETURNING id, name, role_type, scope_level, parent_role_id, permissions, description, created_at
            "#,
        )
        .bind(&name)
        .bind(&permissions_json)
        .bind(&description)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(role.into())
    }

    pub async fn delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM roles WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn grant_role(
        &self,
        request: GrantRoleRequest,
        granted_by: Uuid,
    ) -> DbResult<IdentityRole> {
        let role = sqlx::query_as::<_, IdentityRoleRow>(
            r#"
            INSERT INTO identity_roles (identity_id, role_id, scope_id, granted_by, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (identity_id, role_id, scope_id) DO UPDATE SET expires_at = $5
            RETURNING id, identity_id, role_id, scope_id, granted_by, granted_at, expires_at
            "#,
        )
        .bind(request.identity_id)
        .bind(request.role_id)
        .bind(&request.scope_id)
        .bind(granted_by)
        .bind(&request.expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(role.into())
    }

    pub async fn revoke_role(
        &self,
        identity_id: Uuid,
        role_id: Uuid,
        scope_id: Option<Uuid>,
    ) -> DbResult<()> {
        sqlx::query(
            r#"DELETE FROM identity_roles WHERE identity_id = $1 AND role_id = $2 AND scope_id IS NOT DISTINCT FROM $3"#,
        )
        .bind(identity_id)
        .bind(role_id)
        .bind(&scope_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_identity_roles(&self, identity_id: Uuid) -> DbResult<Vec<IdentityRole>> {
        let roles = sqlx::query_as::<_, IdentityRoleRow>(
            r#"
            SELECT id, identity_id, role_id, scope_id, granted_by, granted_at, expires_at
            FROM identity_roles
            WHERE identity_id = $1 AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(identity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(roles.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_identity_permissions(&self, identity_id: Uuid) -> DbResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT p.code
            FROM identity_roles ir
            JOIN roles r ON ir.role_id = r.id
            JOIN LATERAL jsonb_array_elements_text(r.permissions) AS p(code) ON true
            WHERE ir.identity_id = $1 AND (ir.expires_at IS NULL OR ir.expires_at > NOW())
            "#,
        )
        .bind(identity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|(code,)| code).collect())
    }

    pub async fn has_permission(&self, identity_id: Uuid, permission: &str) -> DbResult<bool> {
        let perms = self.get_identity_permissions(identity_id).await?;
        Ok(perms.iter().any(|p| {
            p == "*" || p == permission || permission.starts_with(&p[..p.len().saturating_sub(1)])
        }))
    }
}

#[derive(sqlx::FromRow)]
struct RoleRow {
    id: Uuid,
    name: String,
    role_type: String,
    scope_level: String,
    parent_role_id: Option<Uuid>,
    permissions: serde_json::Value,
    description: Option<String>,
    created_at: chrono::DateTime<Utc>,
}

impl From<RoleRow> for Role {
    fn from(row: RoleRow) -> Self {
        let permissions: Vec<String> = serde_json::from_value(row.permissions).unwrap_or_default();
        Self {
            id: row.id,
            name: row.name,
            role_type: RoleType::from(row.role_type.as_str()),
            scope_level: crate::models::role::ScopeLevel::from(row.scope_level.as_str()),
            parent_role_id: row.parent_role_id,
            permissions,
            description: row.description,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct IdentityRoleRow {
    id: Uuid,
    identity_id: Uuid,
    role_id: Uuid,
    scope_id: Option<Uuid>,
    granted_by: Option<Uuid>,
    granted_at: chrono::DateTime<Utc>,
    expires_at: Option<chrono::DateTime<Utc>>,
}

impl From<IdentityRoleRow> for IdentityRole {
    fn from(row: IdentityRoleRow) -> Self {
        Self {
            id: row.id,
            identity_id: row.identity_id,
            role_id: row.role_id,
            scope_id: row.scope_id,
            granted_by: row.granted_by,
            granted_at: row.granted_at,
            expires_at: row.expires_at,
        }
    }
}
