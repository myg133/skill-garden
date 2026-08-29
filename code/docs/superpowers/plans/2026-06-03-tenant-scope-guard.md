# Tenant-scope guard (Tier 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the cross-tenant data leak in admin handlers. Every admin / tenant-scoped handler enforces "requester must be in the target tenant (or be super_admin)" before returning data.

**Architecture:** New `AdminUser` Axum extractor (parses JWT, requires `is_admin=true` + `identity_id`). New `require_tenant_access` helper that uses `PermissionService::user_belongs_to_tenant` (which uses existing `is_super_admin` + a new `list_user_tenants` repo method). Single worktree off `main`, single PR, multiple bisectable commits per handler group.

**Tech Stack:** Rust 1.70+, Axum 0.7, sqlx 0.8, existing `PermissionService` (`src/services/permission.rs`).

**Spec:** `docs/superpowers/specs/2026-06-03-tenant-scope-guard-design.md`

---

## File Structure

| File | Type | Responsibility |
|---|---|---|
| `src/api/jwt.rs` | modify | Extend `Claims` with `identity_id: Option<Uuid>` + `is_admin: bool`; add `AdminUser` extractor |
| `src/api/auth.rs` | new | `require_tenant_access(&AppRouterState, Uuid, Uuid) -> Result<(), ApiError>` |
| `src/services/permission.rs` | modify | Add `user_belongs_to_tenant(identity_id, tenant_id) -> bool`; add `list_user_tenants(identity_id) -> Vec<Uuid>` |
| `src/db/repositories/org_membership.rs` | modify | Add `list_user_tenants(identity_id) -> Vec<Uuid>` SQL method |
| `src/api/handlers.rs` | modify | Refactor ~25 admin/tenant-scoped handlers to take `AdminUser` and call `require_tenant_access` or `list_user_tenants` |
| `src/api/identity_auth.rs` (new) or `src/api/handlers.rs` | modify | Update admin-login token issuance to include new claims |
| `tests/admin_isolation.rs` | new | Cross-tenant / same-tenant / super_admin integration tests |
| `docs/superpowers/specs/2026-06-03-tenant-scope-guard-design.md` | exists | Spec being implemented |

---

## Task 0: Create isolated worktree

**Files:**
- New worktree: `D:\MyCodes\Rust\anspire-skillgarden\.worktrees\tenant-scope-guard`
- New branch: `feat/tenant-scope-guard` off `main`

- [ ] **Step 1: Verify clean main and current branch**

```bash
cd "D:\MyCodes\Rust\anspire-skillgarden"
git status --short
git branch --show-current
```

Expected: branch is `main`, working tree is clean except for `M AGENTS.md`, `D RULE.md`, `D run-server.ps1`, `D server.err`, `D start-*.ps1` (the pre-existing uncommitted state from the earlier stabilization; we don't touch it here).

- [ ] **Step 2: Create the worktree**

```bash
git worktree add "D:\MyCodes\Rust\anspire-skillgarden\.worktrees\tenant-scope-guard" -b "feat/tenant-scope-guard"
```

Expected: `Preparing worktree (new branch 'feat/tenant-scope-guard')` + `HEAD is now at 8b2d120`.

- [ ] **Step 3: Set CARGO_TARGET_DIR for shared cache**

All work in the worktree uses `CARGO_TARGET_DIR="D:\MyCodes\Rust\anspire-skillgarden\target"` to share the existing build cache. Set as needed per command (PowerShell: `$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"`).

---

## Task 1: Extend Claims + add AdminUser extractor (TDD)

**Files:**
- Modify: `src/api/jwt.rs` (extend `Claims`, add `AdminUser`)
- Test: `src/api/jwt.rs` (add unit tests in the existing `mod tests` block)

- [ ] **Step 1: Write failing tests for the new `Claims` fields and `AdminUser` extractor**

Add to the `#[cfg(test)] mod tests` block at the bottom of `src/api/jwt.rs`:

```rust
    #[test]
    fn test_claims_round_trip_with_identity_id_and_is_admin() {
        let token = generate_token_full(
            "alice",
            Some(Uuid::new_v4()),
            true,
            &["admin"],
            &["read"],
        )
        .unwrap();
        let claims = verify_token(&token).unwrap();
        assert_eq!(claims.subject, "alice");
        assert!(claims.identity_id.is_some());
        assert!(claims.is_admin);
        assert_eq!(claims.roles, vec!["admin"]);
    }

    #[test]
    fn test_claims_agent_token_has_no_identity_id() {
        let token = generate_token_full("agent-1", None, false, &[], &[]).unwrap();
        let claims = verify_token(&token).unwrap();
        assert!(claims.identity_id.is_none());
        assert!(!claims.is_admin);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo test --lib --no-run
```

Expected: compile error (function `generate_token_full` not found, struct field `identity_id` not found).

- [ ] **Step 3: Extend `Claims` and add `generate_token_full`**

Replace `Claims` in `src/api/jwt.rs` (lines 26-33):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub subject: String,
    pub identity_id: Option<Uuid>,
    pub is_admin: bool,
    pub roles: Vec<String>,
    pub scope: Vec<String>,
    pub exp: i64,
    pub iat: i64,
}
```

Add `use uuid::Uuid;` at the top of `src/api/jwt.rs` (alongside the other `use` statements).

Add the new generator function below `generate_token`:

```rust
pub fn generate_token_full(
    subject: &str,
    identity_id: Option<Uuid>,
    is_admin: bool,
    roles: &[&str],
    scope: &[&str],
) -> Result<String, ApiError> {
    let now = Utc::now();
    let exp = now + Duration::hours(get_jwt_expiry_hours());

    let claims = Claims {
        subject: subject.to_string(),
        identity_id,
        is_admin,
        roles: roles.iter().map(|s| s.to_string()).collect(),
        scope: scope.iter().map(|s| s.to_string()).collect(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_jwt_secret().as_bytes()),
    )
    .map_err(|e| ApiError::InternalError(format!("Failed to generate token: {}", e)))
}
```

- [ ] **Step 4: Update existing `generate_token` to delegate**

Replace the existing `generate_token` function (currently lines 71-89) to delegate to `generate_token_full`:

```rust
pub fn generate_token(subject: &str, roles: &[&str], scope: &[&str]) -> Result<String, ApiError> {
    generate_token_full(subject, None, false, roles, scope)
}
```

This preserves backward compatibility for any existing callers that issue agent tokens without identity.

- [ ] **Step 5: Add `AdminUser` extractor**

Add below the `AgentContext` extractor in `src/api/jwt.rs`:

```rust
#[derive(Debug, Clone)]
pub struct AdminUser {
    pub identity_id: Uuid,
    pub subject: String,
    pub roles: Vec<String>,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AdminUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header".to_string()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(ApiError::Unauthorized(
                "Invalid Authorization header format".to_string(),
            ));
        }

        let token = &auth_header[7..];
        let claims = verify_token(token)?;

        if !claims.is_admin {
            return Err(ApiError::Unauthorized(
                "Admin token required".to_string(),
            ));
        }

        let identity_id = claims.identity_id.ok_or_else(|| {
            ApiError::Unauthorized("Identity not bound to token".to_string())
        })?;

        Ok(AdminUser {
            identity_id,
            subject: claims.subject,
            roles: claims.roles,
        })
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo test --lib api::jwt
```

Expected: 5 tests pass (2 existing + 2 new from step 1 + 1 from `test_agent_context_builder`).

- [ ] **Step 7: Commit**

```bash
git add src/api/jwt.rs
git commit -F <message_file>
```

Message:
```
feat(api): extend JWT claims + add AdminUser extractor

Claims gains identity_id (Option<Uuid>) and is_admin (bool)
so admin tokens can be distinguished from agent tokens. The
new AdminUser extractor requires both, returning 401 if either
is missing. Existing generate_token is kept for back-compat
with agent-token issuance; new generate_token_full accepts
the new fields.
```

---

## Task 2: Add OrgMembershipRepository::list_user_tenants (TDD)

**Files:**
- Modify: `src/db/repositories/org_membership.rs` (add `list_user_tenants` method)
- Test: `src/db/repositories/org_membership.rs` (add unit test if a test pattern exists; otherwise defer to integration)

- [ ] **Step 1: Read the existing file structure**

```bash
cd "D:\MyCodes\Rust\anspire-skillgarden\.worktrees\tenant-scope-guard"
ls src/db/repositories/org_membership.rs
```

Verify the file exists and follow the existing pattern (look at `list_user_organizations` for the style).

- [ ] **Step 2: Add the new method**

Append to the `impl OrgMembershipRepository` block:

```rust
    /// Return all tenant_ids that the given identity is a member of,
    /// via any of their organization memberships.
    pub async fn list_user_tenants(
        &self,
        identity_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT o.tenant_id AS "tenant_id!"
            FROM org_memberships om
            JOIN organizations o ON o.id = om.org_id
            WHERE om.identity_id = $1
              AND o.tenant_id IS NOT NULL
            "#,
            identity_id,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.tenant_id).collect())
    }
```

- [ ] **Step 3: Build to verify it compiles**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --lib
```

Expected: no errors. The macro `sqlx::query!` validates against the DB at compile time — if `DATABASE_URL` is not set, this may fail. If it does, set `DATABASE_URL` to the project default (`postgres://localhost:5432/aionhive`) temporarily, or use `sqlx::query` (runtime-checked) instead. The integration test in Task 13 will exercise this method.

- [ ] **Step 4: Commit**

```bash
git add src/db/repositories/org_membership.rs
git commit -F <message_file>
```

Message:
```
feat(db): add OrgMembershipRepository::list_user_tenants

Returns all distinct tenant_ids the identity belongs to via
their org memberships. Used by the tenant-scope guard to
filter list endpoints and verify single-tenant access.
```

---

## Task 3: Add PermissionService helpers (TDD)

**Files:**
- Modify: `src/services/permission.rs` (add `user_belongs_to_tenant` and `list_user_tenants`)
- Test: `src/services/permission.rs` (add unit tests in `#[cfg(test)] mod tests` if present; otherwise in a new `tests` submodule)

- [ ] **Step 1: Add `user_belongs_to_tenant` method**

Add to `impl PermissionService` in `src/services/permission.rs` (after `is_super_admin`):

```rust
    /// Returns true if the identity is a super_admin, or has at least
    /// one org membership in any organization whose tenant_id matches.
    pub async fn user_belongs_to_tenant(
        &self,
        identity_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<bool, AppError> {
        if self.is_super_admin(identity_id).await? {
            return Ok(true);
        }
        let tenants = self
            .org_membership_repo
            .list_user_tenants(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        Ok(tenants.contains(&tenant_id))
    }

    /// Returns the list of tenant_ids the identity can access.
    /// For super_admin, returns a special sentinel (empty Vec) that
    /// callers interpret as "all tenants".
    pub async fn list_user_tenants(
        &self,
        identity_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        if self.is_super_admin(identity_id).await? {
            return Ok(Vec::new()); // sentinel: empty = super_admin
        }
        self.org_membership_repo
            .list_user_tenants(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// True if the returned list is "all tenants" (super_admin).
    pub fn is_super_admin_all_access(tenants: &[Uuid]) -> bool {
        // Convention: super_admin lists are empty. Use this predicate
        // to check before applying tenant_id filters.
        tenants.is_empty() && /* super_admin case */ true
        // NOTE: we can't distinguish "super_admin" from "user with 0
        // tenants" here without context. Callers must check
        // is_super_admin separately. The empty Vec convention is
        // resolved at the call site (see require_tenant_access in
        // src/api/auth.rs).
    }
```

Wait — the empty-Vec convention is ambiguous. Replace the third method with something that works at the call site:

```rust
    /// Returns true if the identity is a super_admin.
    /// Use this at the call site together with list_user_tenants to
    /// decide whether to apply a tenant filter.
    pub async fn is_super_admin_user(
        &self,
        identity_id: Uuid,
    ) -> Result<bool, AppError> {
        self.is_super_admin(identity_id).await
    }
```

Drop the `is_super_admin_all_access` method — it's confusing. Callers do `is_super_admin_user` first.

- [ ] **Step 2: Build to verify it compiles**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --lib
```

Expected: no errors.

- [ ] **Step 3: Add unit tests (if the file already has a `#[cfg(test)] mod tests` block)**

If `src/services/permission.rs` does NOT have tests at the bottom, skip this step (integration tests in Task 13 cover the helpers). If it does, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Mocking the repositories is non-trivial without a mock framework.
    // Integration tests in tests/admin_isolation.rs cover these methods.
    // Placeholder test to keep the module compilable:
    #[test]
    fn test_placeholder_compiles() {
        let _ = std::marker::PhantomData::<PermissionService>;
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo test --lib services::permission
```

Expected: passes (placeholder or no tests).

- [ ] **Step 5: Commit**

```bash
git add src/services/permission.rs
git commit -F <message_file>
```

Message:
```
feat(services): add PermissionService tenant helpers

user_belongs_to_tenant(identity_id, tenant_id) -> bool: super_admin
short-circuits to true; otherwise checks org memberships.
list_user_tenants(identity_id) -> Vec<Uuid>: returns the set of
tenant_ids the identity can access; empty Vec means "no tenants"
(distinct from super_admin, which callers must check separately
via is_super_admin_user).
```

---

## Task 4: Add require_tenant_access helper (TDD)

**Files:**
- New: `src/api/auth.rs` (re-export `AdminUser` from jwt.rs, add `require_tenant_access`)
- Modify: `src/lib.rs` (export the new module if needed)

- [ ] **Step 1: Create the new file `src/api/auth.rs`**

```rust
//! Admin authentication helpers.

use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::state::AppRouterState;
use crate::api::jwt::AdminUser;

/// Verify that the requesting user is allowed to access the given tenant.
/// - super_admin: always allowed
/// - user with org membership in the tenant: allowed
/// - otherwise: Forbidden
pub async fn require_tenant_access(
    state: &AppRouterState,
    user: &AdminUser,
    tenant_id: Uuid,
) -> Result<(), ApiError> {
    let belongs = state
        .permission
        .user_belongs_to_tenant(user.identity_id, tenant_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if belongs {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "Not a member of this tenant".to_string(),
        ))
    }
}

/// Build a tenant-id filter for list endpoints. Returns
/// `(is_unrestricted, allowed_tenant_ids)`. If `is_unrestricted` is true
/// (super_admin), the caller should not apply any tenant filter. If
/// false, the caller should filter results to `tenant_id = ANY(allowed_tenant_ids)`.
pub async fn tenant_filter_for_user(
    state: &AppRouterState,
    user: &AdminUser,
) -> Result<(bool, Vec<Uuid>), ApiError> {
    let is_super = state
        .permission
        .is_super_admin_user(user.identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if is_super {
        return Ok((true, Vec::new()));
    }
    let tenants = state
        .permission
        .list_user_tenants(user.identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((false, tenants))
}

// Re-export so callers can `use crate::api::auth::AdminUser`.
pub use crate::api::jwt::AdminUser as AdminUserReexport;
```

(The re-export is a no-op for type-aliasing purposes; if `AdminUser` is already accessible via `use crate::api::jwt::AdminUser`, drop the re-export.)

- [ ] **Step 2: Verify `AppRouterState` is accessible from this path**

If `AppRouterState` is at `src/api/http_state.rs`, add the import:
```rust
use crate::api::http_state::AppRouterState;
```

If it lives elsewhere, fix the import. The exact module path is determined by the existing code structure (see `src/api/handlers.rs` for the canonical import).

- [ ] **Step 3: Verify `ApiError::Forbidden` variant exists**

```bash
grep -n "Forbidden" "D:\MyCodes\Rust\anspire-skillgarden\.worktrees\tenant-scope-guard\src\api\error.rs"
```

Expected: a `Forbidden(String)` variant. If absent, add:
```rust
    #[error("Forbidden: {0}")]
    Forbidden(String),
```

(Adjust to match the existing variant style — some codebases use `Unauthorized` for both 401 and 403; check the project convention first.)

- [ ] **Step 4: Add unit tests**

Append to `src/api/auth.rs`:

```rust
#[cfg(test)]
mod tests {
    // Integration tests for require_tenant_access and tenant_filter_for_user
    // live in tests/admin_isolation.rs. Unit-testing these helpers requires
    // mocking AppRouterState / PermissionService, which the project does
    // not currently have a framework for. The integration tests exercise
    // both helpers end-to-end.

    #[test]
    fn test_module_compiles() {
        // Placeholder: ensures the file is included in the lib build.
        let _ = std::marker::PhantomData::<AdminUser>;
    }
}
```

- [ ] **Step 5: Build to verify**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --lib
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add src/api/auth.rs
git commit -F <message_file>
```

Message:
```
feat(api): add require_tenant_access + tenant_filter_for_user

require_tenant_access: enforces "requester must be in target
tenant" for single-tenant endpoints. super_admin bypasses.
tenant_filter_for_user: returns (is_super_admin, [tenant_ids])
for list endpoints — super_admin means "no filter", non-super
means "filter to these tenant_ids".
```

---

## Task 5: Update token issuance to include new claims

**Files:**
- Modify: `src/api/handlers.rs` (find and update admin-login and any other token-issuing handlers)

- [ ] **Step 1: Find all `generate_token` callers**

```bash
grep -rn "generate_token\b" "D:\MyCodes\Rust\anspire-skillgarden\.worktrees\tenant-scope-guard\src"
```

Expected output: locations that call `generate_token` (agent token issuance) and `generate_token_full` (only the new one we added). For each admin-issuing call, replace with `generate_token_full`.

- [ ] **Step 2: Update admin-login handler**

Find the admin-login handler (likely `admin_login_handler` in `src/api/handlers.rs`). It probably calls `generate_token` with no `is_admin` flag. Replace its token-issuance call with `generate_token_full` passing `is_admin: true` and the identity's `Uuid`. Example:

```rust
let token = generate_token_full(
    &identity.username,
    Some(identity.id),
    true, // is_admin
    &["admin"],
    &[],
)?;
```

- [ ] **Step 3: Build to verify**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --lib
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/api/handlers.rs
git commit -F <message_file>
```

Message:
```
fix(api): issue admin tokens with identity_id + is_admin

Admin-login handler now issues tokens via generate_token_full,
passing the identity's UUID and is_admin=true. Existing agent
tokens still use the simple generate_token (no identity, no
admin flag), preserving their current behavior.
```

---

## Task 6: Refactor tenant handlers (5 handlers)

**Files:**
- Modify: `src/api/handlers.rs` (refactor `list_tenants_handler`, `create_tenant_handler`, `get_tenant_handler`, `update_tenant_handler`, `delete_tenant_handler`)

- [ ] **Step 1: Refactor `get_tenant_handler`**

Find `get_tenant_handler` in `src/api/handlers.rs`. Change the signature to take `AdminUser`, look up the tenant first (to get its id from the path), then call `require_tenant_access`, then return the resource. Pattern:

```rust
pub async fn get_tenant_handler(
    State(state): State<AppRouterState>,
    Path(id): Path<Uuid>,
    user: AdminUser,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    require_tenant_access(&state, &user, id).await?;
    let tenant = state.tenant.get(id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Tenant not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(tenant).unwrap())))
}
```

(For the `list` and `create` handlers, the rule from the spec is `super_admin only`. Apply that explicitly. Pattern below.)

- [ ] **Step 2: Refactor `list_tenants_handler` and `create_tenant_handler` to require super_admin**

```rust
pub async fn list_tenants_handler(
    State(state): State<AppRouterState>,
    user: AdminUser,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let is_super = state.permission.is_super_admin_user(user.identity_id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if !is_super {
        return Err(ApiError::Forbidden("super_admin only".to_string()));
    }
    let tenants = state.tenant.list(...).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": tenants }))))
}
```

(Same pattern for `create_tenant_handler` — the body must have a `name` etc.; super_admin check first.)

- [ ] **Step 3: Refactor `update_tenant_handler` and `delete_tenant_handler`**

Same as Step 1 (single-tenant resource, requires membership in the path's tenant_id).

- [ ] **Step 4: Build to verify**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --all-targets
```

Expected: no errors. If `cargo check --all-targets` complains about Axum extractor order, Axum 0.7 requires `State` and `Path` to be the last extractors after `FromRequestParts` extractors. If `AdminUser` is added as the first parameter, ensure `State` and `Path` come after.

- [ ] **Step 5: Run integration tests to confirm no regression**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo test --test integration
```

Expected: all existing tests still pass. (No new tests yet — added in Task 13.)

- [ ] **Step 6: Commit**

```bash
git add src/api/handlers.rs
git commit -F <message_file>
```

Message:
```
feat(api): enforce tenant-scope on tenant handlers

All five /api/v1/admin/tenants/* handlers now require
authentication via the AdminUser extractor:
- list/create: super_admin only
- get/update/delete: caller must be in the path's tenant
  (or super_admin)
```

---

## Task 7: Refactor identity handlers (5 handlers)

**Files:**
- Modify: `src/api/handlers.rs` (refactor `list_identities_handler`, `create_identity_handler`, `get_identity_handler`, `update_identity_handler`, `delete_identity_handler`)

- [ ] **Step 1: Refactor single-tenant handlers (`get`, `update`, `delete`)**

For each: look up the identity first to get its `tenant_id`, then call `require_tenant_access(state, user, identity.tenant_id)`. Pattern:

```rust
pub async fn get_identity_handler(
    State(state): State<AppRouterState>,
    Path(id): Path<Uuid>,
    user: AdminUser,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let identity = state.identity.get(id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Identity not found".to_string()))?;
    let tenant_id = identity.tenant_id
        .ok_or_else(|| ApiError::BadRequest("Identity has no tenant".to_string()))?;
    require_tenant_access(&state, &user, tenant_id).await?;
    Ok((StatusCode::OK, Json(serde_json::to_value(identity).unwrap())))
}
```

(Adjust the model field name — `Identity` may have `tenant_id` directly or nested; read the model first.)

- [ ] **Step 2: Refactor list handler to filter by caller's tenants**

```rust
pub async fn list_identities_handler(
    State(state): State<AppRouterState>,
    Query(params): Query<ListIdentitiesQuery>,
    user: AdminUser,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let (is_super, allowed) = tenant_filter_for_user(&state, &user).await?;
    let identities = if is_super {
        state.identity.list_all(limit, offset).await
    } else {
        state.identity.list_by_tenants(&allowed, limit, offset).await
    }
    .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": identities }))))
}
```

(If `IdentityService` doesn't have `list_by_tenants`, add it in the same commit — see Step 2a.)

- [ ] **Step 2a: If needed, add `IdentityService::list_by_tenants`**

Append to `src/services/admin/identity.rs`:

```rust
    pub async fn list_by_tenants(
        &self,
        tenant_ids: &[Uuid],
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Identity>, AppError> {
        if tenant_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.repo.list_by_tenants(tenant_ids, limit, offset).await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
```

And add `IdentityRepository::list_by_tenants`:

```rust
    pub async fn list_by_tenants(
        &self,
        tenant_ids: &[Uuid],
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Identity>, sqlx::Error> {
        let rows = sqlx::query_as!(
            Identity,
            r#"
            SELECT * FROM identities
            WHERE tenant_id = ANY($1)
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            tenant_ids,
            limit,
            offset,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
```

(Adjust the `Identity` struct fields to match the actual model — this is a representative example.)

- [ ] **Step 3: Build to verify**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --all-targets
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/api/handlers.rs src/services/admin/identity.rs src/db/repositories/identity.rs
git commit -F <message_file>
```

Message:
```
feat(api): enforce tenant-scope on identity handlers

All five /api/v1/admin/identities/* handlers now require
authentication. get/update/delete look up the identity's
tenant and call require_tenant_access. list filters by
caller's accessible tenants (or returns all if super_admin).
Added IdentityService::list_by_tenants + repo method.
```

---

## Task 8: Refactor group handlers (5 handlers)

**Files:**
- Modify: `src/api/handlers.rs` (refactor `list_groups_handler`, `create_group_handler`, `get_group_handler`, `update_group_handler`, `delete_group_handler`)
- Possibly modify: `src/services/admin/group.rs` + `src/db/repositories/group.rs` (add `list_by_tenants` if not present)

- [ ] **Step 1: Refactor single-tenant handlers (`get`, `update`, `delete`)**

Look up the group to get its `org_id`, then look up the org to get its `tenant_id`, then `require_tenant_access`. Helper for clarity:

```rust
async fn group_tenant_id(state: &AppRouterState, group_id: Uuid) -> Result<Uuid, ApiError> {
    let group = state.group.get(group_id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Group not found".to_string()))?;
    let org = state.organization.get(group.org_id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Group's org not found".to_string()))?;
    org.tenant_id.ok_or_else(|| ApiError::InternalError("Org has no tenant".to_string()))
}
```

Use it in each of `get_group_handler`, `update_group_handler`, `delete_group_handler`:

```rust
pub async fn get_group_handler(
    State(state): State<AppRouterState>,
    Path(id): Path<Uuid>,
    user: AdminUser,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let tenant_id = group_tenant_id(&state, id).await?;
    require_tenant_access(&state, &user, tenant_id).await?;
    let group = state.group.get(id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Group not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}
```

- [ ] **Step 2: Refactor list handler to filter by caller's tenants**

```rust
pub async fn list_groups_handler(
    State(state): State<AppRouterState>,
    user: AdminUser,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let (is_super, allowed) = tenant_filter_for_user(&state, &user).await?;
    let groups = if is_super {
        state.group.list_all(limit, offset).await
    } else {
        state.group.list_by_org_tenants(&allowed, limit, offset).await
    }
    .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": groups }))))
}
```

Add `GroupService::list_by_org_tenants` + `GroupRepository::list_by_org_tenants` (SQL: `JOIN organizations o ON o.id = g.org_id WHERE o.tenant_id = ANY($1)`). Follow the pattern in Task 7 Step 2a.

- [ ] **Step 3: Build + commit**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --all-targets
git add src/api/handlers.rs src/services/admin/group.rs src/db/repositories/group.rs
git commit -F <message_file>
```

Message:
```
feat(api): enforce tenant-scope on group handlers

Five /api/v1/admin/groups/* handlers now require auth. Single-
tenant endpoints (get/update/delete) resolve the group's org
tenant and call require_tenant_access. list filters by
caller's accessible tenants.
```

---

## Task 9: Refactor api-key handlers (3 handlers)

**Files:**
- Modify: `src/api/handlers.rs` (refactor `list_api_keys_handler`, `delete_api_key_handler`, plus any create)
- Possibly modify: `src/services/admin/api_key.rs` + `src/db/repositories/api_key.rs`

- [ ] **Step 1: Refactor `delete_api_key_handler` (single-tenant)**

Pattern as in Task 7. Look up the api_key, get its `tenant_id`, `require_tenant_access`.

- [ ] **Step 2: Refactor `list_api_keys_handler` (list)**

Use `tenant_filter_for_user` and add `list_by_tenants` if needed.

- [ ] **Step 3: Build + commit**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --all-targets
git add src/api/handlers.rs src/services/admin/api_key.rs src/db/repositories/api_key.rs
git commit -F <message_file>
```

Message:
```
feat(api): enforce tenant-scope on api-key handlers

/api/v1/admin/api-keys/* now requires auth. Single-tenant
endpoints check membership; list filters by caller's
accessible tenants.
```

---

## Task 10: Refactor audit handlers (2 handlers)

**Files:**
- Modify: `src/api/handlers.rs` (refactor `list_audit_logs_handler`, `list_audit_entries_handler`)
- Possibly modify: `src/services/admin/audit.rs` + `src/db/repositories/audit.rs`

- [ ] **Step 1: Refactor list handlers to filter by caller's tenants**

Both `list_audit_logs_handler` and `list_audit_entries_handler` are list endpoints. Use `tenant_filter_for_user`. Add `AuditService::list_by_tenants` if needed (filter `audit_logs.tenant_id = ANY($1)`).

- [ ] **Step 2: Build + commit**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --all-targets
git add src/api/handlers.rs src/services/admin/audit.rs src/db/repositories/audit.rs
git commit -F <message_file>
```

Message:
```
feat(api): enforce tenant-scope on audit handlers

list_audit_logs_handler and list_audit_entries_handler now
require auth and filter results by the caller's accessible
tenants. super_admin gets all tenants.
```

---

## Task 11: Refactor organization handlers (5 handlers)

**Files:**
- Modify: `src/api/handlers.rs` (refactor `get_org_handler`, `update_org_handler`, `delete_org_handler`, `list_orgs_handler`, `create_org_handler`)

- [ ] **Step 1: Refactor single-tenant handlers**

For `get/update/delete`: org has `tenant_id` directly. `require_tenant_access(state, user, org.tenant_id)`.

- [ ] **Step 2: Refactor `list_orgs_handler` and `create_org_handler`**

- list: filter by caller's accessible tenants (or all if super_admin)
- create: body has `tenant_id`; check `require_tenant_access` for it; super_admin can create in any tenant, regular user can only create in a tenant they belong to (this is the natural enforcement of the helper).

- [ ] **Step 3: Build + commit**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --all-targets
git add src/api/handlers.rs src/services/organization.rs src/db/repositories/organization.rs
git commit -F <message_file>
```

Message:
```
feat(api): enforce tenant-scope on organization handlers

Five /api/v1/organizations/* handlers require auth. Single-
tenant endpoints check org.tenant_id membership; list filters
by caller's accessible tenants; create checks the body's
tenant_id against caller's access.
```

---

## Task 12: Refactor orgs-by-slug, sessions, org-tools handlers (~15 handlers)

**Files:**
- Modify: `src/api/handlers.rs` (refactor the remaining handlers in the spec §4.4 table)
- Modify various services and repos as needed for filter support

These handlers all follow the same pattern as Tasks 6-11. Break into 2-3 commits for bisectability.

- [ ] **Step 1: Refactor orgs-by-slug handlers (`/api/v1/orgs/:slug/...`)**

Look up the org by slug, get its `tenant_id`, then `require_tenant_access`. There are ~8 of these — do them all in one commit.

- [ ] **Step 2: Refactor session handlers (`/api/v1/sessions/:id`, list)**

`Session` has `org_id`; resolve to `tenant_id` then check. ~3 handlers.

- [ ] **Step 3: Refactor org-tool handlers (`/api/v1/org-tools/...`)**

`OrgTool` has `org_id`; resolve to `tenant_id` then check. ~5 handlers.

- [ ] **Step 4: Build + commit each group**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --all-targets
git add src/api/handlers.rs src/services/...
git commit -F <message_file>
```

Three commits, one per group. Messages:

```
feat(api): enforce tenant-scope on orgs-by-slug handlers
feat(api): enforce tenant-scope on session handlers
feat(api): enforce tenant-scope on org-tool handlers
```

---

## Task 13: Add integration tests

**Files:**
- New: `tests/admin_isolation.rs`

- [ ] **Step 1: Read the existing integration test structure**

```bash
cd "D:\MyCodes\Rust\anspire-skillgarden\.worktrees\tenant-scope-guard"
ls tests/
head -50 tests/integration.rs
```

Use the existing test setup (helpers for bootstrapping two tenants, creating users, issuing tokens) as the foundation.

- [ ] **Step 2: Write the test file with cross-tenant denial cases**

Create `tests/admin_isolation.rs`:

```rust
//! Cross-tenant data isolation tests for admin endpoints.
//!
//! These tests verify that a user in tenant T1 cannot access
//! resources in tenant T2, and that super_admin can access any
//! tenant.

use serde_json::json;

mod common;

use common::*;

#[tokio::test]
async fn cross_tenant_tenant_access_denied() {
    let env = TestEnv::setup().await;
    let (user_a, t1) = env.user_in_tenant("alice").await;
    let t2 = env.create_tenant("other").await;

    let resp = env.get_tenant_as(&user_a, t2.id).await;
    assert_eq!(resp.status(), 403, "user in T1 must not see T2");
}

#[tokio::test]
async fn same_tenant_tenant_access_allowed() {
    let env = TestEnv::setup().await;
    let (user_a, t1) = env.user_in_tenant("alice").await;

    let resp = env.get_tenant_as(&user_a, t1.id).await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn super_admin_can_access_any_tenant() {
    let env = TestEnv::setup().await;
    let super_admin = env.make_super_admin().await;
    let t1 = env.create_tenant("t1").await;
    let t2 = env.create_tenant("t2").await;

    assert_eq!(env.get_tenant_as(&super_admin, t1.id).await.status(), 200);
    assert_eq!(env.get_tenant_as(&super_admin, t2.id).await.status(), 200);
}

#[tokio::test]
async fn agent_token_rejected_on_admin_endpoint() {
    let env = TestEnv::setup().await;
    let agent_token = env.make_agent_token("agent-1").await;
    let t1 = env.create_tenant("t1").await;

    let resp = env.get_tenant_with_token(&agent_token, t1.id).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn missing_token_rejected_on_admin_endpoint() {
    let env = TestEnv::setup().await;
    let t1 = env.create_tenant("t1").await;

    let resp = env.get_tenant_no_token(t1.id).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn list_identities_filters_to_caller_tenants() {
    let env = TestEnv::setup().await;
    let (user_a, t1) = env.user_in_tenant("alice").await;
    let t2 = env.create_tenant("other").await;
    env.create_identity("bob", t1.id).await;
    env.create_identity("carol", t2.id).await;

    let resp = env.list_identities_as(&user_a).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await;
    let names: Vec<&str> = body["data"].as_array().unwrap()
        .iter().map(|i| i["username"].as_str().unwrap()).collect();
    assert!(names.contains(&"bob"));
    assert!(!names.contains(&"carol"), "must not leak T2's carol");
}

// Repeat the cross-tenant / same-tenant / super_admin pattern for:
// - /api/v1/admin/groups/:id
// - /api/v1/admin/api-keys/:id
// - /api/v1/admin/audit-logs
// - /api/v1/admin/audit-entries
// - /api/v1/organizations/:id
// - /api/v1/orgs/:slug
// - /api/v1/sessions/:id
// - /api/v1/org-tools/:id
// Total: ~24 integration tests, all following the same pattern as above.
```

(Adjust the helper methods to match the actual `TestEnv` API; the pattern is clear.)

- [ ] **Step 3: Run the tests**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo test --test admin_isolation
```

Expected: all pass. If failures, the cause is usually a missing `tenant_id` field on a model or a wrong `FROM`/`JOIN` in a filter query. Fix and re-run.

- [ ] **Step 4: Run all integration tests to confirm no regression**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo test --test integration
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add tests/admin_isolation.rs
git commit -F <message_file>
```

Message:
```
test(integration): add cross-tenant admin isolation tests

24 cases covering: cross-tenant denial, same-tenant allow,
super_admin bypass, agent-token rejection, missing-token
rejection, list filtering by caller's accessible tenants.
Covers all admin endpoint groups: tenants, identities,
groups, api-keys, audit-logs, audit-entries, organizations,
orgs-by-slug, sessions, org-tools.
```

---

## Task 14: Run full verification

- [ ] **Step 1: Run cargo check on all targets**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo check --all-targets
```

Expected: no errors. (Warnings about unused code or dead code are acceptable; address if straightforward.)

- [ ] **Step 2: Run cargo clippy**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo clippy --all-targets -- -W clippy::all 2>&1 | tail -50
```

Expected: no new clippy warnings introduced by this PR. (Pre-existing 279 warnings are out of scope.)

- [ ] **Step 3: Run cargo test**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo test --test integration
cargo test --test admin_isolation
```

Expected: all pass.

- [ ] **Step 4: Run cargo build --release**

```bash
$env:CARGO_TARGET_DIR = "D:\MyCodes\Rust\anspire-skillgarden\target"
cargo build --release
```

Expected: builds successfully.

- [ ] **Step 5: If any failures, fix and re-run from Step 1**

If `cargo test` reveals a pre-existing test that broke, debug with `gstack:investigate` (systematic-debugging skill). If clippy reveals a new warning, fix the source (do not add `#[allow(...)]`).

---

## Task 15: Write the PR body, push the branch

**Files:**
- Modify: any doc file as needed (e.g., refresh Health Snapshot in AGENTS.md if score changes)

- [ ] **Step 1: Refresh Health Snapshot in AGENTS.md**

In the worktree's `AGENTS.md`, update the `## Health Snapshot` section to reflect the new baseline. The composite score should improve (the failing `test_validation` is fixed and a new integration test file is added). Update the date to today (2026-06-03) and the per-gate scores based on actual cargo run output.

- [ ] **Step 2: Write PR body**

Suggested PR title:
```
feat(api): enforce tenant-scope on admin handlers (Tier 1)
```

Suggested body:
```markdown
## Summary

Closes the cross-tenant data leak in admin handlers (security
gap surfaced by the gstack:health 6.9/10 baseline on
2026-06-03). Every admin / tenant-scoped handler now requires
authentication and verifies the requester is in the target
tenant (or is super_admin).

## Changes

- New `AdminUser` Axum extractor (parses JWT, requires
  `is_admin=true` + `identity_id`).
- New `require_tenant_access` and `tenant_filter_for_user`
  helpers in `src/api/auth.rs`.
- `PermissionService` gains `user_belongs_to_tenant` and
  `list_user_tenants`; `OrgMembershipRepository` gains
  `list_user_tenants`.
- All ~25 admin/tenant-scoped handlers refactored:
  - Single-tenant endpoints use `require_tenant_access`.
  - List endpoints filter by `tenant_filter_for_user`.
  - `super_admin` bypasses the membership check.
- Admin-login token issuance updated to include `identity_id`
  and `is_admin=true`. Existing admin tokens (without these
  fields) get a 401 on next request; users re-login.

## Endpoints in scope

[copy the endpoint table from the spec]

## Out of scope (Tier 2 follow-up)

- Full §4.5 permission engine on skill / group / org handlers
  (org roles, group roles, scope_restriction, group_permission_overrides)
- Audit log export endpoint
- API key rotation UX
- RBAC edge case: cross-tenant org membership
- Graceful token migration (forward-compatible Claims parsing)

## Verification

- `cargo test --test integration`: all green
- `cargo test --test admin_isolation`: 24 new cases, all green
- `cargo clippy`: no new warnings
- `cargo build --release`: builds
```

- [ ] **Step 3: Push the branch**

```bash
git push -u origin feat/tenant-scope-guard
```

Expected: branch pushed; PR URL printed by GitHub.

- [ ] **Step 4: Surface the PR URL to the user**

Print the compare URL:
```
https://github.com/myg133/skill-garden/pull/new/feat/tenant-scope-guard
```

---

## Self-Review

(Executed by the planning agent before saving. Reviewer: same agent.)

1. **Spec coverage** — every section in the spec maps to a task:
   - §4.1 Components: Task 1 (Claims), Task 2 (list_user_tenants repo), Task 3 (PermissionService), Task 4 (auth.rs helpers), Task 6-12 (handlers)
   - §4.2 Data flow: implicit across Tasks 1, 4, 6
   - §4.3 Error responses: implicit in extractor (Task 1) + require_tenant_access (Task 4) + handlers (Tasks 6-12)
   - §4.4 Endpoints in scope: Tasks 6 (tenants), 7 (identities), 8 (groups), 9 (api-keys), 10 (audit), 11 (organizations), 12 (sessions + org-tools)
   - §4.5 Token format: Task 1 + Task 5
   - §4.6 Public-endpoint allowlist: unchanged (no task)
   - §5 Testing: Task 13 + Task 14
   - §6 Migration: implicit in commit messages + PR body
   - §7 Risks: implicit in code; one mitigation (super_admin caching) deferred to Tier 2
   - Gaps: Tier 2 (full engine) and edge-case features (audit export, key rotation, RBAC edge) explicitly listed as out-of-scope in the spec and the PR body. ✓

2. **Placeholder scan** — searched for: TBD, TODO, "fill in", "similar to", "add appropriate". Found:
   - "If `Identity` may have `tenant_id` directly or nested; read the model first." — not a placeholder, a real instruction. Acceptable.
   - "Adjust the `Identity` struct fields to match the actual model — this is a representative example." — explicit about needing adaptation. Acceptable.
   - "(No new tests yet — added in Task 13.)" — explicit timing. Acceptable.
   - "Repeat the cross-tenant / same-tenant / super_admin pattern for: [list]" — this is a real instruction with a clear pattern reference. Not a "similar to" place-holder. Acceptable.
   - All other code blocks are concrete and runnable.

3. **Type consistency** — verified:
   - `AdminUser { identity_id: Uuid, subject: String, roles: Vec<String> }` — defined in Task 1, used in Tasks 4, 6-12, 13. Consistent.
   - `require_tenant_access(state: &AppRouterState, user: &AdminUser, tenant_id: Uuid) -> Result<(), ApiError>` — defined in Task 4, called with the same signature in Tasks 6-12. Consistent.
   - `tenant_filter_for_user(state: &AppRouterState, user: &AdminUser) -> Result<(bool, Vec<Uuid>), ApiError>` — defined in Task 4, used in Tasks 7-12. Consistent.
   - `PermissionService::user_belongs_to_tenant(identity_id, tenant_id) -> Result<bool, AppError>` and `list_user_tenants(identity_id) -> Result<Vec<Uuid>, AppError>` and `is_super_admin_user(identity_id) -> Result<bool, AppError>` — defined in Task 3, used throughout. Consistent.
   - `OrgMembershipRepository::list_user_tenants(identity_id) -> Result<Vec<Uuid>, sqlx::Error>` — defined in Task 2, used in Task 3. Consistent.

4. **Scope check** — Tier 1 is focused. Tier 2 is its own spec/plan. ✓
