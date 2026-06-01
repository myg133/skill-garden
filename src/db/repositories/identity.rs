//! Identity Repository

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::identity::{Identity, IdentityStatus, IdentityType, NewIdentity, IdentityUpdate};

#[derive(Clone)]
pub struct IdentityRepository {
    pool: PgPool,
}

impl IdentityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_identity: NewIdentity) -> DbResult<Identity> {
        let metadata = new_identity.metadata.unwrap_or(serde_json::json!({}));
        let username = new_identity.username.clone().unwrap_or_else(|| new_identity.name.clone());
        let display_name = new_identity.display_name.clone().unwrap_or_else(|| new_identity.name.clone());

        let identity = sqlx::query_as::<_, IdentityRow>(
            r#"
            INSERT INTO identities (identity_type, username, display_name, external_id, name, email, avatar_url, password_hash, is_system_admin, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, identity_type, username, display_name, external_id, name, email, avatar_url, password_hash, is_system_admin, status, metadata, created_at, updated_at
            "#,
        )
        .bind(new_identity.identity_type.to_string())
        .bind(&username)
        .bind(&display_name)
        .bind(&new_identity.external_id)
        .bind(&new_identity.name)
        .bind(&new_identity.email)
        .bind(&new_identity.avatar_url)
        .bind(&new_identity.password_hash)
        .bind(new_identity.is_system_admin)
        .bind(&metadata)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(identity.into())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<Identity>> {
        let identity = sqlx::query_as::<_, IdentityRow>(
            r#"
            SELECT id, identity_type, username, display_name, external_id, name, email, avatar_url, password_hash, is_system_admin, status, metadata, created_at, updated_at
            FROM identities WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(identity.map(|i| i.into()))
    }

    pub async fn find_by_username(&self, username: &str) -> DbResult<Option<Identity>> {
        let identity = sqlx::query_as::<_, IdentityRow>(
            r#"
            SELECT id, identity_type, username, display_name, external_id, name, email, avatar_url, password_hash, is_system_admin, status, metadata, created_at, updated_at
            FROM identities WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(identity.map(|i| i.into()))
    }

    pub async fn find_by_email(&self, email: &str) -> DbResult<Option<Identity>> {
        let identity = sqlx::query_as::<_, IdentityRow>(
            r#"
            SELECT id, identity_type, username, display_name, external_id, name, email, avatar_url, password_hash, is_system_admin, status, metadata, created_at, updated_at
            FROM identities WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(identity.map(|i| i.into()))
    }

    pub async fn find_by_external_id(&self, external_id: &str) -> DbResult<Option<Identity>> {
        let identity = sqlx::query_as::<_, IdentityRow>(
            r#"
            SELECT id, identity_type, username, display_name, external_id, name, email, avatar_url, password_hash, is_system_admin, status, metadata, created_at, updated_at
            FROM identities WHERE external_id = $1
            "#,
        )
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(identity.map(|i| i.into()))
    }

    pub async fn list_all(&self, limit: i64, offset: i64, identity_type: Option<&str>) -> DbResult<Vec<Identity>> {
        let identities = match identity_type {
            Some(t) => {
                sqlx::query_as::<_, IdentityRow>(
                    r#"
                    SELECT id, identity_type, username, display_name, external_id, name, email, avatar_url, password_hash, is_system_admin, status, metadata, created_at, updated_at
                    FROM identities
                    WHERE identity_type = $1
                    ORDER BY created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(t)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, IdentityRow>(
                    r#"
                    SELECT id, identity_type, username, display_name, external_id, name, email, avatar_url, password_hash, is_system_admin, status, metadata, created_at, updated_at
                    FROM identities
                    ORDER BY created_at DESC
                    LIMIT $1 OFFSET $2
                    "#,
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(identities.into_iter().map(|i| i.into()).collect())
    }

    pub async fn update(&self, id: Uuid, update: IdentityUpdate) -> DbResult<Identity> {
        let current = self.find_by_id(id).await?.ok_or_else(|| DbError::NotFound("Identity not found".to_string()))?;

        let name = update.name.unwrap_or(current.name.clone());
        let display_name = update.display_name.or(current.display_name);
        let email = update.email.or(current.email);
        let avatar_url = update.avatar_url.or(current.avatar_url);
        let password_hash = update.password_hash.or(current.password_hash);
        let status = update.status.unwrap_or(current.status);
        let is_system_admin = update.is_system_admin.unwrap_or(current.is_system_admin);
        let metadata = update.metadata.unwrap_or(current.metadata);

        let identity = sqlx::query_as::<_, IdentityRow>(
            r#"
            UPDATE identities
            SET name = $1, display_name = $2, email = $3, avatar_url = $4, password_hash = $5, status = $6, is_system_admin = $7, metadata = $8, updated_at = NOW()
            WHERE id = $9
            RETURNING id, identity_type, username, display_name, external_id, name, email, avatar_url, password_hash, is_system_admin, status, metadata, created_at, updated_at
            "#,
        )
        .bind(&name)
        .bind(&display_name)
        .bind(&email)
        .bind(&avatar_url)
        .bind(&password_hash)
        .bind(status.to_string())
        .bind(is_system_admin)
        .bind(&metadata)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(identity.into())
    }

    pub async fn delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM identities WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn exists(&self, id: Uuid) -> DbResult<bool> {
        let result: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identities WHERE id = $1)")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(result)
    }
}

#[derive(sqlx::FromRow)]
struct IdentityRow {
    id: Uuid,
    identity_type: String,
    username: Option<String>,
    display_name: Option<String>,
    external_id: Option<String>,
    name: String,
    email: Option<String>,
    avatar_url: Option<String>,
    password_hash: Option<String>,
    is_system_admin: bool,
    status: String,
    metadata: serde_json::Value,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl From<IdentityRow> for Identity {
    fn from(row: IdentityRow) -> Self {
        Self {
            id: row.id,
            identity_type: IdentityType::from(row.identity_type.as_str()),
            username: row.username,
            display_name: row.display_name,
            external_id: row.external_id,
            name: row.name,
            email: row.email,
            avatar_url: row.avatar_url,
            password_hash: row.password_hash,
            is_system_admin: row.is_system_admin,
            status: IdentityStatus::from(row.status.as_str()),
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
