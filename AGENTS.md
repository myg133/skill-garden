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

## Version

Current: **0.3.0** (see `Cargo.toml` and `VERSION` file)

Note: README.md incorrectly states 0.2.0
