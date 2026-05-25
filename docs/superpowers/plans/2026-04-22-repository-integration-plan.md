# Phase 3: Repository Pattern Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace file-based storage with PostgreSQL repositories using trait abstraction layer.

**Architecture:** Services hold repository traits,委托数据访问到PostgreSQL实现。全替换策略，文件仅保留备份。

**Tech Stack:** Rust, sqlx, tokio-postgres, bcrypt

---

## File Structure

```
src/
├── db/
│   ├── mod.rs                    # Already exports modules
│   ├── traits.rs                 # NEW: Trait definitions
│   ├── error.rs                 # DbError already defined
│   ├── migrations.rs
│   └── repositories/
│       ├── mod.rs               # Already exports
│       ├── agent.rs             # AgentRepository already implemented
│       ├── skill.rs             # SkillRepository already implemented
│       ├── evaluation.rs        # EvaluationRepository already implemented
│       └── audit.rs             # AuditRepository already implemented
├── lib.rs                       # Add DbError → AppError conversion
├── services/
│   ├── mod.rs                   # Modify: re-export with generic types
│   ├── registry.rs              # Modify: add SkillRepositoryTrait
│   └── evaluator.rs             # Modify: add EvaluationRepositoryTrait
├── api/
│   ├── mod.rs                   # Modify: add AgentRepositoryTrait
│   ├── http_state.rs            # Modify: add repositories to state
│   └── handlers.rs              # Modify: use repositories for auth
└── models/
    └── error.rs                 # AppError already defined
```

---

## Task 1: Define Repository Traits

**Files:**
- Create: `src/db/traits.rs`

- [ ] **Step 1: Create traits file**

```rust
//! Repository traits for dependency injection

use crate::db::error::{DbError, DbResult};
use crate::db::repositories::agent::{Agent, NewAgent};
use crate::db::repositories::skill::{Skill, SkillMetadata, NewSkill};
use crate::db::repositories::evaluation::{Evaluation, NewEvaluation, SkillStats};
use crate::db::repositories::audit::{AuditLog, NewAuditLog};

pub trait AgentRepositoryTrait: Send + Sync {
    async fn create(&self, new_agent: NewAgent) -> DbResult<Agent>;
    async fn find_by_id(&self, agent_id: &str) -> DbResult<Option<Agent>>;
    async fn find_by_username(&self, username: &str) -> DbResult<Option<Agent>>;
    async fn verify_secret(&self, agent_id: &str, secret: &str) -> DbResult<bool>;
}

pub trait SkillRepositoryTrait: Send + Sync {
    async fn create(&self, new_skill: NewSkill) -> DbResult<Skill>;
    async fn find_by_id(&self, skill_id: &str) -> DbResult<Option<Skill>>;
    async fn list(&self, limit: i64, offset: i64) -> DbResult<Vec<SkillMetadata>>;
    async fn count(&self) -> DbResult<i64>;
    async fn update(&self, skill_id: &str, description: Option<&str>, content: Option<&str>, tags: Option<Vec<String>>) -> DbResult<()>;
    async fn delete(&self, skill_id: &str) -> DbResult<()>;
    async fn increment_install_count(&self, skill_id: &str) -> DbResult<()>;
}

pub trait EvaluationRepositoryTrait: Send + Sync {
    async fn create(&self, new_eval: NewEvaluation) -> DbResult<Evaluation>;
    async fn get_stats(&self, skill_id: &str) -> DbResult<SkillStats>;
    async fn list_by_skill(&self, skill_id: &str, limit: i64) -> DbResult<Vec<Evaluation>>;
}

pub trait AuditRepositoryTrait: Send + Sync {
    async fn create(&self, new_log: NewAuditLog) -> DbResult<AuditLog>;
    async fn list_by_agent(&self, agent_id: &str, limit: i64) -> DbResult<Vec<AuditLog>>;
}

// blanket implementation for existing repositories
impl<T: AgentRepositoryTrait + ?Sized> AgentRepositoryTrait for Box<T> {
    async fn create(&self, new_agent: NewAgent) -> DbResult<Agent> {
        (**self).create(new_agent).await
    }
    async fn find_by_id(&self, agent_id: &str) -> DbResult<Option<Agent>> {
        (**self).find_by_id(agent_id).await
    }
    async fn find_by_username(&self, username: &str) -> DbResult<Option<Agent>> {
        (**self).find_by_username(username).await
    }
    async fn verify_secret(&self, agent_id: &str, secret: &str) -> DbResult<bool> {
        (**self).verify_secret(agent_id, secret).await
    }
}

impl<T: SkillRepositoryTrait + ?Sized> SkillRepositoryTrait for Box<T> {
    async fn create(&self, new_skill: NewSkill) -> DbResult<Skill> {
        (**self).create(new_skill).await
    }
    async fn find_by_id(&self, skill_id: &str) -> DbResult<Option<Skill>> {
        (**self).find_by_id(skill_id).await
    }
    async fn list(&self, limit: i64, offset: i64) -> DbResult<Vec<SkillMetadata>> {
        (**self).list(limit, offset).await
    }
    async fn count(&self) -> DbResult<i64> {
        (**self).count().await
    }
    async fn update(&self, skill_id: &str, description: Option<&str>, content: Option<&str>, tags: Option<Vec<String>>) -> DbResult<()> {
        (**self).update(skill_id, description, content, tags).await
    }
    async fn delete(&self, skill_id: &str) -> DbResult<()> {
        (**self).delete(skill_id).await
    }
    async fn increment_install_count(&self, skill_id: &str) -> DbResult<()> {
        (**self).increment_install_count(skill_id).await
    }
}

impl<T: EvaluationRepositoryTrait + ?Sized> EvaluationRepositoryTrait for Box<T> {
    async fn create(&self, new_eval: NewEvaluation) -> DbResult<Evaluation> {
        (**self).create(new_eval).await
    }
    async fn get_stats(&self, skill_id: &str) -> DbResult<SkillStats> {
        (**self).get_stats(skill_id).await
    }
    async fn list_by_skill(&self, skill_id: &str, limit: i64) -> DbResult<Vec<Evaluation>> {
        (**self).list_by_skill(skill_id, limit).await
    }
}

impl<T: AuditRepositoryTrait + ?Sized> AuditRepositoryTrait for Box<T> {
    async fn create(&self, new_log: NewAuditLog) -> DbResult<AuditLog> {
        (**self).create(new_log).await
    }
    async fn list_by_agent(&self, agent_id: &str, limit: i64) -> DbResult<Vec<AuditLog>> {
        (**self).list_by_agent(agent_id, limit).await
    }
}
```

- [ ] **Step 2: Update db/mod.rs to export traits**

```rust
pub mod migrations;
pub mod repositories;
pub mod error;
pub mod traits;  // ADD THIS LINE

pub use error::{DbError, DbResult};
```

- [ ] **Step 3: Run tests to verify compilation**

```
cargo build --lib
```
Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add src/db/traits.rs src/db/mod.rs
git commit -m "feat(db): add repository traits for dependency injection"
```

---

## Task 2: Add DbError → AppError Conversion

**Files:**
- Modify: `src/lib.rs:19-41`

- [ ] **Step 1: Add error conversion impl to lib.rs**

Add this impl block after the AppState impl (around line 41):

```rust
use crate::db::error::DbError;

impl From<DbError> for AppError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound(msg) => AppError::SkillNotFound(msg),
            DbError::AlreadyExists(msg) => AppError::SkillAlreadyExists(msg),
            DbError::QueryError(msg) => AppError::InternalError(msg),
            DbError::ConnectionError(msg) => AppError::InternalError(format!("DB connection: {}", msg)),
            DbError::ValidationError(msg) => AppError::ValidationError(msg),
        }
    }
}
```

- [ ] **Step 2: Run tests to verify**

```
cargo test 2>&1 | head -50
```
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat(lib): add DbError to AppError conversion"
```

---

## Task 3: Refactor RegistryService with SkillRepositoryTrait

**Files:**
- Modify: `src/services/registry.rs`
- Modify: `src/services/mod.rs`

- [ ] **Step 1: Update RegistryService struct to hold repository**

In `src/services/registry.rs`, change the struct (around line 16):

```rust
use crate::db::traits::SkillRepositoryTrait;
use crate::db::repositories::skill::NewSkill as DbNewSkill;

pub struct RegistryService<R: SkillRepositoryTrait> {
    skills_dir: PathBuf,
    registry_dir: PathBuf,
    storage: StorageService,
    skill_repo: R,
}
```

- [ ] **Step 2: Update RegistryService::new to accept repository**

```rust
impl<R: SkillRepositoryTrait> RegistryService<R> {
    pub fn new(skills_dir: PathBuf, registry_dir: PathBuf, skill_repo: R) -> Self {
        let storage = StorageService::new(registry_dir.clone());
        Self {
            skills_dir,
            registry_dir,
            storage,
            skill_repo,
        }
    }
}
```

- [ ] **Step 3: Update create_skill to use repository**

Change the `create_skill` method (around line 63) to:
- Build `DbNewSkill` instead of using file-based storage
- Call `self.skill_repo.create(new_skill_db).await`

```rust
pub async fn create_skill(
    &self,
    new_skill: NewSkill,
    author_agent_id: &str,
    search: &SearchService,
) -> Result<Skill, AppError> {
    validate_skill_name(&new_skill.name)?;
    validate_tags(&new_skill.tags)?;
    validate_description(&new_skill.description)?;
    validate_version(&new_skill.version)?;
    validate_skill_content(&new_skill.content, &new_skill.name)?;

    let skill_id = Skill::generate_id(&new_skill.name, &new_skill.version);

    let new_skill_db = DbNewSkill {
        name: new_skill.name.clone(),
        description: new_skill.description.clone(),
        version: new_skill.version.clone(),
        author_agent_id: author_agent_id.to_string(),
        compatibility: ">=1.0.0".to_string(),
        content: new_skill.content.clone(),
        tags: new_skill.tags.clone(),
        dependencies: Vec::new(),
    };

    let skill = self.skill_repo.create(new_skill_db).await?;

    search.add_skill(&skill)?;
    info!("Created skill: {}", skill.id);

    Ok(skill)
}
```

- [ ] **Step 4: Update other methods (get_skill, list_skills, delete_skill, update_skill)**

For `get_skill` (line 254):
```rust
pub fn get_skill(&self, skill_id: &str) -> Result<Skill, AppError> {
    let skill = futures::executor::block_on(self.skill_repo.find_by_id(skill_id))?
        .ok_or_else(|| AppError::SkillNotFound(skill_id.to_string()))?;
    Ok(skill)
}
```

For `list_skills` (line 274):
```rust
pub fn list_skills(&self) -> Result<Vec<SkillMetadata>, AppError> {
    let skills = futures::executor::block_on(self.skill_repo.list(1000, 0))?;
    Ok(skills.into_iter().map(|m| SkillMetadata {
        id: m.id,
        name: m.name,
        description: m.description,
        version: m.version,
        author_agent_id: m.author_agent_id,
        tags: m.tags,
        created: m.created_at,
        updated: m.updated_at,
        install_count: m.install_count,
    }).collect())
}
```

For `count` (line 280):
```rust
pub fn count(&self) -> Result<u32, AppError> {
    let count = futures::executor::block_on(self.skill_repo.count())?;
    Ok(count as u32)
}
```

For `delete_skill` (line 223):
```rust
pub fn delete_skill(&self, skill_id: &str, search: &SearchService) -> Result<(), AppError> {
    futures::executor::block_on(self.skill_repo.delete(skill_id))?;
    search.delete_skill(skill_id)?;
    info!("Deleted skill: {}", skill_id);
    Ok(())
}
```

For `update_skill` (line 135) - similar pattern, delegate to skill_repo.update()

- [ ] **Step 5: Update services/mod.rs to re-export with generic type**

```rust
pub mod storage;
pub mod search;
pub mod registry;
pub mod evaluator;

pub use storage::*;
pub use search::*;
pub use registry::*;
pub use evaluator::*;
```

- [ ] **Step 6: Run tests**

```
cargo build --lib 2>&1 | head -50
```
Expected: Compilation errors (expected, services need AppRouterState updates)

- [ ] **Step 7: Commit**

```bash
git add src/services/registry.rs src/services/mod.rs
git commit -m "refactor(services): add SkillRepositoryTrait to RegistryService"
```

---

## Task 4: Refactor EvaluatorService with EvaluationRepositoryTrait

**Files:**
- Modify: `src/services/evaluator.rs`

- [ ] **Step 1: Update EvaluatorService struct**

```rust
use crate::db::traits::EvaluationRepositoryTrait;
use crate::db::repositories::evaluation::{NewEvaluation as DbNewEvaluation, SkillStats as DbSkillStats};

pub struct EvaluatorService<R: EvaluationRepositoryTrait> {
    storage: StorageService,
    rate_limiter: RateLimiter,
    eval_repo: R,
}

impl<R: EvaluationRepositoryTrait> EvaluatorService<R> {
    pub fn new(data_dir: PathBuf, eval_repo: R) -> Self {
        Self {
            storage: StorageService::new(data_dir.clone()),
            rate_limiter: RateLimiter::default(),
            eval_repo,
        }
    }
}
```

- [ ] **Step 2: Update add_evaluation method**

```rust
pub async fn add_evaluation(
    &self,
    skill_id: String,
    agent_id: String,
    success: bool,
    duration_ms: u64,
    error_type: Option<ErrorType>,
    tags: Vec<EvalTag>,
) -> Result<EvaluationResult, AppError> {
    validate_evaluation_input(&skill_id, duration_ms)?;

    let rate_key = format!("{}:{}", skill_id, agent_id);
    if !self.rate_limiter.check(&rate_key).await {
        return Err(AppError::EvaluationRateLimited);
    }

    let eval_db = DbNewEvaluation {
        skill_id: skill_id.clone(),
        agent_id: agent_id.clone(),
        success,
        duration_ms: duration_ms as i64,
        error_type: error_type.map(|e| format!("{:?}", e)),
        tags: tags.iter().map(|t| format!("{:?}", t)).collect(),
    };

    let evaluation = self.eval_repo.create(eval_db).await?;
    let stats = self.eval_repo.get_stats(&skill_id).await?;

    debug!("Added evaluation for skill: {}, success: {}", skill_id, success);

    Ok(EvaluationResult {
        success: true,
        evaluation_id: evaluation.id.to_string(),
        new_stats: SkillStats {
            skill_id: stats.skill_id,
            success_rate: stats.success_rate,
            avg_duration_ms: stats.avg_duration_ms,
            total_evaluations: stats.total_evaluations as u32,
            unique_agents: stats.unique_agents as u32,
            confidence: stats.confidence,
            tags: stats.tags,
            local_version: None,
            latest_version: "1.0.0".to_string(),
            upgrade_available: false,
        },
    })
}
```

- [ ] **Step 3: Update get_stats method**

```rust
pub fn get_stats(&self, skill_id: &str) -> Result<SkillStats, AppError> {
    let stats = futures::executor::block_on(self.eval_repo.get_stats(skill_id))?;
    Ok(SkillStats {
        skill_id: stats.skill_id,
        success_rate: stats.success_rate,
        avg_duration_ms: stats.avg_duration_ms,
        total_evaluations: stats.total_evaluations as u32,
        unique_agents: stats.unique_agents as u32,
        confidence: stats.confidence,
        tags: stats.tags,
        local_version: None,
        latest_version: "1.0.0".to_string(),
        upgrade_available: false,
    })
}
```

- [ ] **Step 4: Update list_evaluations method**

```rust
pub fn list_evaluations(&self, skill_id: &str) -> Result<Vec<Evaluation>, AppError> {
    let evals = futures::executor::block_on(self.eval_repo.list_by_skill(skill_id, 100))?;
    Ok(evals.into_iter().map(|e| Evaluation {
        id: e.id.to_string(),
        skill_id: e.skill_id,
        agent_id: e.agent_id,
        success: e.success,
        duration_ms: e.duration_ms as u64,
        error_type: e.error_type.and_then(|s| match s.as_str() {
            "Timeout" => Some(ErrorType::Timeout),
            "Crash" => Some(ErrorType::Crash),
            "LogicError" => Some(ErrorType::LogicError),
            _ => Some(ErrorType::Other),
        }),
        tags: e.tags.into_iter().filter_map(|s| match s.as_str() {
            "Reliable" => Some(EvalTag::Reliable),
            "Fast" => Some(EvalTag::Fast),
            "Stable" => Some(EvalTag::Stable),
            "Experimental" => Some(EvalTag::Experimental),
            _ => None,
        }).collect(),
        timestamp: e.timestamp,
    }).collect())
}
```

- [ ] **Step 5: Update tests in evaluator.rs to work with trait**

The tests need to be updated to pass a mock repository. For now, keep the original file-based implementation as default or update tests.

- [ ] **Step 6: Run tests**

```
cargo build --lib 2>&1 | head -80
```

- [ ] **Step 7: Commit**

```bash
git add src/services/evaluator.rs
git commit -m "refactor(services): add EvaluationRepositoryTrait to EvaluatorService"
```

---

## Task 5: Refactor API Handlers for Agent Auth

**Files:**
- Modify: `src/api/http_state.rs`
- Modify: `src/api/handlers.rs`

- [ ] **Step 1: Update AppRouterState to include repositories**

In `src/api/http_state.rs`:

```rust
use crate::db::repositories::agent::AgentRepository;
use crate::db::repositories::audit::AuditRepository;

#[derive(Clone)]
pub struct AppRouterState {
    pub http: HttpState,
    pub sse: SseState,
    pub registry: RegistryService<AgentRepository>,
    pub search: SearchService,
    pub evaluator: EvaluatorService<AuditRepository>,
    pub agent_repo: AgentRepository,
    pub audit_repo: AuditRepository,
}
```

Actually, this creates a circular dependency. The services need repositories injected, but the state needs both services and repositories. Let's reconsider:

The simpler approach: make the repository concrete in state, and pass it to services on construction.

```rust
use crate::db::repositories::{AgentRepository, SkillRepository, EvaluationRepository, AuditRepository};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppRouterState {
    pub http: HttpState,
    pub sse: SseState,
    pub registry: Arc<RegistryService>,
    pub search: SearchService,
    pub evaluator: Arc<EvaluatorService>,
    pub agent_repo: AgentRepository,
    pub audit_repo: AuditRepository,
}
```

But this requires RegistryService and EvaluatorService to be generic. Let's simplify by using type alias with concrete types after implementing all traits.

For now, update handlers to use agent_repo directly for auth.

- [ ] **Step 2: Update get_token_handler to use AgentRepository**

In `src/api/handlers.rs`, update `get_token_handler` (line 211):

```rust
pub async fn get_token_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::GetTokenBody>,
) -> Result<impl IntoResponse, ApiError> {
    let valid = state.agent_repo
        .verify_secret(&body.agent_id, &body.secret)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("{:?}", e)))?;

    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let token = crate::api::generate_token(&body.agent_id, vec![], vec![])
        .map_err(|e| ApiError::Unauthorized(format!("{:?}", e)))?;

    let response = crate::api::models::TokenResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: 86400,
    };
    Ok((StatusCode::OK, Json(response)))
}
```

- [ ] **Step 3: Update register_agent_handler to use AgentRepository**

```rust
pub async fn register_agent_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::RegisterAgentBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::agent::NewAgent;

    let new_agent = NewAgent {
        agent_id: body.agent_id.clone(),
        agent_secret: body.secret.clone(),
        agent_name: None,
    };

    state.agent_repo
        .create(new_agent)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to register agent: {:?}", e)))?;

    let response = crate::api::models::RegisterAgentResponse {
        agent_id: body.agent_id,
        secret: body.secret,
        message: "Agent registered successfully.".to_string(),
    };
    Ok((StatusCode::CREATED, Json(response)))
}
```

- [ ] **Step 4: Run cargo build**

```
cargo build --lib 2>&1 | head -100
```

- [ ] **Step 5: Commit**

```bash
git add src/api/http_state.rs src/api/handlers.rs
git commit -m "refactor(api): integrate AgentRepository for authentication"
```

---

## Task 6: Update AppRouterState Construction

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update main.rs to construct state with repositories**

Look at current main.rs to understand how state is constructed, then update to:

```rust
let agent_repo = AgentRepository::new(pool.clone());
let skill_repo = SkillRepository::new(pool.clone());
let eval_repo = EvaluationRepository::new(pool.clone());
let audit_repo = AuditRepository::new(pool.clone());

let registry = RegistryService::new(skills_dir, data_dir.join("registry"), skill_repo);
let evaluator = EvaluatorService::new(data_dir.clone(), eval_repo);

let state = AppRouterState {
    http: HttpState { mcp_server },
    sse: SseState::new(),
    registry,
    search,
    evaluator,
    agent_repo,
    audit_repo,
};
```

- [ ] **Step 2: Run cargo build**

```
cargo build 2>&1 | head -100
```

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat(main): wire up repositories in application state"
```

---

## Task 7: Run Full Test Suite

- [ ] **Step 1: Run all tests**

```
cargo test 2>&1
```

Expected: Most tests pass. Some file-based tests may fail (expected, we're switching to DB).

- [ ] **Step 2: Fix any compilation errors**

Address any type mismatches or missing imports.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "test: run full test suite after Phase 3 integration"
```

---

## Verification Checklist

- [ ] All 4 repository traits defined in `src/db/traits.rs`
- [ ] DbError → AppError conversion working
- [ ] RegistryService uses SkillRepositoryTrait
- [ ] EvaluatorService uses EvaluationRepositoryTrait
- [ ] API handlers use AgentRepository for auth
- [ ] Application builds without errors
- [ ] Tests pass (or known failures documented)