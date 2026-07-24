//! 下载凭证模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 下载 Token 记录（数据库行映射）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct DownloadToken {
    pub id: Uuid,
    pub token: String,
    pub skill_name: String,
    pub skill_version: String,
    pub identity_id: Uuid,
    pub api_key_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    /// 资源类型：`skill` 或 `cli`
    #[cfg_attr(feature = "server", sqlx(default))]
    #[serde(default = "default_resource_type")]
    pub resource_type: String,
    /// CLI 下载目标（仅 resource_type = "cli" 时有值）：linux-x86_64 / macos-aarch64 等
    #[cfg_attr(feature = "server", sqlx(default))]
    #[serde(default)]
    pub target: Option<String>,
    /// CLI 下载时的预填 config.toml 内容（仅 resource_type = "cli" 时有值）
    /// 在创建 token 时写入，下载时嵌入 tar.gz，避免需要反查 API key 明文
    #[cfg_attr(feature = "server", sqlx(default))]
    #[serde(default)]
    pub config_data: Option<String>,
}

fn default_resource_type() -> String {
    "skill".to_string()
}
