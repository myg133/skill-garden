# Tenant-scope guard (Tier 1 of multi-tenant isolation)

| Field | Value |
|---|---|
| Date | 2026-06-03 |
| Status | Approved (pending written-spec review) |
| Tier | 1 of 2 (Tier 2 = full §4.5 permission engine on every handler) |
| Owner | (unassigned) |
| Branch target | `main` |
| Source | `gstack:health` 6.9/10 baseline + admin-handler audit (2026-06-03) |

---

## 1. Problem

The admin platform merged in PR #1 (`feature/admin-ui-enhancement`, merged 2026-06-01) added 25+ admin Svelte routes, multi-tenant data model (Tenant → Organization → Group → Identity), RBAC role tables, an audit log, and an API key system. The full permission engine per `docs/MULTI_TENANT_ADMIN_DESIGN.md` §4.5 is **already implemented as code** at `src/services/permission.rs` (HAS_PERMISSION, super_admin bypass, scope_restriction, group_permission_overrides).

**What is missing**: the handler layer does not call the engine. Examples (file `src/api/handlers.rs`):

- `get_tenant_handler` (line 54) calls `state.tenant.get(id)` and returns it. No check that the requester belongs to that tenant.
- `get_org_handler`, `get_group_handler`, `get_identity_handler`, `get_api_key_handler`, `list_audit_logs_handler`: same pattern.
- Tenant `list` (line 32) returns all tenants in the system to any authenticated caller.

**Impact**: any authenticated user can fetch any tenant / org / group / identity / api-key / audit entry by ID, regardless of which tenant they belong to. This is a cross-tenant data leak via the admin API surface.

## 2. Goal

Close the cross-tenant data leak with the minimum needed to make admin handlers enforce "requester must be in the target tenant (or be super_admin)". Org / group / role / scope_restriction enforcement is **out of scope** for this Tier 1 PR.

**Acceptance criteria**:

1. No admin handler returns data from a tenant the requester is not a member of.
2. `super_admin` users can access any tenant.
3. All non-admin / public endpoints (login, register, agent auth) remain reachable without an admin token.
4. All existing integration tests still pass.
5. New integration tests cover: cross-tenant access denied; same-tenant access allowed; super_admin bypass.

## 3. Non-goals (Tier 2+)

- Wiring the full §4.5 permission engine (`has_permission`, `can_edit_skill`, scope_restriction, group_permission_overrides) into skill / group / org handlers. Tier 2 PR.
- Audit log export endpoint.
- API key rotation / revocation UX.
- RBAC edge case: cross-tenant org membership (a user in two orgs, each in a different tenant).
- Graceful migration of existing admin tokens (will require re-login after deploy; acceptable since admin UI is in active development and tokens are short-lived).

## 4. Design

### 4.1 Components

| File | Type | Purpose |
|---|---|---|
| `src/api/jwt.rs` | modify | Extend `Claims` with `identity_id: Option<Uuid>` and `is_admin: bool`. Add `AdminUser` extractor that requires both fields, returns 401 otherwise. |
| `src/api/auth.rs` | new | `AdminUser` extractor re-export (or kept in jwt.rs and re-exported). `require_tenant_access` helper. |
| `src/services/permission.rs` | modify | Add `user_belongs_to_tenant(identity_id, tenant_id) -> bool` and `list_user_tenants(identity_id) -> Vec<Uuid>` (used by list endpoints to filter results). Both use existing `is_super_admin` + `OrgMembershipRepository::list_user_organizations` joined with org.tenant_id. |
| `src/api/handlers.rs` | modify | Refactor tenant-scoped admin handlers to take `AdminUser` + call `require_tenant_access` (single-tenant endpoints) or `list_user_tenants` (list endpoints, then filter the result by `tenant_id IN (...)`). |
| `src/api/error.rs` | modify | No new variants needed (`Unauthorized`, `Forbidden`, `InternalError` already exist). |
| `tests/admin_isolation.rs` | new | Integration tests for cross-tenant denial, same-tenant allow, super_admin bypass, list filtering. |
| `Cargo.toml` | no change | All deps already present. |

### 4.2 Data flow

```
HTTP request
  Authorization: Bearer <jwt>
        │
        ▼
  AdminUser::from_request_parts
        │  (a) read header
        │  (b) jwt::verify_token → Claims
        │  (c) require claims.identity_id.is_some() && claims.is_admin
        │  (d) return AdminUser { identity_id, subject, is_admin }
        ▼
  handler_signature(AdminUser, State<AppRouterState>, Path(tenant_id))
        │
        ▼
  require_tenant_access(&state, user.identity_id, tenant_id)
        │  (1) state.permission.is_super_admin(identity_id)?
        │      → Ok(())
        │  (2) state.permission.user_belongs_to_tenant(identity_id, tenant_id)?
        │      → Ok(())
        │  (3) otherwise
        │      → Err(ApiError::Forbidden("Not a member of this tenant"))
        ▼
  handler proceeds with normal logic

List-endpoint variant (no single tenant_id from path):
  handler_signature(AdminUser, State<AppRouterState>)
        │
        ▼
  let accessible_tenants = state.permission.list_user_tenants(user.identity_id)?;
        │  returns Vec<Uuid> of all tenants the user is a member of
        │  (super_admin → Vec with one sentinel UUID meaning "all tenants";
        │  the handler passes that through to the SQL `WHERE tenant_id = ANY($1)`)
        ▼
  service.list(accessible_tenants)
  handler returns filtered result
```

### 4.3 Error responses

| Condition | HTTP | Body |
|---|---|---|
| Missing Authorization header | 401 | `{"error": "Unauthorized", "message": "Missing Authorization header"}` |
| Token signature invalid | 401 | `{"error": "Unauthorized", "message": "Invalid token: ..."}` |
| Token expired | 401 | `{"error": "Unauthorized", "message": "Invalid token: ExpiredSignature"}` |
| Token valid but `is_admin` is false | 401 | `{"error": "Unauthorized", "message": "Admin token required"}` |
| Token valid, admin, but `identity_id` is None | 401 | `{"error": "Unauthorized", "message": "Identity not bound to token"}` |
| Authenticated, but not in target tenant | 403 | `{"error": "Forbidden", "message": "Not a member of this tenant"}` |
| Internal error during membership lookup | 500 | `{"error": "InternalError", "message": "..."}` |

### 4.4 Endpoints in scope

All endpoints below get `AdminUser` + `require_tenant_access` (with the indicated scope):

| Endpoint | Scope check | Notes |
|---|---|---|
| `GET /api/v1/admin/tenants/:id` | requester must be super_admin | tenant mgmt is super-only |
| `PUT /api/v1/admin/tenants/:id` | super_admin | |
| `DELETE /api/v1/admin/tenants/:id` | super_admin | |
| `GET /api/v1/admin/tenants` | super_admin | (lists all tenants) |
| `POST /api/v1/admin/tenants` | super_admin | |
| `GET /api/v1/admin/identities/:id` | member of identity's tenant | |
| `PUT /api/v1/admin/identities/:id` | member of identity's tenant | |
| `DELETE /api/v1/admin/identities/:id` | member of identity's tenant | |
| `GET /api/v1/admin/groups/:id` | member of group.org.tenant | |
| `PUT /api/v1/admin/groups/:id` | member of group.org.tenant | |
| `DELETE /api/v1/admin/groups/:id` | member of group.org.tenant | |
| `GET /api/v1/admin/groups` | list scoped to caller's tenant(s) | filter result: `org.tenant_id IN caller.list_user_tenants()` |
| `GET /api/v1/admin/roles/:id` | member of role's tenant | roles are tenant-scoped per design §4.1 |
| `GET /api/v1/admin/api-keys/:id` | member of key's tenant | |
| `DELETE /api/v1/admin/api-keys/:id` | member of key's tenant | |
| `GET /api/v1/admin/api-keys` | list scoped to caller's tenant(s) | filter result: `tenant_id IN caller.list_user_tenants()` |
| `GET /api/v1/admin/audit-logs` | scoped to caller's tenant(s) | filter result: `tenant_id IN caller.list_user_tenants()` |
| `GET /api/v1/admin/audit-entries` | scoped to caller's tenant(s) | same |
| `GET /api/v1/organizations/:id` | member of org.tenant | (v0.4 multi-tenant route) |
| `PUT /api/v1/organizations/:id` | member of org.tenant | |
| `DELETE /api/v1/organizations/:id` | member of org.tenant | |
| `GET /api/v1/organizations` | list scoped to caller's tenant(s) | filter result: `tenant_id IN caller.list_user_tenants()` |
| `POST /api/v1/organizations` | requester is in target tenant_id from body | |
| `GET /api/v1/orgs/:slug/...` | member of org.tenant (lookup by slug) | |
| `GET /api/v1/groups/:id/...` | member of group.org.tenant | |
| `GET /api/v1/sessions/:id` | member of session.org.tenant | |
| `POST /api/v1/sessions/:id/end` | member of session.org.tenant | |
| `GET /api/v1/sessions` | list scoped to caller's tenant(s) | filter result: `org.tenant_id IN caller.list_user_tenants()` |
| `GET /api/v1/org-tools/:id` | member of org.tenant | |
| `POST /api/v1/org-tools/:id/approve` | member of org.tenant | Tier 1 = tenant membership; org-role check is Tier 2 |
| `POST /api/v1/org-tools/:id/reject` | member of org.tenant | Tier 1 = tenant membership; org-role check is Tier 2 |
| `DELETE /api/v1/org-tools/:id` | member of org.tenant | Tier 1 = tenant membership; org-role check is Tier 2 |
| `GET /api/v1/org-tools` | list scoped to caller's tenant(s) | filter result: `org.tenant_id IN caller.list_user_tenants()` |
| `POST /api/v1/org-tools` | member of body.tenant_id | |

**Out of scope (not in this PR)**:

- `/api/v1/admin/login` (public)
- `/api/v1/admin/me` (returns caller's identity; needs AdminUser, no tenant check)
- `/api/v1/admin/status` (needs AdminUser, no tenant check)
- `/api/v1/admin/stats` (super_admin only)
- All `/api/v1/skills/...` (skills are personal or org-owned, not tenant-scoped at the API surface; Tier 2)
- All `/api/v1/auth/...` (public)
- All `/api/v1/users/...` (Tier 2: user-level RBAC)
- All `/api/v1/groups/default-permissions`, `/api/v1/groups/:id/permissions/...` (Tier 2: group-level RBAC)

### 4.5 Token format change

Current `Claims`:
```rust
pub struct Claims {
    pub subject: String,
    pub roles: Vec<String>,
    pub scope: Vec<String>,
    pub exp: i64,
    pub iat: i64,
}
```

New `Claims`:
```rust
pub struct Claims {
    pub subject: String,           // username, for back-compat
    pub identity_id: Option<Uuid>, // None for agent tokens, Some for admin
    pub is_admin: bool,            // discriminator
    pub roles: Vec<String>,        // existing
    pub scope: Vec<String>,        // existing
    pub exp: i64,                  // existing
    pub iat: i64,                  // existing
}
```

`generate_token` is updated to accept these new fields. Existing callers (agent token issuance, admin login) are updated to pass them.

### 4.6 Public-endpoint allowlist

These endpoints do **not** require an `AdminUser` extractor (they must remain reachable):

- `POST /api/v1/admin/login`
- `POST /api/v1/auth/login`
- `POST /api/v1/auth/register`
- `POST /api/v1/auth/agent/token`
- `POST /api/v1/auth/agent/register`
- `GET /health`
- `GET /` (MCP server discovery on stdio, or HTTP root)

The `AdminUser` extractor is only added to handlers in the scope table (§4.4). All other handlers are unchanged.

## 5. Testing

### 5.1 Unit tests (in `src/services/permission.rs`)

```rust
#[tokio::test]
async fn super_admin_belongs_to_any_tenant() { ... }

#[tokio::test]
async fn user_in_tenant_returns_true() { ... }

#[tokio::test]
async fn user_not_in_tenant_returns_false() { ... }
```

### 5.2 Unit tests (in `src/api/auth.rs` or `jwt.rs`)

```rust
#[tokio::test]
async fn require_tenant_access_allows_super_admin() { ... }

#[tokio::test]
async fn require_tenant_access_allows_member() { ... }

#[tokio::test]
async fn require_tenant_access_denies_non_member() { ... }

#[tokio::test]
async fn admin_user_extractor_rejects_agent_token() { ... }

#[tokio::test]
async fn admin_user_extractor_rejects_missing_token() { ... }
```

### 5.3 Integration tests (new file `tests/admin_isolation.rs`)

File-based; uses temp dirs for any storage. Tests for each endpoint group:

- **Tenants**: super_admin can `GET /admin/tenants/T1` and `T2`; non-admin with no token gets 401; non-super-admin gets 403 (since tenant mgmt is super-only)
- **Identities**: user A in T1 gets 200 on `GET /admin/identities/<id-in-T1>`, 403 on `<id-in-T2>`
- **Groups**: same pattern
- **API keys**: same pattern
- **Audit logs**: `GET /admin/audit-logs` returns only T1's logs when caller is in T1
- **Orgs**: user A in T1 gets 200 on `GET /orgs/T1-org`, 403 on `T2-org`
- **Sessions**: same pattern
- **Org tools**: same pattern

### 5.4 Regression

All existing integration tests in `tests/integration.rs` must still pass. The test profile is `cargo test --test integration` (file-based, no DB).

## 6. Migration / rollout

1. Backend: extend Claims, add AdminUser, add user_belongs_to_tenant helper, refactor handlers. Single worktree branch off main, single PR.
2. Frontend (`admin/`): the existing `admin_token` storage works as-is. Tokens issued before this PR (without `identity_id` / `is_admin`) will get a 401 on the next admin request. User is prompted to log in again. Acceptable since admin UI is in active development and tokens are short-lived (default 24h, configurable).
3. Rollout order: backend first, then re-login users, then continue. No DB migration needed (Claims change is in the JWT only; DB schema unchanged).

## 7. Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Missing an endpoint in §4.4 leaves a leak | medium | Audit-list of every `/api/v1/admin/*` and tenant-scoped route verified against §4.4 in PR review |
| Existing tokens rejected breaks sessions | low | Tokens are short-lived; users re-login. Documented in PR body. |
| `user_belongs_to_tenant` query slow (no index) | low | `org_memberships.identity_id` and `organizations.tenant_id` are likely indexed (verify in PR review); add index if not |
| `super_admin` check adds DB query per request | medium | Cache super_admin status in `AdminUser` extractor result (in-memory, 60s TTL acceptable) — or accept the latency for now and optimize in Tier 2 |

## 8. Out-of-scope follow-ups (for the PR body and a backlog issue)

- Tier 2: full §4.5 permission engine on every skill / group / org handler
- Audit log export endpoint
- API key rotation / revocation UX
- RBAC edge case: cross-tenant org membership
- Graceful token migration (forward-compatible Claims parsing)

## 9. Decisions made during brainstorming

- **Approach**: Axum extractor (`AdminUser`) for type-level enforcement + helper function (`require_tenant_access`) for the tenant check. Rejected pure middleware (less explicit, harder to override per-handler). Rejected per-handler manual checks (easy to forget).
- **Identity model**: keep `AgentContext` for agent endpoints, add `AdminUser` for admin endpoints. Type-level separation makes the boundary clear.
- **super_admin detection**: query `system_role_assignments` per request (no caching for now; can add in Tier 2 if measured to be a bottleneck).
- **Token format**: extend `Claims` with `Option<Uuid>` rather than bumping to a new `ClaimsV2`. Smaller migration.

## 10. Acceptance walkthrough

After the PR is merged:

1. As super_admin: `GET /api/v1/admin/tenants/T1` → 200 with T1 data; same for T2.
2. As non-admin user in T1: `GET /api/v1/admin/tenants/T2` → 403.
3. As non-admin user in T1: `GET /api/v1/admin/identities/<id-in-T2>` → 403.
4. As non-admin user in T1: `GET /api/v1/admin/audit-logs` → 200 with only T1's logs.
5. As agent (no admin token): any of the above → 401.
6. `POST /api/v1/admin/login` with valid creds → 200 + new token (now with `identity_id` + `is_admin: true`).
7. `cargo test --test integration` → all green, including new `tests/admin_isolation.rs`.

---

## Appendix A — Pre-existing design doc references

- `docs/MULTI_TENANT_ADMIN_DESIGN.md` §4.5 (Permission Decision Engine) — already implemented in `src/services/permission.rs`
- `docs/MULTI_TENANT_ADMIN_DESIGN.md` §3 (Database Model) — schema is the source of truth for `org_memberships`, `system_role_assignments`, etc.
- `docs/MULTI_TENANT_ADMIN_DESIGN.md` §6.3, §6.6 — Org/Group permission models referenced for endpoint scoping

## Appendix B — Pre-existing code references

- `src/api/jwt.rs` — current `AgentContext` extractor (will be supplemented, not replaced)
- `src/services/permission.rs:98-196` — `has_permission` (already implements §4.5)
- `src/services/permission.rs:54-59` — `is_super_admin` (will be reused)
- `src/db/repositories/org_membership.rs` — `list_user_organizations` (will be reused)
- `src/api/handlers.rs:32-99` — current tenant handlers (will be refactored)
- `src/api/routes.rs:122-148` — admin route table (no structural change)
