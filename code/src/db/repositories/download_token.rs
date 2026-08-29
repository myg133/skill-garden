//! 下载凭证数据库操作

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::download_token::DownloadToken;

#[derive(Debug, Clone)]
pub struct DownloadTokenRepository {
    pool: PgPool,
}

impl DownloadTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建下载凭证（MCP skills.install 时调用）
    pub async fn create(
        &self,
        skill_name: &str,
        skill_version: &str,
        identity_id: Uuid,
        api_key_id: Uuid,
        expires_seconds: i64,
    ) -> DbResult<DownloadToken> {
        let token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + chrono::Duration::seconds(expires_seconds);

        let record = sqlx::query_as::<_, DownloadToken>(
            r#"
            INSERT INTO download_tokens (token, skill_name, skill_version, identity_id, api_key_id, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&token)
        .bind(skill_name)
        .bind(skill_version)
        .bind(identity_id)
        .bind(api_key_id)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(format!("Failed to create download token: {}", e)))?;

        Ok(record)
    }

    /// 验证并消费下载凭证（下载时调用）
    /// 返回 token 记录（含身份信息），失败返回 None
    pub async fn validate_and_consume(
        &self,
        token_str: &str,
        skill_name: &str,
        skill_version: &str,
    ) -> DbResult<Option<DownloadToken>> {
        let now = Utc::now();

        let record = sqlx::query_as::<_, DownloadToken>(
            r#"
            UPDATE download_tokens
            SET used_at = $1
            WHERE token = $2
              AND skill_name = $3
              AND skill_version = $4
              AND expires_at > $5
              AND used_at IS NULL
            RETURNING *
            "#,
        )
        .bind(now)
        .bind(token_str)
        .bind(skill_name)
        .bind(skill_version)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(format!("Failed to validate download token: {}", e)))?;

        Ok(record)
    }

    /// 创建 CLI 下载凭证（resource_type = 'cli'）
    /// config_data: 预填的 config.toml 内容，下载时嵌入 tar.gz
    pub async fn create_cli_token(
        &self,
        version: &str,
        target: &str,
        identity_id: Uuid,
        api_key_id: Uuid,
        expires_seconds: i64,
        config_data: Option<String>,
    ) -> DbResult<DownloadToken> {
        let token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + chrono::Duration::seconds(expires_seconds);

        let record = sqlx::query_as::<_, DownloadToken>(
            r#"
            INSERT INTO download_tokens (token, skill_name, skill_version, identity_id, api_key_id, expires_at, resource_type, target, config_data)
            VALUES ($1, $2, $3, $4, $5, $6, 'cli', $7, $8)
            RETURNING *
            "#,
        )
        .bind(&token)
        .bind("cli")
        .bind(version)
        .bind(identity_id)
        .bind(api_key_id)
        .bind(expires_at)
        .bind(target)
        .bind(&config_data)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(format!("Failed to create CLI download token: {}", e)))?;

        Ok(record)
    }

    /// 验证 CLI 下载 token（按 token 字符串查找，不按 skill_name/version 过滤）
    pub async fn validate_cli_token(&self, token_str: &str) -> DbResult<Option<DownloadToken>> {
        let now = Utc::now();

        let record = sqlx::query_as::<_, DownloadToken>(
            r#"
            UPDATE download_tokens
            SET used_at = $1
            WHERE token = $2
              AND resource_type = 'cli'
              AND expires_at > $3
              AND used_at IS NULL
            RETURNING *
            "#,
        )
        .bind(now)
        .bind(token_str)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            DbError::QueryError(format!("Failed to validate CLI download token: {}", e))
        })?;

        Ok(record)
    }
}
