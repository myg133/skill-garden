# Phase 4: Admin API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Admin API capabilities including Skills review workflow and audit logging.

**Architecture:** Extend existing API handlers with admin endpoints. Add status field to skills. Use AuditRepository to log all operations.

**Tech Stack:** Rust, Axum, sqlx, PostgreSQL

---

## File Structure

```
src/
├── api/
│   ├── handlers.rs       # Add admin handlers
│   ├── models.rs        # Add request/response models
│   └── routes.rs        # Add admin routes
├── db/
│   ├── migrations/
│   │   └── 002_add_skill_status.sql  # NEW: Add status column
│   └── repositories/
│       ├── audit.rs     # Add list_with_filters method
│       └── skill.rs     # Add status update method
└── main.rs              # Register admin routes
```

---

## Task 1: Add Skill Status Column

**Files:**
- Create: `src/db/migrations/002_add_skill_status.sql`
- Modify: `src/db/repositories/skill.rs`

- [ ] **Step 1: Create migration file**

Create `src/db/migrations/002_add_skill_status.sql`:
```sql
-- Add status column to skills table
ALTER TABLE skills ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'pending_review';

-- Index for filtering by status
CREATE INDEX idx_skills_status ON skills(status);

-- Index for listing pending review skills
CREATE INDEX idx_skills_status_created ON skills(status, created_at DESC);
```

- [ ] **Step 2: Update SkillRepository to include status**

In `src/db/repositories/skill.rs`, update the `Skill` struct to include `status`:
```rust
#[derive(Debug, Clone)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author_agent_id: String,
    pub compatibility: String,
    pub content: String,
    pub install_count: i32,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub status: String,  // ADD THIS
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

And update the SQL queries to include status.

- [ ] **Step 3: Update SkillMetadata**

```rust
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author_agent_id: String,
    pub install_count: i32,
    pub tags: Vec<String>,
    pub status: String,  // ADD THIS
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 4: Add update_status method to SkillRepository**

```rust
pub async fn update_status(&self, skill_id: &str, status: &str) -> DbResult<()> {
    sqlx::query("UPDATE skills SET status = $1, updated_at = NOW() WHERE id = $2")
        .bind(status)
        .bind(skill_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
    Ok(())
}
```

- [ ] **Step 5: Run migration and verify**

Run: `psql $DATABASE_URL -f src/db/migrations/002_add_skill_status.sql`

- [ ] **Step 6: Commit**

```bash
git add src/db/migrations/002_add_skill_status.sql src/db/repositories/skill.rs
git commit -m "feat(db): add skill status column for review workflow"
```

---

## Task 2: Add Audit Log List with Filters

**Files:**
- Modify: `src/db/repositories/audit.rs`

- [ ] **Step 1: Add list_with_filters method to AuditRepository**

In `src/db/repositories/audit.rs`, add:

```rust
pub async fn list_with_filters(
    &self,
    agent_id: Option<&str>,
    action: Option<&str>,
    resource_type: Option<&str>,
    limit: i64,
    offset: i64,
) -> DbResult<Vec<AuditLog>> {
    let mut query = "SELECT id, agent_id, action, resource_type, resource_id, details, timestamp FROM audit_logs WHERE 1=1".to_string();
    let mut param_count = 0;

    if agent_id.is_some() {
        param_count += 1;
        query.push_str(&format!(" AND agent_id = ${}", param_count));
    }
    if action.is_some() {
        param_count += 1;
        query.push_str(&format!(" AND action = ${}", param_count));
    }
    if resource_type.is_some() {
        param_count += 1;
        query.push_str(&format!(" AND resource_type = ${}", param_count));
    }

    param_count += 1;
    query.push_str(&format!(" ORDER BY timestamp DESC LIMIT ${}", param_count));
    param_count += 1;
    query.push_str(&format!(" OFFSET ${}", param_count));

    let mut q = sqlx::query_as::<_, AuditLogRow>(&query);

    if let Some(aid) = agent_id {
        q = q.bind(aid);
    }
    if let Some(act) = action {
        q = q.bind(act);
    }
    if let Some(rt) = resource_type {
        q = q.bind(rt);
    }
    q = q.bind(limit).bind(offset);

    let rows = q.fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

pub async fn count_with_filters(
    &self,
    agent_id: Option<&str>,
    action: Option<&str>,
    resource_type: Option<&str>,
) -> DbResult<i64> {
    let mut query = "SELECT COUNT(*) FROM audit_logs WHERE 1=1".to_string();
    let mut param_count = 0;

    if agent_id.is_some() {
        param_count += 1;
        query.push_str(&format!(" AND agent_id = ${}", param_count));
    }
    if action.is_some() {
        param_count += 1;
        query.push_str(&format!(" AND action = ${}", param_count));
    }
    if resource_type.is_some() {
        param_count += 1;
        query.push_str(&format!(" AND resource_type = ${}", param_count));
    }

    let mut q = sqlx::query_as::<_, (i64,)>(&query);

    if let Some(aid) = agent_id {
        q = q.bind(aid);
    }
    if let Some(act) = action {
        q = q.bind(act);
    }
    if let Some(rt) = resource_type {
        q = q.bind(rt);
    }

    let row = q.fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

    Ok(row.0)
}
```

- [ ] **Step 2: Commit**

```bash
git add src/db/repositories/audit.rs
git commit -m "feat(db): add filtered audit log queries"
```

---

## Task 3: Add Admin API Models

**Files:**
- Modify: `src/api/models.rs`

- [ ] **Step 1: Add Admin API request/response models**

In `src/api/models.rs`, add:

```rust
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub agent_id: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub id: String,
    pub agent_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: serde_json::Value,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct AuditLogListResponse {
    pub data: Vec<AuditLogResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct RejectSkillBody {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SkillReviewResponse {
    pub message: String,
    pub skill_id: String,
}
```

- [ ] **Step 2: Commit**

```bash
git add src/api/models.rs
git commit -m "feat(api): add admin API models"
```

---

## Task 4: Add Admin Handlers

**Files:**
- Modify: `src/api/handlers.rs`

- [ ] **Step 1: Add admin handlers**

In `src/api/handlers.rs`, add:

```rust
pub async fn list_audit_logs_handler(
    State(state): State<ApiState>,
    AgentContext { agent_id, roles }: AgentContext,
    Query(query): Query<crate::api::models::AuditLogQuery>,
) -> Result<impl IntoResponse, ApiError> {
    if !roles.iter().any(|r| r == "admin") {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let logs = state.audit_repo
        .list_with_filters(
            query.agent_id.as_deref(),
            query.action.as_deref(),
            query.resource_type.as_deref(),
            limit,
            offset,
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total = state.audit_repo
        .count_with_filters(
            query.agent_id.as_deref(),
            query.action.as_deref(),
            query.resource_type.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<_> = logs.into_iter().map(|log| {
        crate::api::models::AuditLogResponse {
            id: log.id.to_string(),
            agent_id: log.agent_id,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            details: log.details,
            timestamp: log.timestamp.to_rfc3339(),
        }
    }).collect();

    Ok((StatusCode::OK, Json(crate::api::models::AuditLogListResponse {
        data,
        total,
        limit,
        offset,
    })))
}

pub async fn list_my_audit_logs_handler(
    State(state): State<ApiState>,
    AgentContext { agent_id, .. }: AgentContext,
    Query(query): Query<crate::api::models::AuditLogQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let logs = state.audit_repo
        .list_with_filters(Some(&agent_id), query.action.as_deref(), query.resource_type.as_deref(), limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total = state.audit_repo
        .count_with_filters(Some(&agent_id), query.action.as_deref(), query.resource_type.as_deref())
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<_> = logs.into_iter().map(|log| {
        crate::api::models::AuditLogResponse {
            id: log.id.to_string(),
            agent_id: log.agent_id,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            details: log.details,
            timestamp: log.timestamp.to_rfc3339(),
        }
    }).collect();

    Ok((StatusCode::OK, Json(crate::api::models::AuditLogListResponse {
        data,
        total,
        limit,
        offset,
    })))
}

pub async fn approve_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { roles, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    if !roles.iter().any(|r| r == "admin") {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    use crate::db::repositories::skill::SkillRepository;
    let repo = SkillRepository::new(state.evaluator.eval_repo.pool.clone());

    repo.update_status(&skill_id, "published")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve skill: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: None,
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "approved"}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(crate::api::models::SkillReviewResponse {
        message: "Skill approved successfully".to_string(),
        skill_id,
    })))
}

pub async fn reject_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { roles, .. }: AgentContext,
    Json(body): Json<crate::api::models::RejectSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    if !roles.iter().any(|r| r == "admin") {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    use crate::db::repositories::skill::SkillRepository;
    let repo = SkillRepository::new(state.evaluator.eval_repo.pool.clone());

    repo.update_status(&skill_id, "rejected")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject skill: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: None,
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "rejected", "reason": body.reason}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(crate::api::models::SkillReviewResponse {
        message: "Skill rejected".to_string(),
        skill_id,
    })))
}
```

- [ ] **Step 2: Commit**

```bash
git add src/api/handlers.rs
git commit -m "feat(api): add admin audit and review handlers"
```

---

## Task 5: Add Admin Routes

**Files:**
- Modify: `src/api/routes.rs`

- [ ] **Step 1: Update routes to include admin endpoints**

In `src/api/routes.rs`, update `create_api_router`:

```rust
pub fn create_api_router(state: ApiState) -> Router<ApiState> {
    Router::new()
        .route("/skills", get(list_skills_handler))
        .route("/skills", post(create_skill_handler))
        .route("/skills/:id", get(get_skill_handler))
        .route("/skills/:id", put(update_skill_handler))
        .route("/skills/:id", delete(delete_skill_handler))
        .route("/skills/:id/stats", get(get_skill_stats_handler))
        .route("/evaluations", post(create_evaluation_handler))
        .route("/agents/register", post(register_agent_handler))
        .route("/agents/token", post(get_token_handler))
        .route("/admin/audit", get(list_audit_logs_handler))      // ADD
        .route("/audit/my", get(list_my_audit_logs_handler))     // ADD
        .route("/admin/skills/:id/approve", post(approve_skill_handler))   // ADD
        .route("/admin/skills/:id/reject", post(reject_skill_handler))       // ADD
        .with_state(state)
}
```

- [ ] **Step 2: Commit**

```bash
git add src/api/routes.rs
git commit -m "feat(api): add admin routes"
```

---

## Task 6: Verify Build and Tests

- [ ] **Step 1: Build the project**

Run: `cargo build 2>&1`
Expected: Compiles without errors

- [ ] **Step 2: Run tests**

Run: `cargo test 2>&1`
Expected: All tests pass

- [ ] **Step 3: Commit final changes**

```bash
git add -A
git commit -m "feat: complete Phase 4 Admin API"
```

---

## Verification Checklist

- [ ] `GET /api/admin/audit` returns all audit logs for admin
- [ ] `GET /api/audit/my` returns current agent's logs
- [ ] `POST /api/admin/skills/{id}/approve` changes skill status to published
- [ ] `POST /api/admin/skills/{id}/reject` changes skill status to rejected
- [ ] Non-admin users get 401 on admin endpoints
- [ ] New skills have status pending_review by default
- [ ] All operations are logged to audit_logs