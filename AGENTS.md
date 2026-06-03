# AGENTS.md - AionHive/OpenCode Session Guide

## Project Reality Check

**This is a Rust project.** CLAUDE.md contains outdated Node.js/npm commands. Do not follow them.

---

## Project Type

- **Language**: Rust (not Node.js/TypeScript)
- **Web Framework**: Axum 0.7 with Tokio async runtime
- **Database**: PostgreSQL via sqlx 0.8
- **Search**: Tantivy 0.22 (full-text search engine)
- **Protocol**: MCP (Model Context Protocol) via `rmcp` crate
- **Admin UI**: Separate Svelte app in `admin/` directory

---

## Critical Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AION_HIVE_TRANSPORT` | `stdio` | Transport mode: `stdio` or `http` |
| `AION_HIVE_HTTP_PORT` | `8080` | HTTP server port (only in http mode) |
| `AION_HIVE_DATA_DIR` | `data` | Data directory path |
| `AION_HIVE_SKILLS_DIR` | `skills` | Skills assets directory |
| `DATABASE_URL` | `postgres://localhost:5432/aionhive` | PostgreSQL connection string |

---

## Build & Run Commands

```bash
# Build (debug)
cargo build

# Build (release)
cargo build --release

# Run (stdio mode - default)
cargo run

# Run (HTTP mode)
$env:AION_HIVE_TRANSPORT="http"
$env:AION_HIVE_HTTP_PORT="8080"
cargo run

# Run integration tests (file-based, no DB required)
cargo test --test integration

# Run all tests including unit tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

---

## Project Structure

```
src/
├── main.rs           # Entry point; handles transport mode selection
├── lib.rs            # AppState initialization; DB migrations
├── api/              # HTTP handlers and routes
├── db/               # Database repositories and migrations
├── mcp/              # MCP server implementation
├── models/            # Data models (Skill, Evaluation, Agent)
├── schemas/           # Validation schemas
├── services/          # Business logic (Registry, Search, Storage, Evaluator)
└── utils/             # Utilities (RateLimiter, weight calculation)
```

---

## Transport Modes

### Stdio Mode (Default)
MCP server communicates via stdin/stdout. Used by OpenClaw agents.

### HTTP Mode
Exposes REST endpoints:
- `POST /mcp` - MCP JSON-RPC handler
- `GET /sse` - SSE endpoint for bidirectional communication
- `POST /sse/:session_id` - SSE message handler
- `GET /health` - Health check

### Server Starters
The PowerShell starter scripts (`start-http-server.ps1`, `start-sse-server.ps1`) were removed in v0.3.1 (commit `ae297ba`). Start the server directly with `cargo run` and the `AION_HIVE_TRANSPORT` / `AION_HIVE_HTTP_PORT` env vars above.

---

## Testing

### Rust Integration Tests
**File-based tests only** - no PostgreSQL required:
```bash
cargo test --test integration
cargo test --test admin_isolation   # Tenant-scope guard (auth/extract boundary; DB-gated cases marked #[ignore])
```

Note: RegistryService and EvaluatorService tests are disabled because they require PostgreSQL.

### E2E Tests (Deno)
Tests MCP HTTP transport - requires running server:
```bash
# Terminal 1: Start server (HTTP mode)
$env:AION_HIVE_TRANSPORT="http"
$env:AION_HIVE_HTTP_PORT="8080"
cargo run

# Terminal 2: Run Deno tests
deno test --allow-all --filter "MCP E2E" tests/e2e/mcp_e2e_test.ts
```

---

## Database

**PostgreSQL required** for full operation:
- Runs migrations on startup via `db::migrations::run_migrations()`
- Migrations stored in `src/db/migrations/`
- Connection configured via `DATABASE_URL` env var

**File-based fallbacks**: SearchService and StorageService use local files when DB unavailable.

---

## Key Skills Structure

Skills are stored in `skills/` directory:
- Each skill has `SKILL.md` (YAML frontmatter + markdown)
- Optional: `src/`, `tests/`, `assets/` subdirectories

---

## Common Mistakes to Avoid

1. **Don't use npm commands** - This is a Rust project
2. **Don't assume stdio is HTTP** - `cargo run` starts stdio mode by default
3. **Don't skip DATABASE_URL** - Full features require PostgreSQL
4. **Don't run integration tests against production DB** - Use temp directories

---

## Development Workflow (gstack + superpowers)

This project follows the gstack + superpowers combined development pattern. Skills are loaded by the host agent at runtime; this section is the project-level reference for **how** to apply them to AionHive work.

### Division of Labor

- **superpowers** (process discipline) — `when` to act: `brainstorming`, `writing-plans`, `using-git-worktrees`, `test-driven-development`, `systematic-debugging`, `verification-before-completion`, `subagent-driven-development`, `executing-plans`, `requesting-code-review`, `receiving-code-review`, `finishing-a-development-branch`.
- **gstack** (executable toolkit) — `how` to act: `browse`, `qa`, `ship`, `land-and-deploy`, `canary`, `review`, `investigate`, `office-hours`, `plan-eng-review`, `plan-design-review`, `plan-devex-review`, `plan-ceo-review`, `spec`, `autoplan`, `health`, `retro`, `learn`, `design-review`, `cso`, `careful`, `freeze`, `guard`, `make-pdf`.

When in doubt: process questions → superpowers (brainstorming, debugging, TDD); action questions → gstack (browse, qa, ship, review).

### The Stable Dev Loop (canonical sequence)

```
[intent] → brainstorming → office-hours / plan-ceo-review → plan-eng-review
        → writing-plans → using-git-worktrees → TDD
        → (systematic-debugging | investigate) on failure
        → review → verification-before-completion
        → requesting-code-review → finishing-a-development-branch
        → ship → land-and-deploy → canary
        → retro + learn
```

The loop is the same for features, refactors, and bug fixes. Trivial edits (typo, config tweak, doc consolidation) may skip brainstorming/planning steps. **No step is skippable for real work.**

### Pre-Flight Checklist (every non-trivial task)

1. **Intent clear?** (else `brainstorming`)
2. **Plan exists?** (else `writing-plans`)
3. **Worktree isolated?** (else `using-git-worktrees`)
4. **Tests written first?** (else `test-driven-development`)
5. **Verification command identified?** (else `verification-before-completion`)

### Hard Rules (project-applicable)

- **Never claim a fix is done without running a verification command and showing its output.** (`verification-before-completion`)
- **Never merge a PR that lacks: a spec (or ticket ref), green tests, a `/review` pass, and a rollback note.** `ship` enforces this.
- **Never start a multi-file change in the main worktree.** Always work in a worktree (`using-git-worktrees`).
- **Never suppress type errors** (`as any`, `@ts-ignore`, `@ts-expect-error`) — Rust equivalent: no `#[allow(...)]` without justification, no `unwrap()` in non-test code without a written reason.
- **Never create a second `AGENTS.md`** — this file is the only contract.
- **No shotgun debugging.** Fix root causes, not symptoms.
- **No empty catch blocks** (`catch(_) {}`) — Rust equivalent: handle or propagate every `Result`; no `let _ =` swallowing errors silently.

### Quality Gates (project-specific)

These are the gates that must be green before any change ships. Run them in order from fastest → slowest:

- **Type-check**: `cargo check` — must pass (fastest feedback)
- **Format**: `cargo fmt --check` — must pass
- **Lint**: `cargo clippy` — must pass (warnings ok only with documented justification)
- **Build**: `cargo build` — must compile without errors
- **Tests (fast)**: `cargo test --test integration` + `cargo test --test admin_isolation` — must pass
- **Tests (full)**: `cargo test` — must pass; some tests skipped without `DATABASE_URL`
- **Build (release)**: `cargo build --release` — must compile (catches release-only issues, run before tagging)

E2E tests (`deno test --allow-all --filter "MCP E2E" tests/e2e/mcp_e2e_test.ts`) require a running HTTP server + PostgreSQL. They are **not** part of the fast gate; run them when changing transport/protocol code.

A red test is **never** "expected" — fix to green before shipping.

---

## Version

Current: **0.3.1** (next release — see Health Snapshot; admin platform PR #1 + stabilization + tenant-scope guard are unreleased)

Note: README.md still says 0.2.0 (will be fixed in a docs follow-up).

---

## Health Snapshot

- Date: 2026-06-03
- Score: **7.5 / 10** — needs work
- Top issue: 279 pre-existing clippy warnings (unaddressed); `test_validation` failure from the `validate_skill_name` whitespace bug is **fixed in `stabilize/v0.3.0-baseline`** but not in this branch's integration suite (the fix lives on a separate branch).
- Per-gate (composite weights: check 25% / clippy 30% / test 45%):
  - `cargo check --all-targets`: **10 / 10** — clean, 16s
  - `cargo clippy --all-targets -- -W clippy::all`: **7 / 10** — 0 errors, 279 warnings (all pre-existing in src/main.rs, src/api/handlers.rs, src/models/*.rs, src/services/*.rs; **0 new warnings introduced by this PR**)
  - `cargo test --lib`: **10 / 10** — 96/96 pass
  - `cargo test --test integration`: **5 / 10** — 7/8 pass; 1 fail = pre-existing `test_validation` (whitespace in `validate_skill_name`), fixed in `stabilize/v0.3.0-baseline` commit `94a4505`, not in this branch
  - `cargo test --test admin_isolation` (new in this PR): **10 / 10** — 12 pass, 0 fail, 8 ignored (DB-gated; `#[ignore = "requires test PostgreSQL instance"]`)
- Other findings surfaced (do not affect composite score):
  - 17 commits in this PR (T1–T15 plan executed; see `docs/superpowers/plans/2026-06-03-tenant-scope-guard.md`)
  - Two audit systems exist: legacy `auditlogs` (no `tenant_id`, filter is auth-only) and new `audit_log_entries` (with `tenant_id`, proper filter). The migration to consolidate is a follow-up.
  - `Identity`, `Group`, `ApiKey`, `OrgTool`, `Session` models have no direct `tenant_id` — tenancy resolves through `org_memberships → organizations.tenant_id` (or `org_id` directly for some). Handlers use a per-resource `*_tenant_id` helper.
  - Plan gaps found and fixed in-flight: `PermissionService` wiring into `AppRouterState` (T4), `generate_token_full` re-export (T5), `list_by_org_tenants` on session/org_tool services (T12 fix).
  - PR scope: closes the cross-tenant data leak surfaced by gstack:health 6.9/10 baseline; sets the security foundation for the full §4.5 permission engine (Tier 2 follow-up).
- Notes: First full subagent-driven run for this project. 17 commits, 0 new clippy warnings, 8/8 admin_isolation tests pass + 8 ignored. The `test_validation` failure carries forward from the baseline (Health Snapshot 2026-06-03) and will be resolved when `stabilize/v0.3.0-baseline` is merged.
