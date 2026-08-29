//! CLI 配置管理
//!
//! 配置存储在 `~/.skill-garden/config.toml`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 全局 CLI 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// 服务端地址（例：https://skill-garden.example.com）
    pub server: Option<String>,
    /// API Key（sk_ 前缀）
    pub token: Option<String>,
    /// Skill 安装目录（例：/home/user/.agent/skills）
    #[serde(default)]
    pub skills_dir: Option<String>,
}

impl CliConfig {
    /// 加载配置文件，不存在则返回默认空配置
    pub fn load() -> Result<Self> {
        let path = config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("无法读取配置文件: {}", path.display()))?;
            toml::from_str(&content)
                .with_context(|| format!("配置文件格式错误: {}", path.display()))
        } else {
            Ok(CliConfig {
                server: None,
                token: None,
                skills_dir: None,
            })
        }
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建配置目录: {}", parent.display()))?;
        }
        let content = toml::to_string_pretty(self).context("序列化配置失败")?;
        std::fs::write(&path, content)
            .with_context(|| format!("无法写入配置文件: {}", path.display()))?;
        Ok(())
    }

    /// 删除配置文件（logout）
    pub fn delete() -> Result<()> {
        let path = config_path();
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("无法删除配置文件: {}", path.display()))?;
        }
        Ok(())
    }
}

/// 配置文件路径：~/.skill-garden/config.toml
fn config_path() -> PathBuf {
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join(".skill-garden").join("config.toml")
}
