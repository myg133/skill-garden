//! Skill 数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::skill_policy::Visibility;

/// Skill 完整模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// 唯一标识符，格式: skill-{name}-{version}
    pub id: String,
    /// Skill 名称
    pub name: String,
    /// 描述（Agent 可解析）
    pub description: String,
    /// 标签
    pub tags: Vec<String>,
    /// 版本号 (semver)
    pub version: String,
    /// 创建者 Agent ID
    pub author_agent_id: String,
    /// 创建者 Identity ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_identity_id: Option<Uuid>,
    /// 所有权类型: 'user' | 'organization'
    #[serde(default = "default_owner_type")]
    pub owner_type: String,
    /// 所有者 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Uuid>,
    /// 创建时间
    pub created: DateTime<Utc>,
    /// 更新时间
    pub updated: DateTime<Utc>,
    /// 兼容性要求
    pub compatibility: String,
    /// 依赖的其他 Skills
    pub dependencies: Vec<String>,
    /// SKILL.md 内容
    pub content: String,
    /// 安装次数
    #[serde(default)]
    pub install_count: u32,
    /// Git 仓库 URL
    #[serde(default)]
    pub git_url: Option<String>,
    /// 可见性
    #[serde(default)]
    pub visibility: Visibility,
    /// 生命周期状态: draft | pending_review | approved | rejected | published
    #[serde(default = "default_status")]
    pub status: String,
    /// Skill 引用的工具列表
    #[serde(default)]
    pub tools: Vec<String>,
    /// 审核人 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<Uuid>,
    /// 审核时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<DateTime<Utc>>,
    /// 审核评论
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_comment: Option<String>,
}

fn default_owner_type() -> String {
    "user".to_string()
}

fn default_status() -> String {
    "draft".to_string()
}

impl Skill {
    /// 创建新 Skill
    pub fn new(
        name: String,
        description: String,
        tags: Vec<String>,
        version: String,
        author_agent_id: String,
        content: String,
    ) -> Self {
        let now = Utc::now();
        let id = format!("skill-{}-{}", name, version);

        Self {
            id,
            name,
            description,
            tags,
            version,
            author_agent_id,
            author_identity_id: None,
            owner_type: "user".to_string(),
            owner_id: None,
            created: now,
            updated: now,
            compatibility: ">=1.0.0".to_string(),
            dependencies: Vec::new(),
            content,
            install_count: 0,
            git_url: None,
            visibility: Visibility::OrgVisible,
            status: "draft".to_string(),
            tools: Vec::new(),
            reviewed_by: None,
            reviewed_at: None,
            review_comment: None,
        }
    }

    /// 生成技能 ID
    pub fn generate_id(name: &str, version: &str) -> String {
        format!("skill-{}-{}", name, version)
    }
}

/// Skill 元数据（不含 content，用于列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub version: String,
    pub author_agent_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_identity_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    #[serde(default = "default_owner_type")]
    pub owner_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Uuid>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub install_count: u32,
    pub status: String,
    #[serde(default)]
    pub git_url: Option<String>,
    #[serde(default)]
    pub visibility: Visibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_comment: Option<String>,
}

impl From<&Skill> for SkillMetadata {
    fn from(skill: &Skill) -> Self {
        Self {
            id: skill.id.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
            tags: skill.tags.clone(),
            version: skill.version.clone(),
            author_agent_id: skill.author_agent_id.clone(),
            author_identity_id: skill.author_identity_id,
            author_name: None,
            owner_type: skill.owner_type.clone(),
            owner_id: skill.owner_id,
            created: skill.created,
            updated: skill.updated,
            install_count: skill.install_count,
            status: skill.status.clone(),
            git_url: skill.git_url.clone(),
            visibility: skill.visibility.clone(),
            reviewed_by: skill.reviewed_by,
            reviewed_at: skill.reviewed_at,
            review_comment: skill.review_comment.clone(),
        }
    }
}

/// Skill 详情（包含完整内容和统计）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetail {
    pub metadata: SkillMetadata,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<super::evaluation::SkillStats>,
}

impl From<Skill> for SkillDetail {
    fn from(skill: Skill) -> Self {
        Self {
            metadata: (&skill).into(),
            content: skill.content,
            stats: None,
        }
    }
}

/// 安装结果 — 包含 Skill 元数据 + 下载链接（tarball）
/// Agent 通过 download_url 一次性下载 tar.gz，解压到 skill 目录即可
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub skill_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author_agent_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_identity_id: Option<Uuid>,
    pub owner_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Uuid>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub install_count: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub git_url: Option<String>,
    /// 依赖的其他 Skills
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// 引用的工具
    #[serde(default)]
    pub tools: Vec<String>,
    /// tarball 下载链接（含签名 token，TTL 300秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    /// URL 有效期（秒）
    #[serde(default)]
    pub expires_in: u64,
    /// 安装指引（人类可读）
    #[serde(default)]
    pub install_hint: String,
    /// 文件总数
    #[serde(default)]
    pub file_count: usize,
    /// tarball 大小（字节）
    #[serde(default)]
    pub tarball_size: u64,
}

/// Skill 更新参数
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
}

/// 创建 Skill 的输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSkill {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub content: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub git_url: Option<String>,
    #[serde(default)]
    pub visibility: Option<Visibility>,
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    #[serde(default = "default_owner_type")]
    pub owner_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Uuid>,
    #[serde(default)]
    pub author_identity_id: Option<Uuid>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_new() {
        let skill = Skill::new(
            "browse".to_string(),
            "A web browsing skill".to_string(),
            vec!["web".to_string(), "http".to_string()],
            "1.0.0".to_string(),
            "agent-1".to_string(),
            "# SKILL.md content".to_string(),
        );
        assert_eq!(skill.name, "browse");
        assert_eq!(skill.id, "skill-browse-1.0.0");
        assert_eq!(skill.version, "1.0.0");
        assert_eq!(skill.install_count, 0);
        assert!(skill.dependencies.is_empty());
    }

    #[test]
    fn test_skill_generate_id() {
        let id = Skill::generate_id("browse", "1.0.0");
        assert_eq!(id, "skill-browse-1.0.0");
    }

    #[test]
    fn test_skill_metadata_from_skill() {
        let skill = Skill::new(
            "browse".to_string(),
            "A web browsing skill".to_string(),
            vec!["web".to_string()],
            "1.0.0".to_string(),
            "agent-1".to_string(),
            "# SKILL.md".to_string(),
        );
        let metadata = SkillMetadata::from(&skill);
        assert_eq!(metadata.id, skill.id);
        assert_eq!(metadata.name, skill.name);
        assert_eq!(metadata.description, skill.description);
        assert_eq!(metadata.tags, skill.tags);
        assert_eq!(metadata.version, skill.version);
        assert_eq!(metadata.install_count, 0);
    }

    #[test]
    fn test_skill_detail_from_skill() {
        let skill = Skill::new(
            "browse".to_string(),
            "A web browsing skill".to_string(),
            vec![],
            "1.0.0".to_string(),
            "agent-1".to_string(),
            "# SKILL.md".to_string(),
        );
        let detail = SkillDetail::from(skill.clone());
        assert_eq!(detail.metadata.name, "browse");
        assert_eq!(detail.content, "# SKILL.md");
        assert!(detail.stats.is_none());
    }

    #[test]
    fn test_skill_update_default() {
        let update = SkillUpdate::default();
        assert!(update.description.is_none());
        assert!(update.tags.is_none());
        assert!(update.content.is_none());
    }

    #[test]
    fn test_new_skill_serde() {
        let json =
            r#"{"name":"browse","description":"test","tags":["web"],"content":"skill content"}"#
                .to_string();
        let new_skill: NewSkill = serde_json::from_str(&json).unwrap();
        assert_eq!(new_skill.name, "browse");
        assert_eq!(new_skill.version, "1.0.0");
    }

    #[test]
    fn test_skills_index_default() {
        let index = SkillsIndex::default();
        assert_eq!(index.version, "1.0");
        assert!(index.skills.is_empty());
    }

    #[test]
    fn test_install_result_serde() {
        let result = InstallResult {
            success: true,
            skill_id: "skill-browse-1.0.0".to_string(),
            name: "browse".to_string(),
            version: "1.0.0".to_string(),
            description: "Browse skill".to_string(),
            author_agent_id: "agent-1".to_string(),
            author_identity_id: None,
            owner_type: "user".to_string(),
            owner_id: None,
            created: chrono::Utc::now(),
            updated: chrono::Utc::now(),
            install_count: 0,
            tags: vec![],
            git_url: None,
            dependencies: vec![],
            tools: vec![],
            download_url: Some(
                "http://localhost:8080/api/v1/skills/browse/download/1.0.0?token=xxx&expires=12345"
                    .to_string(),
            ),
            expires_in: 300,
            install_hint: "Download the tarball and extract to your skills directory".to_string(),
            file_count: 2,
            tarball_size: 1234,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: InstallResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.skill_id, "skill-browse-1.0.0");
        assert_eq!(parsed.file_count, 2);
        assert_eq!(parsed.expires_in, 300);
        assert!(parsed.download_url.is_some());
    }
}

/// Skills 索引文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsIndex {
    pub version: String,
    pub skills: Vec<SkillMetadata>,
}

impl Default for SkillsIndex {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            skills: Vec::new(),
        }
    }
}
