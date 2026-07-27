//! MCP JSON-RPC 客户端
//!
//! 通过 `POST /mcp` 端点与 Skill Garden 服务端通信，
//! 使用 JSON-RPC 2.0 协议调用 MCP tools（skills.search / skills.list / ...）。

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::models::skill::InstallResult;

/// MCP JSON-RPC 客户端
pub struct ApiClient {
    pub server: String,
    pub token: String,
    http: reqwest::Client,
    request_id: AtomicU64,
}

impl ApiClient {
    pub fn new(server: String, token: String) -> Self {
        Self {
            server: server.trim_end_matches('/').to_string(),
            token,
            http: reqwest::Client::new(),
            request_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// 发送 JSON-RPC 请求，返回内部 data（解析自 content[0].text）
    async fn rpc(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let url = format!("{}/mcp", self.server);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .with_context(|| format!("MCP 请求失败: POST {}", url))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            anyhow::bail!("HTTP {}: {}", status.as_u16(), text);
        }

        let envelope: JsonRpcEnvelope = serde_json::from_str(&text)
            .with_context(|| format!("MCP 响应解析失败: {}", truncate(&text, 200)))?;

        match envelope {
            JsonRpcEnvelope::Success { result, .. } => {
                // 从 result.content[0].text 提取内部 JSON
                let inner_json = result
                    .get("content")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|item| item.get("text"))
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| anyhow::anyhow!("MCP 响应格式异常: content[0].text 缺失"))?;

                let data: Value = serde_json::from_str(inner_json).with_context(|| {
                    format!("解析工具返回值失败: {}", truncate(inner_json, 200))
                })?;

                // 检查 isError
                if result
                    .get("isError")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    let msg = data
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    anyhow::bail!("{}", msg);
                }

                Ok(data)
            }
            JsonRpcEnvelope::Error { error, .. } => {
                anyhow::bail!("MCP 错误 [{}]: {}", error.code, error.message);
            }
        }
    }

    /// 调用 MCP tools/call
    async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        self.rpc(
            "tools/call",
            serde_json::json!({
                "name": name,
                "arguments": arguments,
            }),
        )
        .await
    }

    // ============ Skills ============

    /// 搜索技能
    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<SearchItem>> {
        let args = serde_json::json!({
            "query": query,
            "limit": limit,
        });
        let data = self.call_tool("skills.search", args).await?;
        let items: Vec<SearchItem> = serde_json::from_value(data).context("解析搜索结果失败")?;
        Ok(items)
    }

    /// 列出技能（分页+排序）
    pub async fn list(&self, limit: u32, offset: u32, sort_by: &str) -> Result<ListResult> {
        let args = serde_json::json!({
            "limit": limit,
            "offset": offset,
            "sort_by": sort_by,
        });
        let data = self.call_tool("skills.list", args).await?;
        let result: ListResult = serde_json::from_value(data).context("解析技能列表失败")?;
        Ok(result)
    }

    /// 查看技能详情
    pub async fn info(&self, skill_id: &str) -> Result<SkillDetail> {
        let args = serde_json::json!({ "skill_id": skill_id });
        let data = self.call_tool("skills.info", args).await?;
        let detail: SkillDetail = serde_json::from_value(data).context("解析技能详情失败")?;
        Ok(detail)
    }

    /// 查看技能版本
    pub async fn versions(&self, skill_name: &str) -> Result<Vec<VersionItem>> {
        let args = serde_json::json!({ "name": skill_name });
        let data = self.call_tool("skills.versions", args).await?;
        let items: Vec<VersionItem> = serde_json::from_value(data).context("解析版本列表失败")?;
        Ok(items)
    }

    /// 安装技能（获取下载链接）
    pub async fn install(&self, skill_id: &str) -> Result<InstallResult> {
        let args = serde_json::json!({ "skill_id": skill_id });
        let data = self.call_tool("skills.install", args).await?;
        let result: InstallResult = serde_json::from_value(data).context("解析安装信息失败")?;
        Ok(result)
    }

    /// 热门技能
    pub async fn popular(&self, limit: u32) -> Result<Vec<SkillItem>> {
        let args = serde_json::json!({ "limit": limit });
        let data = self.call_tool("skills.popular", args).await?;
        let items: Vec<SkillItem> = serde_json::from_value(data).context("解析热门列表失败")?;
        Ok(items)
    }

    /// 技能统计
    pub async fn stats(&self, skill_id: &str) -> Result<StatsData> {
        let args = serde_json::json!({ "skill_id": skill_id });
        let data = self.call_tool("skills.stats", args).await?;
        let stats: StatsData = serde_json::from_value(data).context("解析统计信息失败")?;
        Ok(stats)
    }

    // ============ 认证验证 ============

    /// 验证 API Key 有效性（调用 skills.list limit=0）
    pub async fn verify_auth(&self) -> Result<()> {
        // skills.list 需要认证，用它验证 API Key
        self.list(0, 0, "created").await?;
        Ok(())
    }

    /// 获取会话信息（用于 whoami）
    pub async fn session_info(&self) -> Result<SessionInfo> {
        let args = serde_json::json!({});
        let data = self.call_tool("session.info", args).await?;
        let info: SessionInfo = serde_json::from_value(data).context("解析会话信息失败")?;
        Ok(info)
    }

    // ============ 下载 ============

    /// 下载 tarball 到指定目录
    pub async fn download_tarball(&self, url: &str, dest_dir: &str) -> Result<()> {
        tracing::debug!("[download_tarball] GET {}", url);

        let resp = self
            .http
            .get(url)
            .send()
            .await
            .with_context(|| format!("下载失败: {}", url))?;

        let status = resp.status();
        let content_len = resp.content_length().unwrap_or(0);
        tracing::debug!("[download_tarball] HTTP {}, Content-Length={}", status.as_u16(), content_len);

        if !status.is_success() {
            anyhow::bail!("下载失败: HTTP {}", status.as_u16());
        }

        let bytes = resp.bytes().await.context("读取响应体失败")?;
        tracing::info!("[download_tarball] 下载完成, {} bytes", bytes.len());

        std::fs::create_dir_all(dest_dir)
            .with_context(|| format!("无法创建目录: {}", dest_dir))?;
        tracing::debug!("[download_tarball] 目录已创建: {}", dest_dir);

        // 列出 tarball 内顶层条目
        {
            let cursor = std::io::Cursor::new(&bytes[..]);
            let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(cursor));
            let mut top_entries: Vec<String> = Vec::new();
            if let Ok(entries) = archive.entries() {
                for entry in entries.flatten() {
                    if let Ok(path) = entry.path() {
                        let s = path.to_string_lossy().to_string();
                        if let Some(first) = s.split('/').next() {
                            if !first.is_empty() && !top_entries.contains(&first.to_string()) {
                                top_entries.push(first.to_string());
                            }
                        }
                    }
                }
            }
            tracing::info!("[download_tarball] tarball 顶层条目: {:?}", top_entries);
        }

        // 解压
        let cursor = std::io::Cursor::new(&bytes[..]);
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(cursor));
        archive.unpack(dest_dir).context("解压 tarball 失败")?;
        tracing::info!("[download_tarball] 解压完成 -> {}", dest_dir);

        Ok(())
    }
}

// ============ JSON-RPC 响应格式 ============

#[derive(Deserialize)]
#[serde(untagged)]
enum JsonRpcEnvelope {
    Success {
        result: Value,
        #[allow(dead_code)]
        id: Option<Value>,
    },
    Error {
        error: JsonRpcError,
        #[allow(dead_code)]
        id: Option<Value>,
    },
}

#[derive(Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

// ============ 响应类型 ============

/// skills.search 搜索结果项
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    pub skill_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub snippet: Option<String>,
}

/// skills.list 响应
#[derive(Debug, Deserialize)]
pub struct ListResult {
    #[serde(default)]
    pub skills: Vec<SkillItem>,
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

/// 技能列表项
#[derive(Debug, Deserialize)]
pub struct SkillItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub install_count: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub visibility: String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub updated: String,
}

/// skills.info 响应
#[derive(Debug, Deserialize)]
pub struct SkillDetail {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author_agent_id: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub install_count: u32,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub visibility: String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub updated: String,
    #[serde(default)]
    pub content: String,
}

/// skills.versions 响应单项
#[derive(Debug, Deserialize)]
pub struct VersionItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub skill_name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub skill_id: String,
    #[serde(default)]
    pub status: String,
}

/// skills.stats 响应
#[derive(Debug, Deserialize)]
pub struct StatsData {
    #[serde(default)]
    pub total_evaluations: u64,
    #[serde(default)]
    pub success_count: u64,
    #[serde(default)]
    pub failure_count: u64,
    #[serde(default)]
    pub success_rate: f64,
    #[serde(default)]
    pub avg_duration_ms: f64,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// session.info 响应
#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub org_id: String,
    #[serde(default)]
    pub identity_id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub created_at: String,
}

// ============ helpers ============

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}
