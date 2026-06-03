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
        .route(
            "/api/v1/skills/:id/submit-review",
            post(submit_review_skill_handler),
        )
        .route(
            "/api/v1/skills/:id/approve",
            post(approve_org_skill_handler),
        )
        .route("/api/v1/skills/:id/reject", post(reject_org_skill_handler))
        .route("/api/v1/skills/:id/publish", post(publish_skill_handler))
        .route("/api/v1/skills/:id/groups", get(list_skill_groups_handler))
        .route(
            "/api/v1/skills/:id/groups",
            post(add_skill_to_group_handler),
        )
        .route(
            "/api/v1/skills/:id/groups/:group_id",
            delete(remove_skill_from_group_handler),
        )
        .route("/api/v1/skills/:id/install", post(install_skill_handler))
        .route("/api/v1/marketplace", get(marketplace_handler))
        .route("/api/v1/evaluations", post(create_evaluation_handler))
        .route("/api/v1/auth/agent/register", post(register_agent_handler))
        .route("/api/v1/auth/agent/token", post(get_token_handler))
        .route("/api/v1/auth/login", post(user_login_handler))
        .route("/api/v1/auth/register", post(user_register_handler))
        // User routes
        .route("/api/v1/users/me", get(get_user_me_handler))
        .route("/api/v1/users/me", put(update_user_me_handler))
        .route("/api/v1/users/me/orgs", get(get_user_orgs_handler))
        .route("/api/v1/users/:username", get(get_user_by_username_handler))
        // API Key routes (user-facing self-service)
        .route("/api/v1/api-keys", get(list_my_api_keys_handler))
        .route("/api/v1/api-keys", post(create_my_api_key_handler))
        .route("/api/v1/api-keys/:id", delete(revoke_my_api_key_handler))
        // Org slug-based Group management (6.6)
        .route("/api/v1/orgs/:slug/groups", get(list_org_groups_handler))
        .route("/api/v1/orgs/:slug/groups", post(create_org_group_handler))
        .route(
            "/api/v1/orgs/:slug/groups/:group_id",
            get(get_org_group_handler),
        )
        .route(
            "/api/v1/orgs/:slug/groups/:group_id",
            put(update_org_group_handler),
        )
        .route(
            "/api/v1/orgs/:slug/groups/:group_id",
            delete(delete_org_group_handler),
        )
        // Org slug-based Group member management (6.6)
        .route(
            "/api/v1/orgs/:slug/groups/:group_id/members",
            get(list_org_group_members_handler),
        )
        .route(
            "/api/v1/orgs/:slug/groups/:group_id/members/:username",
            put(update_org_group_member_handler),
        )
        .route(
            "/api/v1/orgs/:slug/groups/:group_id/members/:username",
            delete(remove_org_group_member_handler),
        )
        // Org slug-based Group-Skill association (6.6)
        .route(
            "/api/v1/orgs/:slug/groups/:group_id/skills",
            get(list_org_group_skills_handler),
        )
        .route(
            "/api/v1/orgs/:slug/groups/:group_id/skills",
            post(add_org_group_skill_handler),
        )
        .route(
            "/api/v1/orgs/:slug/groups/:group_id/skills/:skill_id",
            delete(remove_org_group_skill_handler),
        )
        // Admin routes (under /api/v1/admin)
        .route("/api/v1/admin/login", post(admin_login_handler))
        .route("/api/v1/admin/me", get(get_admin_me_handler))
        .route("/api/v1/admin/stats", get(get_admin_stats_handler))
        .route("/api/v1/admin/audit-logs", get(list_audit_logs_handler))
        .route(
            "/api/v1/admin/skills/:id/approve",
            post(approve_skill_handler),
        )
        .route(
            "/api/v1/admin/skills/:id/reject",
            post(reject_skill_handler),
        )
        .route("/api/v1/admin/status", get(get_admin_status_handler))
        // v0.4 multi-tenant routes
        .route("/api/v1/organizations", post(create_org_handler))
        .route("/api/v1/organizations", get(list_orgs_handler))
        .route("/api/v1/organizations/:id", get(get_org_handler))
        .route("/api/v1/organizations/:id", put(update_org_handler))
        .route("/api/v1/organizations/:id", delete(delete_org_handler))
        // Org slug-based routes (6.3)
        .route("/api/v1/orgs/:slug", get(get_org_by_slug_handler))
        .route("/api/v1/orgs/:slug/skills", get(list_org_skills_handler))
        .route("/api/v1/orgs/:slug/skills", post(create_org_skill_handler))
        .route("/api/v1/orgs/:slug/reviews", get(list_org_reviews_handler))
        .route(
            "/api/v1/orgs/:slug/members",
            get(list_org_members_by_slug_handler),
        )
        .route(
            "/api/v1/orgs/:slug/members",
            post(invite_org_member_handler),
        )
        .route(
            "/api/v1/orgs/id/:org_id/members",
            get(list_org_members_by_id_handler),
        )
        .route(
            "/api/v1/orgs/id/:org_id/members/invite",
            post(invite_org_member_by_id_handler),
        )
        .route(
            "/api/v1/orgs/id/:org_id/members/:username",
            put(update_org_member_by_id_handler),
        )
        .route(
            "/api/v1/orgs/id/:org_id/members/:username",
            delete(remove_org_member_by_id_handler),
        )
        .route(
            "/api/v1/orgs/:slug/members/:username",
            put(update_org_member_handler),
        )
        .route(
            "/api/v1/orgs/:slug/members/:username",
            delete(remove_org_member_by_slug_handler),
        )
        // Group member management (6.6)
        .route(
            "/api/v1/groups/:id/members",
            get(list_group_members_handler),
        )
        .route("/api/v1/groups/:id/members", post(add_group_member_handler))
        .route(
            "/api/v1/groups/:id/members/:agent_id",
            put(update_group_member_handler),
        )
        .route(
            "/api/v1/groups/:id/members/:agent_id",
            delete(remove_group_member_handler),
        )
        // Group permission management
        .route(
            "/api/v1/groups/default-permissions",
            get(list_group_default_permissions_handler),
        )
        .route(
            "/api/v1/groups/:id/permissions",
            get(list_group_permissions_handler),
        )
        .route(
            "/api/v1/groups/:id/permissions",
            put(update_group_permission_handler),
        )
        .route(
            "/api/v1/groups/:id/permissions/:permission_code",
            delete(delete_group_permission_handler),
        )
        // Organization member routes
        .route(
            "/api/v1/admin/orgs/:org_id/members",
            get(list_org_members_handler),
        )
        .route(
            "/api/v1/admin/orgs/:org_id/members",
            post(add_org_member_handler),
        )
        .route(
            "/api/v1/admin/orgs/:org_id/members/:agent_id",
            delete(remove_org_member_handler),
        )
        .route(
            "/api/v1/admin/orgs/:org_id/stats",
            get(get_org_stats_handler),
        )
        // Session routes
        .route("/api/v1/sessions", post(create_session_handler))
        .route("/api/v1/sessions", get(list_sessions_handler))
        .route("/api/v1/sessions/:id", get(get_session_handler))
        .route("/api/v1/sessions/:id/end", post(end_session_handler))
        .route(
            "/api/v1/sessions/:id/declare",
            post(session_declare_handler),
        )
        .route("/api/v1/org-tools", post(register_org_tool_handler))
        .route("/api/v1/org-tools", get(list_all_org_tools_handler))
        .route(
            "/api/v1/org-tools/:id/approve",
            post(approve_org_tool_handler),
        )
        .route(
            "/api/v1/org-tools/:id/reject",
            post(reject_org_tool_handler),
        )
        .route("/api/v1/org-tools/:id", delete(delete_org_tool_handler))
        .route("/api/v1/org-tools/:id", get(list_org_tools_handler))
        // Sandbox routes
        .route("/api/v1/admin/sandboxes", get(list_sandboxes_handler))
        .route(
            "/api/v1/admin/sandboxes/health",
            get(get_sandbox_health_handler),
        )
        .route(
            "/api/v1/admin/sandboxes/:key",
            delete(remove_sandbox_handler),
        )
        .route("/api/v1/tools/execute", post(execute_tool_handler))
        // Git Proxy routes
        .route(
            "/api/v1/admin/git/:repo_id/branches",
            get(list_git_branches_handler),
        )
        .route(
            "/api/v1/admin/git/:repo_id/commits/:limit",
            get(get_git_commits_handler),
        )
        .route(
            "/api/v1/admin/git/:repo_id/file/*path",
            get(get_git_file_handler),
        )
        .route(
            "/api/v1/admin/git/:repo_id/diff/:from/:to",
            get(get_git_diff_handler),
        )
        .route("/api/v1/admin/git/validate", post(validate_git_url_handler))
        .route(
            "/api/v1/admin/git/health",
            get(get_git_proxy_health_handler),
        )
        // Tenant routes
        .route("/api/v1/admin/tenants", get(list_tenants_handler))
        .route("/api/v1/admin/tenants", post(create_tenant_handler))
        .route("/api/v1/admin/tenants/:id", get(get_tenant_handler))
        .route("/api/v1/admin/tenants/:id", put(update_tenant_handler))
        .route("/api/v1/admin/tenants/:id", delete(delete_tenant_handler))
        // Identity routes
        .route("/api/v1/admin/identities", get(list_identities_handler))
        .route("/api/v1/admin/identities", post(create_identity_handler))
        .route("/api/v1/admin/identities/:id", get(get_identity_handler))
        .route("/api/v1/admin/identities/:id", put(update_identity_handler))
        .route(
            "/api/v1/admin/identities/:id",
            delete(delete_identity_handler),
        )
        // Group routes
        .route("/api/v1/admin/groups", get(list_groups_handler))
        .route("/api/v1/admin/groups", post(create_group_handler))
        .route("/api/v1/admin/groups/:id", get(get_group_handler))
        .route("/api/v1/admin/groups/:id", put(update_group_handler))
        .route("/api/v1/admin/groups/:id", delete(delete_group_handler))
        // Role routes
        .route("/api/v1/admin/roles", get(list_roles_handler))
        .route("/api/v1/admin/roles/:id", get(get_role_handler))
        // API Key routes
        .route("/api/v1/admin/api-keys", get(list_api_keys_handler))
        .route("/api/v1/admin/api-keys", post(create_api_key_handler))
        .route("/api/v1/admin/api-keys/:id", delete(delete_api_key_handler))
        // Audit Log Entries
        .route(
            "/api/v1/admin/audit-entries",
            get(list_audit_entries_handler),
        )
        .with_state(state)
}
