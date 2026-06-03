//! Group Repository

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::group::{Group, GroupMember, GroupType, NewGroup, GroupUpdate, Membership};

#[derive(Clone)]
pub struct GroupRepository {
    pool: PgPool,
}

impl GroupRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_group: NewGroup) -> DbResult<Group> {
        let settings = new_group.settings.clone();

        let group = sqlx::query_as::<_, GroupRow>(
            r#"
            INSERT INTO groups (organization_id, name, slug, description, group_type, settings)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, organization_id, name, slug, description, group_type, settings, created_at, updated_at
            "#,
        )
        .bind(new_group.organization_id)
        .bind(&new_group.name)
        .bind(&new_group.slug)
        .bind(&new_group.description)
        .bind(new_group.group_type.to_string())
        .bind(&settings)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                DbError::AlreadyExists(format!("Group with slug '{}' already exists in organization", new_group.slug))
            } else {
                DbError::QueryError(e.to_string())
            }
        })?;

        Ok(group.into())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<Group>> {
        let group = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT id, organization_id, name, slug, description, group_type, settings, created_at, updated_at
            FROM groups WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(group.map(|g| g.into()))
    }

    pub async fn find_by_org(&self, organization_id: Uuid, slug: &str) -> DbResult<Option<Group>> {
        let group = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT id, organization_id, name, slug, description, group_type, settings, created_at, updated_at
            FROM groups WHERE organization_id = $1 AND slug = $2
            "#,
        )
        .bind(organization_id)
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(group.map(|g| g.into()))
    }

    pub async fn list_by_organization(&self, organization_id: Uuid) -> DbResult<Vec<Group>> {
        let groups = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT id, organization_id, name, slug, description, group_type, settings, created_at, updated_at
            FROM groups
            WHERE organization_id = $1
            ORDER BY name
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(groups.into_iter().map(|g| g.into()).collect())
    }

    pub async fn list(&self) -> DbResult<Vec<Group>> {
        let groups = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT id, organization_id, name, slug, description, group_type, settings, created_at, updated_at
            FROM groups
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(groups.into_iter().map(|g| g.into()).collect())
    }

    /// Return all groups that belong to an organization whose tenant_id is
    /// in `tenant_ids`. Used by the tenant-scope guard (Task 8) to filter
    /// the groups list endpoint to the caller's accessible tenants. The
    /// join path is `groups.organization_id -> organizations.id ->
    /// organizations.tenant_id` because the Group model has no direct
    /// tenant_id.
    pub async fn list_by_org_tenants(
        &self,
        tenant_ids: &[Uuid],
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<Group>> {
        let groups = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT g.id, g.organization_id, g.name, g.slug, g.description, g.group_type, g.settings, g.created_at, g.updated_at
            FROM groups g
            JOIN organizations o ON o.id = g.organization_id
            WHERE o.tenant_id = ANY($1)
            ORDER BY g.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_ids)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(groups.into_iter().map(|g| g.into()).collect())
    }

    pub async fn update(&self, id: Uuid, update: GroupUpdate) -> DbResult<Group> {
        let current = self.find_by_id(id).await?.ok_or_else(|| DbError::NotFound("Group not found".to_string()))?;

        let name = update.name.unwrap_or(current.name);
        let slug = update.slug.unwrap_or(current.slug);
        let description = update.description.or(current.description);
        let group_type = update.group_type.unwrap_or(current.group_type);
        let settings = update.settings.unwrap_or(current.settings);

        let group = sqlx::query_as::<_, GroupRow>(
            r#"
            UPDATE groups
            SET name = $1, slug = $2, description = $3, group_type = $4, settings = $5, updated_at = NOW()
            WHERE id = $6
            RETURNING id, organization_id, name, slug, description, group_type, settings, created_at, updated_at
            "#,
        )
        .bind(&name)
        .bind(&slug)
        .bind(&description)
        .bind(group_type.to_string())
        .bind(&settings)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(group.into())
    }

    pub async fn delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM groups WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn add_member(&self, identity_id: Uuid, group_id: Uuid, role: &str) -> DbResult<Membership> {
        let membership = sqlx::query_as::<_, MembershipRow>(
            r#"
            INSERT INTO memberships (identity_id, group_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (identity_id, group_id) DO UPDATE SET role = $3
            RETURNING id, identity_id, group_id, role, joined_at
            "#,
        )
        .bind(identity_id)
        .bind(group_id)
        .bind(role)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(membership.into())
    }

    pub async fn remove_member(&self, identity_id: Uuid, group_id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM memberships WHERE identity_id = $1 AND group_id = $2")
            .bind(identity_id)
            .bind(group_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn update_member_role(&self, identity_id: Uuid, group_id: Uuid, role: &str) -> DbResult<()> {
        let rows_affected = sqlx::query(
            "UPDATE memberships SET role = $1 WHERE identity_id = $2 AND group_id = $3",
        )
        .bind(role)
        .bind(identity_id)
        .bind(group_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?
        .rows_affected();

        if rows_affected == 0 {
            return Err(DbError::NotFound("Group membership not found".to_string()));
        }
        Ok(())
    }

    pub async fn list_members(&self, group_id: Uuid) -> DbResult<Vec<GroupMember>> {
        let members = sqlx::query_as::<_, GroupMemberRow>(
            r#"
            SELECT m.identity_id, i.name as identity_name, i.identity_type, i.email, i.username, m.role, m.joined_at
            FROM memberships m
            JOIN identities i ON m.identity_id = i.id
            WHERE m.group_id = $1
            ORDER BY m.joined_at
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(members.into_iter().map(|m| m.into()).collect())
    }

    pub async fn get_identity_groups(&self, identity_id: Uuid) -> DbResult<Vec<Group>> {
        let groups = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT g.id, g.organization_id, g.name, g.slug, g.description, g.group_type, g.settings, g.created_at, g.updated_at
            FROM groups g
            JOIN memberships m ON g.id = m.group_id
            WHERE m.identity_id = $1
            ORDER BY g.name
            "#,
        )
        .bind(identity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(groups.into_iter().map(|g| g.into()).collect())
    }

    pub async fn list_user_group_memberships(&self, identity_id: Uuid) -> DbResult<Vec<(Uuid, String)>> {
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT group_id, role
            FROM memberships
            WHERE identity_id = $1
            "#,
        )
        .bind(identity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows)
    }
}

#[derive(sqlx::FromRow)]
struct GroupRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    slug: String,
    description: Option<String>,
    group_type: String,
    settings: serde_json::Value,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl From<GroupRow> for Group {
    fn from(row: GroupRow) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            name: row.name,
            slug: row.slug,
            description: row.description,
            group_type: GroupType::from(row.group_type.as_str()),
            settings: row.settings,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct MembershipRow {
    id: Uuid,
    identity_id: Uuid,
    group_id: Uuid,
    role: String,
    joined_at: chrono::DateTime<Utc>,
}

impl From<MembershipRow> for Membership {
    fn from(row: MembershipRow) -> Self {
        Self {
            id: row.id,
            identity_id: row.identity_id,
            group_id: row.group_id,
            role: row.role,
            joined_at: row.joined_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct GroupMemberRow {
    identity_id: Uuid,
    identity_name: String,
    identity_type: String,
    email: Option<String>,
    username: Option<String>,
    role: String,
    joined_at: chrono::DateTime<Utc>,
}

impl From<GroupMemberRow> for GroupMember {
    fn from(row: GroupMemberRow) -> Self {
        Self {
            identity_id: row.identity_id,
            identity_name: row.identity_name,
            identity_type: row.identity_type,
            email: row.email,
            username: row.username,
            role: row.role,
            joined_at: row.joined_at,
        }
    }
}
