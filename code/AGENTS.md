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

### PowerShell Server Starters
```powershell
# HTTP mode
.\start-http-server.ps1 -Port 8080

# SSE mode
.\start-sse-server.ps1 -Port 8080
```

---

## Testing

### Rust Integration Tests
**File-based tests only** - no PostgreSQL required:
```bash
cargo test --test integration
```

Note: RegistryService and EvaluatorService tests are disabled because they require PostgreSQL.

### E2E Tests (Deno)
Tests MCP HTTP transport - requires running server:
```bash
# Terminal 1: Start server
.\start-http-server.ps1 -Port 8080

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

### Skill Routing (quick lookup)

| User intent | Invoke |
|---|---|
| Vague intent / fuzzy idea | `brainstorming` → `office-hours` |
| Lock the architecture | `plan-eng-review` |
| Review design / DX before coding | `plan-design-review` / `plan-devex-review` |
| Full review pipeline | `autoplan` |
| File as a ticket | `spec` |
| Open page / screenshot / verify deploy | `browse` |
| QA the staging URL | `qa` (or `qa-only` for report-only) |
| Review the PR diff | `review` |
| Ship it / open the PR | `ship` → `land-and-deploy` |
| Debug — don't know why | `investigate` (gates `systematic-debugging`) |
| Weekly retro / learnings | `retro`, `learn` |
| Touch only this directory | `freeze` / `guard` |
| Generate a PDF from a markdown | `make-pdf` |
| Refactor / multi-stage plan | `executing-plans` / `subagent-driven-development` |
| Receive PR feedback | `receiving-code-review` |
| Code health dashboard | `health` |

### Hard Rules (project-applicable)

- **Never claim a fix is done without running a verification command and showing its output.** (`verification-before-completion`)
- **Never merge a PR that lacks: a spec (or ticket ref), green tests, a `/review` pass, and a rollback note.** `ship` enforces this.
- **Never start a multi-file change in the main worktree.** Always work in a worktree (`using-git-worktrees`).
- **Never ship on a Friday without `canary` watching the deploy.**
- **Never edit outside `cwd` or declared writable roots without escalation.**
- **Never paste secrets, API keys, or full file contents into a chat log.**
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
- **Tests (fast)**: `cargo test --test integration` — must pass (file-based, no DB)
- **Tests (full)**: `cargo test` — must pass; some tests skipped without `DATABASE_URL`
- **Build (release)**: `cargo build --release` — must compile (catches release-only issues, run before tagging)

E2E tests (`deno test --allow-all --filter "MCP E2E" tests/e2e/mcp_e2e_test.ts`) require a running HTTP server + PostgreSQL. They are **not** part of the fast gate; run them when changing transport/protocol code.

A red test is **never** "expected" — fix to green before shipping.

---

## Version

Current: **0.3.0** (see `Cargo.toml` and `VERSION` file)

Note: README.md incorrectly states 0.2.0

---

## Health Snapshot

- Date: 2026-06-03
- Score: **6.9 / 10** — needs work
- Top issue: `test_validation` failing — `validate_skill_name` allows whitespace (`src/schemas/validation.rs:51`), but the integration test at `tests/integration.rs:148` expects `"invalid name"` (contains a space) to be rejected
- Per-gate (composite weights: check 25% / clippy 30% / test 45%):
  - `cargo check --all-targets`: **10 / 10** — clean, 45.5s
  - `cargo clippy --all-targets -- -W clippy::all`: **7 / 10** — 0 errors, **279 warnings** (heavily duplicated across lib / lib-test / bin / bin-test targets; ~50–80 unique source-warnings). Top lints: `empty_line_after_doc_comments` (6 in `src/api/handlers.rs`), `uninlined_format_args`, `derivable_impls`, `needless_borrows_for_generic_args` (3 in `src/main.rs`).
  - `cargo test --test integration`: **5 / 10** — 7 / 8 pass, 1 fail (`test_validation`), 56.0s. RegistryService and EvaluatorService tests remain DB-only and are correctly skipped per `## Testing` above.
- Other findings surfaced (do not affect the composite score):
  - `README.md` still says version 0.2.0; should be 0.3.0
  - `CHANGELOG.md` is silent on PR #1 (`feature/admin-ui-enhancement`, merged 2026-06-01): multi-tenant + RBAC + audit + API keys + 25+ Svelte routes
  - `AGENTS.md` (this file) still documents `.\start-http-server.ps1` and `.\start-sse-server.ps1` as canonical PowerShell starters, but those files are deleted in the working tree
  - 5 uncommitted `.ps1` deletions: `run-server.ps1`, `server.err`, `start-http-server.ps1`, `start-http-server-blocking.ps1`, `start-sse-server.ps1`
  - `.zread/wiki/` untracked — 26-section Chinese-language project wiki (项目概述 → 置信度权重机制). Comprehensive doc that is not in the tracked `docs/` tree.
- Notes: First gstack:health-equivalent run on AionHive SkillGarden v0.3.0 (Rust / Axum / PostgreSQL / Tantivy / MCP). Raw logs: `cargo check` 51 lines (clean), `cargo clippy` 3540 lines (0 errors / 279 warnings), `cargo test --test integration` 57 lines (1 failure).
