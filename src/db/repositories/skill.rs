//! Skill repository

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use serde_json;

use crate::db::error::{DbError, DbResult};

const VALID_STATUSES: [&str; 4] = ["draft", "pending_review", "published", "rejected"];

#[derive(Debug, Clone)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author_agent_id: String,
    pub compatibility: String,
    pub content: String,
    pub install_count: i32,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub status: String,
    pub git_url: Option<String>,
    pub visibility: String,
    pub tools: Vec<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub approved_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author_agent_id: String,
    pub install_count: i32,
    pub tags: Vec<String>,
    pub status: String,
    pub git_url: Option<String>,
    pub visibility: String,
    pub approved_at: Option<DateTime<Utc>>,
    pub approved_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewSkill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author_agent_id: String,
    pub compatibility: String,
    pub content: String,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub status: String,
    pub git_url: Option<String>,
    pub visibility: Option<String>,
    pub tools: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct SkillRepository {
    pool: PgPool,
}

impl SkillRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_skill: NewSkill) -> DbResult<Skill> {
        let id = format!("skill-{}-{}", new_skill.name, new_skill.version);
        let status = if new_skill.status.is_empty() {
            "pending_review".to_string()
        } else {
            new_skill.status
        };

        let git_url = new_skill.git_url.clone();
        let visibility = new_skill.visibility.clone().unwrap_or_else(|| "org_visible".to_string());
        let tools = new_skill.tools.clone().unwrap_or_default();
        let tools_json = serde_json::to_value(&tools).unwrap_or(serde_json::Value::Array(vec![]));

        let skill_row = sqlx::query_as::<_, SkillRow>(
            r#"
            INSERT INTO skills (id, name, description, version, author_agent_id, compatibility, content, install_count, status, git_url, visibility, skill_tools)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, $9, $10, $11)
            RETURNING id, name, description, version, author_agent_id, compatibility, content, install_count, status, git_url, visibility, skill_tools, approved_at, approved_by, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&new_skill.name)
        .bind(&new_skill.description)
        .bind(&new_skill.version)
        .bind(&new_skill.author_agent_id)
        .bind(&new_skill.compatibility)
        .bind(&new_skill.content)
        .bind(&status)
        .bind(&git_url)
        .bind(&visibility)
        .bind(&tools_json)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                DbError::AlreadyExists(format!("Skill {} already exists", id))
            } else {
                DbError::QueryError(e.to_string())
            }
        })?;

        for tag in &new_skill.tags {
            sqlx::query("INSERT INTO skill_tags (skill_id, tag) VALUES ($1, $2)")
                .bind(&id)
                .bind(tag)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;
        }

        for dep in &new_skill.dependencies {
            sqlx::query("INSERT INTO skill_dependencies (skill_id, dependency_id) VALUES ($1, $2)")
                .bind(&id)
                .bind(dep)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;
        }

        Ok(Skill {
            id: skill_row.id,
            name: skill_row.name,
            description: skill_row.description,
            version: skill_row.version,
            author_agent_id: skill_row.author_agent_id,
            compatibility: skill_row.compatibility,
            content: skill_row.content,
            install_count: skill_row.install_count,
            tags: new_skill.tags,
            dependencies: new_skill.dependencies,
            status: skill_row.status,
            git_url: skill_row.git_url,
            visibility: skill_row.visibility,
            tools: skill_row.tools,
            approved_at: skill_row.approved_at,
            approved_by: skill_row.approved_by,
            created_at: skill_row.created_at,
            updated_at: skill_row.updated_at,
        })
    }

    pub async fn find_by_id(&self, skill_id: &str) -> DbResult<Option<Skill>> {
        let skill_row = sqlx::query_as::<_, SkillRow>(
            r#"
            SELECT id, name, description, version, author_agent_id, compatibility, content, install_count, status, git_url, visibility, skill_tools, approved_at, approved_by, created_at, updated_at
            FROM skills WHERE id = $1
            "#,
        )
        .bind(skill_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        match skill_row {
            Some(row) => {
                let tags = self.get_tags(skill_id).await?;
                let dependencies = self.get_dependencies(skill_id).await?;
                Ok(Some(Skill {
                    id: row.id,
                    name: row.name,
                    description: row.description,
                    version: row.version,
                    author_agent_id: row.author_agent_id,
                    compatibility: row.compatibility,
                    content: row.content,
                    install_count: row.install_count,
                    tags,
                    dependencies,
                    status: row.status,
                    git_url: row.git_url,
                    visibility: row.visibility,
                    tools: row.tools,
                    approved_at: row.approved_at,
                    approved_by: row.approved_by,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn list(&self, limit: i64, offset: i64) -> DbResult<Vec<SkillMetadata>> {
        let rows = sqlx::query_as::<_, SkillMetadataRow>(
            r#"
            SELECT id, name, description, version, author_agent_id, install_count, status, git_url, visibility, approved_at, approved_by, created_at, updated_at
            FROM skills
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let tags = self.get_tags(&row.id).await?;
            results.push(SkillMetadata {
                id: row.id,
                name: row.name,
                description: row.description,
                version: row.version,
                author_agent_id: row.author_agent_id,
                install_count: row.install_count,
                tags,
                status: row.status,
                git_url: row.git_url,
                visibility: row.visibility,
                approved_at: row.approved_at,
                approved_by: row.approved_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(results)
    }

    pub async fn count(&self) -> DbResult<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM skills")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(row.0)
    }

    pub async fn update(&self, skill_id: &str, description: Option<&str>, content: Option<&str>, tags: Option<Vec<String>>) -> DbResult<()> {
        if let Some(desc) = description {
            sqlx::query("UPDATE skills SET description = $1, updated_at = NOW() WHERE id = $2")
                .bind(desc)
                .bind(skill_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;
        }

        if let Some(c) = content {
            sqlx::query("UPDATE skills SET content = $1, updated_at = NOW() WHERE id = $2")
                .bind(c)
                .bind(skill_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;
        }

        if let Some(new_tags) = tags {
            sqlx::query("DELETE FROM skill_tags WHERE skill_id = $1")
                .bind(skill_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;

            for tag in &new_tags {
                sqlx::query("INSERT INTO skill_tags (skill_id, tag) VALUES ($1, $2)")
                    .bind(skill_id)
                    .bind(tag)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| DbError::QueryError(e.to_string()))?;
            }

            sqlx::query("UPDATE skills SET updated_at = NOW() WHERE id = $1")
                .bind(skill_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn delete(&self, skill_id: &str) -> DbResult<()> {
        sqlx::query("DELETE FROM skills WHERE id = $1")
            .bind(skill_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn increment_install_count(&self, skill_id: &str) -> DbResult<()> {
        sqlx::query("UPDATE skills SET install_count = install_count + 1 WHERE id = $1")
            .bind(skill_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    pub async fn update_status(&self, skill_id: &str, status: &str) -> DbResult<()> {
        if !VALID_STATUSES.contains(&status) {
            return Err(DbError::ValidationError(format!("Invalid status: {}", status)));
        }

        let result = sqlx::query("UPDATE skills SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status)
            .bind(skill_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Skill {} not found", skill_id)));
        }
        Ok(())
    }

    async fn get_tags(&self, skill_id: &str) -> DbResult<Vec<String>> {
        let tags: Vec<(String,)> = sqlx::query_as("SELECT tag FROM skill_tags WHERE skill_id = $1")
            .bind(skill_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(tags.into_iter().map(|(t,)| t).collect())
    }

    async fn get_dependencies(&self, skill_id: &str) -> DbResult<Vec<String>> {
        let deps: Vec<(String,)> = sqlx::query_as("SELECT dependency_id FROM skill_dependencies WHERE skill_id = $1")
            .bind(skill_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(deps.into_iter().map(|(d,)| d).collect())
    }
}

#[derive(sqlx::FromRow)]
struct SkillRow {
    id: String,
    name: String,
    description: String,
    version: String,
    author_agent_id: String,
    compatibility: String,
    content: String,
    install_count: i32,
    status: String,
    git_url: Option<String>,
    visibility: String,
    tools: Vec<String>,
    approved_at: Option<DateTime<Utc>>,
    approved_by: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct SkillMetadataRow {
    id: String,
    name: String,
    description: String,
    version: String,
    author_agent_id: String,
    install_count: i32,
    status: String,
    git_url: Option<String>,
    visibility: String,
    approved_at: Option<DateTime<Utc>>,
    approved_by: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
