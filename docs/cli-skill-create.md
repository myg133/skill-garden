# CLI Skill Create — 通过 ZIP 包创建 Skill

## 背景

当前 `skills.create` MCP tool 只支持传入 `content` 字符串（单个 SKILL.md 文本），不支持上传完整的 ZIP 包（含多文件、assets 等）。

CLI 工具只有 API Key（`sk_` 前缀），无法直接调用 REST API 的 upload 接口（REST 接口的 `AgentContext::from_request_parts` 仅支持 JWT）。

## 目标

在 CLI 中新增 `skill-garden create <file.zip>` 命令，支持上传 ZIP 包创建 Skill。

## 约束

- CLI 只持有 API Key，不持有 JWT
- REST API 的 upload preview/confirm 接口使用 JWT `AgentContext` 认证
- 不能通过 MCP JSON-RPC 直接传大文件（base64 编码膨胀 33%，二进制安装包可能几十 MB）
- 不能修改 REST API 的 `AgentContext::from_request_parts` 全局认证逻辑（影响面太大）

## 方案：三阶段流程

```
CLI                              MCP Server                       REST API
 |                                   |                                |
 |-- MCP: skills.prepare_upload ---->|                                |
 |                                   |-- 生成一次性 JWT (60s 有效)     |
 |<-- { upload_url, upload_token } --|                                |
 |                                   |                                |
 |-- REST POST upload_url (ZIP) ---->|------------------------------->|
 |                                   |   POST /api/v1/skills/upload   |
 |                                   |   /preview (multipart)         |
 |<-- { preview_id } ---------------|--------------------------------|
 |                                   |                                |
 |-- MCP: skills.create ------------>|                                |
 |   { preview_id, owner_type,       |                                |
 |     organization_id, visibility } |-- confirm_upload_from_preview  |
 |                                   |   + registry.create_skill      |
 |<-- { skill } ---------------------|                                |
```

### 阶段 1：`skills.prepare_upload`（MCP）

新增 MCP tool，生成一次性上传凭证。

**输入参数：**
- 无（认证信息从 Bearer token 自动提取）

**处理逻辑：**
1. 从 `agent_ctx` 提取 `identity_id`、`org_id`
2. 调用 `generate_token(identity_id, roles, scope)` 生成 JWT，有效期 60 秒
3. 返回 `upload_url` 和 `upload_token`

**返回：**
```json
{
  "upload_url": "http://localhost:8080/api/v1/skills/upload/preview",
  "upload_token": "eyJhbGciOi...",
  "expires_in": 60
}
```

### 阶段 2：REST API 上传 ZIP

CLI 使用 `upload_token` 作为 Bearer token，直接 POST multipart/form-data 到 `upload_url`。

**请求：**
```
POST /api/v1/skills/upload/preview
Authorization: Bearer <upload_token>
Content-Type: multipart/form-data

file: <skill.zip>
```

**处理逻辑：**
- 现有的 `upload_skill_preview_handler` 已经实现
- 只需要它能接受这个一次性 JWT（JWT 中包含 identity_id）
- 无需修改 handler 代码

**返回：**
```json
{
  "preview_id": "abc12345",
  "metadata": {
    "name": "my-skill",
    "version": "1.0.0",
    "description": "...",
    "tags": [...]
  },
  "files": [
    { "path": "SKILL.md", "size": 1024 },
    { "path": "assets/logo.png", "size": 24576 }
  ],
  "total_files": 2,
  "total_size": 25600
}
```

### 阶段 3：`skills.create` 确认（MCP）

修改 `skills.create` tool，新增可选参数 `preview_id`。

**输入参数（新增）：**
```json
{
  "preview_id": "string (optional, 如果提供则从预览目录确认上传)",
  "owner_type": "user | organization",
  "organization_id": "string (UUID, owner_type=organization 时需要)",
  "visibility": "private | org_visible | marketplace | shared"
}
```

**处理逻辑：**
1. 如果提供了 `preview_id`：
   - 调用 `skill_git.confirm_upload_from_preview(preview_id, ...)`
   - 不再需要 `name`、`description`、`content`、`tags`（从 ZIP 中解析）
2. 如果没提供 `preview_id`：
   - 走现有逻辑（`name`、`description`、`content` 必填）
3. 两种路径统一返回创建的 Skill 信息

**注意：** `skills.create` schema 中 `required` 需要调整为条件必填——当 `preview_id` 存在时不需要 `name/description/content`，当 `preview_id` 不存在时需要。

---

## 需要修改的文件

### 1. `src/mcp/server.rs`

#### 1.1 新增 `skills.prepare_upload` tool 注册

在 `list_tools` 方法中新增：
```rust
Tool::new(
    "skills.prepare_upload",
    "Prepare an upload session for creating a skill from a ZIP file. Returns a one-time token and URL for uploading the ZIP via REST API.",
    Arc::new(serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    }).as_object().unwrap().clone()),
),
```

#### 1.2 新增 `skills.prepare_upload` handler

在 `call_tool_internal` 中新增分支：
```rust
"skills.prepare_upload" => {
    let identity_id = agent_ctx.and_then(|c| c.identity_id);
    let org_id = agent_ctx.and_then(|c| c.org_id);
    
    match (identity_id, org_id) {
        (Some(id), org) => {
            // 生成一次性 JWT（60 秒有效）
            let roles: Vec<&str> = vec![];
            let scopes: Vec<&str> = vec!["skill:upload"];
            match crate::api::jwt::generate_identity_token(id, &roles, &scopes) {
                Ok(token) => {
                    let server_url = std::env::var("AION_HIVE_PUBLIC_URL")
                        .unwrap_or_else(|_| "http://localhost:8080".to_string());
                    serde_json::to_value(&serde_json::json!({
                        "success": true,
                        "upload_url": format!("{}/api/v1/skills/upload/preview", server_url),
                        "upload_token": token,
                        "expires_in": 60
                    })).unwrap_or_default()
                }
                Err(e) => Self::json_error(format!("Failed to generate upload token: {}", e)),
            }
        }
        _ => Self::json_error("Authentication required".to_string()),
    }
}
```

> **需要 `AION_HIVE_PUBLIC_URL` 环境变量**：CLI 连接的可能是远程服务端，需要返回完整 URL。如果未设置，默认 `http://localhost:8080`。

#### 1.3 修改 `skills.create` tool schema

```rust
Tool::new(
    "skills.create",
    "Create a new skill",
    Arc::new(serde_json::json!({
        "type": "object",
        "properties": {
            "name": {"type": "string", "description": "Skill name (required if preview_id not provided)"},
            "description": {"type": "string", "description": "Skill description (required if preview_id not provided)"},
            "tags": {"type": "array", "items": {"type": "string"}, "description": "Skill tags"},
            "tools": {"type": "array", "items": {"type": "string"}, "description": "Tool names to include"},
            "content": {"type": "string", "description": "SKILL.md content (required if preview_id not provided)"},
            "version": {"type": "string", "description": "Version (default 1.0.0)"},
            "preview_id": {"type": "string", "description": "Upload preview ID from skills.prepare_upload flow. If provided, name/description/content are extracted from the ZIP."},
            "owner_type": {"type": "string", "enum": ["user", "organization"], "description": "Owner type"},
            "organization_id": {"type": "string", "description": "Organization UUID"},
            "visibility": {"type": "string", "enum": ["private", "org_visible", "marketplace", "shared"], "description": "Skill visibility"}
        },
        "required": []
    }).as_object().unwrap().clone()),
),
```

#### 1.4 修改 `skills.create` handler

在现有 `skills.create` handler 开头新增 `preview_id` 分支：

```rust
"skills.create" => {
    let preview_id = args.get("preview_id").and_then(|v| v.as_str());
    
    if let Some(preview_id) = preview_id {
        // --- ZIP 上传流程 ---
        let owner_type = args.get("owner_type")
            .and_then(|v| v.as_str())
            .unwrap_or("user");
        let owner_id = if owner_type == "organization" {
            args.get("organization_id")
                .and_then(|v| v.as_str())
                .and_then(|s| uuid::Uuid::parse_str(s).ok())
        } else {
            agent_ctx.and_then(|c| c.identity_id)
        };
        let visibility = args.get("visibility")
            .and_then(|v| v.as_str())
            .unwrap_or(if owner_type == "organization" { "org_visible" } else { "private" });
        
        let author_agent_id = Self::resolve_agent_id(agent_ctx);
        let author_identity_id = Self::resolve_owner_id(agent_ctx);
        
        match self.skill_git.confirm_upload_from_preview(
            preview_id,
            &author_agent_id,
            author_identity_id,
            owner_type,
            owner_id,
            &self.registry,
            &self.search,
            &self.skill_repo,    // 需要注入
            &self.version_repo,  // 需要注入
        ).await {
            Ok(skill) => Self::json_success(serde_json::to_value(skill).unwrap_or_default()),
            Err(e) => Self::json_error(format!("Create skill from ZIP failed: {}", e)),
        }
    } else {
        // --- 现有 content 字符串流程 ---
        // ... 保持现有逻辑不变 ...
    }
}
```

> **注意**：`confirm_upload_from_preview` 需要 `SkillRepository` 和 `VersionRepository`，但 `McpServer` 目前没有这两个字段。需要在 `McpServer::new()` 中增加参数并存储。

### 2. `src/cli/bin/cli.rs`

新增 `Create` 子命令：

```rust
/// 从 ZIP 包创建 Skill
Create {
    /// ZIP 文件路径
    zip_file: String,
    /// 归属类型：user 或 organization
    #[arg(short, long, default_value = "user")]
    owner_type: String,
    /// 组织 ID（owner_type=organization 时必填）
    #[arg(short = 'O', long)]
    organization_id: Option<String>,
    /// 可见性
    #[arg(short, long, default_value = "private")]
    visibility: String,
},
```

### 3. `src/cli/commands.rs`

新增 `create` 函数：

```rust
/// 从 ZIP 包创建 Skill（三阶段流程）
pub async fn create(
    client: &ApiClient,
    zip_file: &str,
    owner_type: &str,
    organization_id: Option<&str>,
    visibility: &str,
) -> Result<()> {
    // 阶段 1：获取上传凭证
    let pb = ProgressBar::new_spinner();
    pb.set_message("正在准备上传...");
    let upload_info = client.prepare_upload().await?;
    pb.finish_and_clear();
    
    // 阶段 2：上传 ZIP 到 REST API
    let pb = ProgressBar::new_spinner();
    pb.set_message("正在上传 ZIP 包...");
    let zip_data = std::fs::read(zip_file)?;
    let preview = client.upload_zip(&upload_info.upload_url, &upload_info.upload_token, &zip_data).await?;
    pb.finish_with_message("上传完成");
    
    println!("预览:");
    println!("  名称: {}", preview.metadata.name);
    println!("  版本: {}", preview.metadata.version);
    println!("  文件数: {}", preview.total_files);
    println!("  大小: {}", format_size(preview.total_size));
    
    // 阶段 3：确认创建
    let pb = ProgressBar::new_spinner();
    pb.set_message("正在创建 Skill...");
    let skill = client.create_skill_from_preview(
        &preview.preview_id,
        owner_type,
        organization_id,
        visibility,
    ).await?;
    pb.finish_and_clear();
    
    println!("\nSkill 创建成功!");
    println!("  ID: {}", skill.id);
    println!("  名称: {}", skill.name);
    println!("  版本: {}", skill.version);
    println!("  状态: {}", skill.status);
    
    Ok(())
}
```

### 4. `src/cli/client.rs`

新增三个方法：

```rust
/// 获取上传凭证（阶段 1）
pub async fn prepare_upload(&self) -> Result<PrepareUploadResult> {
    let args = serde_json::json!({});
    let data = self.call_tool("skills.prepare_upload", args).await?;
    let result: PrepareUploadResult = serde_json::from_value(data)?;
    Ok(result)
}

/// 上传 ZIP 文件到 REST API（阶段 2）
pub async fn upload_zip(&self, upload_url: &str, token: &str, zip_data: &[u8]) -> Result<UploadPreviewResult> {
    let part = reqwest::multipart::Part::bytes(zip_data.to_vec())
        .file_name("skill.zip")
        .mime_str("application/zip")?;
    let form = reqwest::multipart::Form::new().part("file", part);
    
    let resp = self.http
        .post(upload_url)
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await?;
    
    if !resp.status().is_success() {
        let body = resp.text().await?;
        anyhow::bail!("Upload failed: {}", body);
    }
    
    let result: UploadPreviewResult = resp.json().await?;
    Ok(result)
}

/// 从 preview_id 创建 Skill（阶段 3）
pub async fn create_skill_from_preview(
    &self,
    preview_id: &str,
    owner_type: &str,
    organization_id: Option<&str>,
    visibility: &str,
) -> Result<SkillDetail> {
    let mut args = serde_json::json!({
        "preview_id": preview_id,
        "owner_type": owner_type,
        "visibility": visibility,
    });
    if let Some(org_id) = organization_id {
        args["organization_id"] = serde_json::json!(org_id);
    }
    let data = self.call_tool("skills.create", args).await?;
    let skill: SkillDetail = serde_json::from_value(data)?;
    Ok(skill)
}
```

### 5. `src/services/skill_git.rs`

无需修改，现有的 `preview_upload()` 和 `confirm_upload_from_preview()` 已经可用。

### 6. `src/mcp/server.rs` — McpServer 结构体

需要新增两个字段（`confirm_upload_from_preview` 依赖）：

```rust
pub struct McpServer {
    // ... 现有字段 ...
    skill_repo: SkillRepository,
    version_repo: VersionRepository,
}
```

并在 `McpServer::new()` 中增加参数。

### 7. 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `AION_HIVE_PUBLIC_URL` | `http://localhost:8080` | 服务端公网地址，用于生成 upload_url |

---

## 数据流

```
skill-garden create my-skill.zip --owner-type user
    │
    ├─ 1. MCP skills.prepare_upload
    │     → 生成一次性 JWT (60s TTL, scope: skill:upload)
    │     → 返回 upload_url + upload_token
    │
    ├─ 2. REST POST /api/v1/skills/upload/preview
    │     Authorization: Bearer <upload_token>
    │     Content-Type: multipart/form-data
    │     Body: file=my-skill.zip
    │     → 返回 preview_id + metadata + file list
    │
    └─ 3. MCP skills.create { preview_id, owner_type, visibility }
          → confirm_upload_from_preview()
          → 返回创建的 Skill
```

---

## 安全考量

1. **一次性 JWT**：`skills.prepare_upload` 生成的 JWT 有效期 60 秒，仅用于 ZIP 上传
2. **scope 限制**：JWT scope 仅包含 `skill:upload`，不能用于其他操作
3. **identity 绑定**：JWT 中携带 `identity_id`，后端验证上传者身份
4. **API Key 不可用于 REST**：upload URL 只能用一次性 JWT 访问，API Key 仍然不能直接调 REST API

---

## 待确认项

- [ ] `McpServer` 是否需要注入 `SkillRepository` 和 `VersionRepository`（目前 `confirm_upload_from_preview` 需要这两个）
- [ ] `AION_HIVE_PUBLIC_URL` 环境变量的命名和默认值
- [ ] 是否需要在 `upload_skill_preview_handler` 中增加额外的 scope 校验（检查 JWT claims 中的 scope 字段）
