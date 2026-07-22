//! 注册服务 - Skills CRUD

use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use crate::db::repositories::download_token::DownloadTokenRepository;
use crate::db::repositories::skill::{NewSkill as DbNewSkill, SkillRepository};
use crate::models::error::AppError;
use crate::models::skill::{InstallResult, NewSkill, Skill, SkillMetadata, SkillUpdate};
use crate::schemas::validation::{
    normalize_description, validate_description, validate_skill_content, validate_skill_name,
    validate_tags, validate_version,
};
use crate::services::skill_git::copy_dir_recursive;
use crate::services::storage::{get_skill_lock, StorageService};
use crate::services::SearchService;

/// 注册服务
#[derive(Debug, Clone)]
pub struct RegistryService {
    skills_dir: PathBuf,
    registry_dir: PathBuf,
    storage: StorageService,
    skill_repo: SkillRepository,
    download_token_repo: DownloadTokenRepository,
}

impl RegistryService {
    pub fn new(
        skills_dir: PathBuf,
        registry_dir: PathBuf,
        skill_repo: SkillRepository,
        download_token_repo: DownloadTokenRepository,
    ) -> Self {
        let storage = StorageService::new(registry_dir.clone());
        Self {
            skills_dir,
            registry_dir,
            storage,
            skill_repo,
            download_token_repo,
        }
    }

    /// 获取索引文件路径
    fn index_path(&self) -> PathBuf {
        self.registry_dir.join("skills-index.json")
    }

    /// 获取 Skill 目录路径
    fn skill_dir(&self, name: &str) -> PathBuf {
        self.skills_dir.join(name)
    }

    /// 公开的 skill 目录路径（供 download handler 使用）
    pub fn skill_dir_path(&self, name: &str) -> PathBuf {
        self.skill_dir(name)
    }

    /// 获取 SKILL.md 路径
    fn skill_md_path(&self, name: &str) -> PathBuf {
        self.skill_dir(name).join("SKILL.md")
    }

    /// 加载索引
    pub fn load_index(&self) -> Result<crate::models::skill::SkillsIndex, AppError> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(crate::models::skill::SkillsIndex::default());
        }
        self.storage.read_json(&path)
    }

    /// 保存索引
    fn save_index(&self, index: &crate::models::skill::SkillsIndex) -> Result<(), AppError> {
        self.storage.atomic_write_json(&self.index_path(), index)
    }

    /// 创建 Skill
    pub async fn create_skill(
        &self,
        new_skill: NewSkill,
        author_agent_id: &str,
        search: &SearchService,
    ) -> Result<Skill, AppError> {
        validate_skill_name(&new_skill.name)?;
        validate_tags(&new_skill.tags)?;
        validate_description(&new_skill.description)?;
        validate_version(&new_skill.version)?;
        validate_skill_content(&new_skill.content, &new_skill.name)?;

        // owner_id 由上游 handler 按 owner_type 设置：
        //   user → identity_id, organization → org_id
        let effective_owner_id = new_skill.owner_id;

        let new_skill_db = DbNewSkill {
            name: new_skill.name.clone(),
            description: normalize_description(&new_skill.description),
            version: new_skill.version.clone(),
            author_agent_id: author_agent_id.to_string(),
            author_identity_id: new_skill.author_identity_id,
            owner_type: new_skill.owner_type.clone(),
            owner_id: effective_owner_id,
            compatibility: ">=1.0.0".to_string(),
            content: new_skill.content.clone(),
            tags: new_skill.tags.clone(),
            dependencies: Vec::new(),
            status: "pending_review".to_string(),
            git_url: new_skill.git_url.clone(),
            visibility: new_skill.visibility.clone().map(|v| match v {
                crate::models::skill_policy::Visibility::Private => "private".to_string(),
                crate::models::skill_policy::Visibility::OrgVisible => "org_visible".to_string(),
                crate::models::skill_policy::Visibility::Marketplace => "marketplace".to_string(),
                crate::models::skill_policy::Visibility::Shared => "shared".to_string(),
            }),
            tools: new_skill.tools.clone(),
        };

        let db_skill = self.skill_repo.create(new_skill_db).await?;
        let skill = Skill {
            id: db_skill.id,
            name: db_skill.name,
            description: db_skill.description,
            version: db_skill.version,
            author_agent_id: db_skill.author_agent_id,
            author_identity_id: db_skill.author_identity_id,
            owner_type: db_skill.owner_type,
            owner_id: db_skill.owner_id,
            created: db_skill.created_at,
            updated: db_skill.updated_at,
            compatibility: db_skill.compatibility,
            dependencies: db_skill.dependencies,
            content: db_skill.content,
            tags: db_skill.tags,
            install_count: db_skill.install_count as u32,
            git_url: db_skill.git_url,
            visibility: match db_skill.visibility.as_str() {
                "private" => crate::models::skill_policy::Visibility::Private,
                "shared" => crate::models::skill_policy::Visibility::Shared,
                "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
                _ => crate::models::skill_policy::Visibility::OrgVisible,
            },
            tools: db_skill.tools,
            status: db_skill.status,
            reviewed_by: db_skill.reviewed_by,
            reviewed_at: db_skill.reviewed_at,
            review_comment: db_skill.review_comment,
            marketplace_status: db_skill.marketplace_status,
            pre_marketplace_visibility: db_skill.pre_marketplace_visibility,
            draft_content: db_skill.draft_content,
        };
        search.add_skill(&skill)?;

        // 写入 SKILL.md 到 skills_dir，确保 get_skill_files 能从磁盘读取
        // （atomic_write 内部已包含 ensure_dir）
        let skill_md_path = self.skill_dir(&skill.name).join("SKILL.md");
        self.storage.atomic_write(&skill_md_path, &skill.content)?;

        info!("Created skill: {}", skill.id);

        Ok(skill)
    }

    /// 将外部目录的所有文件同步到 skills_dir/{name}/
    /// 用于 ZIP/Git 上传场景：git-repos/ 有完整文件，需要拷贝到 skills/ 供 install 读取
    pub fn sync_skill_files_from(
        &self,
        skill_name: &str,
        source_dir: &std::path::Path,
    ) -> Result<(), AppError> {
        let target_dir = self.skill_dir(skill_name);

        // 先删除旧文件（如果有），确保与 Git 仓库完全一致
        if target_dir.exists() {
            std::fs::remove_dir_all(&target_dir).map_err(|e| {
                AppError::InternalError(format!(
                    "Failed to clean existing skill dir {}: {}",
                    target_dir.display(),
                    e
                ))
            })?;
        }
        copy_dir_recursive(source_dir, &target_dir).map_err(|e| {
            AppError::InternalError(format!(
                "Failed to copy files from {} to {}: {}",
                source_dir.display(),
                target_dir.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// 更新 Skill
    pub async fn update_skill(
        &self,
        skill_id: &str,
        update: SkillUpdate,
        _author_agent_id: &str,
        search: &SearchService,
    ) -> Result<Skill, AppError> {
        // 验证更新内容
        if let Some(ref desc) = update.description {
            validate_description(desc)?;
        }
        if let Some(ref tags) = update.tags {
            validate_tags(tags)?;
        }
        if let Some(ref content) = update.content {
            validate_skill_content(content, skill_id)?;
        }

        // 获取文件锁
        let name = self.extract_skill_name(skill_id)?;
        let _lock = get_skill_lock(&name, &self.registry_dir)?;

        self.update_skill_internal(skill_id, update, search).await
    }

    /// 内部更新逻辑
    async fn update_skill_internal(
        &self,
        skill_id: &str,
        update: SkillUpdate,
        search: &SearchService,
    ) -> Result<Skill, AppError> {
        let index = self.load_index()?;

        // 查找 skill — 先尝试文件索引（兼容旧数据），再尝试数据库
        let skill_meta = index.skills.iter().find(|s| s.id == skill_id).cloned();

        if let Some(skill_meta) = skill_meta {
            self.update_skill_file_index(skill_meta, skill_id, update, search)
                .await
        } else {
            self.update_skill_db_fallback(skill_id, update, search)
                .await
        }
    }

    /// 文件索引路径的更新（兼容旧数据）
    async fn update_skill_file_index(
        &self,
        skill_meta: SkillMetadata,
        skill_id: &str,
        update: SkillUpdate,
        search: &SearchService,
    ) -> Result<Skill, AppError> {
        let skill_md_path = self.skill_md_path(&skill_meta.name);

        // 读取现有内容
        let skill_md_content = if skill_md_path.exists() {
            self.storage.read_file(&skill_md_path)?
        } else {
            String::new()
        };

        // 解析现有 SKILL.md
        let mut skill = self.parse_skill_md(&skill_md_content, &skill_meta)?;

        // 应用更新
        if let Some(desc) = update.description {
            skill.description = desc;
        }
        if let Some(tags) = update.tags {
            skill.tags = tags;
        }
        if let Some(content) = update.content {
            skill.content = content;
        }
        if let Some(vis) = update.visibility.clone() {
            skill.visibility = vis;
        }
        skill.updated = Utc::now();

        // 写入文件
        let new_skill_md = self.skill_to_md(&skill)?;
        self.storage.atomic_write(&skill_md_path, &new_skill_md)?;

        // 更新索引中的元数据
        let mut index = self.load_index()?;
        if let Some(idx) = index.skills.iter().position(|s| s.id == skill_id) {
            index.skills[idx].description = skill.description.clone();
            index.skills[idx].tags = skill.tags.clone();
            index.skills[idx].updated = skill.updated;
            index.skills[idx].visibility = skill.visibility.clone();
        }
        self.save_index(&index)?;

        // 更新搜索索引
        search.update_skill(&skill)?;

        info!("Updated skill: {} (file index)", skill_id);

        Ok(skill)
    }

    /// DB 回退：当文件索引中没有该skill时直接更新数据库。
    async fn update_skill_db_fallback(
        &self,
        skill_id: &str,
        update: SkillUpdate,
        search: &SearchService,
    ) -> Result<Skill, AppError> {
        // 确认数据库中存在
        self.skill_repo
            .find_by_id(skill_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
            .ok_or_else(|| AppError::SkillNotFound(skill_id.to_string()))?;

        // 通过 DB repo 更新字段
        let visibility_str = update.visibility.as_ref().map(|v| match v {
            crate::models::skill_policy::Visibility::Private => "private",
            crate::models::skill_policy::Visibility::OrgVisible => "org_visible",
            crate::models::skill_policy::Visibility::Marketplace => "marketplace",
            crate::models::skill_policy::Visibility::Shared => "shared",
        });
        self.skill_repo
            .update(
                skill_id,
                update.description.as_deref(),
                update.content.as_deref(),
                update.tags.clone(),
                visibility_str.as_deref(),
            )
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to update skill: {}", e)))?;

        // 重新从数据库读取完整信息并重建搜索索引
        let updated = self.get_skill(skill_id).await?;
        if let Err(e) = search.update_skill(&updated) {
            tracing::warn!("Failed to update search index for {}: {}", skill_id, e);
        }

        info!("Updated skill: {} (DB fallback)", skill_id);
        Ok(updated)
    }

    /// 删除 Skill
    pub async fn delete_skill(
        &self,
        skill_id: &str,
        search: &SearchService,
    ) -> Result<(), AppError> {
        self.skill_repo.delete(skill_id).await?;
        search.delete_skill(skill_id)?;
        info!("Deleted skill: {}", skill_id);
        Ok(())
    }

    /// 获取 Skill 详情
    pub async fn get_skill(&self, skill_id: &str) -> Result<Skill, AppError> {
        let db_skill = self
            .skill_repo
            .find_by_id(skill_id)
            .await?
            .ok_or_else(|| AppError::SkillNotFound(skill_id.to_string()))?;
        let skill = Skill {
            id: db_skill.id,
            name: db_skill.name,
            description: db_skill.description,
            version: db_skill.version,
            author_agent_id: db_skill.author_agent_id,
            author_identity_id: db_skill.author_identity_id,
            owner_type: db_skill.owner_type,
            owner_id: db_skill.owner_id,
            created: db_skill.created_at,
            updated: db_skill.updated_at,
            compatibility: db_skill.compatibility,
            dependencies: db_skill.dependencies,
            content: db_skill.content,
            tags: db_skill.tags,
            install_count: db_skill.install_count as u32,
            git_url: db_skill.git_url,
            visibility: match db_skill.visibility.as_str() {
                "private" => crate::models::skill_policy::Visibility::Private,
                "shared" => crate::models::skill_policy::Visibility::Shared,
                "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
                _ => crate::models::skill_policy::Visibility::OrgVisible,
            },
            tools: db_skill.tools,
            status: db_skill.status,
            reviewed_by: db_skill.reviewed_by,
            reviewed_at: db_skill.reviewed_at,
            review_comment: db_skill.review_comment,
            marketplace_status: db_skill.marketplace_status,
            pre_marketplace_visibility: db_skill.pre_marketplace_visibility,
            draft_content: db_skill.draft_content,
        };
        Ok(skill)
    }

    /// 获取 Skill 安装信息，返回下载链接而非文件内容
    /// 计算文件统计（数量+总大小），生成数据库下载凭证
    pub async fn get_skill_files(
        &self,
        skill_id: &str,
        identity_id: uuid::Uuid,
        api_key_id: uuid::Uuid,
    ) -> Result<InstallResult, AppError> {
        let skill = self.get_skill(skill_id).await?;
        let skill_dir = self.skill_dir(&skill.name);

        // 确保文件可用：从 git-repos 同步（如果需要）
        if !skill_dir.exists() {
            if let Some(data_dir) = self.registry_dir.parent() {
                let git_repo_dir = data_dir
                    .join("git-repos")
                    .join(format!("skill-{}", skill.name));
                if git_repo_dir.exists() {
                    self.sync_skill_files_from(&skill.name, &git_repo_dir)?;
                }
            }
        }

        // 统计文件数量和总大小
        let (file_count, tarball_size) = if skill_dir.exists() {
            self.count_skill_files(&skill_dir)?
        } else {
            // 文件不存在但有 content：计为 1 个文件（将从 metadata 生成 SKILL.md）
            let fallback_size = skill.content.len() as u64;
            (1, fallback_size)
        };

        // 在数据库中创建下载凭证（5分钟有效），URL 中只暴露随机 UUID
        let expires_in: u64 = 300;
        let token_record = self
            .download_token_repo
            .create(
                &skill.name,
                &skill.version,
                identity_id,
                api_key_id,
                expires_in as i64,
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to create download token: {}", e);
                AppError::InternalError("Failed to generate download token".to_string())
            })?;

        // 构建下载 URL（只带 token，无身份信息泄露）
        let download_url =
            Self::build_download_url(&skill.name, &skill.version, &token_record.token);

        // 生成安装指引
        let install_hint = format!(
            "Download the tarball and extract to your skills directory. Example:\n  mkdir -p skills/{} && curl -sL \"{}\" | tar -xzf - -C skills/{}",
            skill.name, download_url, skill.name
        );

        Ok(InstallResult {
            success: true,
            skill_id: skill.id.clone(),
            name: skill.name.clone(),
            version: skill.version.clone(),
            description: skill.description.clone(),
            author_agent_id: skill.author_agent_id.clone(),
            author_identity_id: skill.author_identity_id,
            owner_type: skill.owner_type.clone(),
            owner_id: skill.owner_id,
            created: skill.created,
            updated: skill.updated,
            install_count: skill.install_count,
            tags: skill.tags.clone(),
            git_url: skill.git_url.clone(),
            dependencies: skill.dependencies.clone(),
            tools: skill.tools.clone(),
            download_url: Some(download_url),
            expires_in,
            install_hint,
            file_count,
            tarball_size,
        })
    }

    /// 构建下载 URL（仅带不透明 token，不暴露身份信息）
    fn build_download_url(name: &str, version: &str, token: &str) -> String {
        let base = std::env::var("AION_HIVE_PUBLIC_URL")
            .unwrap_or_else(|_| {
                format!(
                    "http://localhost:{}",
                    std::env::var("AION_HIVE_HTTP_PORT").unwrap_or_else(|_| "8080".to_string())
                )
            })
            .trim_end_matches('/')
            .to_string();

        format!(
            "{}/api/v1/skills/{}/download/{}?token={}",
            base, name, version, token
        )
    }

    /// 递归统计 skill 目录文件数量和总大小
    fn count_skill_files(&self, dir: &std::path::Path) -> Result<(usize, u64), AppError> {
        let mut count = 0usize;
        let mut total_size = 0u64;

        for entry in std::fs::read_dir(dir).map_err(|e| {
            AppError::RegistryReadFailed(format!("Failed to read dir {}: {}", dir.display(), e))
        })? {
            let entry = entry.map_err(|e| {
                AppError::RegistryReadFailed(format!("Failed to read entry: {}", e))
            })?;
            let path = entry.path();
            if path.is_dir() {
                let (sub_count, sub_size) = self.count_skill_files(&path)?;
                count += sub_count;
                total_size += sub_size;
            } else {
                let metadata = std::fs::metadata(&path).map_err(|e| {
                    AppError::RegistryReadFailed(format!("Failed to read metadata: {}", e))
                })?;
                count += 1;
                total_size += metadata.len();
            }
        }

        // 兜底：至少有 1 个 SKILL.md
        Ok((count.max(1), total_size.max(1)))
    }

    /// 列出所有 Skills
    pub async fn list_skills(&self) -> Result<Vec<SkillMetadata>, AppError> {
        self.list_skills_sorted(1000, 0, "created").await
    }

    /// 列出 Skills，支持分页和排序
    pub async fn list_skills_sorted(
        &self,
        limit: i64,
        offset: i64,
        sort_by: &str,
    ) -> Result<Vec<SkillMetadata>, AppError> {
        let db_skills = self.skill_repo.list_sorted(limit, offset, sort_by).await?;
        Ok(db_skills
            .into_iter()
            .map(|m| SkillMetadata {
                id: m.id,
                name: m.name,
                description: m.description,
                version: m.version,
                author_agent_id: m.author_agent_id,
                author_identity_id: m.author_identity_id,
                author_name: m.author_name,
                owner_type: m.owner_type,
                owner_id: m.owner_id,
                tags: m.tags,
                created: m.created_at,
                updated: m.updated_at,
                install_count: m.install_count as u32,
                status: m.status,
                git_url: m.git_url,
                visibility: match m.visibility.as_str() {
                    "private" => crate::models::skill_policy::Visibility::Private,
                    "shared" => crate::models::skill_policy::Visibility::Shared,
                    "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
                    _ => crate::models::skill_policy::Visibility::OrgVisible,
                },
                reviewed_by: m.reviewed_by,
                reviewed_at: m.reviewed_at,
                review_comment: m.review_comment,
                marketplace_status: m.marketplace_status,
                pre_marketplace_visibility: m.pre_marketplace_visibility,
                draft_content: m.draft_content,
            })
            .collect())
    }

    /// 获取 Skills 数量
    pub async fn count(&self) -> Result<u32, AppError> {
        let count = self.skill_repo.count().await?;
        Ok(count as u32)
    }

    /// 根据可见性规则过滤 Skills 列表（供 MCP 和 REST API 共用）
    ///
    /// 规则（与 `permission::check_skill_permission` Read 一致）：
    /// - `published + marketplace` → 所有人可见
    /// - 个人所有者的 Skill → 所有者可见（任何状态）
    /// - 组织所有者的 Skill → 同组织成员可见（任何状态）
    /// - 无身份 → 仅 `published + marketplace`
    pub fn filter_skills_visible_to(
        skills: Vec<SkillMetadata>,
        identity_id: Option<Uuid>,
        member_org_ids: &[Uuid],
    ) -> Vec<SkillMetadata> {
        let Some(id_id) = identity_id else {
            return skills
                .into_iter()
                .filter(|s| {
                    s.status == "published"
                        && matches!(
                            s.visibility,
                            crate::models::skill_policy::Visibility::Marketplace
                        )
                })
                .collect();
        };

        skills
            .into_iter()
            .filter(|s| {
                // published + marketplace → 所有人可见
                if s.status == "published"
                    && matches!(
                        s.visibility,
                        crate::models::skill_policy::Visibility::Marketplace
                    )
                {
                    return true;
                }
                // 个人所有者的 Skill（任何状态）→ 所有者可见
                if s.owner_type == "user"
                    && (s.owner_id == Some(id_id) || s.author_identity_id == Some(id_id))
                {
                    return true;
                }
                // 组织 Skill → 同组织成员可见（任何状态）
                if s.owner_type == "organization" {
                    if let Some(org_id) = s.owner_id {
                        if member_org_ids.contains(&org_id) {
                            return true;
                        }
                    }
                }
                false
            })
            .collect()
    }

    /// 递增 Skill 安装计数（每次 pull 时调用）
    pub async fn increment_install_count(&self, skill_id: &str) -> Result<(), AppError> {
        self.skill_repo
            .increment_install_count(skill_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 列出同名 Skill 的所有版本
    pub async fn list_versions(&self, name: &str) -> Result<Vec<SkillMetadata>, AppError> {
        let db_skills = self.skill_repo.list_by_name(name).await?;
        Ok(db_skills
            .into_iter()
            .map(|m| SkillMetadata {
                id: m.id,
                name: m.name,
                description: m.description,
                version: m.version,
                author_agent_id: m.author_agent_id,
                author_identity_id: m.author_identity_id,
                author_name: m.author_name,
                owner_type: m.owner_type,
                owner_id: m.owner_id,
                tags: m.tags,
                created: m.created_at,
                updated: m.updated_at,
                install_count: m.install_count as u32,
                status: m.status,
                git_url: m.git_url,
                visibility: match m.visibility.as_str() {
                    "private" => crate::models::skill_policy::Visibility::Private,
                    "shared" => crate::models::skill_policy::Visibility::Shared,
                    "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
                    _ => crate::models::skill_policy::Visibility::OrgVisible,
                },
                reviewed_by: m.reviewed_by,
                reviewed_at: m.reviewed_at,
                review_comment: m.review_comment,
                marketplace_status: m.marketplace_status,
                pre_marketplace_visibility: m.pre_marketplace_visibility,
                draft_content: m.draft_content,
            })
            .collect())
    }

    /// 提取 skill 名称
    fn extract_skill_name(&self, skill_id: &str) -> Result<String, AppError> {
        // skill-{name}-{version} 格式
        let parts: Vec<&str> = skill_id.split('-').collect();
        if parts.len() < 3 {
            return Err(AppError::SkillInvalidFormat(
                "Invalid skill ID format".to_string(),
            ));
        }
        // 跳过 "skill" 前缀，取中间部分作为名称
        // 格式: skill-{name}-{version}
        // 名称可能包含连字符，所以需要找到最后一个 - 之前的部分
        let version_part = parts[parts.len() - 1];
        // 检查最后一部分是否像版本号 (x.y.z)
        if !version_part.contains('.') {
            return Err(AppError::SkillInvalidFormat(
                "Invalid skill ID format".to_string(),
            ));
        }
        let name = parts[1..parts.len() - 1].join("-");
        Ok(name)
    }

    /// 将 Skill 转换为 SKILL.md 格式
    fn skill_to_md(&self, skill: &Skill) -> Result<String, AppError> {
        let frontmatter = format!(
            r#"---
name: {}
description: {}
tags: [{}]
version: {}
author_agent_id: {}
created: {}
updated: {}
compatibility: "{}"
dependencies: [{}]
---

{}
"#,
            skill.name,
            Self::escape_yaml_string(&skill.description),
            skill.tags.join(", "),
            skill.version,
            skill.author_agent_id,
            skill.created.to_rfc3339(),
            skill.updated.to_rfc3339(),
            skill.compatibility,
            skill.dependencies.join(", "),
            skill.content
        );
        Ok(frontmatter)
    }

    /// 转义 YAML 字符串
    fn escape_yaml_string(s: &str) -> String {
        if s.contains(':')
            || s.contains('#')
            || s.contains('"')
            || s.contains('\n')
            || s.contains('\r')
        {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }

    /// 解析 SKILL.md 内容
    fn parse_skill_md(&self, content: &str, meta: &SkillMetadata) -> Result<Skill, AppError> {
        // 简单的 frontmatter 解析
        let (frontmatter, body) = if let Some(_end) = content.find("---") {
            if content.starts_with("---") {
                let rest = &content[3..];
                if let Some(end) = rest.find("---") {
                    (&rest[..end], rest[end + 3..].trim())
                } else {
                    return Err(AppError::SkillInvalidFormat(
                        "Invalid SKILL.md format".to_string(),
                    ));
                }
            } else {
                ("", content)
            }
        } else {
            ("", content)
        };

        // 解析 frontmatter (简化版)
        let mut description = meta.description.clone();
        let mut tags = meta.tags.clone();
        let mut compatibility = ">=1.0.0".to_string();
        let mut dependencies: Vec<String> = Vec::new();

        let mut current_key: Option<&str> = None;
        let mut multiline_buffer: Vec<String> = Vec::new();
        let mut is_multiline = false;

        for raw_line in frontmatter.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                if is_multiline {
                    multiline_buffer.push(String::new());
                }
                continue;
            }

            // 检测新 key: value 行
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim();
                let value = line[colon_pos + 1..].trim();

                // 遇到新 key 时保存之前的多行文本
                if is_multiline && !multiline_buffer.is_empty() {
                    if let Some(prev_key) = current_key {
                        if prev_key == "description" {
                            description = multiline_buffer.join(" ");
                            description = normalize_description(&description);
                        }
                    }
                    multiline_buffer.clear();
                    is_multiline = false;
                }

                // YAML 块标量: | 或 >
                if value == "|" || value == "|-" || value == ">" || value == ">-" || value == "|+" || value == ">+" {
                    current_key = Some(key);
                    is_multiline = true;
                    multiline_buffer = Vec::new();
                    continue;
                }

                // 普通键值对
                current_key = Some(key);

                let cleaned = value
                    .trim_start_matches('"').trim_end_matches('"')
                    .trim_start_matches('\'').trim_end_matches('\'');

                match key {
                    "description" => description = normalize_description(cleaned),
                    "tags" => {
                        if value.starts_with('[') && value.ends_with(']') {
                            tags = value[1..value.len() - 1]
                                .split(',')
                                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }
                    }
                    "compatibility" => compatibility = cleaned.to_string(),
                    "dependencies" => {
                        if value.starts_with('[') && value.ends_with(']') {
                            dependencies = value[1..value.len() - 1]
                                .split(',')
                                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }
                    }
                    _ => {}
                }
            } else if is_multiline {
                // 多行文本内容行
                multiline_buffer.push(raw_line.trim().to_string());
            }
        }

        // 处理结尾的多行文本
        if is_multiline && !multiline_buffer.is_empty() {
            if current_key == Some("description") {
                description = multiline_buffer.join(" ");
                description = normalize_description(&description);
            }
        }

        Ok(Skill {
            id: meta.id.clone(),
            name: meta.name.clone(),
            description,
            tags,
            version: meta.version.clone(),
            author_agent_id: meta.author_agent_id.clone(),
            author_identity_id: meta.author_identity_id,
            owner_type: meta.owner_type.clone(),
            owner_id: meta.owner_id,
            created: meta.created,
            updated: meta.updated,
            compatibility,
            dependencies,
            content: body.to_string(),
            install_count: meta.install_count,
            git_url: meta.git_url.clone(),
            visibility: meta.visibility.clone(),
            status: meta.status.clone(),
            tools: Vec::new(),
            reviewed_by: meta.reviewed_by,
            reviewed_at: meta.reviewed_at,
            review_comment: meta.review_comment.clone(),
            marketplace_status: meta.marketplace_status.clone(),
            pre_marketplace_visibility: meta.pre_marketplace_visibility.clone(),
            draft_content: None,
        })
    }

    /// 增量添加 Skills（用于启动时从文件同步）
    pub fn sync_from_files(&self, search: &SearchService) -> Result<u32, AppError> {
        let mut count = 0u32;

        // 读取索引
        let mut index = self.load_index()?;
        let existing_ids: std::collections::HashSet<String> =
            index.skills.iter().map(|s| s.id.clone()).collect();

        // 扫描 skills 目录
        if self.skills_dir.exists() {
            for entry in std::fs::read_dir(&self.skills_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    let skill_md_path = path.join("SKILL.md");
                    if skill_md_path.exists() {
                        if let Ok(content) = self.storage.read_file(&skill_md_path) {
                            if let Ok(skill) = self.parse_skill_md(
                                &content,
                                &crate::models::skill::SkillMetadata {
                                    id: "temp".to_string(),
                                    name: path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                    description: String::new(),
                                    tags: Vec::new(),
                                    version: "1.0.0".to_string(),
                                    author_agent_id: String::new(),
                                    author_identity_id: None,
                                    author_name: None,
                                    owner_type: "user".to_string(),
                                    owner_id: None,
                                    created: Utc::now(),
                                    updated: Utc::now(),
                                    install_count: 0,
                                    status: "published".to_string(),
                                    git_url: None,
                                    visibility: crate::models::skill_policy::Visibility::OrgVisible,
                                    reviewed_by: None,
                                    reviewed_at: None,
                                    review_comment: None,
                                    marketplace_status: None,
                                    pre_marketplace_visibility: None,
                                    draft_content: None,
                                },
                            ) {
                                if !existing_ids.contains(&skill.id) {
                                    index.skills.push((&skill).into());
                                    search.add_skill(&skill)?;
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        if count > 0 {
            self.save_index(&index)?;
            info!("Synced {} skills from files", count);
        }

        Ok(count)
    }
}
