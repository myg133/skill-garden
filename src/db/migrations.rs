//! Database migrations

use sqlx::PgPool;
use std::path::Path;

use super::error::{DbError, DbResult};

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_initial_schema",
        include_str!("migrations/001_initial_schema.sql"),
    ),
    (
        "002_add_skill_status",
        include_str!("migrations/002_add_skill_status.sql"),
    ),
    (
        "003_seed_admin_agent",
        include_str!("migrations/003_seed_admin_agent.sql"),
    ),
    (
        "004_add_organizations",
        include_str!("migrations/004_add_organizations.sql"),
    ),
    (
        "005_add_sessions",
        include_str!("migrations/005_add_sessions.sql"),
    ),
    (
        "006_add_org_tools",
        include_str!("migrations/006_add_org_tools.sql"),
    ),
    (
        "007_add_skill_policies",
        include_str!("migrations/007_add_skill_policies.sql"),
    ),
    (
        "008_add_skill_git_and_org_fields",
        include_str!("migrations/008_add_skill_git_and_org_fields.sql"),
    ),
    (
        "009_add_agent_id_column",
        include_str!("migrations/009_add_agent_id_column.sql"),
    ),
    (
        "010_add_admin_users",
        include_str!("migrations/010_add_admin_users.sql"),
    ),
    (
        "011_add_session_skill_fields",
        include_str!("migrations/011_add_session_skill_fields.sql"),
    ),
    (
        "012_add_session_context",
        include_str!("migrations/012_add_session_context.sql"),
    ),
    (
        "013_add_tenants",
        include_str!("migrations/013_add_tenants.sql"),
    ),
    (
        "014_add_identities_and_roles",
        include_str!("migrations/014_add_identities_and_roles.sql"),
    ),
    (
        "015_add_api_keys_and_audit",
        include_str!("migrations/015_add_api_keys_and_audit.sql"),
    ),
    (
        "016_drop_skills_agent_fk",
        include_str!("migrations/016_drop_skills_agent_fk.sql"),
    ),
    (
        "017_add_user_model_and_org_memberships",
        include_str!("migrations/017_add_user_model_and_org_memberships.sql"),
    ),
    (
        "018_add_rbac_and_group_skills",
        include_str!("migrations/018_add_rbac_and_group_skills.sql"),
    ),
    (
        "019_add_system_role_assignments",
        include_str!("migrations/019_add_system_role_assignments.sql"),
    ),
    (
        "020_add_organization_slug_unique",
        include_str!("migrations/020_add_organization_slug_unique.sql"),
    ),
    (
        "021_merge_admin_users_into_identities",
        include_str!("migrations/021_merge_admin_users_into_identities.sql"),
    ),
    (
        "022_add_skill_versions",
        include_str!("migrations/022_add_skill_versions.sql"),
    ),
    (
        "023_add_git_remote_url",
        include_str!("migrations/023_add_git_remote_url.sql"),
    ),
    (
        "024_enhance_agents",
        include_str!("migrations/024_enhance_agents.sql"),
    ),
    (
        "025_fix_sessions_identity",
        include_str!("migrations/025_fix_sessions_identity.sql"),
    ),
    (
        "026_rbac_and_download_tokens",
        include_str!("migrations/026_rbac_and_download_tokens.sql"),
    ),
    (
        "027_cli_and_review_enhancements",
        include_str!("migrations/027_cli_and_review_enhancements.sql"),
    ),
    (
        "028_add_admin_unpublished",
        include_str!("migrations/028_add_admin_unpublished.sql"),
    ),
    (
        "029_add_unified_admin_auth",
        include_str!("migrations/029_add_unified_admin_auth.sql"),
    ),
    (
        "030_add_tenant_role_assignments",
        include_str!("migrations/030_add_tenant_role_assignments.sql"),
    ),
    (
        "031_seed_admin_user",
        include_str!("migrations/031_seed_admin_user.sql"),
    ),
    (
        "032_add_marketplace_status",
        include_str!("migrations/032_add_marketplace_status.sql"),
    ),
    (
        "033_add_marketplace_permissions",
        include_str!("migrations/033_add_marketplace_permissions.sql"),
    ),
    (
        "034_cleanup_legacy_permissions",
        include_str!("migrations/034_cleanup_legacy_permissions.sql"),
    ),
    (
        "035_add_pending_delist_status",
        include_str!("migrations/035_add_pending_delist_status.sql"),
    ),
    (
        "036_add_draft_content",
        include_str!("migrations/036_add_draft_content.sql"),
    ),
    (
        "037_remove_market_admin_internal_review",
        include_str!("migrations/037_remove_market_admin_internal_review.sql"),
    ),
];

fn split_sql_statements(sql: &str) -> Vec<&str> {
    sql.split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect()
}

pub async fn run_migrations(pool: &PgPool, _migrations_path: &Path) -> DbResult<()> {
    // Create migrations tracking table if not exists
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            name VARCHAR(255) PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| DbError::QueryError(format!("Failed to create migrations table: {}", e)))?;

    // Run each migration if not already applied
    for (name, sql) in MIGRATIONS {
        let already_applied: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = $1)")
                .bind(*name)
                .fetch_one(pool)
                .await
                .map_err(|e| {
                    DbError::QueryError(format!("Failed to check migration {}: {}", name, e))
                })?;

        if !already_applied {
            tracing::info!("Running migration: {}", name);

            // Split by semicolons and execute each statement
            for stmt in split_sql_statements(sql) {
                sqlx::query(stmt).execute(pool).await.map_err(|e| {
                    DbError::QueryError(format!("Failed to run statement in {}: {}", name, e))
                })?;
            }

            sqlx::query("INSERT INTO _migrations (name) VALUES ($1)")
                .bind(*name)
                .execute(pool)
                .await
                .map_err(|e| {
                    DbError::QueryError(format!("Failed to record migration {}: {}", name, e))
                })?;

            tracing::info!("Migration {} completed", name);
        }
    }

    Ok(())
}

pub async fn check_migrations(pool: &PgPool) -> DbResult<bool> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'agents'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| DbError::QueryError(format!("Failed to check migrations: {}", e)))?;

    Ok(row.0 > 0)
}
