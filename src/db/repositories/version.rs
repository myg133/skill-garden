//! Skill Version repository

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub id: Uuid,
    pub skill_name: String,
    pub version: String,
    pub git_commit_hash: Option<String>,
    pub git_tag: Option<String>,
    pub changelog: Option<String>,
    pub file_count: i32,
    pub total_size_bytes: i64,
    pub uploaded_by: Option<Uuid>,
    pub git_remote_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewSkillVersion {
    pub skill_name: String,
    pub version: String,
    pub git_commit_hash: Option<String>,
    pub git_tag: Option<String>,
    pub changelog: Option<String>,
    pub file_count: i32,
    pub total_size_bytes: i64,
    pub uploaded_by: Option<Uuid>,
    pub git_remote_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VersionRepository {
    pool: PgPool,
}

impl VersionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_version: NewSkillVersion) -> DbResult<SkillVersion> {
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, SkillVersionRow>(
            r#"
            INSERT INTO skill_versions (id, skill_name, version, git_commit_hash, git_tag, changelog, file_count, total_size_bytes, uploaded_by, git_remote_url)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, skill_name, version, git_commit_hash, git_tag, changelog, file_count, total_size_bytes, uploaded_by, git_remote_url, created_at
            "#,
        )
        .bind(id)
        .bind(&new_version.skill_name)
        .bind(&new_version.version)
        .bind(&new_version.git_commit_hash)
        .bind(&new_version.git_tag)
        .bind(&new_version.changelog)
        .bind(new_version.file_count)
        .bind(new_version.total_size_bytes)
        .bind(new_version.uploaded_by)
        .bind(&new_version.git_remote_url)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                DbError::AlreadyExists(format!(
                    "Version {} for skill {} already exists",
                    new_version.version, new_version.skill_name
                ))
            } else {
                DbError::QueryError(e.to_string())
            }
        })?;

        Ok(SkillVersion {
            id: row.id,
            skill_name: row.skill_name,
            version: row.version,
            git_commit_hash: row.git_commit_hash,
            git_tag: row.git_tag,
            changelog: row.changelog,
            file_count: row.file_count,
            total_size_bytes: row.total_size_bytes,
            uploaded_by: row.uploaded_by,
            git_remote_url: row.git_remote_url,
            created_at: row.created_at,
        })
    }

    pub async fn list_by_name(
        &self,
        skill_name: &str,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<SkillVersion>> {
        let rows = sqlx::query_as::<_, SkillVersionRow>(
            r#"SELECT id, skill_name, version, git_commit_hash, git_tag, changelog, file_count, total_size_bytes, uploaded_by, git_remote_url, created_at
               FROM skill_versions WHERE skill_name = $1
               ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
        )
        .bind(skill_name)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| SkillVersion {
                id: r.id,
                skill_name: r.skill_name,
                version: r.version,
                git_commit_hash: r.git_commit_hash,
                git_tag: r.git_tag,
                changelog: r.changelog,
                file_count: r.file_count,
                total_size_bytes: r.total_size_bytes,
                uploaded_by: r.uploaded_by,
                git_remote_url: r.git_remote_url,
                created_at: r.created_at,
            })
            .collect())
    }

    pub async fn find_by_name_and_version(
        &self,
        skill_name: &str,
        version: &str,
    ) -> DbResult<Option<SkillVersion>> {
        let row = sqlx::query_as::<_, SkillVersionRow>(
            r#"SELECT id, skill_name, version, git_commit_hash, git_tag, changelog, file_count, total_size_bytes, uploaded_by, git_remote_url, created_at
               FROM skill_versions WHERE skill_name = $1 AND version = $2"#,
        )
        .bind(skill_name)
        .bind(version)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.map(|r| SkillVersion {
            id: r.id,
            skill_name: r.skill_name,
            version: r.version,
            git_commit_hash: r.git_commit_hash,
            git_tag: r.git_tag,
            changelog: r.changelog,
            file_count: r.file_count,
            total_size_bytes: r.total_size_bytes,
            uploaded_by: r.uploaded_by,
            git_remote_url: r.git_remote_url,
            created_at: r.created_at,
        }))
    }

    /// 获取最新的可恢复版本。只有已经生成 Git tag 的版本才是完整、可回退的历史版本。
    pub async fn find_latest_tagged_by_name(
        &self,
        skill_name: &str,
    ) -> DbResult<Option<SkillVersion>> {
        let row = sqlx::query_as::<_, SkillVersionRow>(
            r#"SELECT id, skill_name, version, git_commit_hash, git_tag, changelog, file_count, total_size_bytes, uploaded_by, git_remote_url, created_at
               FROM skill_versions
               WHERE skill_name = $1 AND git_tag IS NOT NULL
               ORDER BY created_at DESC LIMIT 1"#,
        )
        .bind(skill_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.map(|r| SkillVersion {
            id: r.id,
            skill_name: r.skill_name,
            version: r.version,
            git_commit_hash: r.git_commit_hash,
            git_tag: r.git_tag,
            changelog: r.changelog,
            file_count: r.file_count,
            total_size_bytes: r.total_size_bytes,
            uploaded_by: r.uploaded_by,
            git_remote_url: r.git_remote_url,
            created_at: r.created_at,
        }))
    }

    pub async fn get_latest_version(&self, skill_name: &str) -> DbResult<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            r#"SELECT version FROM skill_versions WHERE skill_name = $1 ORDER BY created_at DESC LIMIT 1"#,
        )
        .bind(skill_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.map(|r| r.0))
    }
}

#[derive(sqlx::FromRow)]
struct SkillVersionRow {
    id: Uuid,
    skill_name: String,
    version: String,
    git_commit_hash: Option<String>,
    git_tag: Option<String>,
    changelog: Option<String>,
    file_count: i32,
    total_size_bytes: i64,
    uploaded_by: Option<Uuid>,
    git_remote_url: Option<String>,
    created_at: DateTime<Utc>,
}
