//! Skill Git Service — ZIP 上传自动解压 + Git 仓库版本管理 + GitLab 远程同步

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;
use zip::ZipArchive;

use crate::db::repositories::skill::SkillRepository;
use crate::db::repositories::version::{NewSkillVersion, VersionRepository};
use crate::models::error::AppError;
use crate::models::skill::NewSkill;
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
    pub version: String,
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

/// Skill Git 版本管理服务
#[derive(Debug, Clone)]
pub struct SkillGitService {
    /// Git 裸仓库存储根目录: {data_dir}/git-repos/
    pub repos_dir: PathBuf,
    /// Skill 文件存储目录: {data_dir}/skills/
    pub skills_dir: PathBuf,
    /// 临时目录
    temp_dir: PathBuf,
    /// GitLab 远程配置
    pub remote_config: GitRemoteConfig,
}

impl SkillGitService {
    /// 允许上传的最大 ZIP 大小: 50 MB
    pub const MAX_UPLOAD_SIZE: u64 = 50 * 1024 * 1024;

    pub fn new(data_dir: PathBuf, skills_dir: PathBuf) -> Self {
        let repos_dir = data_dir.join("git-repos");
        let temp_dir = data_dir.join("tmp");
        Self {
            repos_dir,
            skills_dir,
            temp_dir,
            remote_config: GitRemoteConfig::from_env(),
        }
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

    // ==================== ZIP 上传处理 ====================

    /// 完整的上传流程：ZIP → 解压验证 → Git 提交 → DB 记录
    pub fn process_upload(
        &self,
        zip_data: &[u8],
        author_agent_id: &str,
        author_identity_id: Option<Uuid>,
        owner_type: &str,
        owner_id: Option<Uuid>,
        registry: &RegistryService,
        search: &SearchService,
        skill_repo: &SkillRepository,
        version_repo: &VersionRepository,
    ) -> Result<UploadResult, AppError> {
        // 1. 解压 & 验证
        let unpacked = self.unpack_and_validate(zip_data)?;

        let metadata = &unpacked.metadata;

        // 2. 检查是否是已有 skill 的新版本
        let existing_skill = tokio::runtime::Handle::current()
            .block_on(async {
                skill_repo
                    .find_by_id(&format!("skill-{}-{}", metadata.name, metadata.version))
                    .await
            })
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let is_new_skill = existing_skill.is_none();

        // 如果已存在，检查版本号不重复
        if let Some(_existing) = &existing_skill {
            // 版本已存在则不允许覆盖
            return Err(AppError::SkillAlreadyExists(format!(
                "Skill {} version {} already exists. Upload with a new version.",
                metadata.name, metadata.version
            )));
        }

        // 如果版本不存在但 skill name 已存在（不同版本），允许
        let latest_version = tokio::runtime::Handle::current()
            .block_on(async { version_repo.get_latest_version(&metadata.name).await })
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        // 3. 确保 Git 仓库存在（首次创建）
        let repo_name = format!("skill-{}", metadata.name);
        let repo_path = self.repos_dir.join(format!("{}.git", repo_name));

        if !repo_path.exists() {
            self.init_bare_repo(&repo_path)?;
            info!("Created bare git repo: {}", repo_path.display());
        }

        // 4. 创建 worktree (临时检出目录)
        let worktree_dir = self.temp_dir.join(format!(
            "checkout-{}-{}",
            metadata.name,
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("x")
        ));

        self.create_worktree(&repo_path, &worktree_dir)?;

        // 5. 复制解压文件到 worktree
        self.copy_extracted_to_worktree(&unpacked.extract_dir, &worktree_dir)?;

        // 6. Git add + commit + tag
        let commit_msg = format!(
            "v{}: {} by {}",
            metadata.version,
            if is_new_skill {
                "Initial skill upload"
            } else {
                "New version upload"
            },
            author_agent_id
        );
        let tag_name = format!("v{}", metadata.version);
        let commit_hash = self.git_commit_and_tag(&worktree_dir, &commit_msg, &tag_name)?;

        // 6b. Push 到 GitLab（如启用）
        let git_remote_url = if self.remote_config.push_enabled {
            let url = self.remote_config.remote_url(&repo_name);
            match self.push_to_remote(&repo_path, &url) {
                Ok(()) => {
                    info!("Pushed to GitLab: {}", url);
                    Some(url)
                }
                Err(e) => {
                    warn!("GitLab push failed (non-fatal): {}", e);
                    None
                }
            }
        } else {
            None
        };

        // 7. 写入 DB — skill_versions 表
        let file_count = unpacked.files.len() as i32;
        let total_size = unpacked.total_size_bytes as i64;
        let _db_version = tokio::runtime::Handle::current()
            .block_on(async {
                version_repo
                    .create(NewSkillVersion {
                        skill_name: metadata.name.clone(),
                        version: metadata.version.clone(),
                        git_commit_hash: Some(commit_hash.clone()),
                        git_tag: Some(tag_name.clone()),
                        changelog: Some(commit_msg.clone()),
                        file_count,
                        total_size_bytes: total_size,
                        uploaded_by: author_identity_id,
                        git_remote_url: git_remote_url.clone(),
                    })
                    .await
            })
            .map_err(|e| AppError::InternalError(format!("Failed to record version: {}", e)))?;

        // 8. 写入 skill 到 registry（文件系统 + DB + 搜索索引）
        let new_skill = NewSkill {
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            tags: metadata.tags.clone(),
            content: unpacked.skill_md_content.clone(),
            version: metadata.version.clone(),
            git_url: None,
            visibility: None,
            tools: None,
            owner_type: owner_type.to_string(),
            owner_id,
        };

        let skill = tokio::runtime::Handle::current().block_on(async {
            registry
                .create_skill(new_skill, author_agent_id, search)
                .await
        })?;

        // 9. 清理 worktree
        let _ = self.cleanup_worktree(&repo_path, &worktree_dir);

        // 10. 清理临时解压目录
        let _ = fs::remove_dir_all(&unpacked.extract_dir);

        Ok(UploadResult {
            skill_id: skill.id,
            skill_name: metadata.name.clone(),
            version: metadata.version.clone(),
            git_commit: commit_hash,
            git_tag: tag_name,
            git_repo_name: repo_name,
            is_new_skill: latest_version.is_none(),
            files: unpacked.files,
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
        if metadata.version.is_empty() {
            return Err(AppError::ValidationError(
                "SKILL.md frontmatter: 'version' is required".to_string(),
            ));
        }

        Ok(UnpackedSkill {
            extract_dir,
            files,
            skill_md_content,
            metadata,
            total_size_bytes: total_size,
        })
    }

    // ==================== Git 操作 ====================

    /// 初始化 bare 仓库
    fn init_bare_repo(&self, repo_path: &Path) -> Result<(), AppError> {
        let output = Command::new("git")
            .arg("init")
            .arg("--bare")
            .arg(repo_path)
            .output()
            .map_err(|e| AppError::InternalError(format!("git init failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::InternalError(format!(
                "git init failed: {}",
                stderr
            )));
        }
        Ok(())
    }

    /// 创建 worktree
    fn create_worktree(&self, repo_path: &Path, worktree_dir: &Path) -> Result<(), AppError> {
        // 清理可能残留的 worktree
        let _ = fs::remove_dir_all(worktree_dir);

        let output = Command::new("git")
            .args(["--git-dir", &repo_path.to_string_lossy()])
            .args(["worktree", "add", &worktree_dir.to_string_lossy(), "HEAD"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git worktree add failed: {}", e)))?;

        // worktree add 初次可能是失败（如果没 HEAD），用 --orphan
        if !output.status.success() {
            let _ = fs::remove_dir_all(worktree_dir);
            let output = Command::new("git")
                .args(["--git-dir", &repo_path.to_string_lossy()])
                .args([
                    "worktree",
                    "add",
                    "--orphan",
                    "main",
                    &worktree_dir.to_string_lossy(),
                ])
                .output()
                .map_err(|e| {
                    AppError::InternalError(format!("git worktree add --orphan failed: {}", e))
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(AppError::InternalError(format!(
                    "git worktree add failed: {}",
                    stderr
                )));
            }
        }

        Ok(())
    }

    /// 复制解压文件到 worktree
    fn copy_extracted_to_worktree(
        &self,
        extract_dir: &Path,
        worktree_dir: &Path,
    ) -> Result<(), AppError> {
        // 递归复制
        copy_dir_recursive(extract_dir, worktree_dir)
            .map_err(|e| AppError::InternalError(format!("Copy to worktree failed: {}", e)))?;
        Ok(())
    }

    /// Git add → commit → tag
    fn git_commit_and_tag(
        &self,
        worktree_dir: &Path,
        message: &str,
        tag_name: &str,
    ) -> Result<String, AppError> {
        // git add -A
        let add = Command::new("git")
            .current_dir(worktree_dir)
            .args(["add", "-A"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git add failed: {}", e)))?;

        if !add.status.success() {
            let stderr = String::from_utf8_lossy(&add.stderr);
            warn!("git add warning: {}", stderr);
        }

        // git commit
        let commit = Command::new("git")
            .current_dir(worktree_dir)
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

        // 获取 commit hash
        let hash_output = Command::new("git")
            .current_dir(worktree_dir)
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
            .current_dir(worktree_dir)
            .args(["tag", "-a", tag_name, "-m", message])
            .output()
            .map_err(|e| AppError::InternalError(format!("git tag failed: {}", e)))?;

        if !tag_output.status.success() {
            // 尝试 force update tag（可能标签已存在）
            let force_tag = Command::new("git")
                .current_dir(worktree_dir)
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
            "Git commit {} tagged as {} for {}",
            commit_hash,
            tag_name,
            worktree_dir.display()
        );
        Ok(commit_hash)
    }

    /// 清理 worktree
    fn cleanup_worktree(&self, repo_path: &Path, worktree_dir: &Path) -> Result<(), AppError> {
        let output = Command::new("git")
            .args(["--git-dir", &repo_path.to_string_lossy()])
            .args([
                "worktree",
                "remove",
                "--force",
                &worktree_dir.to_string_lossy(),
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                debug!("Worktree cleanup: {}", worktree_dir.display());
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                warn!("Worktree cleanup warning: {}", stderr);
                // 强制删除目录
                let _ = fs::remove_dir_all(worktree_dir);
            }
            Err(_) => {
                let _ = fs::remove_dir_all(worktree_dir);
            }
        }
        Ok(())
    }

    // ==================== GitLab 远程操作 ====================

    /// 设置 remote origin 并推送所有 tags 到 GitLab
    pub fn push_to_remote(&self, repo_path: &Path, remote_url: &str) -> Result<(), AppError> {
        if !self.remote_config.push_enabled {
            return Ok(());
        }

        let repo_str = repo_path.to_string_lossy();

        // 设置 remote（如果已存在则更新 URL）
        let remote_exists = Command::new("git")
            .args(["--git-dir", &repo_str, "remote", "get-url", "origin"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if remote_exists {
            Command::new("git")
                .args([
                    "--git-dir",
                    &repo_str,
                    "remote",
                    "set-url",
                    "origin",
                    remote_url,
                ])
                .output()
                .map_err(|e| {
                    AppError::InternalError(format!("git remote set-url failed: {}", e))
                })?;
        } else {
            Command::new("git")
                .args([
                    "--git-dir",
                    &repo_str,
                    "remote",
                    "add",
                    "origin",
                    remote_url,
                ])
                .output()
                .map_err(|e| AppError::InternalError(format!("git remote add failed: {}", e)))?;
        }

        // Push 主分支
        let push_output = Command::new("git")
            .args(["--git-dir", &repo_str, "push", "-u", "origin", "main"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git push failed: {}", e)))?;

        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            warn!("git push stderr: {}", stderr);
            return Err(AppError::InternalError(format!(
                "git push failed: {}",
                stderr
            )));
        }

        // Push all tags
        let tag_output = Command::new("git")
            .args(["--git-dir", &repo_str, "push", "--tags", "origin"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git push --tags failed: {}", e)))?;

        if !tag_output.status.success() {
            let stderr = String::from_utf8_lossy(&tag_output.stderr);
            warn!("git push --tags: {}", stderr);
            // tags 推送失败不阻断主流程
        }

        info!("Successfully pushed repo to {}", remote_url);
        Ok(())
    }

    /// 从 GitLab 克隆 skill 仓库到本地（Admin 手动触发或首次部署）
    pub fn clone_from_gitlab(&self, skill_name: &str) -> Result<PathBuf, AppError> {
        let repo_name = format!("skill-{}", skill_name);
        let repo_path = self.repos_dir.join(format!("{}.git", repo_name));

        if repo_path.exists() {
            return Err(AppError::SkillAlreadyExists(format!(
                "Local repo for '{}' already exists",
                skill_name
            )));
        }

        let remote_url = self.remote_config.remote_url(&repo_name);

        let output = Command::new("git")
            .args(["clone", "--bare", &remote_url, &repo_path.to_string_lossy()])
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
        Ok(repo_path)
    }

    /// 从 GitLab 拉取最新更新
    pub fn fetch_from_gitlab(&self, skill_name: &str) -> Result<(), AppError> {
        let repo_name = format!("skill-{}", skill_name);
        let repo_path = self.repos_dir.join(format!("{}.git", repo_name));

        if !repo_path.exists() {
            return Err(AppError::SkillNotFound(format!(
                "Local repo for '{}' not found. Clone it first.",
                skill_name
            )));
        }

        let repo_str = repo_path.to_string_lossy();

        let output = Command::new("git")
            .args([
                "--git-dir",
                &repo_str,
                "fetch",
                "origin",
                "--tags",
                "--prune",
            ])
            .output()
            .map_err(|e| AppError::InternalError(format!("git fetch failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::InternalError(format!(
                "git fetch failed: {}",
                stderr
            )));
        }

        info!("Fetched latest for {} from GitLab", repo_name);
        Ok(())
    }

    // ==================== 版本查询 ====================

    /// 列出所有 Git tags（版本）
    pub fn list_git_tags(&self, skill_name: &str) -> Result<Vec<String>, AppError> {
        let repo_path = self.repos_dir.join(format!("skill-{}.git", skill_name));
        if !repo_path.exists() {
            return Ok(vec![]);
        }

        let output = Command::new("git")
            .args(["--git-dir", &repo_path.to_string_lossy()])
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

    /// 获取版本间的 diff
    pub fn get_version_diff(
        &self,
        skill_name: &str,
        from_version: &str,
        to_version: &str,
    ) -> Result<String, AppError> {
        let repo_path = self.repos_dir.join(format!("skill-{}.git", skill_name));
        if !repo_path.exists() {
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
            .args(["--git-dir", &repo_path.to_string_lossy()])
            .args(["diff", &from_tag, &to_tag, "--", "SKILL.md"])
            .output()
            .map_err(|e| AppError::InternalError(format!("git diff failed: {}", e)))?;

        // diff 可能为空（相同内容），这不是错误
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// 获取特定版本的 SKILL.md 内容
    pub fn get_file_at_version(
        &self,
        skill_name: &str,
        version: &str,
        file_path: &str,
    ) -> Result<String, AppError> {
        let repo_path = self.repos_dir.join(format!("skill-{}.git", skill_name));
        if !repo_path.exists() {
            return Err(AppError::SkillNotFound(skill_name.to_string()));
        }

        let tag = if version.starts_with('v') {
            version.to_string()
        } else {
            format!("v{}", version)
        };
        let ref_spec = format!("{}:{}", tag, file_path);

        let output = Command::new("git")
            .args(["--git-dir", &repo_path.to_string_lossy()])
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

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// 简易 YAML frontmatter 解析
/// 从 SKILL.md 中提取 `---` 包裹的元数据
pub fn parse_skill_md_frontmatter(content: &str) -> Result<ParsedSkillMetadata, AppError> {
    let mut metadata = ParsedSkillMetadata {
        name: String::new(),
        description: String::new(),
        tags: Vec::new(),
        version: String::new(),
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

    for line in fm_body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(colon_pos) = line.find(':') {
            // 遇到新 key 时保存之前的 list
            if let Some(ref key) = current_key {
                if !list_buffer.is_empty() {
                    apply_list_value(&mut metadata, key, &list_buffer);
                    list_buffer.clear();
                }
            }

            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();

            if value.is_empty() || value == "[]" {
                // 可能是多行列表的开始
                current_key = Some(key.clone());
                if value == "[]" {
                    list_buffer = Vec::new();
                }
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
            } else {
                apply_scalar_value(&mut metadata, &key, &value);
                current_key = None;
                list_buffer.clear();
            }
        } else if let Some(ref _key) = current_key {
            // 列表项：- item
            let item = line.trim_start_matches('-').trim();
            if !item.is_empty() {
                let cleaned = item.trim_matches('"').trim_matches('\'').to_string();
                list_buffer.push(cleaned);
            }
        }
    }

    // 处理结尾的 list
    if let Some(ref key) = current_key {
        if !list_buffer.is_empty() {
            apply_list_value(&mut metadata, key, &list_buffer);
        }
    }

    Ok(metadata)
}

fn apply_scalar_value(meta: &mut ParsedSkillMetadata, key: &str, value: &str) {
    match key {
        "name" => meta.name = value.to_string(),
        "description" => meta.description = value.to_string(),
        "version" => meta.version = value.to_string(),
        "compatibility" => meta.compatibility = value.to_string(),
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
        assert_eq!(meta.version, "1.0.0");
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
        assert_eq!(meta.version, "2.0.0");
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let md = "# Just a heading\n\nSome content";
        let meta = parse_skill_md_frontmatter(md).unwrap();
        assert!(meta.name.is_empty());
        assert!(meta.version.is_empty());
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
