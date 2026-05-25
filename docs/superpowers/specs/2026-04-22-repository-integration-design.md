# Phase 3: Repository Pattern Integration Design

**Date**: 2026-04-22
**Status**: Approved

## Overview

Phase 3 integrates the PostgreSQL repositories (completed in Phase 2) with the existing services, replacing file-based storage with database storage using a trait-based abstraction layer.

## Architecture

```
API Handlers
    ↓
Services (business logic, hold Repository traits)
    ↓
Repository Traits (defined in src/db/traits.rs)
    ↓
PostgreSQL Repositories (implement traits)
```

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Migration strategy | Full replacement | Simplest, no technical debt |
| Scope | All entities (Agent, Skill, Evaluation, Audit) | Consistent approach |
| File handling | Retain files (backup only) | Safe, reversible |
| Integration pattern | Traits abstraction | Most flexible, swappable |
| Trait granularity | Per-entity (4 traits) | Aligns with existing repository implementations |
| Error handling | Centralized DbError → AppError conversion | Unified error management |
| Trait location | src/db/traits.rs | Couples traits with db module |

## Trait Definitions

### AgentRepositoryTrait
- `create(NewAgent) -> DbResult<Agent>`
- `find_by_id(agent_id) -> DbResult<Option<Agent>>`
- `find_by_username(username) -> DbResult<Option<Agent>>`
- `list(limit, offset) -> DbResult<Vec<Agent>>`
- `verify_password(agent_id, password) -> DbResult<bool>`

### SkillRepositoryTrait
- `create(NewSkill) -> DbResult<Skill>`
- `find_by_id(skill_id) -> DbResult<Option<Skill>>`
- `list(limit, offset) -> DbResult<Vec<SkillMetadata>>`
- `count() -> DbResult<i64>`
- `update(skill_id, description, content, tags) -> DbResult<()>`
- `delete(skill_id) -> DbResult<()>`
- `increment_install_count(skill_id) -> DbResult<()>`

### EvaluationRepositoryTrait
- `create(NewEvaluation) -> DbResult<Evaluation>`
- `get_stats(skill_id) -> DbResult<SkillStats>`
- `list_by_skill(skill_id, limit) -> DbResult<Vec<Evaluation>>`

### AuditRepositoryTrait
- `create(NewAudit) -> DbResult<Audit>`
- `list_by_agent(agent_id, limit) -> DbResult<Vec<Audit>>`

## Error Conversion

In `src/lib.rs`:
```rust
impl From<DbError> for AppError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound(msg) => AppError::NotFound(msg),
            DbError::AlreadyExists(msg) => AppError::Conflict(msg),
            DbError::QueryError(msg) => AppError::InternalError(msg),
        }
    }
}
```

## Service Layer Changes

### RegistryService
```rust
pub struct RegistryService<R: SkillRepositoryTrait> {
    skills_dir: PathBuf,
    registry_dir: PathBuf,
    storage: StorageService,
    skill_repo: R,
}
```

### EvaluatorService
```rust
pub struct EvaluatorService<R: EvaluationRepositoryTrait> {
    storage: StorageService,
    rate_limiter: RateLimiter,
    eval_repo: R,
}
```

## File Handling

Existing files in `data/registry/` and `data/evaluations/` are retained as backup after migration. They are not read during normal operation.

## Implementation Order

1. Define traits in `src/db/traits.rs`
2. Add DbError → AppError conversion in `src/lib.rs`
3. Refactor RegistryService to use SkillRepositoryTrait
4. Refactor EvaluatorService to use EvaluationRepositoryTrait
5. Refactor agent authentication to use AgentRepositoryTrait
6. Refactor audit logging to use AuditRepositoryTrait
7. Update and run tests

## Data Models

The Phase 2 PostgreSQL schema uses:
- `agents` table with bcrypt-hashed passwords
- `skills` table with separate `skill_tags` and `skill_dependencies` tables
- `evaluations` table with ARRAY column for tags
- `audit_logs` table for audit trail

All data is migrated from file-based storage to PostgreSQL during this phase.