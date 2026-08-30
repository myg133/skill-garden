//! Skill repository

use chrono::{DateTime, Utc};
use serde_json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

const VALID_STATUSES: [&str; 6] = [
    "draft",
    "pending_review",
    "approved",
    "rejected",
    "published",
    "archived",
];

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author_agent_id: String,
    pub author_identity_id: Option<Uuid>,
    pub owner_type: String,
    pub owner_id: Option<Uuid>,
    pub compatibility: String,
    pub content: String,
    pub install_count: i32,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub status: String,
    pub git_url: Option<String>,
    pub visibility: String,
    pub tools: Vec<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_comment: Option<String>,
    pub admin_unpublished: bool,
    pub marketplace_status: Option<String>,
    pub pre_marketplace_visibility: Option<String>,
    pub draft_content: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author_agent_id: String,
    pub author_identity_id: Option<Uuid>,
    pub author_name: Option<String>,
    pub owner_type: String,
    pub owner_id: Option<Uuid>,
    pub install_count: i32,
    pub tags: Vec<String>,
    pub status: String,
    pub git_url: Option<String>,
    pub visibility: String,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_comment: Option<String>,
    pub admin_unpublished: bool,
    pub marketplace_status: Option<String>,
    pub pre_marketplace_visibility: Option<String>,
    pub draft_content: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewSkill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author_agent_id: String,
    pub author_identity_id: Option<Uuid>,
    pub owner_type: String,
    pub owner_id: Option<Uuid>,
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
            "draft".to_string()
        } else {
            new_skill.status
        };

        let git_url = new_skill.git_url.clone();
        let visibility = new_skill
            .visibility
            .clone()
            .unwrap_or_else(|| "private".to_string());
        let tools = new_skill.tools.clone().unwrap_or_default();
        let tools_json = serde_json::to_value(&tools).unwrap_or(serde_json::Value::Array(vec![]));

        // 新版本 is_current = true，旧已发布版本也保持 is_current（保证市场用户仍可见）
        // 审核通过并发布后，旧版本 is_current 才设为 false
        sqlx::query("UPDATE skills SET is_current = false WHERE name = $1 AND is_current = true AND status NOT IN ('published')")
            .bind(&new_skill.name)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let skill_row = sqlx::query_as::<_, SkillRow>(
            r#"
            INSERT INTO skills (id, name, description, version, author_agent_id, author_identity_id, owner_type, owner_id, compatibility, content, install_count, status, git_url, visibility, skill_tools, is_current)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 0, $11, $12, $13, $14, true)
            RETURNING id, name, description, version, author_agent_id, author_identity_id, owner_type, owner_id, compatibility, content, install_count, status, git_url, visibility, skill_tools, reviewed_by, reviewed_at, review_comment, admin_unpublished, marketplace_status, pre_marketplace_visibility, draft_content, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&new_skill.name)
        .bind(&new_skill.description)
        .bind(&new_skill.version)
        .bind(&new_skill.author_agent_id)
        .bind(new_skill.author_identity_id)
        .bind(&new_skill.owner_type)
        .bind(new_skill.owner_id)
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
            sqlx::query(
                "INSERT INTO skill_dependencies (skill_id, dependency_skill_id) VALUES ($1, $2)",
            )
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
            author_identity_id: skill_row.author_identity_id,
            owner_type: skill_row.owner_type,
            owner_id: skill_row.owner_id,
            compatibility: skill_row.compatibility,
            content: skill_row.content,
            install_count: skill_row.install_count,
            tags: new_skill.tags,
            dependencies: new_skill.dependencies,
            status: skill_row.status,
            git_url: skill_row.git_url,
            visibility: skill_row.visibility,
            tools: skill_row.tools,
            reviewed_by: skill_row.reviewed_by,
            reviewed_at: skill_row.reviewed_at,
            review_comment: skill_row.review_comment,
            admin_unpublished: skill_row.admin_unpublished,
            marketplace_status: skill_row.marketplace_status,
            pre_marketplace_visibility: skill_row.pre_marketplace_visibility,
            draft_content: skill_row.draft_content,
            created_at: skill_row.created_at,
            updated_at: skill_row.updated_at,
        })
    }

    pub async fn find_by_id(&self, skill_id: &str) -> DbResult<Option<Skill>> {
        let skill_row = sqlx::query_as::<_, SkillRow>(
            r#"
            SELECT id, name, description, version, author_agent_id, author_identity_id, owner_type, owner_id, compatibility, content, install_count, status, git_url, visibility, skill_tools, reviewed_by, reviewed_at, review_comment, admin_unpublished, marketplace_status, pre_marketplace_visibility, draft_content, created_at, updated_at
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
                    author_identity_id: row.author_identity_id,
                    owner_type: row.owner_type,
                    owner_id: row.owner_id,
                    compatibility: row.compatibility,
                    content: row.content,
                    install_count: row.install_count,
                    tags,
                    dependencies,
                    status: row.status,
                    git_url: row.git_url,
                    visibility: row.visibility,
                    tools: row.tools,
                    reviewed_by: row.reviewed_by,
                    reviewed_at: row.reviewed_at,
                    review_comment: row.review_comment,
                    admin_unpublished: row.admin_unpublished,
                    marketplace_status: row.marketplace_status,
                    pre_marketplace_visibility: row.pre_marketplace_visibility,
                    draft_content: row.draft_content,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// 加载所有 Skill（用于索引重建），仅取 is_current=true 的最新版本
    pub async fn list_all(&self) -> DbResult<Vec<Skill>> {
        let rows = sqlx::query_as::<_, SkillRow>(
            r#"SELECT id, name, description, version, author_agent_id, author_identity_id, owner_type, owner_id, compatibility, content, install_count, status, git_url, visibility, skill_tools, reviewed_by, reviewed_at, review_comment, admin_unpublished, marketplace_status, pre_marketplace_visibility, draft_content, created_at, updated_at
               FROM skills WHERE is_current = true ORDER BY created_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        let mut skills = Vec::with_capacity(rows.len());
        for row in rows {
            let tags = self.get_tags(&row.id).await?;
            skills.push(Skill {
                id: row.id,
                name: row.name,
                description: row.description,
                version: row.version,
                author_agent_id: row.author_agent_id,
                author_identity_id: row.author_identity_id,
                owner_type: row.owner_type,
                owner_id: row.owner_id,
                compatibility: row.compatibility,
                content: row.content,
                install_count: row.install_count,
                tags,
                dependencies: vec![],
                status: row.status,
                git_url: row.git_url,
                visibility: row.visibility,
                tools: row.tools,
                reviewed_by: row.reviewed_by,
                reviewed_at: row.reviewed_at,
                review_comment: row.review_comment,
                admin_unpublished: row.admin_unpublished,
                marketplace_status: row.marketplace_status,
                pre_marketplace_visibility: row.pre_marketplace_visibility,
                draft_content: row.draft_content,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }
        Ok(skills)
    }

    pub async fn list(&self, limit: i64, offset: i64) -> DbResult<Vec<SkillMetadata>> {
        self.list_sorted(limit, offset, "created").await
    }

    pub async fn list_sorted(
        &self,
        limit: i64,
        offset: i64,
        sort_by: &str,
    ) -> DbResult<Vec<SkillMetadata>> {
        let order_clause = match sort_by {
            "installs" => "ORDER BY install_count DESC, created_at DESC",
            "name" => "ORDER BY name ASC, created_at DESC",
            "updated" => "ORDER BY updated_at DESC",
            _ => "ORDER BY created_at DESC",
        };

        let query = format!(
            r#"
            SELECT s.id, s.name, s.description, s.version, s.author_agent_id, s.author_identity_id,
                   s.owner_type, s.owner_id, s.install_count, s.status, s.git_url, s.visibility,
                   s.reviewed_by, s.reviewed_at, s.review_comment, s.admin_unpublished,
                   s.marketplace_status, s.pre_marketplace_visibility, s.draft_content, s.is_current, s.created_at, s.updated_at,
                   COALESCE(i.display_name, i.username, i.name) AS author_name
            FROM skills s
            LEFT JOIN identities i ON i.id = s.author_identity_id
            WHERE s.status != 'rejected' AND s.is_current = true
            {} LIMIT $1 OFFSET $2
            "#,
            order_clause
        );

        let rows = sqlx::query_as::<_, SkillMetadataRow>(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let tags = self.get_tags(&row.id).await?;
            results.push(self.build_metadata(row, tags));
        }

        Ok(results)
    }

    pub async fn count(&self) -> DbResult<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM skills WHERE status != 'rejected' AND is_current = true",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(row.0)
    }

    pub async fn list_by_visibility(
        &self,
        visibility: &str,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<SkillMetadata>> {
        let rows = sqlx::query_as::<_, SkillMetadataRow>(
            r#"
            SELECT s.id, s.name, s.description, s.version, s.author_agent_id,
                   s.author_identity_id, s.owner_type, s.owner_id,
                   s.install_count, s.status, s.git_url, s.visibility,
                   s.reviewed_by, s.reviewed_at, s.review_comment, s.admin_unpublished,
                   s.marketplace_status, s.pre_marketplace_visibility, s.draft_content, s.is_current, s.created_at, s.updated_at,
                   COALESCE(i.display_name, i.username, i.name) AS author_name
            FROM skills s
            LEFT JOIN identities i ON i.id = s.author_identity_id
            WHERE s.visibility = $1 AND s.status = 'published'
            ORDER BY s.install_count DESC, s.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(visibility)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let tags = self.get_tags(&row.id).await?;
            results.push(self.build_metadata(row, tags));
        }

        Ok(results)
    }

    pub async fn list_by_org(&self, org_id: &str) -> DbResult<Vec<SkillMetadata>> {
        let rows = sqlx::query_as::<_, SkillMetadataRow>(
            r#"
            SELECT s.id, s.name, s.description, s.version, s.author_agent_id,
                   s.author_identity_id, s.owner_type, s.owner_id,
                   s.install_count, s.status, s.git_url, s.visibility,
                   s.reviewed_by, s.reviewed_at, s.review_comment, s.admin_unpublished,
                   s.marketplace_status, s.pre_marketplace_visibility, s.draft_content, s.is_current, s.created_at, s.updated_at,
                   COALESCE(i.display_name, i.username, i.name) AS author_name
            FROM skills s
            LEFT JOIN identities i ON i.id = s.author_identity_id
            WHERE s.owner_type = 'organization' AND s.owner_id::text = $1
            ORDER BY s.created_at DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let tags = self.get_tags(&row.id).await?;
            results.push(self.build_metadata(row, tags));
        }

        Ok(results)
    }

    pub async fn update(
        &self,
        skill_id: &str,
        description: Option<&str>,
        content: Option<&str>,
        tags: Option<Vec<String>>,
        visibility: Option<&str>,
    ) -> DbResult<()> {
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

        if let Some(vis) = visibility {
            sqlx::query("UPDATE skills SET visibility = $1, updated_at = NOW() WHERE id = $2")
                .bind(vis)
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

    /// 按名称查找最新版本（按 created_at DESC 取第一条）
    pub async fn find_latest_by_name(&self, name: &str) -> DbResult<Option<Skill>> {
        let row = sqlx::query_as::<_, SkillRow>(
            r#"
            SELECT id, name, description, version, author_agent_id, author_identity_id, owner_type, owner_id, compatibility, content, install_count, status, git_url, visibility, skill_tools, reviewed_by, reviewed_at, review_comment, admin_unpublished, marketplace_status, pre_marketplace_visibility, draft_content, created_at, updated_at
            FROM skills WHERE name = $1
            ORDER BY created_at DESC LIMIT 1
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        match row {
            Some(row) => {
                let tags = self.get_tags(&row.id).await?;
                Ok(Some(Skill {
                    id: row.id,
                    name: row.name,
                    description: row.description,
                    version: row.version,
                    author_agent_id: row.author_agent_id,
                    author_identity_id: row.author_identity_id,
                    owner_type: row.owner_type,
                    owner_id: row.owner_id,
                    compatibility: row.compatibility,
                    content: row.content,
                    install_count: row.install_count,
                    tags,
                    dependencies: vec![],
                    status: row.status,
                    git_url: row.git_url,
                    visibility: row.visibility,
                    tools: row.tools,
                    reviewed_by: row.reviewed_by,
                    reviewed_at: row.reviewed_at,
                    review_comment: row.review_comment,
                    admin_unpublished: row.admin_unpublished,
                    marketplace_status: row.marketplace_status,
                    pre_marketplace_visibility: row.pre_marketplace_visibility,
                    draft_content: row.draft_content,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn list_by_name(&self, name: &str) -> DbResult<Vec<SkillMetadata>> {
        let rows = sqlx::query_as::<_, SkillMetadataRow>(
            r#"
            SELECT s.id, s.name, s.description, s.version, s.author_agent_id, s.author_identity_id,
                   s.owner_type, s.owner_id, s.install_count, s.status, s.git_url, s.visibility,
                   s.reviewed_by, s.reviewed_at, s.review_comment, s.admin_unpublished,
                   s.marketplace_status, s.pre_marketplace_visibility, s.draft_content, s.is_current, s.created_at, s.updated_at,
                   COALESCE(i.display_name, i.username, i.name) AS author_name
            FROM skills s
            LEFT JOIN identities i ON i.id = s.author_identity_id
            WHERE s.name = $1
            ORDER BY s.created_at DESC
            "#,
        )
        .bind(name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let tags = self.get_tags(&row.id).await?;
            results.push(self.build_metadata(row, tags));
        }
        Ok(results)
    }

    /// 更新 skill 状态（统一 status 字段，替代原来的 status + review_status）
    /// reviewed_by 和 review_comment 用于审批/驳回时记录审核信息
    pub async fn update_status(
        &self,
        skill_id: &str,
        status: &str,
        reviewed_by: Option<Uuid>,
        review_comment: Option<&str>,
    ) -> DbResult<()> {
        if !VALID_STATUSES.contains(&status) {
            return Err(DbError::ValidationError(format!(
                "Invalid status: {}",
                status
            )));
        }

        let result = sqlx::query(
            "UPDATE skills SET status = $1, reviewed_by = COALESCE($2, reviewed_by), reviewed_at = CASE WHEN $2 IS NOT NULL THEN NOW() ELSE reviewed_at END, review_comment = COALESCE($3, review_comment), updated_at = NOW() WHERE id = $4",
        )
        .bind(status)
        .bind(reviewed_by)
        .bind(review_comment)
        .bind(skill_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Skill {} not found", skill_id)));
        }
        Ok(())
    }

    /// 设置 admin 下架标记（DEPRECATED: 使用 update_marketplace_status 替代）
    pub async fn set_admin_unpublished(&self, skill_id: &str, value: bool) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE skills SET admin_unpublished = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(value)
        .bind(skill_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Skill {} not found", skill_id)));
        }
        Ok(())
    }

    /// 更新市场状态
    pub async fn update_marketplace_status(
        &self,
        skill_id: &str,
        marketplace_status: Option<&str>,
    ) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE skills SET marketplace_status = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(marketplace_status)
        .bind(skill_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Skill {} not found", skill_id)));
        }
        Ok(())
    }

    /// 保存提交市场前的原始可见性
    pub async fn set_pre_marketplace_visibility(
        &self,
        skill_id: &str,
        visibility: Option<&str>,
    ) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE skills SET pre_marketplace_visibility = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(visibility)
        .bind(skill_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Skill {} not found", skill_id)));
        }
        Ok(())
    }

    /// 保存更新草稿（用于 pending_update 流程）
    pub async fn save_draft_content(
        &self,
        skill_id: &str,
        draft: &serde_json::Value,
    ) -> DbResult<()> {
        let result =
            sqlx::query("UPDATE skills SET draft_content = $1, updated_at = NOW() WHERE id = $2")
                .bind(draft)
                .bind(skill_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Skill {} not found", skill_id)));
        }
        Ok(())
    }

    /// 清空更新草稿
    pub async fn clear_draft_content(&self, skill_id: &str) -> DbResult<()> {
        let result =
            sqlx::query("UPDATE skills SET draft_content = NULL, updated_at = NOW() WHERE id = $1")
                .bind(skill_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Skill {} not found", skill_id)));
        }
        Ok(())
    }

    /// 应用 draft_content 到主字段（审核通过时调用）
    pub async fn apply_draft_content(
        &self,
        skill_id: &str,
        draft: &serde_json::Value,
    ) -> DbResult<()> {
        if let Some(desc) = draft.get("description").and_then(|v| v.as_str()) {
            sqlx::query("UPDATE skills SET description = $1, updated_at = NOW() WHERE id = $2")
                .bind(desc)
                .bind(skill_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;
        }
        if let Some(tags) = draft.get("tags").and_then(|v| v.as_array()) {
            sqlx::query("DELETE FROM skill_tags WHERE skill_id = $1")
                .bind(skill_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;
            for tag in tags {
                if let Some(t) = tag.as_str() {
                    sqlx::query("INSERT INTO skill_tags (skill_id, tag) VALUES ($1, $2)")
                        .bind(skill_id)
                        .bind(t)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| DbError::QueryError(e.to_string()))?;
                }
            }
            sqlx::query("UPDATE skills SET updated_at = NOW() WHERE id = $1")
                .bind(skill_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;
        }
        if let Some(content) = draft.get("content").and_then(|v| v.as_str()) {
            sqlx::query("UPDATE skills SET content = $1, updated_at = NOW() WHERE id = $2")
                .bind(content)
                .bind(skill_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;
        }
        // 清空 draft_content
        self.clear_draft_content(skill_id).await?;
        Ok(())
    }

    /// 按市场状态列出 Skill（用于市场审核队列等）
    pub async fn list_by_marketplace_status(
        &self,
        marketplace_status: &str,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<SkillMetadata>> {
        let rows = sqlx::query_as::<_, SkillMetadataRow>(
            r#"
            SELECT s.id, s.name, s.description, s.version, s.author_agent_id,
                   s.author_identity_id, s.owner_type, s.owner_id,
                   s.install_count, s.status, s.git_url, s.visibility,
                   s.reviewed_by, s.reviewed_at, s.review_comment, s.admin_unpublished,
                   s.marketplace_status, s.pre_marketplace_visibility, s.draft_content, s.is_current, s.created_at, s.updated_at,
                   COALESCE(i.display_name, i.username, i.name) AS author_name
            FROM skills s
            LEFT JOIN identities i ON i.id = s.author_identity_id
            WHERE s.marketplace_status = $1
            ORDER BY s.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(marketplace_status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let tags = self.get_tags(&row.id).await?;
            results.push(self.build_metadata(row, tags));
        }
        Ok(results)
    }

    /// 列出市场的上架 Skill（用于公开市场页面）
    /// 新逻辑：status=published AND marketplace_status='listed'
    pub async fn list_marketplace_listed(
        &self,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<SkillMetadata>> {
        let rows = sqlx::query_as::<_, SkillMetadataRow>(
            r#"
            SELECT s.id, s.name, s.description, s.version, s.author_agent_id,
                   s.author_identity_id, s.owner_type, s.owner_id,
                   s.install_count, s.status, s.git_url, s.visibility,
                   s.reviewed_by, s.reviewed_at, s.review_comment, s.admin_unpublished,
                   s.marketplace_status, s.pre_marketplace_visibility, s.draft_content, s.is_current, s.created_at, s.updated_at,
                   COALESCE(i.display_name, i.username, i.name) AS author_name
            FROM skills s
            LEFT JOIN identities i ON i.id = s.author_identity_id
            WHERE s.status = 'published' AND s.marketplace_status = 'listed'
            ORDER BY s.install_count DESC, s.created_at DESC
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
            results.push(self.build_metadata(row, tags));
        }
        Ok(results)
    }

    /// 列出用户自己创建的 Skill（owner_type=user 且 owner_id 或 author_identity_id 匹配）
    pub async fn list_by_owner(&self, identity_id: Uuid) -> DbResult<Vec<SkillMetadata>> {
        let rows = sqlx::query_as::<_, SkillMetadataRow>(
            r#"
            SELECT s.id, s.name, s.description, s.version, s.author_agent_id, s.author_identity_id,
                   s.owner_type, s.owner_id, s.install_count, s.status, s.git_url, s.visibility,
                   s.reviewed_by, s.reviewed_at, s.review_comment, s.admin_unpublished,
                   s.marketplace_status, s.pre_marketplace_visibility, s.draft_content, s.is_current, s.created_at, s.updated_at,
                   COALESCE(i.display_name, i.username, i.name) AS author_name
            FROM skills s
            LEFT JOIN identities i ON i.id = s.author_identity_id
            WHERE s.owner_type = 'user'
              AND (s.owner_id = $1 OR s.author_identity_id = $1)
            ORDER BY s.created_at DESC
            "#,
        )
        .bind(identity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let tags = self.get_tags(&row.id).await?;
            results.push(self.build_metadata(row, tags));
        }
        Ok(results)
    }

    /// 列出用户可访问的 Skill（个人 + 所在组织 + 市场公开）
    pub async fn list_user_accessible(
        &self,
        identity_id: Uuid,
        org_ids: &[Uuid],
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<SkillMetadata>> {
        if org_ids.is_empty() {
            let rows = sqlx::query_as::<_, SkillMetadataRow>(
                r#"
                SELECT s.id, s.name, s.description, s.version, s.author_agent_id, s.author_identity_id,
                       s.owner_type, s.owner_id, s.install_count, s.status, s.git_url, s.visibility,
                       s.reviewed_by, s.reviewed_at, s.review_comment, s.admin_unpublished,
                       s.marketplace_status, s.pre_marketplace_visibility, s.draft_content, s.is_current, s.created_at, s.updated_at,
                       COALESCE(i.display_name, i.username, i.name) AS author_name
                FROM skills s
                LEFT JOIN identities i ON i.id = s.author_identity_id
                WHERE (s.owner_type = 'user' AND (s.owner_id = $1 OR s.author_identity_id = $1))
                   OR (s.status = 'published' AND s.visibility = 'marketplace')
                ORDER BY s.created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(identity_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

            let mut results = Vec::new();
            for row in rows {
                let tags = self.get_tags(&row.id).await?;
                results.push(self.build_metadata(row, tags));
            }
            return Ok(results);
        }

        let org_ids_str: Vec<String> = org_ids.iter().map(|id| id.to_string()).collect();
        let query = format!(
            r#"
            SELECT s.id, s.name, s.description, s.version, s.author_agent_id, s.author_identity_id,
                   s.owner_type, s.owner_id, s.install_count, s.status, s.git_url, s.visibility,
                   s.reviewed_by, s.reviewed_at, s.review_comment, s.admin_unpublished,
                   s.marketplace_status, s.pre_marketplace_visibility, s.draft_content, s.is_current, s.created_at, s.updated_at,
                   COALESCE(i.display_name, i.username, i.name) AS author_name
            FROM skills s
            LEFT JOIN identities i ON i.id = s.author_identity_id
            WHERE (s.owner_type = 'user' AND (s.owner_id = $1 OR s.author_identity_id = $1))
               OR (s.owner_type = 'organization' AND s.owner_id::text = ANY($2))
               OR (s.status = 'published' AND s.visibility = 'marketplace')
            ORDER BY s.created_at DESC
            LIMIT $3 OFFSET $4
            "#
        );

        let rows = sqlx::query_as::<_, SkillMetadataRow>(&query)
            .bind(identity_id)
            .bind(&org_ids_str)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let tags = self.get_tags(&row.id).await?;
            results.push(self.build_metadata(row, tags));
        }
        Ok(results)
    }

    fn build_metadata(&self, row: SkillMetadataRow, tags: Vec<String>) -> SkillMetadata {
        SkillMetadata {
            id: row.id,
            name: row.name,
            description: row.description,
            version: row.version,
            author_agent_id: row.author_agent_id,
            author_identity_id: row.author_identity_id,
            author_name: row.author_name,
            owner_type: row.owner_type,
            owner_id: row.owner_id,
            install_count: row.install_count,
            tags,
            status: row.status,
            git_url: row.git_url,
            visibility: row.visibility,
            reviewed_by: row.reviewed_by,
            reviewed_at: row.reviewed_at,
            review_comment: row.review_comment,
            admin_unpublished: row.admin_unpublished,
            marketplace_status: row.marketplace_status,
            pre_marketplace_visibility: row.pre_marketplace_visibility,
            draft_content: row.draft_content,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }

    pub async fn get_tags(&self, skill_id: &str) -> DbResult<Vec<String>> {
        let tags: Vec<(String,)> = sqlx::query_as("SELECT tag FROM skill_tags WHERE skill_id = $1")
            .bind(skill_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(tags.into_iter().map(|(t,)| t).collect())
    }

    async fn get_dependencies(&self, skill_id: &str) -> DbResult<Vec<String>> {
        let deps: Vec<(String,)> = sqlx::query_as(
            "SELECT dependency_skill_id FROM skill_dependencies WHERE skill_id = $1",
        )
        .bind(skill_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(deps.into_iter().map(|(d,)| d).collect())
    }

    /// 批量查询 Skill 元数据（仅过滤所需字段，不含 content/tags/dependencies）
    pub async fn find_meta_by_ids(&self, ids: &[&str]) -> DbResult<Vec<SkillMetadataRow>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 1))
            .collect();
        let sql = format!(
            r#"SELECT s.id, s.name, s.description, s.version, s.author_agent_id, s.author_identity_id,
                      i.display_name AS author_name, s.owner_type, s.owner_id, s.install_count,
                      s.status, s.git_url, s.visibility, s.reviewed_by, s.reviewed_at, s.review_comment,
                      s.admin_unpublished, s.marketplace_status, s.pre_marketplace_visibility,
                      s.draft_content, true AS is_current, s.created_at, s.updated_at
               FROM skills s
               LEFT JOIN identities i ON s.author_identity_id = i.id
               WHERE s.id IN ({})
               ORDER BY s.created_at DESC"#,
            placeholders.join(", ")
        );
        let mut query = sqlx::query_as::<_, SkillMetadataRow>(&sql);
        for id in ids {
            query = query.bind(*id);
        }
        query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))
    }
}

#[derive(sqlx::FromRow)]
struct SkillRow {
    id: String,
    name: String,
    description: String,
    version: String,
    author_agent_id: String,
    author_identity_id: Option<Uuid>,
    owner_type: String,
    owner_id: Option<Uuid>,
    compatibility: String,
    content: String,
    install_count: i32,
    status: String,
    git_url: Option<String>,
    visibility: String,
    #[sqlx(rename = "skill_tools", json)]
    tools: Vec<String>,
    reviewed_by: Option<Uuid>,
    reviewed_at: Option<DateTime<Utc>>,
    review_comment: Option<String>,
    admin_unpublished: bool,
    marketplace_status: Option<String>,
    pre_marketplace_visibility: Option<String>,
    draft_content: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct SkillMetadataRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author_agent_id: String,
    pub author_identity_id: Option<Uuid>,
    pub author_name: Option<String>,
    pub owner_type: String,
    pub owner_id: Option<Uuid>,
    pub install_count: i32,
    pub status: String,
    pub git_url: Option<String>,
    pub visibility: String,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_comment: Option<String>,
    pub admin_unpublished: bool,
    pub marketplace_status: Option<String>,
    pub pre_marketplace_visibility: Option<String>,
    pub draft_content: Option<serde_json::Value>,
    pub is_current: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
