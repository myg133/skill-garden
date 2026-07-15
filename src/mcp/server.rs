//! MCP Server 实现
//!
//! 使用 rmcp 1.x 实现 Skills 访问协议

use rmcp::{
    model::{
        CallToolRequestParams, CallToolResult, Content, GetPromptRequestParams, GetPromptResult,
        Implementation, ListPromptsResult, ListToolsResult, Prompt, PromptArgument, PromptMessage,
        PromptMessageRole, ServerCapabilities, ServerInfo, Tool,
    },
    service::{serve_server, RoleServer},
    transport::stdio,
    ServerHandler,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::api::jwt::{agent_context_from_identity, is_api_key_format, verify_token, AgentContext};
use crate::db::repositories::download_token::DownloadTokenRepository;
use crate::models::evaluation::{ErrorType, EvalTag};
use crate::models::skill::NewSkill;
use crate::services::admin::{ApiKeyService, IdentityService};
use crate::services::{
    EvaluatorService, OrgToolService, PermissionService, RegistryService, SandboxService,
    SearchService, SessionService, ToolRouterService,
};

#[allow(dead_code)]
pub struct McpServer {
    registry: RegistryService,
    search: SearchService,
    evaluator: EvaluatorService,
    session: SessionService,
    org_tool: OrgToolService,
    tool_router: ToolRouterService,
    sandbox: SandboxService,
    agent_context: Option<AgentContext>,
    api_key: ApiKeyService,
    identity: IdentityService,
    permission: PermissionService,
    download_token_repo: DownloadTokenRepository,
    /// CLI 二进制文件存放根目录：cli-dist/
    cli_dir: std::path::PathBuf,
}

impl McpServer {
    pub fn new(
        registry: RegistryService,
        search: SearchService,
        evaluator: EvaluatorService,
        session: SessionService,
        org_tool: OrgToolService,
        tool_router: ToolRouterService,
        sandbox: SandboxService,
        api_key: ApiKeyService,
        identity: IdentityService,
        permission: PermissionService,
        download_token_repo: DownloadTokenRepository,
        cli_dir: std::path::PathBuf,
    ) -> Self {
        // Try to extract and verify JWT/API key from environment (for stdio transport)
        let agent_context = Self::extract_auth_from_env(&api_key, &identity);
        Self {
            registry,
            search,
            evaluator,
            session,
            org_tool,
            tool_router,
            sandbox,
            agent_context,
            api_key,
            identity,
            permission,
            download_token_repo,
            cli_dir,
        }
    }

    /// Extract and verify JWT or API key from environment variable AION_HIVE_AUTH_TOKEN
    /// (for stdio transport). If the token starts with `sk_`, treat it as API key and
    /// validate. Otherwise treat as JWT.
    fn extract_auth_from_env(
        api_key: &ApiKeyService,
        identity: &IdentityService,
    ) -> Option<AgentContext> {
        let token = std::env::var("AION_HIVE_JWT_TOKEN")
            .or_else(|_| std::env::var("AION_HIVE_AUTH_TOKEN"))
            .ok()?;

        if is_api_key_format(&token) {
            Self::resolve_api_key_sync(api_key, identity, &token)
        } else {
            verify_token(&token).map(AgentContext::from_claims).ok()
        }
    }

    /// Synchronous wrapper for resolve_identity_from_api_key (used during construction).
    /// Session is not created here — only resolves the identity from env token.
    fn resolve_api_key_sync(
        api_key: &ApiKeyService,
        identity: &IdentityService,
        token: &str,
    ) -> Option<AgentContext> {
        let api_key_svc = api_key.clone();
        let identity = identity.clone();
        let token = token.to_string();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                Self::resolve_identity_from_api_key(&api_key_svc, &identity, &token)
                    .await
                    .ok()
                    .map(|(ctx, _org_id)| ctx)
            })
        })
    }

    /// Check if the agent is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.agent_context.is_some()
    }

    /// Get the agent context if authenticated
    pub fn get_agent_context(&self) -> Option<&AgentContext> {
        self.agent_context.as_ref()
    }

    /// Resolve API key → (AgentContext + org_id) — validate key + lookup identity.
    /// Returns (context, organization_id) for subsequent session creation.
    pub async fn resolve_identity_from_api_key(
        api_key: &ApiKeyService,
        identity: &IdentityService,
        api_key_raw: &str,
    ) -> Result<(AgentContext, Option<Uuid>), String> {
        // 1. Validate API key
        let key_record = api_key
            .validate(api_key_raw)
            .await
            .map_err(|e| {
                tracing::warn!("API key validation failed: {}", e);
                format!("API key validation error: {}", e)
            })?
            .ok_or_else(|| {
                tracing::warn!("API key not found or revoked");
                "Invalid or revoked API Key".to_string()
            })?;

        tracing::debug!(
            "API key validated: id={}, org_id={:?}, identity_id={}",
            key_record.id,
            key_record.organization_id,
            key_record.identity_id
        );

        // 2. Check if expired
        if let Some(ref expires_at) = key_record.expires_at {
            if expires_at < &chrono::Utc::now() {
                tracing::warn!("API key expired: expires_at={:?}", expires_at);
                return Err("API Key has expired".to_string());
            }
        }

        let identity_id = key_record.identity_id;
        let org_id = key_record.organization_id;

        // 3. Look up identity name
        let identity_record = identity
            .get(identity_id)
            .await
            .map_err(|e| {
                tracing::warn!("Identity lookup error for {}: {}", identity_id, e);
                format!("Identity lookup error: {}", e)
            })?
            .ok_or_else(|| {
                tracing::warn!("Identity not found: {}", identity_id);
                "Identity not found".to_string()
            })?;

        let identity_name = identity_record
            .display_name
            .unwrap_or_else(|| identity_record.name);

        // 4. Update API key last_used_at
        let _ = api_key.mark_used(key_record.id).await;

        // 5. Build AgentContext (no agent registration, no JWT, no session yet)
        Ok((
            agent_context_from_identity(
                identity_id,
                &identity_name,
                None,
                org_id,
                Some(key_record.id),
                Some(api_key_raw.to_string()),
            ),
            org_id,
        ))
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (stdin, stdout) = stdio();
        serve_server(self, (stdin, stdout)).await?;
        Ok(())
    }

    pub async fn run_sse(
        self,
        _tx: broadcast::Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (stdin, stdout) = stdio();
        serve_server(self, (stdin, stdout)).await?;
        Ok(())
    }

    /// Handle JSON-RPC request with optional per-request authentication.
    ///
    /// `auth_header` should be the value of the `Authorization` HTTP header (e.g. "Bearer eyJ...").
    /// If provided, the API key / JWT is verified and a session is auto-created (or reused)
    /// for the calling identity.
    /// If not provided or invalid, tools will return an auth error.
    pub async fn handle_jsonrpc(
        &self,
        body: &str,
        auth_header: Option<&str>,
    ) -> Result<String, String> {
        // Extract per-request agent context from Authorization header.
        // Supports two modes:
        //   - API key mode:  Bearer sk_xxx     → validate key + lookup identity + auto-create session
        //   - JWT mode:      Bearer eyJhbG...   → verify JWT (backward compat, no session)
        //   - No auth:       no header          → anonymous (tools will reject)
        let agent_ctx = if let Some(token) = auth_header.and_then(|h| h.strip_prefix("Bearer ")) {
            if is_api_key_format(token) {
                // Mask API key for safe logging (show only prefix + last 4 chars)
                let masked = if token.len() > 10 {
                    format!("{}...{}", &token[..6], &token[token.len() - 4..])
                } else {
                    "***".to_string()
                };
                match Self::resolve_identity_from_api_key(&self.api_key, &self.identity, token)
                    .await
                {
                    Ok((mut ctx, org_id)) => {
                        // Auto-create or reuse session for this identity
                        if let Some(identity_id) = ctx.identity_id {
                            if let Some(org_id) = org_id {
                                match self
                                    .session
                                    .find_or_create_session(identity_id, org_id)
                                    .await
                                {
                                    Ok(session) => {
                                        ctx.session_id = Some(session.id);
                                        // Touch last_active_at so idle cleanup doesn't end this session
                                        let _ = self.session.touch_session(session.id).await;
                                        tracing::debug!(
                                            "Session {} bound to identity {}",
                                            session.id,
                                            identity_id
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to create session for identity {}: {}",
                                            identity_id,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                        Some(ctx)
                    }
                    Err(e) => {
                        tracing::warn!(
                            "MCP auth failed for API key {} ({} chars): {}",
                            masked,
                            token.len(),
                            e
                        );
                        None
                    }
                }
            } else {
                let result = verify_token(token).map(AgentContext::from_claims).ok();
                if result.is_none() {
                    tracing::warn!("MCP auth failed: JWT verification failed");
                }
                result
            }
        } else {
            tracing::warn!("MCP auth failed: no Authorization header provided");
            None
        };

        let request: Value =
            serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;

        let method = request
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or("Missing method")?;

        // Handle notifications (no id) - just return empty success
        if !request.get("id").is_some() {
            match method {
                "notifications/initialized" => {
                    return Ok("{}".to_string());
                }
                _ => {
                    return Err(format!("Unknown notification: {}", method));
                }
            }
        }

        let id = request.get("id");

        let result = match method {
            "initialize" => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {},
                        "serverInfo": {
                            "name": "aion-hive",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }
                })
            }
            "tools/list" => {
                let tools = Self::tools();
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": tools.iter().map(|t| {
                            serde_json::json!({
                                "name": t.name,
                                "description": t.description,
                                "inputSchema": t.input_schema
                            })
                        }).collect::<Vec<_>>()
                    }
                })
            }
            "tools/call" => {
                let params = request.get("params");
                let tool_name = params
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                let args: std::collections::HashMap<String, Value> = params
                    .and_then(|p| p.get("arguments"))
                    .and_then(|a| a.as_object())
                    .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();

                let call_result = self
                    .call_tool_internal(tool_name, args, agent_ctx.as_ref())
                    .await;

                if call_result
                    .get("isError")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    let error_msg = call_result
                        .get("content")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.get("text"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Ok(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32603,
                            "message": error_msg
                        }
                    })
                    .to_string());
                }

                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": call_result
                })
            }
            _ => {
                return Err(format!("Unknown method: {}", method));
            }
        };

        Ok(serde_json::to_string(&result).unwrap_or_default())
    }

    /// Resolve the identity string used for author_agent_id in skill operations.
    ///
    /// Uses the caller's identity UUID (from API key), falling back to legacy JWT subject.
    fn resolve_agent_id(ctx: Option<&AgentContext>) -> String {
        match ctx {
            Some(c) => {
                // Preferred: identity UUID
                if let Some(ref id_id) = c.identity_id {
                    return id_id.to_string();
                }
                // Fallback: JWT subject (legacy)
                c.subject.clone()
            }
            None => "unknown".to_string(),
        }
    }

    /// Resolve the owner_id for NewSkill (identity UUID when available).
    fn resolve_owner_id(ctx: Option<&AgentContext>) -> Option<uuid::Uuid> {
        ctx.and_then(|c| c.identity_id)
    }

    async fn call_tool_internal(
        &self,
        name: &str,
        args: std::collections::HashMap<String, Value>,
        agent_ctx: Option<&AgentContext>,
    ) -> Value {
        let args = args
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();

        // All tools (except health_check) require API Key authentication
        if name != "health_check" && agent_ctx.is_none() {
            let err = Self::json_error(
                "Authentication required. Use Authorization: Bearer <api_key> (sk_xxx) header."
                    .to_string(),
            );
            return serde_json::to_value(&err)
                .unwrap_or(serde_json::json!({"error": "auth required"}));
        }

        let result = match name {
            "health_check" => Self::json_success(serde_json::json!({"status": "OK"})),

            "skills.search" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                });
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

                match self.search.search(&query, tags.as_deref(), limit) {
                    Ok(results) => {
                        let identity_id = agent_ctx.and_then(|c| c.identity_id);
                        let org_ids = self
                            .get_user_org_ids(identity_id)
                            .await
                            .unwrap_or_default();
                        let mut filtered = Vec::new();
                        for r in results {
                            if let Ok(skill) = self.registry.get_skill(&r.skill_id).await {
                                let meta: crate::models::skill::SkillMetadata = (&skill).into();
                                let visible = crate::services::RegistryService::filter_skills_visible_to(
                                    vec![meta],
                                    identity_id,
                                    &org_ids,
                                );
                                if !visible.is_empty() {
                                    filtered.push(r);
                                }
                            }
                        }
                        Self::json_success(serde_json::to_value(filtered).unwrap_or_default())
                    }
                    Err(e) => Self::json_error(format!("Search failed: {}", e)),
                }
            }

            "skills.list" => {
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as i64;
                let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as i64;
                let sort_by = args
                    .get("sort_by")
                    .and_then(|v| v.as_str())
                    .unwrap_or("created");

                match self
                    .registry
                    .list_skills_sorted(limit, offset, sort_by)
                    .await
                {
                    Ok(skills) => {
                        let identity_id = agent_ctx.and_then(|c| c.identity_id);
                        let org_ids = self
                            .get_user_org_ids(identity_id)
                            .await
                            .unwrap_or_default();
                        let filtered = crate::services::RegistryService::filter_skills_visible_to(
                            skills, identity_id, &org_ids,
                        );
                        let total = filtered.len();
                        Self::json_success(serde_json::json!({
                            "skills": filtered,
                            "total": total,
                            "limit": limit,
                            "offset": offset,
                        }))
                    }
                    Err(e) => Self::json_error(format!("List failed: {}", e)),
                }
            }

            "skills.info" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());

                match skill_id {
                    Some(id) => {
                        match self.registry.get_skill(id).await {
                            Ok(skill) => {
                                // 权限校验：与 HTTP get_skill_handler 一致
                                let identity_id = agent_ctx.and_then(|c| c.identity_id);
                                let org_ids = self
                                    .get_user_org_ids(identity_id)
                                    .await
                                    .unwrap_or_default();

                                let visible = if let Some(id_id) = identity_id {
                                    if skill.status == "published"
                                        && matches!(
                                            skill.visibility,
                                            crate::models::skill_policy::Visibility::Marketplace
                                        )
                                    {
                                        true
                                    } else if skill.owner_type == "user"
                                        && (skill.owner_id == Some(id_id)
                                            || skill.author_identity_id == Some(id_id))
                                    {
                                        true
                                    } else if skill.owner_type == "organization" {
                                        skill
                                            .owner_id
                                            .map_or(false, |oid| org_ids.contains(&oid))
                                    } else {
                                        false
                                    }
                                } else {
                                    skill.status == "published"
                                        && matches!(
                                            skill.visibility,
                                            crate::models::skill_policy::Visibility::Marketplace
                                        )
                                };

                                if visible {
                                    Self::json_success(
                                        serde_json::to_value(skill).unwrap_or_default(),
                                    )
                                } else {
                                    Self::json_error(format!(
                                        "Skill {} not found or access denied",
                                        id
                                    ))
                                }
                            }
                            Err(e) => Self::json_error(format!("Get skill failed: {}", e)),
                        }
                    }
                    None => Self::json_error("skill_id is required".to_string()),
                }
            }

            "skills.versions" => {
                let name = args.get("name").and_then(|v| v.as_str()).or_else(|| {
                    args.get("skill_id")
                        .and_then(|v| v.as_str())
                        .and_then(|id| {
                            // Extract name from skill-{name}-{version} format
                            id.strip_prefix("skill-")
                                .and_then(|s| s.rsplitn(2, '-').nth(1))
                        })
                });

                match name {
                    Some(n) => match self.registry.list_versions(n).await {
                        Ok(versions) => {
                            Self::json_success(serde_json::to_value(versions).unwrap_or_default())
                        }
                        Err(e) => Self::json_error(format!("List versions failed: {}", e)),
                    },
                    None => Self::json_error("name or skill_id is required".to_string()),
                }
            }

            "skills.popular" => {
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as i64;

                match self.registry.list_skills_sorted(limit, 0, "installs").await {
                    Ok(skills) => {
                        let identity_id = agent_ctx.and_then(|c| c.identity_id);
                        let org_ids = self
                            .get_user_org_ids(identity_id)
                            .await
                            .unwrap_or_default();
                        let filtered = crate::services::RegistryService::filter_skills_visible_to(
                            skills, identity_id, &org_ids,
                        );
                        Self::json_success(serde_json::to_value(filtered).unwrap_or_default())
                    }
                    Err(e) => Self::json_error(format!("List popular failed: {}", e)),
                }
            }

            "skills.create" => {
                let name = args.get("name").and_then(|v| v.as_str());
                let description = args.get("description").and_then(|v| v.as_str());
                let tags = args
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let content = args.get("content").and_then(|v| v.as_str());
                let version = args
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("1.0.0");
                let tools = args.get("tools").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                });

                match (name, description, content) {
                    (Some(name), Some(description), Some(content)) => {
                        let agent_id = Self::resolve_agent_id(agent_ctx);
                        let author_identity_id = Self::resolve_owner_id(agent_ctx);

                        // 从 API Key 获取调用者归属的组织（若有关联）
                        let caller_org_id = agent_ctx.and_then(|c| c.org_id);

                        // owner_type: 优先 args 显式指定，否则根据 caller_org_id 自动推断
                        let mcp_owner_type = args
                            .get("owner_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or_else(|| {
                                if caller_org_id.is_some() { "organization" } else { "user" }
                            });

                        let (owner_type, owner_id, default_visibility) = if mcp_owner_type == "organization" {
                            // 优先 args 指定的 org_id，其次使用调用者关联的 org
                            let org_id = args
                                .get("organization_id")
                                .and_then(|v| v.as_str())
                                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                .or(caller_org_id);
                            ("organization".to_string(), org_id, "org_visible")
                        } else {
                            ("user".to_string(), author_identity_id, "private")
                        };

                        let visibility = match args.get("visibility").and_then(|v| v.as_str()) {
                            Some("private") => crate::models::skill_policy::Visibility::Private,
                            Some("org_visible") => crate::models::skill_policy::Visibility::OrgVisible,
                            Some("marketplace") => crate::models::skill_policy::Visibility::Marketplace,
                            Some("shared") => crate::models::skill_policy::Visibility::Shared,
                            _ => match default_visibility {
                                "private" => crate::models::skill_policy::Visibility::Private,
                                "org_visible" => crate::models::skill_policy::Visibility::OrgVisible,
                                _ => crate::models::skill_policy::Visibility::Private,
                            },
                        };

                        let new_skill = NewSkill {
                            name: name.to_string(),
                            description: description.to_string(),
                            tags,
                            content: content.to_string(),
                            version: version.to_string(),
                            git_url: None,
                            visibility: Some(visibility),
                            tools,
                            owner_type,
                            owner_id,
                            author_identity_id,
                        };
                        match self
                            .registry
                            .create_skill(new_skill, &agent_id, &self.search)
                            .await
                        {
                            Ok(skill) => {
                                tracing::info!(
                                    "Skill created via MCP: name={} agent_id={}",
                                    skill.name,
                                    agent_id
                                );
                                Self::json_success(serde_json::to_value(skill).unwrap_or_default())
                            }
                            Err(e) => Self::json_error(format!("Create skill failed: {}", e)),
                        }
                    }
                    _ => {
                        Self::json_error("name, description, and content are required".to_string())
                    }
                }
            }

            "skills.update" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());

                let update = crate::models::skill::SkillUpdate {
                    description: args
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    tags: args.get("tags").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    }),
                    content: args
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    git_url: None,
                    visibility: None,
                    tools: args.get("tools").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    }),
                };

                match skill_id {
                    Some(id) => {
                        let agent_id = Self::resolve_agent_id(agent_ctx);
                        match self
                            .registry
                            .update_skill(id, update, &agent_id, &self.search)
                            .await
                        {
                            Ok(skill) => {
                                Self::json_success(serde_json::to_value(skill).unwrap_or_default())
                            }
                            Err(e) => Self::json_error(format!("Update skill failed: {}", e)),
                        }
                    }
                    None => Self::json_error("skill_id is required".to_string()),
                }
            }

            // Note: skills_delete is Admin-only via REST API, not MCP
            "skills.install" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());

                // Extract identity info from agent context for download audit
                let identity_id = agent_ctx.and_then(|c| c.identity_id);
                let api_key_id = agent_ctx.and_then(|c| c.api_key_id);

                match skill_id {
                    Some(id) => match (identity_id, api_key_id) {
                        (Some(identity), Some(api_key)) => {
                            match self.registry.get_skill_files(id, identity, api_key).await {
                                Ok(result) => {
                                    // 递增安装计数
                                    if let Err(e) = self.registry.increment_install_count(id).await
                                    {
                                        tracing::warn!(
                                            "Failed to increment install count for {}: {}",
                                            id,
                                            e
                                        );
                                    }
                                    Self::json_success(
                                        serde_json::to_value(result).unwrap_or_default(),
                                    )
                                }
                                Err(e) => Self::json_error(format!("Install failed: {}", e)),
                            }
                        }
                        _ => Self::json_error(
                            "Authentication required: valid API key with identity".to_string(),
                        ),
                    },
                    None => Self::json_error("skill_id is required".to_string()),
                }
            }

            "evaluate_skill" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());
                let agent_id = args.get("agent_id").and_then(|v| v.as_str());
                let success = args.get("success").and_then(|v| v.as_bool());
                let duration_ms = args.get("duration_ms").and_then(|v| v.as_u64());

                let error_type = args
                    .get("error_type")
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "timeout" => ErrorType::Timeout,
                        "crash" => ErrorType::Crash,
                        "logic_error" => ErrorType::LogicError,
                        _ => ErrorType::Other,
                    });

                let tags = args
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| {
                                v.as_str().and_then(|s| match s {
                                    "reliable" => Some(EvalTag::Reliable),
                                    "fast" => Some(EvalTag::Fast),
                                    "stable" => Some(EvalTag::Stable),
                                    "experimental" => Some(EvalTag::Experimental),
                                    _ => None,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                match (skill_id, agent_id, success, duration_ms) {
                    (Some(skill_id), Some(agent_id), Some(success), Some(duration_ms)) => {
                        match self
                            .evaluator
                            .add_evaluation(
                                skill_id.to_string(),
                                agent_id.to_string(),
                                success,
                                duration_ms,
                                error_type,
                                tags,
                            )
                            .await
                        {
                            Ok(result) => {
                                Self::json_success(serde_json::to_value(result).unwrap_or_default())
                            }
                            Err(e) => Self::json_error(format!("Evaluate skill failed: {}", e)),
                        }
                    }
                    _ => Self::json_error(
                        "skill_id, agent_id, success, and duration_ms are required".to_string(),
                    ),
                }
            }

            "skills.stats" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());

                match skill_id {
                    Some(id) => match self.evaluator.get_stats(id).await {
                        Ok(stats) => {
                            Self::json_success(serde_json::to_value(stats).unwrap_or_default())
                        }
                        Err(e) => Self::json_error(format!("Get stats failed: {}", e)),
                    },
                    None => Self::json_error("skill_id is required".to_string()),
                }
            }

            "session.info" => {
                // Use session_id from args, or fall back to auto-created session from auth context
                let lookup_id = args
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        agent_ctx
                            .and_then(|c| c.session_id)
                            .map(|id| id.to_string())
                    });

                match lookup_id {
                    Some(id) => {
                        let session_uuid = uuid::Uuid::parse_str(&id);
                        match session_uuid {
                            Ok(uuid) => match self.session.get_session(uuid).await {
                                Ok(Some(session)) => Self::json_success(serde_json::json!({
                                    "session_id": session.id.to_string(),
                                    "org_id": session.org_id.to_string(),
                                    "identity_id": session.identity_id.to_string(),
                                    "status": session.status,
                                    "created_at": session.created_at.to_rfc3339()
                                })),
                                Ok(None) => Self::json_error(format!("Session {} not found", id)),
                                Err(e) => Self::json_error(format!("Get session failed: {}", e)),
                            },
                            Err(_) => Self::json_error("Invalid session ID format".to_string()),
                        }
                    }
                    None => Self::json_error(
                        "session_id not provided and no active session".to_string(),
                    ),
                }
            }

            "session.declare" => {
                let lookup_id = args
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        agent_ctx
                            .and_then(|c| c.session_id)
                            .map(|id| id.to_string())
                    });

                let capabilities = args
                    .get("capabilities")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                match lookup_id {
                    Some(id) => {
                        let session_uuid = uuid::Uuid::parse_str(&id);
                        match session_uuid {
                            Ok(uuid) => {
                                match self.session.declare_capabilities(uuid, capabilities).await {
                                    Ok(router) => {
                                        let browse = router
                                            .routes
                                            .get("browse")
                                            .map(|t| match t {
                                                crate::models::session::RouteTarget::Local => {
                                                    "local"
                                                }
                                                crate::models::session::RouteTarget::Platform => {
                                                    "platform"
                                                }
                                                crate::models::session::RouteTarget::OrgTool(s) => {
                                                    s.as_str()
                                                }
                                            })
                                            .unwrap_or("platform");
                                        let qa = router
                                            .routes
                                            .get("qa")
                                            .map(|t| match t {
                                                crate::models::session::RouteTarget::Local => {
                                                    "local"
                                                }
                                                crate::models::session::RouteTarget::Platform => {
                                                    "platform"
                                                }
                                                crate::models::session::RouteTarget::OrgTool(o) => {
                                                    o.as_str()
                                                }
                                            })
                                            .unwrap_or("platform");
                                        let tool_router_json = serde_json::json!({
                                            "browse": browse,
                                            "qa": qa
                                        });
                                        Self::json_success(tool_router_json)
                                    }
                                    Err(e) => Self::json_error(format!(
                                        "Declare capabilities failed: {}",
                                        e
                                    )),
                                }
                            }
                            Err(_) => Self::json_error("Invalid session ID format".to_string()),
                        }
                    }
                    None => {
                        Self::json_error("session_id and capabilities are required".to_string())
                    }
                }
            }

            "tools.list" => {
                let org_id = agent_ctx.and_then(|c| c.org_id);
                match org_id {
                    Some(oid) => match self.org_tool.list_approved_tools(oid).await {
                        Ok(tools) => {
                            let tools_json: Vec<serde_json::Value> = tools
                                .iter()
                                .map(|t| {
                                    serde_json::json!({
                                        "tool_id": t.tool_id,
                                        "name": t.name,
                                        "description": t.description,
                                        "schema": t.schema,
                                        "status": t.status,
                                    })
                                })
                                .collect();
                            Self::json_success(serde_json::json!({
                                "org_id": oid.to_string(),
                                "tools": tools_json,
                                "total": tools_json.len()
                            }))
                        }
                        Err(e) => Self::json_error(format!("List tools failed: {}", e)),
                    },
                    None => Self::json_error(
                        "org_id not available — tools.list requires API key authentication"
                            .to_string(),
                    ),
                }
            }

            "tools.execute" => {
                let tool_id = args.get("tool_id").and_then(|v| v.as_str());
                // org_id from args first, then fall back to auth context
                let org_id_from_auth = agent_ctx.and_then(|c| c.org_id).map(|id| id.to_string());
                let org_id = args
                    .get("org_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or(org_id_from_auth);
                let parameters = args
                    .get("parameters")
                    .and_then(|v| v.as_object())
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();

                match (tool_id, org_id.as_deref()) {
                    (Some(tid), Some(oid)) => {
                        // Ensure the org tool exists and is approved before execution
                        let tool_result = match Uuid::parse_str(oid) {
                            Ok(org_uuid) => {
                                match self.org_tool.get_tool_by_tool_id(org_uuid, tid).await {
                                    Ok(Some(tool)) if tool.status == "approved" => Ok(tool),
                                    Ok(Some(_)) => {
                                        Err("Tool must be approved before execution".to_string())
                                    }
                                    Ok(None) => Err(format!(
                                        "Tool {} not found in organization {}",
                                        tid, oid
                                    )),
                                    Err(e) => Err(format!("Failed to verify tool status: {}", e)),
                                }
                            }
                            Err(_) => Err("Invalid org_id".to_string()),
                        };

                        match tool_result {
                            Ok(tool) => {
                                // Read defaults from stored implementation config
                                let impl_docker = tool
                                    .implementation
                                    .get("docker_image")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                                let impl_timeout = tool
                                    .implementation
                                    .get("timeout_seconds")
                                    .and_then(|v| v.as_u64());
                                let impl_cmd = tool
                                    .implementation
                                    .get("cmd")
                                    .and_then(|v| v.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect::<Vec<String>>()
                                    });

                                let request = crate::services::ToolExecutionRequest {
                                    tool_id: tid.to_string(),
                                    org_id: oid.to_string(),
                                    parameters,
                                    timeout_seconds: impl_timeout.unwrap_or(30),
                                    docker_image: impl_docker,
                                    session_id: None,
                                    cmd: impl_cmd,
                                };
                                match self.sandbox.execute_org_tool(request).await {
                                    Ok(result) => {
                                        let response = serde_json::json!({
                                            "success": result.success,
                                            "output": result.output,
                                            "error": result.error,
                                            "execution_time_ms": result.execution_time_ms
                                        });
                                        Self::json_success(response)
                                    }
                                    Err(e) => {
                                        Self::json_error(format!("Tool execution failed: {}", e))
                                    }
                                }
                            }
                            Err(msg) => Self::json_error(msg),
                        }
                    }
                    _ => Self::json_error("tool_id and org_id are required".to_string()),
                }
            }

            "tools.platform.execute" => {
                let tool_name = args.get("tool_name").and_then(|v| v.as_str());
                let parameters = args
                    .get("parameters")
                    .and_then(|v| v.as_object())
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();
                let timeout_seconds = args.get("timeout_seconds").and_then(|v| v.as_u64());

                match tool_name {
                    Some(name) => {
                        match self
                            .sandbox
                            .execute_platform_tool(name, parameters, timeout_seconds)
                            .await
                        {
                            Ok(result) => {
                                let response = serde_json::json!({
                                    "success": result.success,
                                    "output": result.output,
                                    "error": result.error,
                                    "execution_time_ms": result.execution_time_ms
                                });
                                Self::json_success(response)
                            }
                            Err(e) => {
                                Self::json_error(format!("Platform tool execution failed: {}", e))
                            }
                        }
                    }
                    None => Self::json_error("tool_name is required".to_string()),
                }
            }

            "cli.setup" => {
                let platform = args
                    .get("platform")
                    .and_then(|v| v.as_str())
                    .unwrap_or("auto");
                let arch = args
                    .get("arch")
                    .and_then(|v| v.as_str())
                    .unwrap_or("x86_64");

                let identity_id = agent_ctx.and_then(|c| c.identity_id);
                let api_key_id = agent_ctx.and_then(|c| c.api_key_id);
                // 优先从 HTTP 请求上下文获取原始 API key（HTTP/SSE 模式），
                // 其次从环境变量获取（stdio 模式），最后回退到占位符
                let api_key_token = agent_ctx
                    .and_then(|c| c.raw_api_key.clone())
                    .or_else(|| {
                        std::env::var("AION_HIVE_JWT_TOKEN")
                            .or_else(|_| std::env::var("AION_HIVE_AUTH_TOKEN"))
                            .ok()
                    });

                match (identity_id, api_key_id) {
                    (Some(identity), Some(api_key)) => {
                        let version = env!("CARGO_PKG_VERSION");
                        let target = Self::resolve_cli_target(platform, arch);
                        let (os_label, arch_label) = Self::os_arch_labels(platform, arch);
                        let base_url = Self::build_server_url();
                        let is_windows = target.starts_with("windows");

                        let filename = if is_windows {
                            "skill-garden.exe"
                        } else {
                            "skill-garden"
                        };

                        let token_str = api_key_token.as_deref().unwrap_or("sk_<YOUR_API_KEY>");

                        // 生成 config.toml（含真实 API Key，写入 token 的 config_data，下载时嵌入 tar.gz）
                        let config_toml = format!(
                            r#"# Skill Garden CLI config
# Generated by cli.setup

server = "{base_url}"
token = "{token}"

# Optional: Skill 安装目录（skill-garden install 默认下载到此目录）
# 设置方法: skill-garden config set skills_dir <路径>
# skills_dir = "~/.agent/skills"
"#,
                            base_url = base_url,
                            token = token_str,
                        );

                        let verify_cmd = if is_windows {
                            "skill-garden.exe whoami"
                        } else {
                            "skill-garden whoami"
                        };

                        // 检查二进制文件是否存在
                        let bin_path = self
                            .cli_dir
                            .join(&version)
                            .join(&target)
                            .join(filename);

                        if bin_path.exists() {
                            // 创建下载 token，config.toml 内容存入 DB，下载时嵌入 tar.gz
                            match self
                                .download_token_repo
                                .create_cli_token(
                                    &version,
                                    &target,
                                    identity,
                                    api_key,
                                    300,
                                    Some(config_toml),
                                )
                                .await
                            {
                                Ok(token_record) => {
                                    let download_url = Self::build_cli_download_url(
                                        &version, &target, &token_record.token,
                                    );

                                    let install_step = if is_windows {
                                        "cd skill-garden-cli && .\\install.ps1"
                                    } else {
                                        "cd skill-garden-cli && chmod +x install.sh && ./install.sh"
                                    };

                                    let instructions = include_str!("../../cli-dist/instructions.md")
                                        .replace("{version}", version)
                                        .replace("{os}", os_label)
                                        .replace("{arch}", arch_label)
                                        .replace("{url}", &download_url)
                                        .replace("{install}", install_step)
                                        .replace("{verify}", verify_cmd)
                                        .replace("{filename}", filename);

                                    let result = crate::models::skill::CliSetupResult {
                                        success: true,
                                        version: version.to_string(),
                                        target: target.clone(),
                                        download_url: Some(download_url),
                                        expires_in: 300,
                                        instructions,
                                    };

                                    Self::json_success(
                                        serde_json::to_value(result).unwrap_or_default(),
                                    )
                                }
                                Err(e) => Self::json_error(format!(
                                    "Failed to generate download token: {}", e
                                )),
                            }
                        } else {
                            let result = crate::models::skill::CliSetupResult {
                                success: false,
                                version: version.to_string(),
                                target: target.clone(),
                                download_url: None,
                                expires_in: 0,
                                instructions: format!(
                                    "CLI binary v{}/{} not available. Build and place it at cli-dist/{}/{}/{}",
                                    version, target, version, target, filename
                                ),
                            };
                            Self::json_success(
                                serde_json::to_value(result).unwrap_or_default(),
                            )
                        }
                    }
                    _ => Self::json_error(
                        "Authentication required: valid API key with identity".to_string(),
                    ),
                }
            }

            _ => Self::json_error(format!("Unknown tool: {}", name)),
        };

        let call_result = result;
        serde_json::to_value(&call_result)
            .unwrap_or(serde_json::json!({"error": "serialization failed"}))
    }

    /// 获取用户所属的所有组织 ID 列表
    /// 与 `filter_skills_visible_to` 保持一致的多组织支持
    async fn get_user_org_ids(&self, identity_id: Option<Uuid>) -> Result<Vec<Uuid>, String> {
        let Some(id_id) = identity_id else {
            return Ok(vec![]);
        };
        self.permission
            .get_user_org_ids(id_id)
            .await
            .map_err(|e| format!("Failed to get org memberships: {}", e))
    }

    /// 归一化平台名 → 目录名（linux / macos / windows）
    /// 兼容 agent 可能传入的各种变体：darwin, mac, win32, win 等
    fn normalize_platform(input: &str) -> &'static str {
        match input.to_lowercase().as_str() {
            "linux" => "linux",
            "macos" | "darwin" | "mac" | "osx" => "macos",
            "windows" | "win32" | "win" | "win64" => "windows",
            "auto" | "" => {
                if cfg!(target_os = "windows") {
                    "windows"
                } else if cfg!(target_os = "macos") {
                    "macos"
                } else {
                    "linux"
                }
            }
            // 未知平台 → auto-detect 兜底
            _ => {
                if cfg!(target_os = "windows") {
                    "windows"
                } else if cfg!(target_os = "macos") {
                    "macos"
                } else {
                    "linux"
                }
            }
        }
    }

    /// 归一化架构名 → 目录后缀（x86_64 / aarch64）
    /// 兼容 agent 可能传入的各种变体：amd64, x64, arm64, armv8 等
    fn normalize_arch(input: &str) -> &'static str {
        match input.to_lowercase().as_str() {
            "x86_64" | "amd64" | "x64" | "x86-64" => "x86_64",
            "aarch64" | "arm64" | "armv8" | "armv8-a" => "aarch64",
            // 未知架构 → x86_64 兜底
            _ => "x86_64",
        }
    }

    /// 解析 CLI target 目录名：{os}-{arch}
    /// 输入经过归一化处理，兼容各种常见变体
    fn resolve_cli_target(platform: &str, arch: &str) -> String {
        let os = Self::normalize_platform(platform);
        let arch = Self::normalize_arch(arch);
        format!("{}-{}", os, arch)
    }

    /// 返回 (os_label, arch_label) 用于展示
    fn os_arch_labels(platform: &str, arch: &str) -> (&'static str, &'static str) {
        let os = match Self::normalize_platform(platform) {
            "linux" => "Linux",
            "macos" => "macOS",
            _ => "Windows",
        };
        let arch_label = match Self::normalize_arch(arch) {
            "aarch64" => "ARM64",
            _ => "x86_64",
        };
        (os, arch_label)
    }

    /// 构建 CLI 下载 URL（token 保护）
    fn build_cli_download_url(version: &str, target: &str, token: &str) -> String {
        let base = std::env::var("AION_HIVE_PUBLIC_URL")
            .unwrap_or_else(|_| {
                format!(
                    "http://localhost:{}",
                    std::env::var("AION_HIVE_HTTP_PORT").unwrap_or_else(|_| "8080".to_string())
                )
            })
            .trim_end_matches('/')
            .to_string();

        format!(
            "{}/api/v1/cli/download/{}/{}?token={}",
            base, version, target, token
        )
    }

    /// 构建当前服务器的对外 URL
    fn build_server_url() -> String {
        std::env::var("AION_HIVE_PUBLIC_URL").unwrap_or_else(|_| {
            format!(
                "http://localhost:{}",
                std::env::var("AION_HIVE_HTTP_PORT").unwrap_or_else(|_| "8080".to_string())
            )
        })
    }

    fn tools() -> Vec<Tool> {
        vec![
            Tool::new(
                "health_check",
                "Check if the MCP server is healthy",
                Arc::new(serde_json::json!({"type": "object", "properties": {}}).as_object().unwrap().clone()),
            ),
            Tool::new(
                "skills.search",
                "Search skills by query and optional tags",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "tags": {"type": "array", "items": {"type": "string"}, "description": "Optional tags to filter by"},
                        "limit": {"type": "number", "description": "Maximum results (default 10)"}
                    }
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "skills.list",
                "List available skills with pagination and sorting",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {"type": "number", "description": "Maximum results (default 100)"},
                        "offset": {"type": "number", "description": "Pagination offset (default 0)"},
                        "sort_by": {"type": "string", "enum": ["created", "installs", "name", "updated"], "description": "Sort field (default: created)"}
                    }
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "skills.info",
                "Get detailed information about a skill",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "skill_id": {"type": "string", "description": "Skill ID (format: skill-{name}-{version})"}
                    },
                    "required": ["skill_id"]
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "skills.create",
                "Create a new skill",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Skill name"},
                        "description": {"type": "string", "description": "Skill description"},
                        "tags": {"type": "array", "items": {"type": "string"}, "description": "Skill tags"},
                        "tools": {"type": "array", "items": {"type": "string"}, "description": "Tool names to include"},
                        "content": {"type": "string", "description": "SKILL.md content"},
                        "version": {"type": "string", "description": "Version (default 1.0.0)"}
                    },
                    "required": ["name", "description", "content"]
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "skills.update",
                "Update an existing skill",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "skill_id": {"type": "string", "description": "Skill ID"},
                        "description": {"type": "string", "description": "New description"},
                        "tags": {"type": "array", "items": {"type": "string"}, "description": "New tags"},
                        "content": {"type": "string", "description": "New content"}
                    },
                    "required": ["skill_id"]
                }).as_object().unwrap().clone()),
            ),
            // Note: skills_delete is Admin-only via REST API, not MCP
            Tool::new(
                "skills.popular",
                "List most popular skills sorted by install count",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {"type": "number", "description": "Maximum results (default 20)"}
                    }
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "skills.versions",
                "List all versions of a skill by name",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Skill name (without version)"},
                        "skill_id": {"type": "string", "description": "Skill ID - name will be extracted from it"}
                    },
                    "required": []
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "skills.install",
                "安装 Skill。返回元数据 + 签名下载链接（tar.gz），内含 skill 目录。Agent 需将 tar.gz 解压后得到的目录完整移动到你的 Skills 管理目录中。下载链接 300 秒内有效。返回值含文件数和包大小，便于进度追踪。",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "skill_id": {"type": "string", "description": "Skill ID (format: skill-{name}-{version})"}
                    },
                    "required": ["skill_id"]
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "evaluate_skill",
                "Submit an evaluation for a skill",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "skill_id": {"type": "string", "description": "Skill ID"},
                        "agent_id": {"type": "string", "description": "Agent ID"},
                        "success": {"type": "boolean", "description": "Whether the skill execution was successful"},
                        "duration_ms": {"type": "number", "description": "Execution duration in milliseconds"},
                        "error_type": {"type": "string", "enum": ["timeout", "crash", "logic_error", "other"], "description": "Error type if failed"},
                        "tags": {"type": "array", "items": {"type": "string"}, "description": "Evaluation tags (reliable, fast, stable, experimental)"}
                    },
                    "required": ["skill_id", "agent_id", "success", "duration_ms"]
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "skills.stats",
                "Get statistics (success rate, avg duration, etc) for a skill",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "skill_id": {"type": "string", "description": "Skill ID"}
                    },
                    "required": ["skill_id"]
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "session.info",
                "Get current session info. session_id is optional — falls back to the auth context session.",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "Session ID"}
                    },
                    "required": ["session_id"]
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "session.declare",
                "Declare agent capabilities for the session. session_id is optional — falls back to the auth context session.",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "Session ID"},
                        "capabilities": {"type": "array", "items": {"type": "string"}, "description": "Agent capabilities"}
                    },
                    "required": ["session_id", "capabilities"]
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "tools.list",
                "List approved organization tools available for the current org (from API key scope)",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {}
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "tools.execute",
                "Execute an organization tool in sandboxed environment. org_id is optional — falls back to API key's org.",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tool_id": {"type": "string", "description": "Tool ID to execute"},
                        "org_id": {"type": "string", "description": "Organization ID (optional, falls back to API key org)"},
                        "parameters": {"type": "object", "description": "Tool parameters as key-value pairs"}
                    },
                    "required": ["tool_id"]
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "tools.platform.execute",
                "Execute a platform built-in tool (browse, qa, exec, storage) in sandboxed environment",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tool_name": {"type": "string", "enum": ["browse", "qa", "exec", "storage"], "description": "Platform tool name"},
                        "parameters": {"type": "object", "description": "Tool parameters as key-value pairs"},
                        "timeout_seconds": {"type": "number", "description": "Optional timeout in seconds"}
                    },
                    "required": ["tool_name"]
                }).as_object().unwrap().clone()),
            ),
            Tool::new(
                "cli.setup",
                "安装 Skill Garden CLI。返回 tar.gz 下载链接，解压后得到两个顶层目录：\n1. skill-garden-cli/ — 内含 skill-garden 二进制、预填 API Key 的 config.toml、install.sh、install.ps1\n2. skill-garden/ — 独立目录，内含 SKILL.md，作为本地 Skill 定义\n\n操作流程：\n1. cli.setup(platform, arch) → 获取 download_url\n2. 下载 tar.gz 并解压\n3. 运行 skill-garden-cli/install.sh（Linux/macOS）或 install.ps1（Windows）完成二进制安装和 PATH 配置\n4. 将 skill-garden-cli/config.toml 写入 ~/.skill-garden/config.toml\n5. 将解压出的 skill-garden/ 目录整个移动到你的 Skills 管理目录中，作为本地 Skill 被发现和加载\n6. 执行 skill-garden config set skills_dir <你的Skills目录路径>，此后 skill-garden install 会自动安装到该目录\n\nCLI 支持：search、list、info、install、versions、popular、stats、whoami、config。",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "platform": {"type": "string", "enum": ["auto", "linux", "macos", "darwin", "windows"], "description": "Target OS (default: auto-detect). Aliases: darwin→macos"},
                        "arch": {"type": "string", "enum": ["x86_64", "amd64", "aarch64", "arm64"], "description": "Target architecture (default: x86_64). Aliases: amd64→x86_64, arm64→aarch64"},
                    }
                }).as_object().unwrap().clone()),
            ),
        ]
    }

    fn json_error(msg: String) -> CallToolResult {
        CallToolResult::error(vec![Content::text(format!("{{\"error\": \"{}\"}}", msg))])
    }

    fn json_success(data: serde_json::Value) -> CallToolResult {
        CallToolResult::success(vec![Content::text(data.to_string())])
    }

    fn prompts() -> Vec<Prompt> {
        vec![
            Prompt::new(
                "discover-skill",
                Some("Skill 发现 — 搜索并选择最适合当前任务的 Skill"),
                Some(vec![
                    PromptArgument::new("query")
                        .with_description("描述你的需求（如：PDF 文件处理、图片识别），用于 search 查询")
                        .with_required(true),
                    PromptArgument::new("tags")
                        .with_description("可选的标签过滤，逗号分隔（如：文档,图像）")
                        .with_required(false),
                ]),
            ),
            Prompt::new(
                "create-skill",
                Some("Skill 创建 — 引导 Agent 创建标准规范的 SKILL.md 并上传"),
                Some(vec![
                    PromptArgument::new("skill_name")
                        .with_description("Skill 名称，须符合 kebab-case 命名规范")
                        .with_required(true),
                    PromptArgument::new("owner_type")
                        .with_description("归属类型：user（个人）或 organization（组织），留空则根据 API Key 自动推断")
                        .with_required(false),
                ]),
            ),
            Prompt::new(
                "install-skill",
                Some("Skill 安装 — 将 Skill 下载并部署到 Agent 的 Skills 目录"),
                Some(vec![
                    PromptArgument::new("skill_id")
                        .with_description("目标 Skill 的 ID（通过 search 或 info 获取）")
                        .with_required(true),
                ]),
            ),
            Prompt::new(
                "evaluate-skill-reliability",
                Some("Skill 可靠性评估 — 获取 Skill 的加权统计并解读信度"),
                Some(vec![
                    PromptArgument::new("skill_id")
                        .with_description("需要评估的 Skill ID")
                        .with_required(true),
                ]),
            ),
            Prompt::new(
                "generate-skill-template",
                Some("Skill 模板生成 — 输出可直接填充的 SKILL.md 模板"),
                Some(vec![
                    PromptArgument::new("skill_name")
                        .with_description("Skill 名称，须符合 kebab-case 命名规范")
                        .with_required(true),
                ]),
            ),
        ]
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder().enable_prompts().build();
        rmcp::model::InitializeResult::new(capabilities)
            .with_server_info(Implementation::new("aion-hive", env!("CARGO_PKG_VERSION")))
            .with_instructions("Enterprise Skills Sharing Platform for AI Agents")
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        Ok(ListToolsResult {
            tools: Self::tools(),
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let name = request.name.as_ref();
        let args: std::collections::HashMap<String, Value> = request
            .arguments
            .map(|m| m.into_iter().collect())
            .unwrap_or_default();

        // For stdio transport: use the env-var JWT as agent context (legacy fallback).
        // For HTTP transport: auth is extracted per-request in handle_jsonrpc.
        let agent_ctx = self.agent_context.as_ref();

        let value = self.call_tool_internal(name, args, agent_ctx).await;

        // Convert Value to CallToolResult
        if let Some(content_arr) = value.get("content").and_then(|v| v.as_array()) {
            let content = content_arr
                .iter()
                .map(|item| {
                    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                        Content::text(text.to_string())
                    } else {
                        Content::text(item.to_string())
                    }
                })
                .collect::<Vec<_>>();
            if value
                .get("isError")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                Ok(CallToolResult::error(content))
            } else {
                Ok(CallToolResult::success(content))
            }
        } else {
            Ok(CallToolResult::success(vec![Content::text(
                value.to_string(),
            )]))
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, rmcp::ErrorData> {
        Ok(ListPromptsResult::with_all_items(Self::prompts()))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, rmcp::ErrorData> {
        let args = request.arguments.unwrap_or_default();

        let messages = match request.name.as_str() {
            "discover-skill" => {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let tags = args.get("tags").and_then(|v| v.as_str()).unwrap_or("");

                let tags_hint = if tags.is_empty() {
                    String::new()
                } else {
                    format!("\n- 标签过滤: {}", tags)
                };

                vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "## Skill 发现工作流\n\n\
                        目标：找到最适合下面需求的 Skill。\n\n\
                        需求描述：{}{}\n\n\
                        ### 步骤\n\n\
                        1. 调用 `skills.search` 工具，传入 query 和 tags 参数\n\
                        2. 查看搜索结果，对候选 Skill 调用 `skills.info` 获取详细信息\n\
                        3. 对候选 Skill 调用 `skills.stats` 查看质量指标：\n\
                           - success_rate（成功率，越高越好）\n\
                           - confidence（置信度，越高越可靠）\n\
                           - total_evaluations（评估次数，越多越可信）\n\
                           - avg_duration_ms（平均耗时）\n\
                        4. 综合 popularity（`skills.popular`）和质量指标，选出最佳候选项\n\
                        5. 告知用户推荐结果，说明推荐理由",
                        query, tags_hint
                    ),
                ),
                PromptMessage::new_text(
                    PromptMessageRole::Assistant,
                    "我会帮你搜索并评估最合适的 Skill。现在开始执行发现流程。",
                )]
            }

            "create-skill" => {
                let skill_name = args.get("skill_name").and_then(|v| v.as_str()).unwrap_or("my-skill");
                let owner_type = args.get("owner_type").and_then(|v| v.as_str()).unwrap_or("auto");

                vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "## Skill 创建工作流\n\n\
                        目标：创建一个名为 `{}` 的新 Skill（归属类型：{}）。\n\n\
                        ### SKILL.md 文件格式\n\n\
                        ```yaml\n\
                        ---\n\
                        name: {}\n\
                        version: 1.0.0\n\
                        description: |\n\
                          （在此处描述 Skill 的功能和用途）\n\
                        tags:\n\
                          - 分类标签1\n\
                          - 分类标签2\n\
                        author: <你的 Agent ID>\n\
                        compatibility: \"1.0\"\n\
                        dependencies: []\n\
                        tools: []\n\
                        ---\n\n\
                        # {} — 使用说明\n\n\
                        ## 功能\n\
                        （详细功能描述）\n\n\
                        ## 用法\n\
                        （使用示例和参数说明）\n\n\
                        ## 限制\n\
                        （已知限制和注意事项）\n\
                        ```\n\n\
                        ### 步骤\n\n\
                        1. 根据上述模板编写 SKILL.md 的 YAML 头部和 Markdown 正文\n\
                        2. 调用 `skills.create` 工具，传入 name、description（简短描述）、content（完整 SKILL.md 内容）、version、tags、tools 等参数\n\
                           - 如果当前用户属于某个组织，Skill 将自动归属到该组织并设为 org_visible\n\
                           - 如果是个人用户，Skill 将为个人所有并设为 private\n\
                        3. 创建成功后，Skill 进入 draft 状态，可提交审核（submit-review）",
                        skill_name, owner_type, skill_name, skill_name
                    ),
                ),
                PromptMessage::new_text(
                    PromptMessageRole::Assistant,
                    format!("我已了解 SKILL.md 格式。现在帮你创建名为 `{}` 的 Skill。", skill_name),
                )]
            }

            "install-skill" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str()).unwrap_or("");

                vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "## Skill 安装工作流\n\n\
                        目标：将 `{}` 安装到 Agent 的 Skills 目录中。\n\n\
                        ### 步骤\n\n\
                        1. 调用 `skills.install` 工具，传入 skill_id = \"{}\"\n\
                        2. 从返回结果中获取 download_url 和文件信息（file_count、tarball_size）\n\
                        3. 下载 tar.gz 压缩包\n\
                        4. 解压后得到一个以 Skill 名称命名的目录\n\
                        5. 将该目录**整体移动**到你的 Skills 管理目录中（与 skill-garden/ SKILL.md 同级）\n\
                        6. 确认目录结构正确：Skills 目录下有该 Skill 的 SKILL.md 及其他资源文件\n\n\
                        ### 注意\n\
                        - 下载链接 300 秒内有效，尽快下载\n\
                        - 解压后的目录结构直接就是 Skill 内容，无需额外嵌套",
                        skill_id, skill_id
                    ),
                ),
                PromptMessage::new_text(
                    PromptMessageRole::Assistant,
                    format!("开始安装 Skill: {}。正在获取下载链接…", skill_id),
                )]
            }

            "evaluate-skill-reliability" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str()).unwrap_or("");

                vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "## Skill 可靠性评估工作流\n\n\
                        目标：评估 Skill `{}` 的可靠性并给出量化结论。\n\n\
                        ### 步骤\n\n\
                        1. 调用 `skills.info` 工具获取 Skill 基本信息（name、version、install_count 等）\n\
                        2. 调用 `skills.stats` 工具获取加权统计指标\n\
                        3. 解读关键指标（权重从高到低）：\n\n\
                           | 指标 | 含义 | 优秀阈值 |\n\
                           |---|---|---|\n\
                           | success_rate | 加权成功率（0-1） | ≥ 0.90 |\n\
                           | confidence | 置信度，评估数据越多越接近真实值 | ≥ 0.80 |\n\
                           | total_evaluations | 总评估次数 | ≥ 10 |\n\
                           | unique_agents | 独立 Agent 数 | ≥ 3 |\n\
                           | avg_duration_ms | 平均执行耗时 | 越低越好 |\n\n\
                        4. 综合打分：\n\
                           - 高可靠：success_rate ≥ 0.95 && confidence ≥ 0.85 && total_evaluations ≥ 20\n\
                           - 中等可靠：success_rate ≥ 0.85 && confidence ≥ 0.70 && total_evaluations ≥ 10\n\
                           - 谨慎使用：不满足以上条件\n\
                           - 数据不足：total_evaluations < 5（样本量太小）\n\n\
                        5. 输出评估结论并给出使用建议",
                        skill_id
                    ),
                ),
                PromptMessage::new_text(
                    PromptMessageRole::Assistant,
                    format!("开始评估 Skill: {} 的可靠性，正在获取数据和统计…", skill_id),
                )]
            }

            "generate-skill-template" => {
                let skill_name = args.get("skill_name").and_then(|v| v.as_str()).unwrap_or("my-skill");

                let template = format!(
                    "---\n\
                    name: {}\n\
                    version: 1.0.0\n\
                    description: |\n\
                      （在此处用 1-3 句话描述 Skill 的功能和使用场景）\n\
                    tags:\n\
                      - 示例标签\n\
                    author: <Agent ID>\n\
                    compatibility: \"1.0\"\n\
                    dependencies: []\n\
                    tools: []\n\
                    ---\n\n\
                    # {} — 使用说明\n\n\
                    ## 功能概述\n\n\
                    （描述这个 Skill 解决什么问题）\n\n\
                    ## 使用方法\n\n\
                    （描述 Agent 应如何调用和利用这个 Skill）\n\n\
                    ### 前置条件\n\n\
                    - （列出需要的依赖、环境或权限）\n\n\
                    ### 输入\n\n\
                    - （参数名）：（描述）\n\n\
                    ### 输出\n\n\
                    - （输出格式说明）\n\n\
                    ## 示例\n\n\
                    ```\n\
                    （提供 1-2 个实际使用示例）\n\
                    ```\n\n\
                    ## 注意事项\n\n\
                    - （已知限制和边界条件）\n\n\
                    ## 参考\n\n\
                    - （相关的文档链接或参考资料）",
                    skill_name, skill_name
                );

                vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "## Skill 模板生成\n\n\
                        以下是 `{}` 的 SKILL.md 标准模板，请根据实际需求填充内容后，调用 `skills.create` 工具创建。\n\n\
                        ### 填充指南\n\n\
                        - `name`：必须使用 kebab-case 格式，全小写英文，用连字符分隔\n\
                        - `description`：前端 YAML 的 description 是简短摘要（1-2 行），正文中可以展开详述\n\
                        - `tags`：选择 2-5 个分类标签，建议从已有热门 Skill 的标签中参考\n\
                        - `tools`：如果 Skill 依赖特定工具，在此列出工具名称\n\
                        - 正文部分用清晰的标题和段落，方便 Agent 解析和行为决策\n\n\
                        ```yaml\n\
                        {}\n\
                        ```\n\n\
                        ### 后续步骤\n\n\
                        1. 补充模板中的占位内容\n\
                        2. 调用 `skills.create` 工具提交\n\
                        3. Skill 创建后默认为 draft 状态，需要时可调用 submit-review 提交审核",
                        skill_name, template
                    ),
                ),
                PromptMessage::new_text(
                    PromptMessageRole::Assistant,
                    format!("已为你生成 `{}` 的 SKILL.md 模板。请逐一填充占位内容后创建。", skill_name),
                )]
            }

            _ => {
                return Err(rmcp::ErrorData::invalid_request(
                    format!(
                        "Unknown prompt: {}. Available prompts: discover-skill, create-skill, install-skill, evaluate-skill-reliability, generate-skill-template",
                        request.name
                    ),
                    None,
                ))
            }
        };

        Ok(GetPromptResult::new(messages))
    }
}
