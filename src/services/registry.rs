//! 注册服务 - Skills CRUD

use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use tracing::info;

use crate::db::repositories::skill::{NewSkill as DbNewSkill, SkillRepository};
use crate::models::error::AppError;
use crate::services::skill_git::copy_dir_recursive;
use crate::models::skill::{InstallResult, NewSkill, Skill, SkillMetadata, SkillUpdate};
use crate::schemas::validation::{
    validate_description, validate_skill_content, validate_skill_name, validate_tags,
    validate_version,
};
use crate::services::storage::{get_skill_lock, StorageService};
use crate::services::SearchService;

/// 注册服务
#[derive(Debug, Clone)]
pub struct RegistryService {
    skills_dir: PathBuf,
    registry_dir: PathBuf,
    storage: StorageService,
    skill_repo: SkillRepository,
}

impl RegistryService {
    pub fn new(skills_dir: PathBuf, registry_dir: PathBuf, skill_repo: SkillRepository) -> Self {
        let storage = StorageService::new(registry_dir.clone());
        Self {
            skills_dir,
            registry_dir,
            storage,
            skill_repo,
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

        let new_skill_db = DbNewSkill {
            name: new_skill.name.clone(),
            description: new_skill.description.clone(),
            version: new_skill.version.clone(),
            author_agent_id: author_agent_id.to_string(),
            author_identity_id: None,
            owner_type: new_skill.owner_type.clone(),
            owner_id: new_skill.owner_id,
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
            visibility: crate::models::skill_policy::Visibility::OrgVisible,
            tools: db_skill.tools,
            review_status: db_skill.review_status,
            reviewed_by: db_skill.reviewed_by,
            reviewed_at: db_skill.reviewed_at,
            review_comment: db_skill.review_comment,
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
        self.skill_repo
            .update(
                skill_id,
                update.description.as_deref(),
                update.content.as_deref(),
                update.tags.clone(),
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
            visibility: crate::models::skill_policy::Visibility::OrgVisible,
            tools: db_skill.tools,
            review_status: db_skill.review_status,
            reviewed_by: db_skill.reviewed_by,
            reviewed_at: db_skill.reviewed_at,
            review_comment: db_skill.review_comment,
        };
        Ok(skill)
    }

    /// 获取 Skill 全部文件内容，供 install 使用
    /// 优先从磁盘读取；磁盘没有 SKILL.md 时从 DB content 兜底，content 也为空时从 metadata 生成
    pub async fn get_skill_files(&self, skill_id: &str) -> Result<InstallResult, AppError> {
        // 从 DB 获取元数据（包含 content 字段）
        let skill = self.get_skill(skill_id).await?;

        // 从磁盘递归读取 skill 目录下的所有文件
        let mut files: Vec<crate::models::skill::SkillFile> = Vec::new();
        let skill_dir = self.skill_dir(&skill.name);

        if skill_dir.exists() {
            self.collect_skill_files(&skill_dir, "", &mut files)?;
        } else {
            // 回退：尝试从 git-repos/skill-{name}/ 同步文件
            if let Some(data_dir) = self.registry_dir.parent() {
                let git_repo_dir = data_dir.join("git-repos").join(format!("skill-{}", skill.name));
                if git_repo_dir.exists() {
                    if let Ok(()) = self.sync_skill_files_from(&skill.name, &git_repo_dir) {
                        if skill_dir.exists() {
                            self.collect_skill_files(&skill_dir, "", &mut files)?;
                        }
                    }
                }
            }
        }

        // 确保 SKILL.md 一定存在：磁盘有就用磁盘的，否则 content 优先，最后从 metadata 生成
        let has_skill_md = files.iter().any(|f| f.path == "SKILL.md");
        if !has_skill_md {
            let content = if !skill.content.is_empty() {
                skill.content.clone()
            } else {
                // content 也是空的 → 用 description/name/tags 生成最小 SKILL.md
                let mut md = String::new();
                md.push_str(&format!("# {}\n\n", skill.name));
                md.push_str(&format!("{}\n\n", skill.description));
                md.push_str("## Version\n\n");
                md.push_str(&format!("Version: {}\n", skill.version));
                if !skill.tags.is_empty() {
                    md.push_str(&format!("Tags: {}\n", skill.tags.join(", ")));
                }
                md
            };
            files.push(crate::models::skill::SkillFile {
                path: "SKILL.md".to_string(),
                content,
            });
        }

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
            files,
        })
    }

    /// 递归收集 skill 目录下的所有文件（包括 SKILL.md）
    fn collect_skill_files(
        &self,
        dir: &std::path::Path,
        prefix: &str,
        files: &mut Vec<crate::models::skill::SkillFile>,
    ) -> Result<(), AppError> {
        for entry in std::fs::read_dir(dir).map_err(|e| {
            AppError::RegistryReadFailed(format!("Failed to read dir {}: {}", dir.display(), e))
        })? {
            let entry = entry.map_err(|e| {
                AppError::RegistryReadFailed(format!("Failed to read entry: {}", e))
            })?;
            let path = entry.path();
            let rel_path = if prefix.is_empty() {
                entry.file_name().to_string_lossy().to_string()
            } else {
                format!("{}/{}", prefix, entry.file_name().to_string_lossy())
            };

            if path.is_dir() {
                self.collect_skill_files(&path, &rel_path, files)?;
            } else {
                let content = self.storage.read_file(&path)?;
                files.push(crate::models::skill::SkillFile {
                    path: rel_path,
                    content,
                });
            }
        }
        Ok(())
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
                review_status: m.review_status,
                reviewed_by: m.reviewed_by,
                reviewed_at: m.reviewed_at,
                review_comment: m.review_comment,
            })
            .collect())
    }

    /// 获取 Skills 数量
    pub async fn count(&self) -> Result<u32, AppError> {
        let count = self.skill_repo.count().await?;
        Ok(count as u32)
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
                review_status: m.review_status,
                reviewed_by: m.reviewed_by,
                reviewed_at: m.reviewed_at,
                review_comment: m.review_comment,
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

        for line in frontmatter.lines() {
            let line = line.trim();
            if line.starts_with("description:") {
                description = line.trim_start_matches("description:").trim().to_string();
            } else if line.starts_with("tags:") {
                let tags_str = line.trim_start_matches("tags:").trim();
                if tags_str.starts_with('[') && tags_str.ends_with(']') {
                    tags = tags_str[1..tags_str.len() - 1]
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            } else if line.starts_with("compatibility:") {
                compatibility = line.trim_start_matches("compatibility:").trim().to_string();
            } else if line.starts_with("dependencies:") {
                let deps_str = line.trim_start_matches("dependencies:").trim();
                if deps_str.starts_with('[') && deps_str.ends_with(']') {
                    dependencies = deps_str[1..deps_str.len() - 1]
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
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
            tools: Vec::new(),
            review_status: meta.review_status.clone(),
            reviewed_by: meta.reviewed_by,
            reviewed_at: meta.reviewed_at,
            review_comment: meta.review_comment.clone(),
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
                                    owner_type: "user".to_string(),
                                    owner_id: None,
                                    created: Utc::now(),
                                    updated: Utc::now(),
                                    install_count: 0,
                                    status: "published".to_string(),
                                    git_url: None,
                                    visibility: crate::models::skill_policy::Visibility::OrgVisible,
                                    review_status: "published".to_string(),
                                    reviewed_by: None,
                                    reviewed_at: None,
                                    review_comment: None,
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

