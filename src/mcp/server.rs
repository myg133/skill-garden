//! MCP Server 实现
//!
//! 使用 rmcp 1.x 实现 Skills 访问协议

use rmcp::{
    model::{
        CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
        ServerCapabilities, ServerInfo, Tool,
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
use crate::models::evaluation::{ErrorType, EvalTag};
use crate::models::skill::NewSkill;
use crate::services::admin::{ApiKeyService, IdentityService};
use crate::services::{
    EvaluatorService, OrgToolService, RegistryService, SandboxService, SearchService,
    SessionService, ToolRouterService,
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
                        // Filter out rejected skills (Tantivy index doesn't store status)
                        let mut filtered = Vec::new();
                        for r in results {
                            if let Ok(skill) = self.registry.get_skill(&r.skill_id).await {
                                if skill.status != "rejected" {
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
                        let total = self.registry.count().await.unwrap_or(0);
                        Self::json_success(serde_json::json!({
                            "skills": skills,
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
                    Some(id) => match self.registry.get_skill(id).await {
                        Ok(skill) => {
                            Self::json_success(serde_json::to_value(skill).unwrap_or_default())
                        }
                        Err(e) => Self::json_error(format!("Get skill failed: {}", e)),
                    },
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
                        Self::json_success(serde_json::to_value(skills).unwrap_or_default())
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
                        let owner_id = Self::resolve_owner_id(agent_ctx);
                        let new_skill = NewSkill {
                            name: name.to_string(),
                            description: description.to_string(),
                            tags,
                            content: content.to_string(),
                            version: version.to_string(),
                            git_url: None,
                            visibility: None,
                            tools,
                            owner_type: "user".to_string(),
                            owner_id,
                            author_identity_id: None,
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

            _ => Self::json_error(format!("Unknown tool: {}", name)),
        };

        let call_result = result;
        serde_json::to_value(&call_result)
            .unwrap_or(serde_json::json!({"error": "serialization failed"}))
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
                "Install a skill. Returns metadata + a signed download URL (tarball). The agent should download the tar.gz from download_url and extract it to the skills directory. URL expires in 300 seconds. File count and tarball size are included for progress tracking.",
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
        ]
    }

    fn json_error(msg: String) -> CallToolResult {
        CallToolResult::error(vec![Content::text(format!("{{\"error\": \"{}\"}}", msg))])
    }

    fn json_success(data: serde_json::Value) -> CallToolResult {
        CallToolResult::success(vec![Content::text(data.to_string())])
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        rmcp::model::InitializeResult::new(ServerCapabilities::default())
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
}
