//! AionHive - 企业级 AI Skills 共享平台
//!
//! 核心功能：
//! - Skills 注册与发现
//! - Skills 全文搜索
//! - Skills 贡献与评价
//! - 置信度权重机制

use anyhow::Result;
use axum::{
    extract::Request,
    middleware::{self, Next},
    response::Response,
};
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Router,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio_stream::StreamExt;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use aion_hive::api::create_api_router;
use aion_hive::api::http_state::{AppRouterState, HttpState, SseState};
use aion_hive::db::repositories::{AgentRepository, AuditRepository, EvaluationRepository};
use aion_hive::{mcp::McpServer, AppState};

async fn request_logging_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let uri = req.uri().to_string();

    let response = next.run(req).await;
    let latency_ms = start.elapsed().as_millis();
    let status = response.status();

    tracing::info!("{} {} {} {}ms", method, uri, status.as_u16(), latency_ms);
    response
}

async fn mcp_handler(State(state): State<Arc<AppRouterState>>, body: String) -> impl IntoResponse {
    let server = state.http.mcp_server.read().await;
    let result = server.handle_jsonrpc(&body).await;
    match result {
        Ok(response) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            response,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "application/json")],
            format!("{{\"error\": \"{}\"}}", e),
        )
            .into_response(),
    }
}

async fn health_handler(State(state): State<Arc<AppRouterState>>) -> impl IntoResponse {
    let skills_count = state.registry.count().await.unwrap_or(0);
    let response = serde_json::json!({
        "status": "OK",
        "version": env!("CARGO_PKG_VERSION"),
        "skills_count": skills_count
    });
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        response.to_string(),
    )
}

async fn sse_handler(State(state): State<Arc<AppRouterState>>) -> impl IntoResponse {
    let session_id = Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::broadcast::channel::<String>(100);

    {
        let mut sessions = state.sse.sessions.write().await;
        sessions.insert(session_id.clone(), tx.clone());
    }

    let message_endpoint = format!("/sse/{}", session_id);

    let endpoint_event = Event::default().event("endpoint").data(&message_endpoint);

    let stream = tokio_stream::wrappers::BroadcastStream::new(rx).map(move |result| match result {
        Ok(msg) => Ok::<_, std::convert::Infallible>(Event::default().event("message").data(msg)),
        Err(e) => {
            tracing::warn!("SSE broadcast error: {}", e);
            Ok(Event::default().event("error").data("broadcast error"))
        }
    });

    let initial_stream =
        tokio_stream::iter(vec![Ok::<_, std::convert::Infallible>(endpoint_event)]);

    let combined_stream = initial_stream.chain(stream);

    Sse::new(combined_stream).keep_alive(axum::response::sse::KeepAlive::default())
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
        (
            StatusCode::ACCEPTED,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"status":"accepted"}"#,
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"error":"session not found"}"#,
        )
            .into_response()
    }
}

async fn run_http_server(state: AppState, port: u16) -> Result<()> {
    let pool = sqlx::PgPool::connect(
        &std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost:5432/aionhive".to_string()),
    )
    .await?;
    let eval_repo = EvaluationRepository::new(pool.clone());
    let agent_repo = AgentRepository::new(pool.clone());
    let audit_repo = AuditRepository::new(pool.clone());
    let group_perm_override_repo = aion_hive::db::repositories::group_permission_override::GroupPermissionOverrideRepository::new(pool.clone());
    let system_role_assignment_repo =
        aion_hive::db::repositories::SystemRoleAssignmentRepository::new(pool.clone());
    let org_membership_repo =
        aion_hive::db::repositories::OrgMembershipRepository::new(pool.clone());
    let role_permission_repo =
        aion_hive::db::repositories::RolePermissionRepository::new(pool.clone());
    let group_repo_for_perm = aion_hive::db::repositories::GroupRepository::new(pool.clone());
    let permission = aion_hive::services::permission::PermissionService::new(
        system_role_assignment_repo,
        org_membership_repo,
        role_permission_repo,
        group_perm_override_repo.clone(),
        group_repo_for_perm,
    );
    let evaluator =
        aion_hive::services::EvaluatorService::new(state.data_dir.join("evaluations"), eval_repo);
    let sandbox = aion_hive::services::SandboxService::new();
    let mcp_server = McpServer::new(
        state.registry.clone(),
        state.search.clone(),
        evaluator.clone(),
        state.session.clone(),
        state.org_tool.clone(),
        state.tool_router.clone(),
        sandbox,
    );
    let mcp_server_arc = Arc::new(tokio::sync::RwLock::new(mcp_server));

    let http_state = HttpState {
        mcp_server: mcp_server_arc,
    };
    let sse_state = SseState::new();

    let app_state: Arc<AppRouterState> = Arc::new(AppRouterState {
        http: http_state,
        sse: sse_state,
        registry: state.registry,
        search: state.search,
        evaluator,
        agent_repo,
        audit_repo,
        organization: state.organization.clone(),
        session: state.session.clone(),
        org_tool: state.org_tool.clone(),
        sandbox: state.sandbox.clone(),
        git_proxy: state.git_proxy.clone(),
        tenant: state.tenant.clone(),
        identity: state.identity.clone(),
        role: state.role.clone(),
        group: state.group.clone(),
        api_key: state.api_key.clone(),
        audit: state.audit.clone(),
        group_perm_override_repo: group_perm_override_repo.clone(),
        permission,
    });

    let api_router = create_api_router(app_state.clone());

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/mcp", post(mcp_handler))
        .route("/sse", get(sse_handler))
        .route("/sse/:session_id", post(sse_message_handler))
        .merge(api_router)
        .layer(middleware::from_fn(request_logging_middleware))
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
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
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
    info!(
        "Registry: {} skills",
        state.registry.count().await.unwrap_or(0)
    );

    let port = std::env::var("AION_HIVE_HTTP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()?;

    info!(
        "Starting MCP server with streamable-http + SSE transport on port {}",
        port
    );
    run_http_server(state, port).await?;

    Ok(())
}
