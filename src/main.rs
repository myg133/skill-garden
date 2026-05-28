//! AionHive - 企业级 AI Skills 共享平台
//!
//! 核心功能：
//! - Skills 注册与发现
//! - Skills 全文搜索
//! - Skills 贡献与评价
//! - 置信度权重机制

use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use axum::{
    routing::{get, post, put, delete},
    Router,
    extract::{State, Path},
    http::{StatusCode, header},
    response::{IntoResponse, sse::{Event, Sse}},
};
use tokio_stream::StreamExt;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use aion_hive::{AppState, mcp::McpServer};
use aion_hive::api::http_state::{AppRouterState, HttpState, SseState};
use aion_hive::api::handlers::{
    list_skills_handler, get_skill_handler, create_skill_handler,
    update_skill_handler, delete_skill_handler, get_skill_stats_handler,
    create_evaluation_handler, register_agent_handler, get_token_handler,
    admin_login_handler, list_audit_logs_handler, approve_skill_handler, reject_skill_handler,
    get_admin_status_handler,
    // v0.4 multi-tenant handlers
    create_org_handler, get_org_handler, list_orgs_handler, update_org_handler, delete_org_handler,
    create_session_handler, get_session_handler, list_sessions_handler, end_session_handler, session_declare_handler,
    register_org_tool_handler, list_org_tools_handler, list_all_org_tools_handler, approve_org_tool_handler, reject_org_tool_handler,
};
use aion_hive::db::repositories::{AgentRepository, EvaluationRepository, AuditRepository, AdminUserRepository};

async fn mcp_handler(
    State(state): State<Arc<AppRouterState>>,
    body: String,
) -> impl IntoResponse {
    let server = state.http.mcp_server.read().await;
    let result = server.handle_jsonrpc(&body).await;
    match result {
        Ok(response) => (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], response).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, [(header::CONTENT_TYPE, "application/json")], format!("{{\"error\": \"{}\"}}", e)).into_response(),
    }
}

async fn health_handler(State(state): State<Arc<AppRouterState>>) -> impl IntoResponse {
    let skills_count = state.registry.count().await.unwrap_or(0);
    let response = serde_json::json!({
        "status": "OK",
        "version": env!("CARGO_PKG_VERSION"),
        "skills_count": skills_count
    });
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], response.to_string())
}

async fn sse_handler(
    State(state): State<Arc<AppRouterState>>,
) -> impl IntoResponse {
    let session_id = Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::broadcast::channel::<String>(100);

    {
        let mut sessions = state.sse.sessions.write().await;
        sessions.insert(session_id.clone(), tx.clone());
    }

    let message_endpoint = format!("/sse/{}", session_id);

    let endpoint_event = Event::default()
        .event("endpoint")
        .data(&message_endpoint);

    let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
        .map(move |result| {
            match result {
                Ok(msg) => Ok::<_, std::convert::Infallible>(Event::default()
                    .event("message")
                    .data(msg)),
                Err(e) => {
                    tracing::warn!("SSE broadcast error: {}", e);
                    Ok(Event::default().event("error").data("broadcast error"))
                }
            }
        });

    let initial_stream = tokio_stream::iter(vec![
        Ok::<_, std::convert::Infallible>(endpoint_event),
    ]);

    let combined_stream = initial_stream.chain(stream);

    Sse::new(combined_stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
}

async fn sse_message_handler(
    State(state): State<Arc<AppRouterState>>,
    Path(session_id): Path<String>,
    body: String,
) -> impl IntoResponse {
    let sessions = state.sse.sessions.read().await;
    if let Some(tx) = sessions.get(&session_id) {
        let mcp_server = state.http.mcp_server.read().await;
        match mcp_server.handle_jsonrpc(&body).await {
            Ok(response) => {
                if tx.send(response).is_err() {
                    drop(sessions);
                    let mut sessions = state.sse.sessions.write().await;
                    sessions.remove(&session_id);
                }
            }
            Err(e) => {
                let error_response = format!("{{\"error\": \"{}\"}}", e);
                if tx.send(error_response).is_err() {
                    drop(sessions);
                    let mut sessions = state.sse.sessions.write().await;
                    sessions.remove(&session_id);
                }
            }
        }
        (StatusCode::ACCEPTED, [(header::CONTENT_TYPE, "application/json")], r#"{"status":"accepted"}"#).into_response()
    } else {
        (StatusCode::NOT_FOUND, [(header::CONTENT_TYPE, "application/json")], r#"{"error":"session not found"}"#).into_response()
    }
}

async fn run_http_server(state: AppState, port: u16) -> Result<()> {
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost:5432/aionhive".to_string())).await?;
    let eval_repo = EvaluationRepository::new(pool.clone());
    let agent_repo = AgentRepository::new(pool.clone());
    let audit_repo = AuditRepository::new(pool.clone());
    let admin_user_repo = AdminUserRepository::new(pool.clone());
    let evaluator = aion_hive::services::EvaluatorService::new(state.data_dir.join("evaluations"), eval_repo);
    let mcp_server = McpServer::new(
        state.registry.clone(),
        state.search.clone(),
        evaluator.clone(),
        state.session.clone(),
        state.org_tool.clone(),
        state.tool_router.clone(),
    );
    let mcp_server_arc = Arc::new(tokio::sync::RwLock::new(mcp_server));

    let http_state = HttpState { mcp_server: mcp_server_arc };
    let sse_state = SseState::new();

    let app_state: Arc<AppRouterState> = Arc::new(AppRouterState {
        http: http_state,
        sse: sse_state,
        registry: state.registry,
        search: state.search,
        evaluator,
        agent_repo,
        audit_repo,
        admin_user_repo,
        organization: state.organization.clone(),
        session: state.session.clone(),
        org_tool: state.org_tool.clone(),
    });

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/mcp", post(mcp_handler))
        .route("/sse", get(sse_handler))
        .route("/sse/:session_id", post(sse_message_handler))
        // v1 API routes
        .route("/api/v1/skills", get(list_skills_handler))
        .route("/api/v1/skills", post(create_skill_handler))
        .route("/api/v1/skills/:id", get(get_skill_handler))
        .route("/api/v1/skills/:id", put(update_skill_handler))
        .route("/api/v1/skills/:id", delete(delete_skill_handler))
        .route("/api/v1/skills/:id/stats", get(get_skill_stats_handler))
        .route("/api/v1/evaluations", post(create_evaluation_handler))
        .route("/api/v1/auth/agent/register", post(register_agent_handler))
        .route("/api/v1/auth/agent/token", post(get_token_handler))
        // Admin routes
        .route("/api/v1/admin/login", post(admin_login_handler))
        .route("/api/v1/admin/audit-logs", get(list_audit_logs_handler))
        .route("/api/v1/admin/skills/:id/approve", post(approve_skill_handler))
        .route("/api/v1/admin/skills/:id/reject", post(reject_skill_handler))
        .route("/api/v1/admin/status", get(get_admin_status_handler))
        // v0.4 multi-tenant routes
        .route("/api/v1/organizations", post(create_org_handler))
        .route("/api/v1/organizations", get(list_orgs_handler))
        .route("/api/v1/organizations/:id", get(get_org_handler))
        .route("/api/v1/organizations/:id", put(update_org_handler))
        .route("/api/v1/organizations/:id", delete(delete_org_handler))
        .route("/api/v1/sessions", post(create_session_handler))
        .route("/api/v1/sessions", get(list_sessions_handler))
        .route("/api/v1/sessions/:id", get(get_session_handler))
        .route("/api/v1/sessions/:id/end", post(end_session_handler))
        .route("/api/v1/sessions/:id/declare", post(session_declare_handler))
        .route("/api/v1/org-tools", post(register_org_tool_handler))
        .route("/api/v1/org-tools", get(list_all_org_tools_handler))
        .route("/api/v1/org-tools/:org_id", get(list_org_tools_handler))
        .route("/api/v1/org-tools/:id/approve", post(approve_org_tool_handler))
        .route("/api/v1/org-tools/:id/reject", post(reject_org_tool_handler))
        .with_state(app_state);

    let addr = format!("127.0.0.1:{}", port);
    info!("Starting HTTP server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting AionHive v{}", env!("CARGO_PKG_VERSION"));

    let data_dir = std::env::var("AION_HIVE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data"));

    let skills_dir = std::env::var("AION_HIVE_SKILLS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("skills"));

    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(&data_dir.join("registry"))?;
    std::fs::create_dir_all(&data_dir.join("evaluations"))?;
    std::fs::create_dir_all(&data_dir.join("search_index"))?;

    let state = AppState::new(data_dir.clone(), skills_dir).await?;

    info!("AionHive initialized successfully");
    info!("Registry: {} skills", state.registry.count().await.unwrap_or(0));

    let port = std::env::var("AION_HIVE_HTTP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()?;

    info!("Starting MCP server with streamable-http + SSE transport on port {}", port);
    run_http_server(state, port).await?;

    Ok(())
}