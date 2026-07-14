//! 下载凭证模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 下载 Token 记录（数据库行映射）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
}
