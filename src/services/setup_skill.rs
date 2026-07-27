//! Setup skill builder — generates the `SKILL.md` content returned by
//! `GET /api/v1/onboarding/setup-skill`.
//!
//! The template is embedded as a Rust constant so no runtime file I/O is
//! performed. Server URL resolution is documented in
//! `docs/api-key-setup-skill-design.md` (section 4).

/// `SKILL.md` template for the `skill-garden-setup` skill.
///
/// The template is intentionally in Chinese for friendliness. Placeholders
/// are populated by [`SetupSkillBuilder::build`]:
///
/// - `{server_url}` — public base URL of the Skill Garden deployment
/// - `{mcp_url}`    — MCP HTTP endpoint
/// - `{sse_url}`    — SSE fallback endpoint
const SETUP_SKILL_TEMPLATE: &str = r#"---
name: skill-garden-setup
description: 为当前 Agent 配置 Skill Garden MCP 并安装 Skill Garden CLI。
version: "1"
---

# Skill Garden 安装引导

当用户希望让当前 Agent 接入 Skill Garden、配置 MCP，或安装 Skill Garden CLI 时，使用本 Skill。

## 默认端点

- 服务地址：`{server_url}`
- MCP：`{mcp_url}`
- SSE 备用：`{sse_url}`

如果服务地址缺失，或用户希望使用其他部署，请向用户询问 Skill Garden 的服务地址。

## 安全要求

本 Skill 只负责**安装引导**。涉及 API Key 的内容遵循以下约束：

- **API Key 由用户主动提供**：用户在平台创建 API Key 后自行复制，再把服务地址和 API Key 提交给 Agent。
- **Agent 不自动生成、猜测或尝试获取**任何 API Key。
- 把 API Key 视为敏感信息。
- 不要在回复、日志、命令、URL 或项目文件中重复完整 API Key。
- 只展示脱敏后的值，例如 `sk_abcd****`。
- 优先使用 Agent 自带的 Secret 管理或环境变量能力。
- 必须把明文 API Key 写入配置文件时，先提示用户该文件包含敏感信息。
- 禁止把 MCP 配置或 CLI 凭证提交到代码仓库。

## 必填输入

只有服务地址和 API Key 需要由用户主动提供，**其他参数由 Agent 自动处理**。

向用户索取：

1. Skill Garden 服务地址；
2. 以 `sk_` 开头的 API Key（用户在平台创建后自行复制提供）。

由 Agent 自动识别：

3. 当前 Agent / 客户端类型；
4. 操作系统；
5. CPU 架构；
6. Skills 目录（如果 Agent 使用 Skills）。

## 可恢复式安装

在动手修改任何配置前，先按顺序检查：

1. Skill Garden MCP 是否已经配置；
2. `cli.setup` 工具是否已经可用；
3. `skill-garden` CLI 是否已经安装；
4. CLI 是否已经成功认证；
5. `skills_dir` 是否已经配置。

已经完成的步骤必须跳过，不要覆盖已经可用的配置。

## 配置 MCP

1. 优先使用 `{mcp_url}`。
2. 仅在当前客户端不支持 MCP HTTP 端点时使用 `{sse_url}`。
3. 使用 `Authorization: Bearer <API_KEY>` 认证。
4. 优先使用当前 Agent 原生的 MCP 管理能力。
5. 保留所有已有的 MCP Server 和设置。
6. 如果无法自动配置，生成当前客户端所需的人工配置片段。
7. 不要猜测未知的配置格式；请向用户确认使用的客户端。

如果客户端需要重启或重载，告知用户如何重载，并请用户在重载后再次运行本 Skill。下一次执行时先确认 `cli.setup` 已经可用，并从该步骤继续。

## 验证 MCP

MCP 加载完成后：

1. 检查 Skill Garden 工具是否已经可见；
2. 确认 `health_check` 可用；
3. 确认 `cli.setup` 可用；
4. 调用一个需要认证的操作，确认 API Key 被接受；
5. 如果认证失败，请用户检查 API Key，不要回显完整 Key。

## 安装 CLI

1. 检测真实的操作系统和 CPU 架构；
2. 使用检测到的值调用 `cli.setup`；
3. 按 `cli.setup` 返回的指令执行；
4. 下载链接有效期短，获取后请立即下载；
5. 如果链接过期，再次调用 `cli.setup`；
6. 在执行命令前按当前 Agent 的安全策略向用户确认；
7. 把 `cli.setup` 附带的标准 `skill-garden` Skill 安装到当前 Agent 的 Skills 目录；
8. 按需设置 `skills_dir`。

## 验证安装

执行 `skill-garden whoami`；Windows 上执行 `skill-garden.exe whoami`。

只向用户汇报以下非敏感信息：

- MCP 连接状态；
- CLI 安装状态；
- 服务地址；
- 安全的身份摘要；
- Skills 目录；
- 脱敏后的 API Key 前缀。

## 安装后清理

安装与验证全部成功后，本 Skill 就完成了它的使命。Agent **必须**主动清理本文件及安装过程中产生的临时信息，避免遗留敏感数据或冗余文件：

1. 删除本文件 `skill-garden-setup.md`（如果用户把它放在目录中，删除该目录）。
2. 删除下载下来的 `skill-garden-cli/` 临时目录（含 `config.toml`、`install.sh`、`install.ps1`）。
3. 清理下载缓存：
   - 临时压缩包 / 安装包；
   - `~/.skill-garden/bin/` 中已不再使用的旧版本二进制；
   - `~/.skill-garden/config.toml` 备份文件（`*.<ext>.bak` 或 `*.tmp`）。
4. 清理 Agent 会话内的中间数据：
   - 用户提供的 API Key 不再保留在会话上下文；
   - 复制过的 agent_prompt 不再缓存；
   - 与本次安装相关的临时变量从内存中清空。
5. 通知用户：本次安装已完成，本 Skill 将在重启或重新加载 Agent 后被识别为“已安装”状态，后续不再触发。

清理完成后，本 Skill 在该 Agent 中不应再次被调用或被提示使用。
"#;

/// `agent_prompt` shown in the preview modal. Contains no secrets.
pub const SETUP_SKILL_AGENT_PROMPT: &str =
    "请读取 skill-garden-setup.md，并帮助我完成 Skill Garden MCP 和 CLI 安装。";

/// Current response schema version. Bump when the JSON shape changes.
pub const SETUP_SKILL_VERSION: &str = "1";

/// Result of [`SetupSkillBuilder::build`].
pub struct SetupSkillDoc {
    pub filename: &'static str,
    pub directory_name: &'static str,
    pub content_type: &'static str,
    pub encoding: &'static str,
    pub content: String,
    pub agent_prompt: &'static str,
    pub server_url: String,
    pub mcp_url: String,
    pub sse_url: String,
    pub version: &'static str,
}

/// Builder that produces a [`SetupSkillDoc`] for the current deployment.
pub struct SetupSkillBuilder;

impl SetupSkillBuilder {
    /// Build a setup skill document using the current deployment's public URL.
    pub fn build() -> SetupSkillDoc {
        let server_url = normalize_base(&resolve_server_url());
        let mcp_url = format!("{}/mcp", server_url);
        let sse_url = format!("{}/sse", server_url);

        let content = SETUP_SKILL_TEMPLATE
            .replace("{server_url}", &server_url)
            .replace("{mcp_url}", &mcp_url)
            .replace("{sse_url}", &sse_url);

        SetupSkillDoc {
            filename: "skill-garden-setup.md",
            directory_name: "skill-garden-setup",
            content_type: "text/markdown; charset=utf-8",
            encoding: "utf-8",
            content,
            agent_prompt: SETUP_SKILL_AGENT_PROMPT,
            server_url,
            mcp_url,
            sse_url,
            version: SETUP_SKILL_VERSION,
        }
    }
}

/// Resolve the public server URL.
///
/// Priority (matches `docs/api-key-setup-skill-design.md` § 4):
///
/// 1. `AION_HIVE_PUBLIC_URL` env var.
/// 2. `http://localhost:<AION_HIVE_HTTP_PORT>` for local development.
fn resolve_server_url() -> String {
    if let Ok(value) = std::env::var("AION_HIVE_PUBLIC_URL") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let port = std::env::var("AION_HIVE_HTTP_PORT").unwrap_or_else(|_| "8080".to_string());
    format!("http://localhost:{}", port.trim())
}

/// Strip trailing slashes and ensure the scheme is one of `http`/`https`.
fn normalize_base(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/').to_string();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed;
    }

    let port = std::env::var("AION_HIVE_HTTP_PORT").unwrap_or_else(|_| "8080".to_string());
    format!("http://localhost:{}", port.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_env() {
        std::env::remove_var("AION_HIVE_PUBLIC_URL");
        std::env::remove_var("AION_HIVE_HTTP_PORT");
    }

    #[test]
    fn build_replaces_placeholders() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("AION_HIVE_PUBLIC_URL", "https://skill.example.com");

        let doc = SetupSkillBuilder::build();
        assert!(doc.content.contains("https://skill.example.com"));
        assert!(doc.content.contains("https://skill.example.com/mcp"));
        assert!(doc.content.contains("https://skill.example.com/sse"));
        assert!(!doc.content.contains("{server_url}"));
        assert!(!doc.content.contains("{mcp_url}"));
        assert!(!doc.content.contains("{sse_url}"));
    }

    #[test]
    fn build_normalises_trailing_slash() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("AION_HIVE_PUBLIC_URL", "https://skill.example.com/");

        let doc = SetupSkillBuilder::build();
        assert_eq!(doc.server_url, "https://skill.example.com");
        assert_eq!(doc.mcp_url, "https://skill.example.com/mcp");
    }

    #[test]
    fn build_falls_back_to_localhost() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();

        let doc = SetupSkillBuilder::build();
        assert!(doc.server_url.starts_with("http://localhost:"));
    }

    #[test]
    fn normalize_base_rejects_non_http_scheme() {
        std::env::remove_var("AION_HIVE_HTTP_PORT");
        let result = normalize_base("ftp://example.com");
        assert!(result.starts_with("http://localhost:"));
    }
}
