//! API Routes Configuration

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use super::handlers::*;

pub fn create_api_router(state: ApiState) -> Router<ApiState> {
    Router::new()
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
        // Admin routes (under /api/v1/admin)
        .route("/api/v1/admin/login", post(admin_login_handler))
        .route("/api/v1/admin/audit-logs", get(list_audit_logs_handler))
        .route("/api/v1/admin/skills/:id/approve", post(approve_skill_handler))
        .route("/api/v1/admin/skills/:id/reject", post(reject_skill_handler))
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
        .with_state(state)
}