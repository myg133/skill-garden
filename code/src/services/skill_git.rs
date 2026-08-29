//! Skill Git Service — ZIP 上传自动解压 + 本地 Git 仓库版本管理
//!
//! 每个 skill 对应一个普通 Git 仓库，文件直接放在仓库工作目录中。
//! 版本用 Git tag（v1.0.0, v1.1.0...）管理。
//! 远程同步（GitLab）作为可选扩展，后续通过管理后台触发。

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;
use zip::ZipArchive;

use crate::db::repositories::skill::SkillRepository;
use crate::db::repositories::version::{NewSkillVersion, VersionRepository};
use crate::models::error::AppError;
use crate::models::skill::NewSkill;
use crate::schemas::validation::normalize_description;
use crate::services::search::SearchService;
use crate::services::RegistryService;

/// GitLab 远程仓库配置
#[derive(Debug, Clone)]
pub struct GitRemoteConfig {
    /// GitLab 实例 URL，如 https://gitlab.example.com
    pub gitlab_url: String,
    /// GitLab group/namespace，如 skill-garden
    pub gitlab_group: String,
    /// Personal Access Token 或 Project Access Token
    pub gitlab_token: String,
    /// 是否推送到远程（false 则仅本地管理）
    pub push_enabled: bool,
}

impl GitRemoteConfig {
    /// 从环境变量读取 GitLab 配置
    pub fn from_env() -> Self {
        Self {
            gitlab_url: std::env::var("GITLAB_URL")
                .unwrap_or_else(|_| "https://gitlab.com".to_string()),
            gitlab_group: std::env::var("GITLAB_GROUP")
                .unwrap_or_else(|_| "skill-garden".to_string()),
            gitlab_token: std::env::var("GITLAB_TOKEN").unwrap_or_default(),
            push_enabled: std::env::var("GITLAB_PUSH_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }

    /// 构造远端 URL: https://oauth2:{token}@gitlab.example.com/group/repo.git
    pub fn remote_url(&self, repo_name: &str) -> String {
        format!(
            "https://oauth2:{}@{}/{}/{}.git",
            self.gitlab_token,
            self.gitlab_url.trim_end_matches('/'),
            self.gitlab_group,
            repo_name
        )
    }
}

/// 上传 ZIP 包时解析出的 SKILL.md 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSkillMetadata {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub version: Option<String>,
    pub dependencies: Vec<String>,
    pub compatibility: String,
}

/// ZIP 解压验证结果
#[derive(Debug)]
pub struct UnpackedSkill {
    /// 临时解压目录路径
    pub extract_dir: PathBuf,
    /// 解压出的所有文件路径（相对于解压目录）
    pub files: Vec<String>,
    /// SKILL.md 文件内容
    pub skill_md_content: String,
    /// 解析出的元数据
    pub metadata: ParsedSkillMetadata,
    /// 总解压大小 (bytes)
    pub total_size_bytes: u64,
}

/// 上传流程的结果
#[derive(Debug, Serialize)]
pub struct UploadResult {
    pub skill_id: String,
    pub skill_name: String,
    pub version: String,
    pub git_commit: String,
    pub git_tag: String,
    pub git_repo_name: String,
    pub is_new_skill: bool,
    pub files: Vec<String>,
}

/// 预览阶段单文件信息
#[derive(Debug, Clone, Serialize)]
pub struct PreviewFile {
    pub path: String,
    pub size: u64,
}

/// 预览结果（解压后，未提交 Git/DB）
#[derive(Debug, Clone, Serialize)]
pub struct PreviewResult {
    pub preview_id: String,
    pub metadata: ParsedSkillMetadata,
    pub files: Vec<PreviewFile>,
    pub total_files: usize,
    pub total_size: u64,
}

/// Skill Git 版本管理服务
#[derive(Debug, Clone)]
pub struct SkillGitService {
    /// Git 仓库存储根目录: {data_dir}/git-repos/
    /// 每个 skill 对应一个子目录: {repos_dir}/skill-{name}/
    pub repos_dir: PathBuf,
    /// 临时目录（ZIP 解压、预览）
    temp_dir: PathBuf,
    /// GitLab 远程配置
    pub remote_config: GitRemoteConfig,
}

impl SkillGitService {
    /// 允许上传的最大 ZIP 大小: 50 MB
    pub const MAX_UPLOAD_SIZE: u64 = 50 * 1024 * 1024;

    pub fn new(data_dir: PathBuf) -> Self {
        let repos_dir = data_dir.join("git-repos");
        let temp_dir = data_dir.join("tmp");
        Self {
            repos_dir,
            temp_dir,
            remote_config: GitRemoteConfig::from_env(),
        }
    }

    /// 构造 skill 本地仓库路径
    pub fn repo_path(&self, skill_name: &str) -> PathBuf {
        self.repos_dir.join(format!("skill-{}", skill_name))
    }

    /// 构造 releases 目录路径
    pub fn releases_dir(&self) -> PathBuf {
        self.repos_dir.parent().unwrap_or(&self.repos_dir).join("releases")
    }

    /// 生成版本 tarball（审核通过后调用）
    pub fn generate_release_tarball(
        &self,
        skill_name: &str,
        version: &str,
    ) -> Result<PathBuf, AppError> {
        let repo_dir = self.repo_path(skill_name);
        let releases_dir = self.releases_dir().join(skill_name);
        fs::create_dir_all(&releases_dir)
            .map_err(|e| AppError::InternalError(format!("Failed to create releases dir: {}", e)))?;

        let tarball_path = releases_dir.join(format!("v{}.tar.gz", version));
        let tag_name = format!("v{}", version);

        let output = Command::new("git")
            .current_dir(&repo_dir)
            .args(["archive", "--format=tar.gz", &tag_name, "-o"])
            .arg(&tarball_path)
            .output()
            .map_err(|e| AppError::InternalError(format!("git archive failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::InternalError(format!(
                "git archive failed: {}", stderr
            )));
        }

        info!("Generated release tarball: {}", tarball_path.display());
        Ok(tarball_path)
    }

    /// 确保服务目录存在
    pub fn ensure_dirs(&self) -> Result<(), AppError> {
        fs::create_dir_all(&self.repos_dir).map_err(|e| {
            AppError::InternalError(format!("Failed to create git-repos dir: {}", e))
        })?;
        fs::create_dir_all(&self.temp_dir)
            .map_err(|e| AppError::InternalError(format!("Failed to create temp dir: {}", e)))?;
        Ok(())
    }

    // ==================== 完整上传：ZIP → 解压验证 → 本地 Git 提交 → DB 记录 ====================

    /// 完整的上传流程：ZIP → 解压验证 → 拷贝到 Git 仓库 → commit + tag → DB 记录
    pub fn process_upload(
        &self,
        zip_data: &[u8],
        author_agent_id: &str,
        _author_identity_id: Option<Uuid>,
        owner_type: &str,
        owner_id: Option<Uuid>,
        registry: &RegistryService,
        search: &SearchService,
        skill_repo: &SkillRepository,
        version_repo: &VersionRepository,
    ) -> Result<UploadResult, AppError> {
        // 1. 解压 & 验证
        let unpacked = self.unpack_and_validate(zip_data)?;
        let metadata = unpacked.metadata;

        // 2. 确定版本号
        let latest_version = tokio::runtime::Handle::current()
            .block_on(async { version_repo.get_latest_version(&metadata.name).await })
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        let version = resolve_version(&metadata.name, &latest_version, &metadata.version)?;

        // 3. 检查版本是否已存在
        let existing_skill = tokio::runtime::Handle::current()
            .block_on(async {
                skill_repo
                    .find_by_id(&format!("skill-{}-{}", metadata.name, version))
                    .await
            })
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        if existing_skill.is_some() {
            return Err(AppError::SkillAlreadyExists(format!(
                "Skill {} version {} already exists. Upload with a new version.",
                metadata.name, version
            )));
        }

        // 4. 准备 Git 仓库（首次 init，后续复用）
        let repo_dir = self.repo_path(&metadata.name);
        let repo_is_new = self.prepare_repo(&repo_dir)?;

        // 5. 清空旧文件 + 拷贝新文件到仓库工作目录
        self.clean_working_dir(&repo_dir)?;
        copy_dir_recursive(&unpacked.extract_dir, &repo_dir)
            .map_err(|e| AppError::InternalError(format!("Copy to repo failed: {}", e)))?;

        // 6. Git add + commit（不打 tag，审核通过后才打）
        let commit_msg = format!(
            "v{}: {} by {}",
            version,
            if repo_is_new { "Initial skill upload" } else { "New version upload" },
            author_agent_id
        );
        let commit_hash = self.git_commit_only(&repo_dir, &commit_msg)?;

        // 7. 写入 skill 到 registry（文件系统 + DB + 搜索索引）
        // tags/description：首次上传用 ZIP 中的值，更新上传继承 DB 当前值
        let (desc, tgs) = if latest_version.is_none() {
            (metadata.description.clone(), metadata.tags.clone())
        } else {
            let current = tokio::runtime::Handle::current().block_on(async {
                skill_repo.find_latest_by_name(&metadata.name).await
            }).unwrap_or(None);
            (
                current.as_ref().map(|s| s.description.clone()).unwrap_or_default(),
                current.as_ref().map(|s| s.tags.clone()).unwrap_or_default(),
            )
        };

        let new_skill = NewSkill {
            name: metadata.name.clone(),
            description: desc,
            tags: tgs,
            content: unpacked.skill_md_content.clone(),
            version: version.clone(),
            git_url: None,
            visibility: None,
            tools: None,
            owner_type: owner_type.to_string(),
            owner_id,
            author_identity_id: None,
        };

        let skill = tokio::runtime::Handle::current().block_on(async {
            registry.create_skill(new_skill, author_agent_id, search).await
        })?;

        // 不 sync_skill_files_from、不写 skill_versions、不打 tag
        // 这些操作在审核通过后由 approve_org_skill_handler 完成

        tokio::runtime::Handle::current()
            .block_on(async {
                skill_repo.update_status(&skill.id, "pending_review", None, None).await
            })
            .map_err(|e| AppError::InternalError(format!("Failed to update status: {}", e)))?;

        // 8. 清理临时解压目录
        let _ = fs::remove_dir_all(&unpacked.extract_dir);

        info!(
            "Skill {} v{} uploaded (commit={}, files={}, is_new={})",
            metadata.name, version, commit_hash, unpacked.files.len(), repo_is_new
        );

        Ok(UploadResult {
            skill_id: skill.id,
            skill_name: metadata.name.clone(),
            version: version.clone(),
            git_commit: commit_hash,
            git_tag: String::new(), // tag 审核通过后才打
            git_repo_name: format!("skill-{}", metadata.name),
            is_new_skill: latest_version.is_none(),
            files: unpacked.files,
        })
    }

    // ==================== 预览模式（仅解压，不提交） ====================

    /// 上传 ZIP → 解压验证 → 保存到临时预览目录 → 返回文件列表+元数据
    pub fn preview_upload(&self, zip_data: &[u8]) -> Result<PreviewResult, AppError> {
        let unpacked = self.unpack_and_validate(zip_data)?;
        let preview_id = Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("p")
            .to_string();

        // 移动解压目录到 preview 子目录（带 preview_id）
        let preview_dir = self.temp_dir.join(format!("preview-{}", preview_id));
        // 重命名 extract_dir 到 preview_dir
        if preview_dir.exists() {
            fs::remove_dir_all(&preview_dir).ok();
        }
        fs::rename(&unpacked.extract_dir, &preview_dir).map_err(|e| {
            AppError::InternalError(format!("Failed to move to preview dir: {}", e))
        })?;

        let files: Vec<PreviewFile> = unpacked
            .files
            .iter()
            .map(|f| {
                let size = fs::metadata(preview_dir.join(f))
                    .map(|m| m.len())
                    .unwrap_or(0);
                PreviewFile {
                    path: f.clone(),
                    size,
                }
            })
            .collect();

        let total_files = files.len();
        let total_size = files.iter().map(|f| f.size).sum();

        info!(
            "Preview {} created: {} files, {} bytes, skill={} v{}",
            preview_id,
            total_files,
            total_size,
            unpacked.metadata.name,
            unpacked.metadata.version.as_deref().unwrap_or("?")
        );

        Ok(PreviewResult {
            preview_id,
            metadata: unpacked.metadata,
            files,
            total_files,
            total_size,
        })
    }

    /// 获取预览中的某个文件内容
    pub fn get_preview_file(
        &self,
        preview_id: &str,
        file_path: &str,
    ) -> Result<(Vec<u8>, String, u64), AppError> {
        let preview_dir = self.temp_dir.join(format!("preview-{}", preview_id));
        let safe_path = sanitize_path(file_path)?;
        let full_path = preview_dir.join(&safe_path);

        // 安全检查
        if !full_path.starts_with(&preview_dir) {
            return Err(AppError::ValidationError(
                "Path traversal attempt".to_string(),
            ));
        }

        if !full_path.exists() {
            return Err(AppError::FileNotFound(format!(
                "File not found in preview: {}",
                file_path
            )));
        }

        if !full_path.is_file() {
            return Err(AppError::ValidationError(format!(
                "Not a file: {}",
                file_path
            )));
        }

        let content = fs::read(&full_path)
            .map_err(|e| AppError::InternalError(format!("Failed to read file: {}", e)))?;

        let size = content.len() as u64;

        // 判断是否为文本文件（尝试 UTF-8 解码）
        let mime_type = if String::from_utf8(content.clone()).is_ok() {
            "text/plain".to_string()
        } else {
            "application/octet-stream".to_string()
        };

        Ok((content, mime_type, size))
    }

    /// 确认上传：从预览目录复制到 Git 仓库 → commit + tag → DB insert
    pub async fn confirm_upload_from_preview(
        &self,
        preview_id: &str,
        author_agent_id: &str,
        author_identity_id: Option<Uuid>,
        owner_type: &str,
        owner_id: Option<Uuid>,
        registry: &RegistryService,
        search: &SearchService,
        skill_repo: &SkillRepository,
        version_repo: &VersionRepository,
    ) -> Result<UploadResult, AppError> {
        let preview_dir = self.temp_dir.join(format!("preview-{}", preview_id));

        if !preview_dir.exists() {
            return Err(AppError::FileNotFound(format!(
                "Preview session {} not found",
                preview_id
            )));
        }

        // 读取 SKILL.md 获取元数据
        let skill_md_path = find_skill_md_recursive(&preview_dir)?;
        let skill_md_content = fs::read_to_string(&skill_md_path)
            .map_err(|e| AppError::InternalError(format!("Failed to read SKILL.md: {}", e)))?;
        let metadata = parse_skill_md_frontmatter(&skill_md_content)?;

        // 收集文件列表
        let mut files: Vec<String> = Vec::new();
        collect_files(&preview_dir, &preview_dir, &mut files)
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        // 确定版本号
        let latest_version = version_repo
            .get_latest_version(&metadata.name)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        let version = resolve_version(&metadata.name, &latest_version, &metadata.version)?;

        // 检查版本是否已存在
        let existing_skill = skill_repo
            .find_by_id(&format!("skill-{}-{}", metadata.name, version))
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        if existing_skill.is_some() {
            // 如果 skill_versions 中没有版本记录，说明是上次上传部分失败残留
            // 清理后继续（正常重试）
            if latest_version.is_none() {
                warn!(
                    "Cleaning up partially-failed upload for {} v{} (no version record found)",
                    metadata.name, version
                );
                skill_repo
                    .delete(&format!("skill-{}-{}", metadata.name, version))
                    .await
                    .map_err(|e| AppError::InternalError(e.to_string()))?;
            } else {
                // 正常版本冲突
                let suggested = latest_version.as_ref().map_or_else(
                    || "1.0.0".to_string(),
                    |v| {
                        semver::Version::parse(v)
                            .map(|sv| format!("{}.{}.{}", sv.major, sv.minor, sv.patch + 1))
                            .unwrap_or_else(|_| "1.0.1".to_string())
                    },
                );
                return Err(AppError::SkillAlreadyExists(format!(
                    "版本 {} 已存在，建议使用版本 {}",
                    version, suggested
                )));
            }
        }

        // 准备 Git 仓库
        let repo_dir = self.repo_path(&metadata.name);
        let repo_is_new = self.prepare_repo(&repo_dir)?;

        // 清空旧文件 + 拷贝
        self.clean_working_dir(&repo_dir)?;
        let mut _total_size: u64 = 0;
        for rel_path in &files {
            let src = preview_dir.join(rel_path);
            let dst = repo_dir.join(rel_path);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| AppError::InternalError(format!("Create dir failed: {}", e)))?;
            }
            fs::copy(&src, &dst)
                .map_err(|e| AppError::InternalError(format!("Copy file failed: {}", e)))?;
            _total_size += fs::metadata(&src).map(|m| m.len()).unwrap_or(0) as u64;
        }

        // 检查是否有实际变更：用 git diff 对比新旧文件
        if !repo_is_new && latest_version.is_some() {
            let diff_output = Command::new("git")
                .current_dir(&repo_dir)
                .args(["diff", "--stat", "HEAD"])
                .output()
                .map_err(|e| AppError::InternalError(format!("git diff failed: {}", e)))?;

            let diff_stat = String::from_utf8_lossy(&diff_output.stdout);
            if diff_stat.trim().is_empty() {
                // 无变更，还原工作目录
                let _ = Command::new("git")
                    .current_dir(&repo_dir)
                    .args(["checkout", "--", "."])
                    .output();
                return Err(AppError::ValidationError(
                    "上传的内容与当前版本完全相同，无需更新".to_string(),
                ));
            }
        }

        // Git commit（不打 tag，审核通过后才打）
        let commit_msg = format!(
            "v{}: {} by {}",
            version,
            if repo_is_new { "Initial skill upload" } else { "New version upload" },
            author_agent_id
        );
        let commit_hash = self.git_commit_only(&repo_dir, &commit_msg)?;

        // 写入 DB skill（不写 version、不 sync_skill_files_from、不打 tag）
        // tags/description：首次上传用 ZIP 中的值，更新上传继承 DB 当前值
        let (desc, tgs) = if latest_version.is_none() {
            (metadata.description.clone(), metadata.tags.clone())
        } else {
            let current = skill_repo.find_latest_by_name(&metadata.name).await.unwrap_or(None);
            (
                current.as_ref().map(|s| s.description.clone()).unwrap_or_default(),
                current.as_ref().map(|s| s.tags.clone()).unwrap_or_default(),
            )
        };

        let new_skill = NewSkill {
            name: metadata.name.clone(),
            description: desc,
            tags: tgs,
            content: skill_md_content,
            version: version.clone(),
            git_url: None,
            visibility: None,
            tools: None,
            owner_type: owner_type.to_string(),
            owner_id,
            author_identity_id,
        };

        let skill = registry
            .create_skill(new_skill, author_agent_id, search)
            .await?;

        // 清理预览目录
        let _ = fs::remove_dir_all(&preview_dir);

        info!(
            "Skill {} v{} uploaded (commit={}, files={}, is_new={})",
            metadata.name, version, commit_hash, files.len(), repo_is_new
        );

        Ok(UploadResult {
            skill_id: skill.id,
            skill_name: metadata.name.clone(),
            version: version.clone(),
            git_commit: commit_hash,
            git_tag: String::new(),
            git_repo_name: format!("skill-{}", metadata.name),
            is_new_skill: latest_version.is_none(),
            files,
        })
    }

    // ==================== ZIP 解压 & 验证 ====================

    pub fn unpack_and_validate(&self, zip_data: &[u8]) -> Result<UnpackedSkill, AppError> {
        if zip_data.len() > Self::MAX_UPLOAD_SIZE as usize {
            return Err(AppError::ValidationError(format!(
                "ZIP file too large: {} bytes, max {} bytes",
                zip_data.len(),
                Self::MAX_UPLOAD_SIZE
            )));
        }

        let cursor = Cursor::new(zip_data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| AppError::ValidationError(format!("Invalid ZIP file: {}", e)))?;

        // 创建临时解压目录
        let extract_dir = self.temp_dir.join(format!(
            "extract-{}",
            Uuid::new_v4().to_string().split('-').next().unwrap_or("x")
        ));
        fs::create_dir_all(&extract_dir)
            .map_err(|e| AppError::InternalError(format!("Failed to create extract dir: {}", e)))?;

        let mut files: Vec<String> = Vec::new();
        let mut skill_md_content: Option<String> = None;
        let mut total_size: u64 = 0;

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| AppError::ValidationError(format!("ZIP entry read error: {}", e)))?;

            let entry_name = entry.name().to_string();

            // 安全检查：防止 zip slip 攻击
            let safe_path = sanitize_path(&entry_name)?;
            let output_path = extract_dir.join(&safe_path);

            if entry.is_dir() {
                fs::create_dir_all(&output_path)
                    .map_err(|e| AppError::InternalError(format!("Create dir failed: {}", e)))?;
                continue;
            }

            // 确保父目录存在
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    AppError::InternalError(format!("Create parent dir failed: {}", e))
                })?;
            }

            let mut buffer = Vec::new();
            entry
                .read_to_end(&mut buffer)
                .map_err(|e| AppError::InternalError(format!("Read entry failed: {}", e)))?;

            total_size += buffer.len() as u64;
            if total_size > Self::MAX_UPLOAD_SIZE {
                // 清理
                let _ = fs::remove_dir_all(&extract_dir);
                return Err(AppError::ValidationError(
                    "Total unpacked size exceeds 50MB limit".to_string(),
                ));
            }

            fs::write(&output_path, &buffer)
                .map_err(|e| AppError::InternalError(format!("Write file failed: {}", e)))?;

            files.push(safe_path.clone());

            // 检测 SKILL.md
            if safe_path == "SKILL.md" || safe_path.ends_with("/SKILL.md") {
                skill_md_content = Some(String::from_utf8(buffer).map_err(|e| {
                    AppError::ValidationError(format!("SKILL.md is not valid UTF-8: {}", e))
                })?);
            }
        }

        // 清理关闭的临时文件
        drop(archive);

        // 必须包含 SKILL.md
        let skill_md_content = skill_md_content.ok_or_else(|| {
            AppError::ValidationError("ZIP must contain SKILL.md at root".to_string())
        })?;

        // 解析 frontmatter
        let metadata = parse_skill_md_frontmatter(&skill_md_content)?;

        // 验证必填字段
        if metadata.name.is_empty() {
            return Err(AppError::ValidationError(
                "SKILL.md frontmatter: 'name' is required".to_string(),
            ));
        }
        if metadata.description.is_empty() {
            return Err(AppError::ValidationError(
                "SKILL.md frontmatter: 'description' is required".to_string(),
            ));
        }
        // 版本号可选——未提供时由后端自动递增

        Ok(UnpackedSkill {
            extract_dir,
            files,
            skill_md_content,
            metadata,
            total_size_bytes: total_size,
        })
    }

    // ==================== Git 操作（本地仓库） ====================

    /// 准备 Git 仓库：不存在则 git init，已存在则复用
    /// 返回 true 表示是新创建的仓库
    fn prepare_repo(&self, repo_dir: &Path) -> Result<bool, AppError> {
        if repo_dir.join(".git").exists() {
            return Ok(false);
        }

        // 删除可能遗留的空目录（非 git 仓库）
        if repo_dir.exists() {
            fs::remove_dir_all(repo_dir).map_err(|e| {
                AppError::InternalError(format!("Failed to clean stale dir: {}", e))
            })?;
        }

        fs::create_dir_all(repo_dir)
            .map_err(|e| AppError::InternalError(format!("Failed to create repo dir: {}", e)))?;

        let output = Command::new("git")
            .current_dir(repo_dir)
            .arg("init")
            .output()
            .map_err(|e| AppError::InternalError(format!("git init failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::InternalError(format!(
                "git init failed: {}",
                stderr
            )));
        }

        info!("Created git repo: {}", repo_dir.display());
        Ok(true)
    }

    /// 清空工作目录中除 .git 外的所有文件（用于版本更新）
    fn clean_working_dir(&self, repo_dir: &Path) -> Result<(), AppError> {
        for entry in fs::read_dir(repo_dir).map_err(|e| AppError::InternalError(e.to_string()))? {
            let entry = entry.map_err(|e| AppError::InternalError(e.to_string()))?;
            let path = entry.path();
            if path.file_name().map(|n| n != ".git").unwrap_or(false) {
                if path.is_dir() {
                    fs::remove_dir_all(&path).ok();
                } else {
                    fs::remove_file(&path).ok();
                }
            }
        }
        Ok(())
    }

    /// Git add → commit（不打 tag，审核通过后才打 tag）
    fn git_commit_only(
        &self,
        repo_dir: &Path,
        message: &str,
    ) -> Result<String, AppError> {
        // git add -A
        let add = Command::new("git")
            .current_dir(repo_dir)
            .args(["add", "-A"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git add failed: {}", e)))?;

        if !add.status.success() {
            let stderr = String::from_utf8_lossy(&add.stderr);
            warn!("git add warning: {}", stderr);
        }

        // git commit
        let commit = Command::new("git")
            .current_dir(repo_dir)
            .args(["commit", "-m", message, "--allow-empty"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git commit failed: {}", e)))?;

        if !commit.status.success() {
            let stderr = String::from_utf8_lossy(&commit.stderr);
            return Err(AppError::InternalError(format!(
                "git commit failed: {}",
                stderr
            )));
        }

        // git rev-parse HEAD
        let hash_output = Command::new("git")
            .current_dir(repo_dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git rev-parse failed: {}", e)))?;

        let commit_hash = String::from_utf8_lossy(&hash_output.stdout)
            .trim()
            .to_string();

        if commit_hash.is_empty() {
            return Err(AppError::InternalError(
                "Failed to get commit hash".to_string(),
            ));
        }

        Ok(commit_hash)
    }

    /// 审核通过后打 tag
    pub fn git_tag_approved(
        &self,
        repo_dir: &Path,
        tag_name: &str,
        message: &str,
    ) -> Result<(), AppError> {
        let tag_output = Command::new("git")
            .current_dir(repo_dir)
            .args(["tag", "-a", tag_name, "-m", message])
            .output()
            .map_err(|e| AppError::InternalError(format!("git tag failed: {}", e)))?;

        if !tag_output.status.success() {
            let force_tag = Command::new("git")
                .current_dir(repo_dir)
                .args(["tag", "-af", tag_name, "-m", message])
                .output()
                .map_err(|e| AppError::InternalError(format!("git tag force failed: {}", e)))?;
            if !force_tag.status.success() {
                let stderr = String::from_utf8_lossy(&force_tag.stderr);
                return Err(AppError::InternalError(format!(
                    "git tag failed: {}",
                    stderr
                )));
            }
        }

        info!("Tagged {} as {}", repo_dir.display(), tag_name);
        Ok(())
    }

    /// 审核驳回后撤销最后一次 commit
    pub fn git_reset_soft_head(&self, repo_dir: &Path) -> Result<(), AppError> {
        let reset = Command::new("git")
            .current_dir(repo_dir)
            .args(["reset", "--soft", "HEAD~1"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git reset failed: {}", e)))?;

        if !reset.status.success() {
            let stderr = String::from_utf8_lossy(&reset.stderr);
            return Err(AppError::InternalError(format!(
                "git reset failed: {}",
                stderr
            )));
        }

        info!("Reset HEAD~1 in {}", repo_dir.display());
        Ok(())
    }

    /// Git add → commit → tag（在仓库目录中操作）— 保留兼容旧调用
    fn git_commit_and_tag(
        &self,
        repo_dir: &Path,
        message: &str,
        tag_name: &str,
    ) -> Result<String, AppError> {
        // git add -A
        let add = Command::new("git")
            .current_dir(repo_dir)
            .args(["add", "-A"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git add failed: {}", e)))?;

        if !add.status.success() {
            let stderr = String::from_utf8_lossy(&add.stderr);
            warn!("git add warning: {}", stderr);
        }

        // git commit
        let commit = Command::new("git")
            .current_dir(repo_dir)
            .args(["commit", "-m", message, "--allow-empty"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git commit failed: {}", e)))?;

        if !commit.status.success() {
            let stderr = String::from_utf8_lossy(&commit.stderr);
            return Err(AppError::InternalError(format!(
                "git commit failed: {}",
                stderr
            )));
        }

        // git rev-parse HEAD
        let hash_output = Command::new("git")
            .current_dir(repo_dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git rev-parse failed: {}", e)))?;

        let commit_hash = String::from_utf8_lossy(&hash_output.stdout)
            .trim()
            .to_string();

        if commit_hash.is_empty() {
            return Err(AppError::InternalError(
                "Failed to get commit hash".to_string(),
            ));
        }

        // git tag
        let tag_output = Command::new("git")
            .current_dir(repo_dir)
            .args(["tag", "-a", tag_name, "-m", message])
            .output()
            .map_err(|e| AppError::InternalError(format!("git tag failed: {}", e)))?;

        if !tag_output.status.success() {
            // force update tag
            let force_tag = Command::new("git")
                .current_dir(repo_dir)
                .args(["tag", "-fa", tag_name, "-m", message])
                .output()
                .map_err(|e| AppError::InternalError(format!("git tag -f failed: {}", e)))?;

            if !force_tag.status.success() {
                let stderr = String::from_utf8_lossy(&force_tag.stderr);
                return Err(AppError::InternalError(format!(
                    "git tag failed: {}",
                    stderr
                )));
            }
        }

        info!(
            "Git commit {} tagged as {} in {}",
            commit_hash,
            tag_name,
            repo_dir.display()
        );
        Ok(commit_hash)
    }

    // ==================== GitLab 远程操作（可选扩展） ====================

    /// 设置 remote 并推送（后续通过管理后台触发）
    pub fn push_to_remote(&self, skill_name: &str, remote_url: &str) -> Result<(), AppError> {
        let repo_dir = self.repo_path(skill_name);
        if !repo_dir.join(".git").exists() {
            return Err(AppError::SkillNotFound(format!(
                "Local repo for '{}' not found",
                skill_name
            )));
        }

        // 设置 remote origin（如已存在则更新）
        let remote_exists = Command::new("git")
            .current_dir(&repo_dir)
            .args(["remote", "get-url", "origin"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if remote_exists {
            Command::new("git")
                .current_dir(&repo_dir)
                .args(["remote", "set-url", "origin", remote_url])
                .output()
                .map_err(|e| {
                    AppError::InternalError(format!("git remote set-url failed: {}", e))
                })?;
        } else {
            Command::new("git")
                .current_dir(&repo_dir)
                .args(["remote", "add", "origin", remote_url])
                .output()
                .map_err(|e| AppError::InternalError(format!("git remote add failed: {}", e)))?;
        }

        // Push 主分支
        let push_output = Command::new("git")
            .current_dir(&repo_dir)
            .args(["push", "-u", "origin", "main"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git push failed: {}", e)))?;

        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            return Err(AppError::InternalError(format!(
                "git push failed: {}",
                stderr
            )));
        }

        // Push tags
        let tag_output = Command::new("git")
            .current_dir(&repo_dir)
            .args(["push", "--tags", "origin"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git push --tags failed: {}", e)))?;

        if !tag_output.status.success() {
            let stderr = String::from_utf8_lossy(&tag_output.stderr);
            warn!("git push --tags: {}", stderr);
        }

        info!("Pushed repo {} to {}", skill_name, remote_url);
        Ok(())
    }

    /// 从 GitLab 克隆 skill 仓库到本地
    pub fn clone_from_gitlab(&self, skill_name: &str) -> Result<PathBuf, AppError> {
        let repo_dir = self.repo_path(skill_name);

        if repo_dir.exists() {
            return Err(AppError::SkillAlreadyExists(format!(
                "Local repo for '{}' already exists",
                skill_name
            )));
        }

        let repo_name = format!("skill-{}", skill_name);
        let remote_url = self.remote_config.remote_url(&repo_name);

        let output = Command::new("git")
            .args(["clone", &remote_url, &repo_dir.to_string_lossy()])
            .output()
            .map_err(|e| AppError::InternalError(format!("git clone failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::InternalError(format!(
                "git clone failed: {}",
                stderr
            )));
        }

        info!("Cloned {} from GitLab", repo_name);
        Ok(repo_dir)
    }

    /// 从 GitLab 拉取最新更新
    pub fn fetch_from_gitlab(&self, skill_name: &str) -> Result<(), AppError> {
        let repo_dir = self.repo_path(skill_name);

        if !repo_dir.join(".git").exists() {
            return Err(AppError::SkillNotFound(format!(
                "Local repo for '{}' not found. Clone it first.",
                skill_name
            )));
        }

        let output = Command::new("git")
            .current_dir(&repo_dir)
            .args(["fetch", "origin", "--tags", "--prune"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git fetch failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::InternalError(format!(
                "git fetch failed: {}",
                stderr
            )));
        }

        info!("Fetched latest for {} from GitLab", skill_name);
        Ok(())
    }

    // ==================== 版本查询 ====================

    /// 列出所有 Git tags（版本号）
    pub fn list_git_tags(&self, skill_name: &str) -> Result<Vec<String>, AppError> {
        let repo_dir = self.repo_path(skill_name);
        if !repo_dir.join(".git").exists() {
            return Ok(vec![]);
        }

        let output = Command::new("git")
            .current_dir(&repo_dir)
            .args(["tag", "-l", "--sort=-creatordate"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git tag list failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    /// 获取版本间的 diff（SKILL.md）
    pub fn get_version_diff(
        &self,
        skill_name: &str,
        from_version: &str,
        to_version: &str,
    ) -> Result<String, AppError> {
        let repo_dir = self.repo_path(skill_name);
        if !repo_dir.join(".git").exists() {
            return Err(AppError::SkillNotFound(skill_name.to_string()));
        }

        let from_tag = if from_version.starts_with('v') {
            from_version.to_string()
        } else {
            format!("v{}", from_version)
        };
        let to_tag = if to_version.starts_with('v') {
            to_version.to_string()
        } else {
            format!("v{}", to_version)
        };

        let output = Command::new("git")
            .current_dir(&repo_dir)
            .args(["diff", &from_tag, &to_tag, "--", "SKILL.md"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git diff failed: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// 获取特定版本的文件内容
    /// 优先使用 git tag，若 tag 不存在则使用 HEAD
    pub fn get_file_at_version(
        &self,
        skill_name: &str,
        version: &str,
        file_path: &str,
    ) -> Result<String, AppError> {
        let repo_dir = self.repo_path(skill_name);
        if !repo_dir.join(".git").exists() {
            return Err(AppError::SkillNotFound(skill_name.to_string()));
        }

        let tag = if version.starts_with('v') {
            version.to_string()
        } else {
            format!("v{}", version)
        };

        // 检查 tag 是否存在，不存在则用 HEAD
        let ref_name = if Command::new("git")
            .current_dir(&repo_dir)
            .args(["rev-parse", &tag])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            tag
        } else {
            "HEAD".to_string()
        };
        let ref_spec = format!("{}:{}", ref_name, file_path);

        let output = Command::new("git")
            .current_dir(&repo_dir)
            .args(["show", &ref_spec])
            .output()
            .map_err(|e| AppError::InternalError(format!("git show failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::SkillNotFound(format!(
                "File '{}' not found at version {}: {}",
                file_path, version, stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// 列出特定版本的所有文件
    /// 优先使用 git tag，若 tag 不存在则使用 HEAD（pending_review 未打 tag 时）
    pub fn list_files_at_version(
        &self,
        skill_name: &str,
        version: &str,
    ) -> Result<Vec<String>, AppError> {
        let repo_dir = self.repo_path(skill_name);
        if !repo_dir.join(".git").exists() {
            return Ok(vec![]);
        }

        let tag = if version.starts_with('v') {
            version.to_string()
        } else {
            format!("v{}", version)
        };

        // 检查 tag 是否存在，不存在则用 HEAD
        let ref_name = if Command::new("git")
            .current_dir(&repo_dir)
            .args(["rev-parse", &tag])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            tag
        } else {
            "HEAD".to_string()
        };

        let output = Command::new("git")
            .current_dir(&repo_dir)
            .args(["ls-tree", "-r", "--name-only", "-z", &ref_name])
            .output()
            .map_err(|e| AppError::InternalError(format!("git ls-tree failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::InternalError(format!(
                "Failed to list files at {}: {}",
                ref_name, stderr
            )));
        }

        // -z 输出以 null 分隔，不转义中文等非 ASCII 路径
        Ok(output
            .stdout
            .split(|&b| b == 0)
            .map(|chunk| String::from_utf8_lossy(chunk).to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }
}

// ==================== 工具函数 ====================

/// 防止 Zip Slip 攻击：规范化路径，拒绝 `..` 和绝对路径
fn sanitize_path(entry_name: &str) -> Result<String, AppError> {
    let path = Path::new(entry_name);

    // 拒绝绝对路径
    if path.is_absolute() {
        return Err(AppError::ValidationError(format!(
            "Invalid ZIP entry (absolute path): {}",
            entry_name
        )));
    }

    // 检查是否包含 ..
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(AppError::ValidationError(format!(
                    "Invalid ZIP entry (path traversal): {}",
                    entry_name
                )));
            }
            _ => {}
        }
    }

    Ok(entry_name.to_string())
}

/// 递归查找预览目录中的 SKILL.md
///
/// ZIP 包可能包含根目录（如 `my-skill/SKILL.md`），
/// 本函数递归搜索直到找到 SKILL.md 为止。
fn find_skill_md_recursive(dir: &Path) -> Result<PathBuf, AppError> {
    // 先尝试直接在当前目录找
    let direct = dir.join("SKILL.md");
    if direct.exists() {
        return Ok(direct);
    }

    // 递归进入子目录查找（取第一个匹配）
    for entry in
        fs::read_dir(dir).map_err(|e| AppError::InternalError(format!("Read dir failed: {}", e)))?
    {
        let entry =
            entry.map_err(|e| AppError::InternalError(format!("Dir entry error: {}", e)))?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(found) = find_skill_md_recursive(&path) {
                return Ok(found);
            }
        }
    }

    Err(AppError::ValidationError(
        "SKILL.md not found in preview".to_string(),
    ))
}

pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;

    // 收集有效条目（排除 .git）
    let entries: Vec<_> = fs::read_dir(src)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() != ".git")
        .collect();

    // 如果源目录只有一个子目录，展开它
    // 处理 ZIP 自带顶层目录的情况（如 my-skill/SKILL.md 解压后 extract_dir/ 只含 my-skill/）
    if entries.len() == 1 && entries[0].file_type().map(|t| t.is_dir()).unwrap_or(false) {
        return copy_dir_recursive(&entries[0].path(), dst);
    }

    for entry in entries {
        let file_type = entry.file_type()?;
        let file_name = entry.file_name();
        let src_path = entry.path();
        let dst_path = dst.join(&file_name);

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

// ==================== 版本回退 ====================

/// 回退结果
#[derive(Debug, Serialize)]
pub struct RollbackResult {
    pub skill_name: String,
    pub from_version: String,
    pub target_version: String,
    pub new_version: String,
    pub git_commit: String,
    pub git_tag: String,
    pub file_count: i32,
    pub total_size_bytes: i64,
}

impl SkillGitService {
    /// 回退到指定版本：从 Git tag 恢复文件，创建新的 patch 版本
    ///
    /// 步骤：
    /// 1. 验证目标版本的 Git tag 存在
    /// 2. 检出目标版本的文件到仓库工作目录
    /// 3. 解析 SKILL.md 获取元数据
    /// 4. 计算新版本号（当前最新版本 patch+1）
    /// 5. Git commit + tag 新版本
    /// 6. 记录 skill_versions 表
    pub fn rollback_version(
        &self,
        skill_name: &str,
        target_version: &str,
        admin_identity_id: Uuid,
        version_repo: &VersionRepository,
    ) -> Result<RollbackResult, AppError> {
        let repo_dir = self.repo_path(skill_name);
        if !repo_dir.join(".git").exists() {
            return Err(AppError::SkillNotFound(format!(
                "Git repo for skill {} not found",
                skill_name
            )));
        }

        let target_tag = if target_version.starts_with('v') {
            target_version.to_string()
        } else {
            format!("v{}", target_version)
        };

        // 1. 验证目标 tag 存在
        let tag_check = Command::new("git")
            .current_dir(&repo_dir)
            .args(["rev-parse", &target_tag])
            .output()
            .map_err(|e| AppError::InternalError(format!("git rev-parse failed: {}", e)))?;

        if !tag_check.status.success() {
            return Err(AppError::SkillNotFound(format!(
                "Version tag {} not found for skill {}",
                target_tag, skill_name
            )));
        }

        // 2. 获取最新版本号，计算新版本号
        let latest = tokio::runtime::Handle::current()
            .block_on(async { version_repo.get_latest_version(skill_name).await })
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let current_version = latest.unwrap_or_else(|| "1.0.0".to_string());
        let new_version = {
            let parsed = semver::Version::parse(&current_version).map_err(|e| {
                AppError::ValidationError(format!("Invalid version {}: {}", current_version, e))
            })?;
            format!("{}.{}.{}", parsed.major, parsed.minor, parsed.patch + 1)
        };

        // 3. 清空工作目录，从目标 tag 检出文件
        self.clean_working_dir(&repo_dir)?;

        // git checkout target_tag -- .
        // 先用 git checkout {tag} -- . 检出所有文件
        let checkout = Command::new("git")
            .current_dir(&repo_dir)
            .args(["checkout", &target_tag, "--", "."])
            .output()
            .map_err(|e| AppError::InternalError(format!("git checkout failed: {}", e)))?;

        if !checkout.status.success() {
            let stderr = String::from_utf8_lossy(&checkout.stderr);
            return Err(AppError::InternalError(format!(
                "Failed to checkout version {}: {}",
                target_tag, stderr
            )));
        }

        // 4. 读取并解析 SKILL.md
        let skill_md_path = repo_dir.join("SKILL.md");
        let skill_md_content = fs::read_to_string(&skill_md_path).map_err(|e| {
            AppError::InternalError(format!(
                "SKILL.md not found at version {}: {}",
                target_tag, e
            ))
        })?;
        let _meta = parse_skill_md_frontmatter(&skill_md_content)?;

        // 5. 获取文件列表和总大小
        let mut file_paths: Vec<String> = Vec::new();
        collect_files(&repo_dir, &repo_dir, &mut file_paths)
            .map_err(|e| AppError::InternalError(format!("Failed to collect files: {}", e)))?;
        let file_count = file_paths.len() as i32;
        let total_size: u64 = file_paths
            .iter()
            .filter_map(|p| fs::metadata(repo_dir.join(p)).ok())
            .map(|m| m.len())
            .sum();

        // 6. Git commit + tag
        let new_tag = format!("v{}", new_version);
        let commit_msg = format!(
            "v{}: Rollback from v{} to v{} by admin {}",
            new_version, current_version, target_version, admin_identity_id
        );
        let commit_hash = self.git_commit_and_tag(&repo_dir, &commit_msg, &new_tag)?;

        // 7. 写入 skill_versions 表
        tokio::runtime::Handle::current()
            .block_on(async {
                version_repo
                    .create(NewSkillVersion {
                        skill_name: skill_name.to_string(),
                        version: new_version.clone(),
                        git_commit_hash: Some(commit_hash.clone()),
                        git_tag: Some(new_tag.clone()),
                        changelog: Some(commit_msg.clone()),
                        file_count,
                        total_size_bytes: total_size as i64,
                        uploaded_by: Some(admin_identity_id),
                        git_remote_url: None,
                    })
                    .await
            })
            .map_err(|e| {
                AppError::InternalError(format!("Failed to record rollback version: {}", e))
            })?;

        info!(
            "Rollback: skill={} {} -> {} (new: {}, commit={})",
            skill_name, current_version, target_version, new_version, commit_hash
        );

        Ok(RollbackResult {
            skill_name: skill_name.to_string(),
            from_version: current_version,
            target_version: target_version.to_string(),
            new_version,
            git_commit: commit_hash,
            git_tag: new_tag,
            file_count,
            total_size_bytes: total_size as i64,
        })
    }

    /// 回退到指定版本（仅 commit，不打 tag — 审核通过后再打 tag）
    ///
    /// 与 `rollback_version` 的区别：
    /// - 只做 git commit，不打 git tag
    /// - skill_versions 记录中 git_tag 为 None
    /// - 审核通过后由 approve_org_skill_handler 打 tag + 生成 tarball
    pub fn rollback_version_commit_only(
        &self,
        skill_name: &str,
        target_version: &str,
        author_identity_id: Uuid,
        version_repo: &VersionRepository,
    ) -> Result<RollbackResult, AppError> {
        let repo_dir = self.repo_path(skill_name);
        if !repo_dir.join(".git").exists() {
            return Err(AppError::SkillNotFound(format!(
                "Git repo for skill {} not found",
                skill_name
            )));
        }

        let target_tag = if target_version.starts_with('v') {
            target_version.to_string()
        } else {
            format!("v{}", target_version)
        };

        // 1. 验证目标 tag 存在
        let tag_check = Command::new("git")
            .current_dir(&repo_dir)
            .args(["rev-parse", &target_tag])
            .output()
            .map_err(|e| AppError::InternalError(format!("git rev-parse failed: {}", e)))?;

        if !tag_check.status.success() {
            return Err(AppError::SkillNotFound(format!(
                "Version tag {} not found for skill {}",
                target_tag, skill_name
            )));
        }

        // 2. 获取最新版本号，计算新版本号
        let latest = tokio::runtime::Handle::current()
            .block_on(async { version_repo.get_latest_version(skill_name).await })
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let current_version = latest.unwrap_or_else(|| "1.0.0".to_string());
        let new_version = {
            let parsed = semver::Version::parse(&current_version).map_err(|e| {
                AppError::ValidationError(format!("Invalid version {}: {}", current_version, e))
            })?;
            format!("{}.{}.{}", parsed.major, parsed.minor, parsed.patch + 1)
        };

        // 3. 清空工作目录，从目标 tag 检出文件
        self.clean_working_dir(&repo_dir)?;

        let checkout = Command::new("git")
            .current_dir(&repo_dir)
            .args(["checkout", &target_tag, "--", "."])
            .output()
            .map_err(|e| AppError::InternalError(format!("git checkout failed: {}", e)))?;

        if !checkout.status.success() {
            let stderr = String::from_utf8_lossy(&checkout.stderr);
            return Err(AppError::InternalError(format!(
                "Failed to checkout version {}: {}",
                target_tag, stderr
            )));
        }

        // 4. 读取并解析 SKILL.md
        let skill_md_path = repo_dir.join("SKILL.md");
        let _skill_md_content = fs::read_to_string(&skill_md_path).map_err(|e| {
            AppError::InternalError(format!(
                "SKILL.md not found at version {}: {}",
                target_tag, e
            ))
        })?;

        // 5. 获取文件列表和总大小
        let mut file_paths: Vec<String> = Vec::new();
        collect_files(&repo_dir, &repo_dir, &mut file_paths)
            .map_err(|e| AppError::InternalError(format!("Failed to collect files: {}", e)))?;
        let file_count = file_paths.len() as i32;
        let total_size: u64 = file_paths
            .iter()
            .filter_map(|p| fs::metadata(repo_dir.join(p)).ok())
            .map(|m| m.len())
            .sum();

        // 6. Git commit only（不打 tag，审核通过后再打）
        let commit_msg = format!(
            "v{}: Rollback from v{} to v{} by {}",
            new_version, current_version, target_version, author_identity_id
        );
        let commit_hash = self.git_commit_only(&repo_dir, &commit_msg)?;

        // 7. 写入 skill_versions 表（git_tag 为 None，审核通过后再补充）
        tokio::runtime::Handle::current()
            .block_on(async {
                version_repo
                    .create(NewSkillVersion {
                        skill_name: skill_name.to_string(),
                        version: new_version.clone(),
                        git_commit_hash: Some(commit_hash.clone()),
                        git_tag: None, // 审核通过后补充
                        changelog: Some(commit_msg.clone()),
                        file_count,
                        total_size_bytes: total_size as i64,
                        uploaded_by: Some(author_identity_id),
                        git_remote_url: None,
                    })
                    .await
            })
            .map_err(|e| {
                AppError::InternalError(format!("Failed to record rollback version: {}", e))
            })?;

        info!(
            "Rollback (commit-only): skill={} {} -> {} (new: {}, commit={})",
            skill_name, current_version, target_version, new_version, commit_hash
        );

        Ok(RollbackResult {
            skill_name: skill_name.to_string(),
            from_version: current_version,
            target_version: target_version.to_string(),
            new_version,
            git_commit: commit_hash,
            git_tag: String::new(), // 审核通过后才有 tag
            file_count,
            total_size_bytes: total_size as i64,
        })
    }
}

/// 递归收集目录下所有文件的相对路径
fn collect_files(base: &Path, current: &Path, out: &mut Vec<String>) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(base)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            out.push(rel.to_string_lossy().to_string().replace('\\', "/"));
        }
    }
    Ok(())
}

/// 解析用户指定的版本号，若未提供则在后端自动递增
///
/// 规则：
/// - 用户显式指定版本 → 直接使用
/// - 首次上传（无历史版本） → 默认 `1.0.0`
/// - 同一 skill 已有历史版本 → patch +1（如 `1.0.3` → `1.0.4`）
fn resolve_version(
    skill_name: &str,
    latest_version: &Option<String>,
    user_version: &Option<String>,
) -> Result<String, AppError> {
    if let Some(v) = user_version {
        if !v.is_empty() {
            info!(
                "Using user-specified version {} for skill {}",
                v, skill_name
            );
            return Ok(v.clone());
        }
    }

    if let Some(latest) = latest_version {
        let parsed = semver::Version::parse(latest).map_err(|e| {
            AppError::ValidationError(format!(
                "Latest version '{}' for skill {} is not valid semver: {}",
                latest, skill_name, e
            ))
        })?;
        let next = format!("{}.{}.{}", parsed.major, parsed.minor, parsed.patch + 1);
        info!(
            "Auto-incremented version for skill {}: {} → {}",
            skill_name, latest, next
        );
        Ok(next)
    } else {
        info!("First upload for skill {}, defaulting to 1.0.0", skill_name);
        Ok("1.0.0".to_string())
    }
}

/// 简易 YAML frontmatter 解析
/// 从 SKILL.md 中提取 `---` 包裹的元数据
pub fn parse_skill_md_frontmatter(content: &str) -> Result<ParsedSkillMetadata, AppError> {
    let mut metadata = ParsedSkillMetadata {
        name: String::new(),
        description: String::new(),
        tags: Vec::new(),
        version: None,
        dependencies: Vec::new(),
        compatibility: ">=1.0.0".to_string(),
    };

    // 尝试解析 frontmatter
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        // 没有 frontmatter，可能整个文件就是内容
        return Ok(metadata);
    }

    let rest = &trimmed[3..];
    let (fm_body, _) = match rest.find("---") {
        Some(end_idx) => (rest[..end_idx].trim(), rest[end_idx + 3..].trim()),
        None => {
            return Ok(metadata);
        }
    };

    let mut current_key: Option<String> = None;
    let mut list_buffer: Vec<String> = Vec::new();
    let mut multiline_buffer: Vec<String> = Vec::new(); // 多行标量文本缓冲
    let mut is_multiline_scalar = false; // 是否为 YAML | 或 > 多行文本

    for line in fm_body.lines() {
        let raw_line = line;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            // 多行文本中的空行也保留（| 模式保留换行，> 模式合并）
            if is_multiline_scalar {
                multiline_buffer.push(String::new());
            }
            continue;
        }

        if let Some(colon_pos) = line.find(':') {
            // 遇到新 key 时保存之前的多行文本或列表
            if let Some(ref key) = current_key {
                if is_multiline_scalar && !multiline_buffer.is_empty() {
                    let merged = multiline_buffer.join(" ");
                    apply_scalar_value(&mut metadata, key, &merged);
                    multiline_buffer.clear();
                } else if !list_buffer.is_empty() {
                    apply_list_value(&mut metadata, key, &list_buffer);
                    list_buffer.clear();
                }
            }

            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();

            if value.is_empty() || value == "[]" {
                // 空值：可能是多行列表或多行文本的开始
                current_key = Some(key.clone());
                is_multiline_scalar = false;
                if value == "[]" {
                    list_buffer = Vec::new();
                } else {
                    list_buffer = Vec::new();
                    multiline_buffer = Vec::new();
                }
                continue;
            }

            // YAML 块标量: | 或 > 或 |-
            if value == "|" || value == "|-" || value == ">-" || value == ">" || value == "|+" || value == ">+" {
                current_key = Some(key.clone());
                is_multiline_scalar = true;
                multiline_buffer = Vec::new();
                list_buffer.clear();
                continue;
            }

            // 内联列表: tags: [a, b, c]
            if value.starts_with('[') && value.ends_with(']') {
                let items: Vec<String> = value[1..value.len() - 1]
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                apply_list_value(&mut metadata, &key, &items);
                current_key = None;
                list_buffer.clear();
                is_multiline_scalar = false;
            } else {
                apply_scalar_value(&mut metadata, &key, &value);
                current_key = None;
                list_buffer.clear();
            }
        } else if is_multiline_scalar {
            // 多行文本内容行
            multiline_buffer.push(raw_line.trim().to_string());
        } else if let Some(ref _key) = current_key {
            // 列表项：- item
            let item = line.trim_start_matches('-').trim();
            if !item.is_empty() {
                let cleaned = item.trim_matches('"').trim_matches('\'').to_string();
                list_buffer.push(cleaned);
            }
        }
    }

    // 处理结尾的多行文本或列表
    if let Some(ref key) = current_key {
        if is_multiline_scalar && !multiline_buffer.is_empty() {
            let merged = multiline_buffer.join(" ");
            apply_scalar_value(&mut metadata, key, &merged);
        } else if !list_buffer.is_empty() {
            apply_list_value(&mut metadata, key, &list_buffer);
        }
    }

    Ok(metadata)
}

fn apply_scalar_value(meta: &mut ParsedSkillMetadata, key: &str, value: &str) {
    // 去掉外层引号（支持 "..." 和 '...'）
    let cleaned = value
        .trim()
        .trim_start_matches('"').trim_end_matches('"')
        .trim_start_matches('\'').trim_end_matches('\'');
    match key {
        "name" => meta.name = cleaned.to_string(),
        "description" => meta.description = normalize_description(cleaned),
        "version" => meta.version = Some(cleaned.to_string()),
        "compatibility" => meta.compatibility = cleaned.to_string(),
        _ => {}
    }
}

fn apply_list_value(meta: &mut ParsedSkillMetadata, key: &str, items: &[String]) {
    match key {
        "tags" => meta.tags = items.to_vec(),
        "dependencies" => meta.dependencies = items.to_vec(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_basic() {
        let md = r#"---
name: my-skill
description: A test skill
version: 1.0.0
tags: [web, http]
---
# Content here
"#;
        let meta = parse_skill_md_frontmatter(md).unwrap();
        assert_eq!(meta.name, "my-skill");
        assert_eq!(meta.description, "A test skill");
        assert_eq!(meta.version, Some("1.0.0".to_string()));
        assert_eq!(meta.tags, vec!["web", "http"]);
    }

    #[test]
    fn test_parse_frontmatter_multiline_list() {
        let md = r#"---
name: browse
description: Web browsing skill
tags:
  - web
  - browser
  - http
version: 2.0.0
---
# Content
"#;
        let meta = parse_skill_md_frontmatter(md).unwrap();
        assert_eq!(meta.name, "browse");
        assert_eq!(meta.tags, vec!["web", "browser", "http"]);
        assert_eq!(meta.version, Some("2.0.0".to_string()));
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let md = "# Just a heading\n\nSome content";
        let meta = parse_skill_md_frontmatter(md).unwrap();
        assert!(meta.name.is_empty());
        assert!(meta.version.is_none());
    }

    #[test]
    fn test_sanitize_path_normal() {
        let result = sanitize_path("SKILL.md").unwrap();
        assert_eq!(result, "SKILL.md");
    }

    #[test]
    fn test_sanitize_path_traversal_blocked() {
        let result = sanitize_path("../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_path_absolute_blocked() {
        let result = sanitize_path("/etc/passwd");
        assert!(result.is_err());
    }
}
