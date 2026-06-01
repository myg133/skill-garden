//! Organization Membership Repository

use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::org_membership::{OrgMemberInfo, OrgMembership, OrgRole};

#[derive(Clone)]
pub struct OrgMembershipRepository {
    pool: PgPool,
}

impl OrgMembershipRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn add_member(
        &self,
        identity_id: Uuid,
        organization_id: Uuid,
        role: OrgRole,
        invited_by: Option<Uuid>,
    ) -> DbResult<OrgMembership> {
        let membership = sqlx::query_as::<_, OrgMembershipRow>(
            r#"
            INSERT INTO org_memberships (identity_id, organization_id, role, invited_by)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (identity_id, organization_id) DO UPDATE SET role = $3, invited_by = $4
            RETURNING id, identity_id, organization_id, role, joined_at, invited_by
            "#,
        )
        .bind(identity_id)
        .bind(organization_id)
        .bind(role.to_string())
        .bind(invited_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(membership.into())
    }

    pub async fn remove_member(&self, identity_id: Uuid, organization_id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM org_memberships WHERE identity_id = $1 AND organization_id = $2")
            .bind(identity_id)
            .bind(organization_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_member(&self, identity_id: Uuid, organization_id: Uuid) -> DbResult<Option<OrgMembership>> {
        let membership = sqlx::query_as::<_, OrgMembershipRow>(
            r#"
            SELECT id, identity_id, organization_id, role, joined_at, invited_by
            FROM org_memberships
            WHERE identity_id = $1 AND organization_id = $2
            "#,
        )
        .bind(identity_id)
        .bind(organization_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(membership.map(|m| m.into()))
    }

    pub async fn update_role(&self, identity_id: Uuid, organization_id: Uuid, role: OrgRole) -> DbResult<OrgMembership> {
        let membership = sqlx::query_as::<_, OrgMembershipRow>(
            r#"
            UPDATE org_memberships SET role = $1
            WHERE identity_id = $2 AND organization_id = $3
            RETURNING id, identity_id, organization_id, role, joined_at, invited_by
            "#,
        )
        .bind(role.to_string())
        .bind(identity_id)
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(membership.into())
    }

    pub async fn list_members(&self, organization_id: Uuid) -> DbResult<Vec<OrgMemberInfo>> {
        let members = sqlx::query_as::<_, OrgMemberInfoRow>(
            r#"
            SELECT
                i.id as identity_id,
                i.username,
                i.email,
                i.display_name,
                i.name,
                i.identity_type,
                om.role,
                om.joined_at
            FROM org_memberships om
            JOIN identities i ON i.id = om.identity_id
            WHERE om.organization_id = $1
            ORDER BY om.joined_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(members.into_iter().map(|m| m.into()).collect())
    }

    pub async fn list_user_organizations(&self, identity_id: Uuid) -> DbResult<Vec<(Uuid, String)>> {
        let orgs = sqlx::query_as::<_, OrgSummary>(
            r#"
            SELECT o.id, o.name, om.role
            FROM org_memberships om
            JOIN organizations o ON o.id = om.organization_id
            WHERE om.identity_id = $1
            ORDER BY o.name
            "#,
        )
        .bind(identity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(orgs.into_iter().map(|o| (o.id, o.role)).collect())
    }
}

#[derive(sqlx::FromRow)]
struct OrgMembershipRow {
    id: Uuid,
    identity_id: Uuid,
    organization_id: Uuid,
    role: String,
    joined_at: chrono::DateTime<chrono::Utc>,
    invited_by: Option<Uuid>,
}

impl From<OrgMembershipRow> for OrgMembership {
    fn from(row: OrgMembershipRow) -> Self {
        Self {
            id: row.id,
            identity_id: row.identity_id,
            organization_id: row.organization_id,
            role: OrgRole::from(row.role.as_str()),
            joined_at: row.joined_at,
            invited_by: row.invited_by,
        }
    }
}

#[derive(sqlx::FromRow)]
struct OrgMemberInfoRow {
    identity_id: Uuid,
    username: Option<String>,
    email: Option<String>,
    display_name: Option<String>,
    name: String,
    identity_type: String,
    role: String,
    joined_at: chrono::DateTime<chrono::Utc>,
}

impl From<OrgMemberInfoRow> for OrgMemberInfo {
    fn from(row: OrgMemberInfoRow) -> Self {
        Self {
            identity_id: row.identity_id,
            username: row.username,
            email: row.email,
            display_name: row.display_name,
            name: row.name,
            identity_type: row.identity_type,
            role: row.role,
            joined_at: row.joined_at,
        }
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct OrgSummary {
    id: Uuid,
    name: String,
    role: String,
}