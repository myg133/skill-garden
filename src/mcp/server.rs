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
use std::sync::Arc;
use tokio::sync::broadcast;
use serde_json::Value;

use crate::api::jwt::verify_token;
use crate::models::evaluation::{ErrorType, EvalTag};
use crate::models::skill::NewSkill;
use crate::services::{EvaluatorService, OrgToolService, RegistryService, SearchService, SessionService, ToolRouterService};

pub struct McpServer {
    registry: RegistryService,
    search: SearchService,
    evaluator: EvaluatorService,
    session: SessionService,
    org_tool: OrgToolService,
    tool_router: ToolRouterService,
    agent_context: Option<AgentContext>,
}

/// Agent context extracted from JWT token
#[derive(Debug, Clone)]
pub struct AgentContext {
    pub agent_id: String,
    pub org_id: Option<uuid::Uuid>,
    pub session_id: Option<uuid::Uuid>,
    pub roles: Vec<String>,
    pub scope: Vec<String>,
}

impl McpServer {
    pub fn new(
        registry: RegistryService,
        search: SearchService,
        evaluator: EvaluatorService,
        session: SessionService,
        org_tool: OrgToolService,
        tool_router: ToolRouterService,
    ) -> Self {
        // Try to extract and verify JWT from environment (for stdio transport)
        let agent_context = Self::extract_jwt_from_env();
        Self {
            registry,
            search,
            evaluator,
            session,
            org_tool,
            tool_router,
            agent_context,
        }
    }

    /// Extract and verify JWT from environment variable AION_HIVE_JWT_TOKEN
    fn extract_jwt_from_env() -> Option<AgentContext> {
        let token = std::env::var("AION_HIVE_JWT_TOKEN").ok()?;
        let claims = verify_token(&token).ok()?;
        Some(AgentContext {
            agent_id: claims.agent_id,
            org_id: claims.org_id,
            session_id: claims.session_id,
            roles: claims.roles,
            scope: claims.scope,
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

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (stdin, stdout) = stdio();
        serve_server(self, (stdin, stdout)).await?;
        Ok(())
    }

    pub async fn run_sse(self, _tx: broadcast::Sender<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (stdin, stdout) = stdio();
        serve_server(self, (stdin, stdout)).await?;
        Ok(())
    }

    pub async fn handle_jsonrpc(&self, body: &str) -> Result<String, String> {
        let request: Value = serde_json::from_str(body)
            .map_err(|e| format!("Invalid JSON: {}", e))?;

        let method = request.get("method")
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
                let tool_name = params.and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                let args: std::collections::HashMap<String, Value> = params
                    .and_then(|p| p.get("arguments"))
                    .and_then(|a| a.as_object())
                    .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();

                let call_result = self.call_tool_internal(tool_name, args).await;

                if call_result.get("isError").and_then(|v| v.as_bool()).unwrap_or(false) {
                    let error_msg = call_result.get("content")
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
                    }).to_string());
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

    async fn call_tool_internal(&self, name: &str, args: std::collections::HashMap<String, Value>) -> Value {
        let args = args.into_iter().collect::<std::collections::HashMap<_, _>>();
        let result = match name {
            "health_check" => Self::json_success(serde_json::json!({"status": "OK"})),

            "skills.search" => {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                });
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as usize;

                match self.search.search(&query, tags.as_deref(), limit) {
                    Ok(results) => Self::json_success(serde_json::to_value(results).unwrap_or_default()),
                    Err(e) => Self::json_error(format!("Search failed: {}", e)),
                }
            }

            "skills.list" => {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;

                match self.registry.list_skills().await {
                    Ok(skills) => {
                        let limited: Vec<_> = skills.into_iter().take(limit).collect();
                        Self::json_success(serde_json::to_value(limited).unwrap_or_default())
                    }
                    Err(e) => Self::json_error(format!("List failed: {}", e)),
                }
            }

            "skills.info" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());

                match skill_id {
                    Some(id) => match self.registry.get_skill(id).await {
                        Ok(skill) => Self::json_success(serde_json::to_value(skill).unwrap_or_default()),
                        Err(e) => Self::json_error(format!("Get skill failed: {}", e)),
                    },
                    None => Self::json_error("skill_id is required".to_string()),
                }
            }

            "skills.create" => {
                let name = args.get("name").and_then(|v| v.as_str());
                let description = args.get("description").and_then(|v| v.as_str());
                let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                }).unwrap_or_default();
                let content = args.get("content").and_then(|v| v.as_str());
                let version = args.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0");

                match (name, description, content) {
                    (Some(name), Some(description), Some(content)) => {
                        let new_skill = NewSkill {
                            name: name.to_string(),
                            description: description.to_string(),
                            tags,
                            content: content.to_string(),
                            version: version.to_string(),
                            git_url: None,
                            visibility: None,
                            tools: None,
                        };
                        let agent_id = "http-client";
                        match self.registry.create_skill(new_skill, agent_id, &self.search).await {
                            Ok(skill) => Self::json_success(serde_json::to_value(skill).unwrap_or_default()),
                            Err(e) => Self::json_error(format!("Create skill failed: {}", e)),
                        }
                    }
                    _ => Self::json_error("name, description, and content are required".to_string()),
                }
            }

            "skills.update" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());

                let update = crate::models::skill::SkillUpdate {
                    description: args.get("description").and_then(|v| v.as_str()).map(String::from),
                    tags: args.get("tags").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    }),
                    content: args.get("content").and_then(|v| v.as_str()).map(String::from),
                    git_url: None,
                    visibility: None,
                    tools: None,
                };

                match skill_id {
                    Some(id) => {
                        let agent_id = "http-client";
                        match self.registry.update_skill(id, update, agent_id, &self.search).await {
                            Ok(skill) => Self::json_success(serde_json::to_value(skill).unwrap_or_default()),
                            Err(e) => Self::json_error(format!("Update skill failed: {}", e)),
                        }
                    }
                    None => Self::json_error("skill_id is required".to_string()),
                }
            }

            // Note: skills_delete is Admin-only via REST API, not MCP

            "skills.install" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());

                match skill_id {
                    Some(id) => Self::json_success(serde_json::json!({"installed": id})),
                    None => Self::json_error("skill_id is required".to_string()),
                }
            }

            "evaluate_skill" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());
                let agent_id = args.get("agent_id").and_then(|v| v.as_str());
                let success = args.get("success").and_then(|v| v.as_bool());
                let duration_ms = args.get("duration_ms").and_then(|v| v.as_u64());

                let error_type = args.get("error_type").and_then(|v| v.as_str()).map(|s| match s {
                    "timeout" => ErrorType::Timeout,
                    "crash" => ErrorType::Crash,
                    "logic_error" => ErrorType::LogicError,
                    _ => ErrorType::Other,
                });

                let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
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
                }).unwrap_or_default();

                match (skill_id, agent_id, success, duration_ms) {
                    (Some(skill_id), Some(agent_id), Some(success), Some(duration_ms)) => {
                        match self.evaluator.add_evaluation(
                            skill_id.to_string(),
                            agent_id.to_string(),
                            success,
                            duration_ms,
                            error_type,
                            tags,
                        ).await {
                            Ok(result) => Self::json_success(serde_json::to_value(result).unwrap_or_default()),
                            Err(e) => Self::json_error(format!("Evaluate skill failed: {}", e)),
                        }
                    }
                    _ => Self::json_error("skill_id, agent_id, success, and duration_ms are required".to_string()),
                }
            }

            "skills.stats" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());

                match skill_id {
                    Some(id) => match self.evaluator.get_stats(id) {
                        Ok(stats) => Self::json_success(serde_json::to_value(stats).unwrap_or_default()),
                        Err(e) => Self::json_error(format!("Get stats failed: {}", e)),
                    },
                    None => Self::json_error("skill_id is required".to_string()),
                }
            }

            _ => Self::json_error(format!("Unknown tool: {}", name)),
        };

        let call_result = result;
        serde_json::to_value(&call_result).unwrap_or(serde_json::json!({"error": "serialization failed"}))
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
                "List all available skills",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {"type": "number", "description": "Maximum results (default 100)"}
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
                "skills.install",
                "Mark a skill as installed (for tracking)",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "skill_id": {"type": "string", "description": "Skill ID"}
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
                "Get statistics for a skill",
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
                "Get current session information",
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
                "Declare agent capabilities for the session",
                Arc::new(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string", "description": "Session ID"},
                        "capabilities": {"type": "array", "items": {"type": "string"}, "description": "Agent capabilities"}
                    },
                    "required": ["session_id", "capabilities"]
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
        let args = request.arguments.unwrap_or_default();

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
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as usize;

                match self.search.search(&query, tags.as_deref(), limit) {
                    Ok(results) => Self::json_success(serde_json::to_value(results).unwrap_or_default()),
                    Err(e) => Self::json_error(format!("Search failed: {}", e)),
                }
            }

            "skills.list" => {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;

                match self.registry.list_skills().await {
                    Ok(skills) => {
                        let limited: Vec<_> = skills.into_iter().take(limit).collect();
                        Self::json_success(serde_json::to_value(limited).unwrap_or_default())
                    }
                    Err(e) => Self::json_error(format!("List failed: {}", e)),
                }
            }

            "skills.info" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());

                match skill_id {
                    Some(id) => match self.registry.get_skill(id).await {
                        Ok(skill) => Self::json_success(serde_json::to_value(skill).unwrap_or_default()),
                        Err(e) => Self::json_error(format!("Get skill failed: {}", e)),
                    },
                    None => Self::json_error("skill_id is required".to_string()),
                }
            }

            "skills.create" => {
                let name = args.get("name").and_then(|v| v.as_str());
                let description = args.get("description").and_then(|v| v.as_str());
                let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                }).unwrap_or_default();
                let content = args.get("content").and_then(|v| v.as_str());
                let version = args.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0");

                match (name, description, content) {
                    (Some(name), Some(description), Some(content)) => {
                        let new_skill = NewSkill {
                            name: name.to_string(),
                            description: description.to_string(),
                            tags,
                            content: content.to_string(),
                            version: version.to_string(),
                            git_url: None,
                            visibility: None,
                            tools: None,
                        };
                        let agent_id = "mcp-client";
                        match self.registry.create_skill(new_skill, agent_id, &self.search).await {
                            Ok(skill) => Self::json_success(serde_json::to_value(skill).unwrap_or_default()),
                            Err(e) => Self::json_error(format!("Create skill failed: {}", e)),
                        }
                    }
                    _ => Self::json_error("name, description, and content are required".to_string()),
                }
            }

            "skills.update" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());

                let update = crate::models::skill::SkillUpdate {
                    description: args.get("description").and_then(|v| v.as_str()).map(String::from),
                    tags: args.get("tags").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    }),
                    content: args.get("content").and_then(|v| v.as_str()).map(String::from),
                    git_url: None,
                    visibility: None,
                    tools: None,
                };

                match skill_id {
                    Some(id) => {
                        let agent_id = "mcp-client";
                        match self.registry.update_skill(id, update, agent_id, &self.search).await {
                            Ok(skill) => Self::json_success(serde_json::to_value(skill).unwrap_or_default()),
                            Err(e) => Self::json_error(format!("Update skill failed: {}", e)),
                        }
                    }
                    None => Self::json_error("skill_id is required".to_string()),
                }
            }

            // Note: skills_delete is Admin-only via REST API, not MCP

            "skills.install" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());
                let agent_id = args.get("agent_id").and_then(|v| v.as_str());
                let success = args.get("success").and_then(|v| v.as_bool());
                let duration_ms = args.get("duration_ms").and_then(|v| v.as_u64());

                let error_type = args.get("error_type").and_then(|v| v.as_str()).map(|s| match s {
                    "timeout" => ErrorType::Timeout,
                    "crash" => ErrorType::Crash,
                    "logic_error" => ErrorType::LogicError,
                    _ => ErrorType::Other,
                });

                let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
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
                }).unwrap_or_default();

                match (skill_id, agent_id, success, duration_ms) {
                    (Some(skill_id), Some(agent_id), Some(success), Some(duration_ms)) => {
                        match self.evaluator.add_evaluation(
                            skill_id.to_string(),
                            agent_id.to_string(),
                            success,
                            duration_ms,
                            error_type,
                            tags,
                        ).await {
                            Ok(result) => Self::json_success(serde_json::to_value(result).unwrap_or_default()),
                            Err(e) => Self::json_error(format!("Evaluate skill failed: {}", e)),
                        }
                    }
                    _ => Self::json_error("skill_id, agent_id, success, and duration_ms are required".to_string()),
                }
            }

            "skills.stats" => {
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());

                match skill_id {
                    Some(id) => match self.evaluator.get_stats(id) {
                        Ok(stats) => Self::json_success(serde_json::to_value(stats).unwrap_or_default()),
                        Err(e) => Self::json_error(format!("Get stats failed: {}", e)),
                    },
                    None => Self::json_error("skill_id is required".to_string()),
                }
            }

            "session.info" => {
                let session_id = args.get("session_id").and_then(|v| v.as_str());

                // Verify JWT authentication
                let ctx = match self.agent_context.as_ref() {
                    Some(ctx) => ctx,
                    None => {
                        // Return error - will be wrapped in Ok by caller
                        return Ok(Self::json_error("JWT authentication required. Set AION_HIVE_JWT_TOKEN environment variable.".to_string()));
                    }
                };

                // If session_id provided, verify it matches the JWT session
                if let Some(req_session_id) = session_id {
                    if let Some(jwt_session_id) = ctx.session_id {
                        let expected = req_session_id.to_string();
                        if expected != jwt_session_id.to_string() {
                            return Ok(Self::json_error("Session ID mismatch. Cannot access other sessions.".to_string()));
                        }
                    }
                }

                // Return session info
                let session_info = serde_json::json!({
                    "session_id": ctx.session_id.map(|s| s.to_string()),
                    "agent_id": ctx.agent_id,
                    "org_id": ctx.org_id.map(|s| s.to_string()),
                    "capabilities": ctx.scope,
                    "roles": ctx.roles,
                });
                Self::json_success(session_info)
            }

            "session.declare" => {
                // Verify JWT authentication
                let _ctx = match self.agent_context.as_ref() {
                    Some(ctx) => ctx,
                    None => {
                        // Return error - will be wrapped in Ok by caller
                        return Ok(Self::json_error("JWT authentication required. Set AION_HIVE_JWT_TOKEN environment variable.".to_string()));
                    }
                };

                // TODO: Store declared capabilities in session/routing table
                // This requires database access to update session capabilities
                Self::json_error("session.declare not yet implemented. Capabilities will be stored on session update.".to_string())
            }

            _ => Self::json_error(format!("Unknown tool: {}", name)),
        };

        Ok(result)
    }
}
