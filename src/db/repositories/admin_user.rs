//! Admin user repository

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use bcrypt::{hash, verify, DEFAULT_COST};

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone)]
pub struct AdminUser {
    pub id: uuid::Uuid,
    pub username: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewAdminUser {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Clone)]
pub struct AdminUserRepository {
    pool: PgPool,
}

impl AdminUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_username(&self, username: &str) -> DbResult<Option<AdminUser>> {
        let user = sqlx::query_as::<_, AdminUserRow>(
            r#"
            SELECT id, username, password_hash, display_name, is_active, created_at, updated_at
            FROM admin_users WHERE username = $1 AND is_active = true
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(user.map(|u| u.into()))
    }

    pub async fn verify_password(&self, username: &str, password: &str) -> DbResult<bool> {
        let user = self.find_by_username(username).await?;
        match user {
            Some(u) => {
                let valid = verify(password, &u.password_hash)
                    .map_err(|e| DbError::ValidationError(format!("Failed to verify password: {}", e)))?;
                Ok(valid)
            }
            None => Ok(false),
        }
    }

    pub async fn create(&self, new_user: NewAdminUser) -> DbResult<AdminUser> {
        let password_hash = hash(&new_user.password, DEFAULT_COST)
            .map_err(|e| DbError::ValidationError(format!("Failed to hash password: {}", e)))?;

        let user = sqlx::query_as::<_, AdminUserRow>(
            r#"
            INSERT INTO admin_users (username, password_hash, display_name)
            VALUES ($1, $2, $3)
            RETURNING id, username, password_hash, display_name, is_active, created_at, updated_at
            "#,
        )
        .bind(&new_user.username)
        .bind(&password_hash)
        .bind(&new_user.display_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                DbError::AlreadyExists(format!("Admin user {} already exists", new_user.username))
            } else {
                DbError::QueryError(e.to_string())
            }
        })?;

        Ok(user.into())
    }
}

#[derive(sqlx::FromRow)]
struct AdminUserRow {
    id: uuid::Uuid,
    username: String,
    password_hash: String,
    display_name: Option<String>,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<AdminUserRow> for AdminUser {
    fn from(row: AdminUserRow) -> Self {
        Self {
            id: row.id,
            username: row.username,
            password_hash: row.password_hash,
            display_name: row.display_name,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
