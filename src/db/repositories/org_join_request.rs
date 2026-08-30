//! Organization Join Request Repository

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrgJoinRequestRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub identity_id: Uuid,
    pub status: String,
    pub message: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrgJoinRequest {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub identity_id: Uuid,
    pub status: String,
    pub message: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl From<OrgJoinRequestRow> for OrgJoinRequest {
    fn from(row: OrgJoinRequestRow) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            identity_id: row.identity_id,
            status: row.status,
            message: row.message,
            reviewed_by: row.reviewed_by,
            reviewed_at: row.reviewed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrgJoinRequestWithIdentityRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub identity_id: Uuid,
    pub status: String,
    pub message: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    // Identity fields
    pub identity_name: String,
    pub identity_email: Option<String>,
    pub identity_username: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrgJoinRequestWithIdentity {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub identity_id: Uuid,
    pub status: String,
    pub message: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub identity: IdentitySummary,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdentitySummary {
    pub id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub username: Option<String>,
}

impl From<OrgJoinRequestWithIdentityRow> for OrgJoinRequestWithIdentity {
    fn from(row: OrgJoinRequestWithIdentityRow) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            identity_id: row.identity_id,
            status: row.status,
            message: row.message,
            reviewed_by: row.reviewed_by,
            reviewed_at: row.reviewed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            identity: IdentitySummary {
                id: row.identity_id,
                name: row.identity_name,
                email: row.identity_email,
                username: row.identity_username,
            },
        }
    }
}

#[derive(Clone)]
pub struct OrgJoinRequestRepository {
    pool: PgPool,
}

impl OrgJoinRequestRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new join request
    pub async fn create(
        &self,
        organization_id: Uuid,
        identity_id: Uuid,
        message: Option<String>,
    ) -> DbResult<OrgJoinRequest> {
        let request = sqlx::query_as::<_, OrgJoinRequestRow>(
            r#"
            INSERT INTO org_join_requests (organization_id, identity_id, message, status)
            VALUES ($1, $2, $3, 'pending')
            RETURNING id, organization_id, identity_id, status, message, reviewed_by, reviewed_at, created_at, updated_at
            "#,
        )
        .bind(organization_id)
        .bind(identity_id)
        .bind(&message)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(request.into())
    }

    /// Find a request by ID
    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<OrgJoinRequest>> {
        let request = sqlx::query_as::<_, OrgJoinRequestRow>(
            r#"
            SELECT id, organization_id, identity_id, status, message, reviewed_by, reviewed_at, created_at, updated_at
            FROM org_join_requests
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(request.map(|r| r.into()))
    }

    /// Find all requests for an organization with optional status filter
    pub async fn find_by_org(
        &self,
        organization_id: Uuid,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<OrgJoinRequestWithIdentity>> {
        let requests = match status {
            Some(s) => {
                sqlx::query_as::<_, OrgJoinRequestWithIdentityRow>(
                    r#"
                    SELECT 
                        r.id, r.organization_id, r.identity_id, r.status, r.message,
                        r.reviewed_by, r.reviewed_at, r.created_at, r.updated_at,
                        i.name as identity_name, i.email as identity_email, i.username as identity_username
                    FROM org_join_requests r
                    JOIN identities i ON r.identity_id = i.id
                    WHERE r.organization_id = $1 AND r.status = $2
                    ORDER BY r.created_at DESC
                    LIMIT $3 OFFSET $4
                    "#,
                )
                .bind(organization_id)
                .bind(s)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, OrgJoinRequestWithIdentityRow>(
                    r#"
                    SELECT 
                        r.id, r.organization_id, r.identity_id, r.status, r.message,
                        r.reviewed_by, r.reviewed_at, r.created_at, r.updated_at,
                        i.name as identity_name, i.email as identity_email, i.username as identity_username
                    FROM org_join_requests r
                    JOIN identities i ON r.identity_id = i.id
                    WHERE r.organization_id = $1
                    ORDER BY r.created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(organization_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(requests.into_iter().map(|r| r.into()).collect())
    }

    /// Find pending request by org and identity
    pub async fn find_pending_by_org_and_identity(
        &self,
        organization_id: Uuid,
        identity_id: Uuid,
    ) -> DbResult<Option<OrgJoinRequest>> {
        let request = sqlx::query_as::<_, OrgJoinRequestRow>(
            r#"
            SELECT id, organization_id, identity_id, status, message, reviewed_by, reviewed_at, created_at, updated_at
            FROM org_join_requests
            WHERE organization_id = $1 AND identity_id = $2 AND status = 'pending'
            "#,
        )
        .bind(organization_id)
        .bind(identity_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(request.map(|r| r.into()))
    }

    /// Update request status (approve/reject)
    pub async fn update_status(
        &self,
        id: Uuid,
        status: &str,
        reviewed_by: Uuid,
    ) -> DbResult<OrgJoinRequest> {
        let request = sqlx::query_as::<_, OrgJoinRequestRow>(
            r#"
            UPDATE org_join_requests
            SET status = $1, reviewed_by = $2, reviewed_at = NOW(), updated_at = NOW()
            WHERE id = $3
            RETURNING id, organization_id, identity_id, status, message, reviewed_by, reviewed_at, created_at, updated_at
            "#,
        )
        .bind(status)
        .bind(reviewed_by)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(request.into())
    }

    /// Delete a request by ID
    pub async fn delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM org_join_requests WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    /// Delete pending request by org and identity (for user cancellation)
    pub async fn delete_pending_by_org_and_identity(
        &self,
        organization_id: Uuid,
        identity_id: Uuid,
    ) -> DbResult<()> {
        sqlx::query(
            r#"
            DELETE FROM org_join_requests
            WHERE organization_id = $1 AND identity_id = $2 AND status = 'pending'
            "#,
        )
        .bind(organization_id)
        .bind(identity_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    /// Count requests by org and status
    pub async fn count_by_org_and_status(
        &self,
        organization_id: Uuid,
        status: Option<&str>,
    ) -> DbResult<i64> {
        let count: i64 = match status {
            Some(s) => {
                sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*) FROM org_join_requests
                    WHERE organization_id = $1 AND status = $2
                    "#,
                )
                .bind(organization_id)
                .bind(s)
                .fetch_one(&self.pool)
                .await
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*) FROM org_join_requests
                    WHERE organization_id = $1
                    "#,
                )
                .bind(organization_id)
                .fetch_one(&self.pool)
                .await
            }
        }
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(count)
    }
}
