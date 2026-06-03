//! Cross-tenant data isolation tests for admin endpoints.
//!
//! These tests verify the auth/extract boundary for admin endpoints after the
//! tenant-scope-guard refactor (Tasks 6-12). They cover the JWT-based
//! `AdminUser` and `AgentContext` extractors, which are the entry point for
//! every protected admin handler.
//!
//! What is covered here (no DB required):
//! * `AdminUser` extractor rejects non-admin tokens
//! * `AdminUser` extractor rejects admin tokens missing `identity_id`
//! * `AdminUser` extractor rejects missing / malformed / invalid `Authorization` headers
//! * `AdminUser` extractor accepts a well-formed admin token
//! * `AgentContext` extractor behaves symmetrically
//! * `generate_token_full` round-trips admin claims
//!
//! What is **not** covered here (gated on a test PostgreSQL):
//! * `require_tenant_access` / `require_identity_access` / `tenant_filter_for_user`
//!   in `src/api/auth.rs` — these call into `PermissionService`, which requires
//!   a live DB connection. They are listed as `#[ignore]` below with
//!   `#[ignore = "requires test PostgreSQL instance"]` so a follow-up
//!   contributor can flip the flag once a `DATABASE_URL` is provisioned for CI.
//!
//! The full end-to-end isolation matrix (24 cases per the plan) is tracked
//! under `docs/superpowers/specs/2026-06-03-tenant-scope-guard-design.md`
//! and is deferred to a follow-up that adds a test DB.

use axum::extract::FromRequestParts;
use axum::http::Request;
use uuid::Uuid;

use aion_hive::api::error::ApiError;
use aion_hive::api::jwt::{generate_token_full, verify_token, AdminUser, AgentContext};

mod common;

// ============================================================================
// helpers
// ============================================================================

fn make_request(auth_header: Option<&str>) -> axum::http::request::Parts {
    let mut builder = Request::builder();
    if let Some(h) = auth_header {
        builder = builder.header("Authorization", h);
    }
    let req = builder.body(()).expect("build test request");
    let (parts, _body) = req.into_parts();
    parts
}

// ============================================================================
// AdminUser extractor
// ============================================================================

#[tokio::test]
async fn admin_user_extractor_accepts_valid_admin_token() {
    let identity = Uuid::new_v4();
    let token = generate_token_full("admin-user", Some(identity), true, &["admin"], &[]).unwrap();
    let mut parts = make_request(Some(&format!("Bearer {}", token)));

    let admin = AdminUser::from_request_parts(&mut parts, &())
        .await
        .expect("admin token should be accepted");

    assert_eq!(admin.identity_id, identity);
    assert_eq!(admin.subject, "admin-user");
    assert_eq!(admin.roles, vec!["admin".to_string()]);
}

#[tokio::test]
async fn admin_user_extractor_rejects_agent_token() {
    let token = generate_token_full("agent-1", None, false, &[], &[]).unwrap();
    let mut parts = make_request(Some(&format!("Bearer {}", token)));

    let result = AdminUser::from_request_parts(&mut parts, &()).await;
    match result {
        Err(ApiError::Unauthorized(msg)) => {
            assert_eq!(msg, "Admin token required");
        }
        other => panic!(
            "expected Unauthorized(\"Admin token required\"), got {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn admin_user_extractor_rejects_admin_token_without_identity_id() {
    let token = generate_token_full("admin-no-id", None, true, &["admin"], &[]).unwrap();
    let mut parts = make_request(Some(&format!("Bearer {}", token)));

    let result = AdminUser::from_request_parts(&mut parts, &()).await;
    match result {
        Err(ApiError::Unauthorized(msg)) => {
            assert_eq!(msg, "Identity not bound to token");
        }
        other => panic!(
            "expected Unauthorized(\"Identity not bound to token\"), got {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn admin_user_extractor_rejects_missing_authorization_header() {
    let mut parts = make_request(None);
    let result = AdminUser::from_request_parts(&mut parts, &()).await;
    match result {
        Err(ApiError::Unauthorized(msg)) => {
            assert_eq!(msg, "Missing Authorization header");
        }
        other => panic!(
            "expected Unauthorized(\"Missing Authorization header\"), got {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn admin_user_extractor_rejects_malformed_authorization_header() {
    let mut parts = make_request(Some("NotBearer some-token"));
    let result = AdminUser::from_request_parts(&mut parts, &()).await;
    match result {
        Err(ApiError::Unauthorized(msg)) => {
            assert_eq!(msg, "Invalid Authorization header format");
        }
        other => panic!(
            "expected Unauthorized(\"Invalid Authorization header format\"), got {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn admin_user_extractor_rejects_invalid_token_signature() {
    let mut parts = make_request(Some("Bearer not-a-real-jwt"));
    let result = AdminUser::from_request_parts(&mut parts, &()).await;
    match result {
        Err(ApiError::Unauthorized(_)) => {}
        other => panic!("expected Unauthorized(_), got {:?}", other),
    }
}

// ============================================================================
// AgentContext extractor
// ============================================================================

#[tokio::test]
async fn agent_context_extractor_accepts_valid_token() {
    let token = generate_token_full(
        "agent-1",
        Some(Uuid::new_v4()),
        false,
        &["member"],
        &["read"],
    )
    .unwrap();
    let mut parts = make_request(Some(&format!("Bearer {}", token)));

    let ctx = AgentContext::from_request_parts(&mut parts, &())
        .await
        .expect("agent token should be accepted");

    assert_eq!(ctx.subject, "agent-1");
    assert_eq!(ctx.roles, vec!["member".to_string()]);
    assert_eq!(ctx.scope, vec!["read".to_string()]);
}

#[tokio::test]
async fn agent_context_extractor_accepts_admin_token() {
    let identity = Uuid::new_v4();
    let token = generate_token_full("admin-user", Some(identity), true, &["admin"], &[]).unwrap();
    let mut parts = make_request(Some(&format!("Bearer {}", token)));

    let ctx = AgentContext::from_request_parts(&mut parts, &())
        .await
        .expect("admin token should also be accepted by AgentContext");

    assert_eq!(ctx.subject, "admin-user");
    assert_eq!(ctx.roles, vec!["admin".to_string()]);
}

#[tokio::test]
async fn agent_context_extractor_rejects_missing_authorization_header() {
    let mut parts = make_request(None);
    let result = AgentContext::from_request_parts(&mut parts, &()).await;
    match result {
        Err(ApiError::Unauthorized(msg)) => {
            assert_eq!(msg, "Missing Authorization header");
        }
        other => panic!(
            "expected Unauthorized(\"Missing Authorization header\"), got {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn agent_context_extractor_rejects_malformed_authorization_header() {
    let mut parts = make_request(Some("NotBearer some-token"));
    let result = AgentContext::from_request_parts(&mut parts, &()).await;
    match result {
        Err(ApiError::Unauthorized(msg)) => {
            assert_eq!(msg, "Invalid Authorization header format");
        }
        other => panic!(
            "expected Unauthorized(\"Invalid Authorization header format\"), got {:?}",
            other
        ),
    }
}

// ============================================================================
// generate_token_full claim semantics (regression coverage for fix 3f77db5)
// ============================================================================

#[test]
fn generate_token_full_admin_token_preserves_identity_id_and_is_admin() {
    let identity = Uuid::new_v4();
    let token =
        generate_token_full("admin-1", Some(identity), true, &["admin"], &["write"]).unwrap();
    let claims = verify_token(&token).expect("token must verify");
    assert_eq!(claims.subject, "admin-1");
    assert_eq!(claims.identity_id, Some(identity));
    assert!(claims.is_admin);
    assert_eq!(claims.roles, vec!["admin".to_string()]);
    assert_eq!(claims.scope, vec!["write".to_string()]);
}

#[test]
fn generate_token_full_agent_token_omits_identity_id_and_is_admin() {
    let token = generate_token_full("agent-1", None, false, &[], &[]).unwrap();
    let claims = verify_token(&token).expect("token must verify");
    assert_eq!(claims.subject, "agent-1");
    assert!(claims.identity_id.is_none());
    assert!(!claims.is_admin);
    assert!(claims.roles.is_empty());
    assert!(claims.scope.is_empty());
}

// ============================================================================
// DB-backed isolation cases (skipped — require test PostgreSQL)
//
// These mirror the full end-to-end matrix from the plan. They are written
// as `#[ignore]` so `cargo test --test admin_isolation` succeeds in this
// environment, and so a follow-up that provisions `DATABASE_URL` for CI can
// flip the flag without touching the test file.
// ============================================================================

#[ignore = "requires test PostgreSQL instance — see spec docs/superpowers/specs/2026-06-03-tenant-scope-guard-design.md"]
#[tokio::test]
async fn require_tenant_access_denies_cross_tenant_user() {
    // Setup: tenants T1, T2; user A has membership in T1 only.
    // Action: require_tenant_access(state, A, T2.id)
    // Expect: Err(ApiError::Forbidden("Not a member of this tenant"))
    let _ = (common::create_test_temp_dir(), Uuid::new_v4());
    unimplemented!("requires test DB")
}

#[ignore = "requires test PostgreSQL instance"]
#[tokio::test]
async fn require_tenant_access_allows_same_tenant_user() {
    // Setup: tenants T1; user A has membership in T1.
    // Action: require_tenant_access(state, A, T1.id)
    // Expect: Ok(())
    unimplemented!("requires test DB")
}

#[ignore = "requires test PostgreSQL instance"]
#[tokio::test]
async fn require_tenant_access_allows_super_admin() {
    // Setup: super_admin user S (no membership in target tenant T1).
    // Action: require_tenant_access(state, S, T1.id)
    // Expect: Ok(())
    unimplemented!("requires test DB")
}

#[ignore = "requires test PostgreSQL instance"]
#[tokio::test]
async fn tenant_filter_for_user_returns_all_tenants_for_super_admin() {
    // Setup: super_admin S.
    // Action: tenant_filter_for_user(state, S)
    // Expect: (true, Vec::new()) — caller does not apply tenant filter
    unimplemented!("requires test DB")
}

#[ignore = "requires test PostgreSQL instance"]
#[tokio::test]
async fn tenant_filter_for_user_returns_only_memberships_for_regular_user() {
    // Setup: user A in T1, T2; not in T3.
    // Action: tenant_filter_for_user(state, A)
    // Expect: (false, vec![T1, T2])
    unimplemented!("requires test DB")
}

#[ignore = "requires test PostgreSQL instance"]
#[tokio::test]
async fn require_identity_access_denies_user_with_no_shared_tenant() {
    // Setup: identity X in T1 only; user A in T2 only.
    // Action: require_identity_access(state, A, X.identity_id)
    // Expect: Err(ApiError::Forbidden("Not authorized to access this identity"))
    unimplemented!("requires test DB")
}

#[ignore = "requires test PostgreSQL instance"]
#[tokio::test]
async fn require_identity_access_allows_super_admin() {
    // Setup: super_admin S; identity X in T1 only.
    // Action: require_identity_access(state, S, X.identity_id)
    // Expect: Ok(())
    unimplemented!("requires test DB")
}

#[ignore = "requires test PostgreSQL instance"]
#[tokio::test]
async fn require_identity_access_allows_user_in_shared_tenant() {
    // Setup: identity X in T1; user A also in T1.
    // Action: require_identity_access(state, A, X.identity_id)
    // Expect: Ok(())
    unimplemented!("requires test DB")
}
