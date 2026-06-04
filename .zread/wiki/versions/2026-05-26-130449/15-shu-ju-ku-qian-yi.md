本文档详细说明 AionHive 技能共享平台的数据库迁移系统架构、迁移执行流程以及数据库模式演进的完整历史。

## 迁移系统概述

AionHive 采用基于 SQLx 的自定义迁移系统，该系统具有以下核心特性：

```mermaid
flowchart TB
    subgraph 迁移系统架构
        A[AppState::new] --> B[db::migrations::run_migrations]
        B --> C[创建 _migrations 跟踪表]
        C --> D{遍历迁移列表}
        D -->|检查| E{已应用?}
        E -->|否| F[split_sql_statements]
        E -->|是| G[跳过]
        F --> H[逐条执行 SQL]
        H --> I[记录到 _migrations]
        I --> D
    end
    
    subgraph 数据存储
        J[_migrations 表] 
        K[业务表]
    end
    
    I --> J
    H --> K
```

**设计特点**：

1. **内嵌式迁移**：SQL 文件通过 `include_str!` 宏内嵌到二进制中，确保迁移与代码版本一致
2. **幂等性检查**：通过 `_migrations` 表跟踪已执行的迁移，避免重复应用
3. **语句级分割**：使用 `split_sql_statements()` 函数按分号分割，支持多语句迁移文件
4. **自动初始化**：首次运行时自动创建 `_migrations` 跟踪表

Sources: [migrations.rs](src/db/migrations.rs#L1-L85), [lib.rs](src/lib.rs#L48-L50)

## 迁移执行流程

### 应用初始化时的迁移触发

迁移在 `AppState::new()` 中自动执行，这是应用启动的关键路径：

```mermaid
sequenceDiagram
    participant Main as main.rs
    participant AppState as AppState::new()
    participant Pool as PgPool
    participant Migrations as run_migrations()
    participant Postgres as PostgreSQL
    
    Main->>AppState: new(data_dir, skills_dir)
    AppState->>Pool: PgPool::connect(DATABASE_URL)
    Pool-->>AppState: pool
    AppState->>Migrations: run_migrations(&pool, &data_dir)
    Migrations->>Postgres: CREATE TABLE IF NOT EXISTS _migrations
    Migrations->>Postgres: SELECT FROM _migrations WHERE name = ?
    loop 每个迁移文件
        alt 未应用
            Migrations->>Postgres: EXECUTE SQL statements
            Migrations->>Postgres: INSERT INTO _migrations
        else 已应用
            Migrations->>Migrations: skip
        end
    end
    Migrations-->>AppState: Ok(())
```

Sources: [lib.rs](src/lib.rs#L45-L60), [migrations.rs](src/db/migrations.rs#L43-L78)

### 数据库连接配置

数据库连接通过 `DATABASE_URL` 环境变量配置，默认为 `postgres://localhost:5432/aionhive`：

```bash
# .env.example
DATABASE_URL=postgres://postgres:password@localhost:5432/aionhive
```

Sources: [.env.example](.env.example#L6), [lib.rs](src/lib.rs#L48)

## 迁移版本历史

系统共经历 **11 个迁移版本**，逐步构建完整的数据模型：

### 迁移清单概览

| 编号 | 迁移名称 | 主要变更 | 迁移类型 |
|------|----------|----------|----------|
| 001 | initial_schema | 创建核心表结构 | Schema |
| 002 | add_skill_status | 添加技能状态字段 | Schema |
| 003 | seed_admin_agent | 初始化管理员账户 | Seed |
| 004 | add_organizations | 多租户支持 | Schema |
| 005 | add_sessions | 会话管理 | Schema |
| 006 | add_org_tools | 组织私有工具 | Schema |
| 007 | add_skill_policies | 技能可见性策略 | Schema |
| 008 | add_skill_git_and_org_fields | Git 集成与组织字段 | Schema |
| 009 | add_agent_id_column | 添加 UUID 主键 | Schema |
| 010 | add_admin_users | 管理员用户表 | Schema + Seed |
| 011 | add_session_skill_fields | 会话技能字段扩展 | Schema |

### 001_initial_schema - 核心数据模型

```mermaid
erDiagram
    agents {
        varchar agent_id PK
        varchar agent_secret_hash
        varchar agent_name
        text[] roles
        timestamptz created_at
        timestamptz updated_at
    }
    
    skills {
        varchar id PK
        varchar name
        text description
        varchar version
        varchar author_agent_id FK
        varchar compatibility
        text content
        int install_count
        timestamptz created_at
        timestamptz updated_at
    }
    
    skill_tags ||--o{ skills : tags
    skill_dependencies ||--o{ skills : dependencies
    
    skills ||--o{ evaluations : has
    agents ||--o{ evaluations : evaluates
```

**核心表结构**：
- **agents**：JWT 认证的代理账户表
- **skills**：技能定义表，支持版本控制和兼容性声明
- **skill_tags**：技能标签多对多关联
- **skill_dependencies**：技能依赖关系
- **evaluations**：技能使用评价记录
- **audit_logs**：审计日志表

Sources: [001_initial_schema.sql](src/db/migrations/001_initial_schema.sql#L1-L80)

### 002-003 - 状态管理与种子数据

```sql
-- 002: 添加技能状态枚举
ALTER TABLE skills ADD COLUMN status VARCHAR(50);

-- 003: 幂等性更新管理员角色
UPDATE agents
SET roles = ARRAY['admin'], updated_at = NOW()
WHERE agent_id = 'admin-1'
AND NOT ('admin' = ANY(roles));
```

**设计模式**：迁移 003 展示了幂等性迁移的最佳实践 —— 使用 `AND NOT ('admin' = ANY(roles))` 条件确保重复执行安全。

Sources: [002_add_skill_status.sql](src/db/migrations/002_add_skill_status.sql#L1-L6), [003_seed_admin_agent.sql](src/db/migrations/003_seed_admin_agent.sql#L1-L9)

### 004-007 - 多租户架构演进

```mermaid
flowchart LR
    subgraph Organizations
        O1[组织 A]
        O2[组织 B]
    end
    
    subgraph Sessions
        S1[会话 1]
        S2[会话 2]
    end
    
    subgraph OrgTools
        T1[工具 A]
        T2[工具 B]
    end
    
    subgraph SkillPolicies
        P1[策略 1]
        P2[策略 2]
    end
    
    O1 --> S1
    O1 --> T1
    O1 --> P1
    O2 --> S2
    O2 --> T2
    O2 --> P2
```

**多租户扩展**：
- **organizations**：租户隔离单元
- **sessions**：租户内会话管理，集成工具路由器
- **org_tools**：租户私有工具注册
- **skill_policies**：租户级技能可见性控制

Sources: [004_add_organizations.sql](src/db/migrations/004_add_organizations.sql#L1-L12), [005_add_sessions.sql](src/db/migrations/005_add_sessions.sql#L1-L22), [006_add_org_tools.sql](src/db/migrations/006_add_org_tools.sql#L1-L20), [007_add_skill_policies.sql](src/db/migrations/007_add_skill_policies.sql#L1-L17)

### 008 - Git 集成与可见性扩展

```sql
-- 技能表扩展
ALTER TABLE skills ADD COLUMN git_url VARCHAR(500);
ALTER TABLE skills ADD COLUMN visibility VARCHAR(50) DEFAULT 'org_visible';
ALTER TABLE skills ADD COLUMN skill_tools JSONB DEFAULT '[]';

-- 代理表扩展
ALTER TABLE agents ADD COLUMN org_id UUID;
ALTER TABLE agents ADD COLUMN capabilities JSONB DEFAULT '[]';

CREATE INDEX idx_skills_visibility ON skills(visibility);
```

**可见性模型**：支持 `org_visible`、`public` 等多种可见性级别。

Sources: [008_add_skill_git_and_org_fields.sql](src/db/migrations/008_add_skill_git_and_org_fields.sql#L1-L15)

### 009-011 - 治理能力增强

```sql
-- 009: 代理 UUID 主键
ALTER TABLE agents ADD COLUMN id UUID DEFAULT uuid_generate_v4();
UPDATE agents SET id = uuid_generate_v4() WHERE id IS NULL;
ALTER TABLE agents ALTER COLUMN id SET NOT NULL;

-- 010: 管理员用户表
CREATE TABLE admin_users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    is_active BOOLEAN DEFAULT true,
    ...
);

-- 011: 会话与技能审批字段
ALTER TABLE sessions ADD COLUMN capabilities JSONB DEFAULT '[]';
ALTER TABLE sessions ADD COLUMN last_active_at TIMESTAMPTZ;
ALTER TABLE skills ADD COLUMN approved_at TIMESTAMPTZ;
ALTER TABLE skills ADD COLUMN approved_by VARCHAR(255);
```

Sources: [009_add_agent_id_column.sql](src/db/migrations/009_add_agent_id_column.sql#L1-L15), [010_add_admin_users.sql](src/db/migrations/010_add_admin_users.sql#L1-L19), [011_add_session_skill_fields.sql](src/db/migrations/011_add_session_skill_fields.sql#L1-L14)

## 仓库层与迁移后数据访问

迁移创建的表通过仓储模式（Repository Pattern）进行访问：

```mermaid
classDiagram
    class SkillRepository {
        +PgPool pool
        +create(NewSkill) Skill
        +find_by_id(String) Option~Skill~
        +list(i64, i64) Vec~SkillMetadata~
        +update(String, SkillUpdate) Skill
        +delete(String) bool
    }
    
    class OrganizationRepository {
        +PgPool pool
        +create(NewOrganization) Organization
        +find_by_id(UUID) Option~Organization~
    }
    
    class SessionRepository {
        +PgPool pool
        +create(NewSession) Session
        +update_status(UUID, SessionStatus) Session
    }
    
    SkillRepository ..> DbError
    OrganizationRepository ..> DbError
    SessionRepository ..> DbError
```

Sources: [repositories/mod.rs](src/db/repositories/mod.rs#L1-L22), [repositories/skill.rs](src/db/repositories/skill.rs#L1-L100)

## 错误处理机制

迁移系统定义了统一的数据库错误类型：

```rust
pub enum DbError {
    ConnectionError(String),   // 连接失败
    QueryError(String),        // SQL 执行错误
    NotFound(String),          // 记录未找到
    AlreadyExists(String),     // 记录已存在
    ValidationError(String),   // 数据验证失败
}

pub type DbResult<T> = Result<T, DbError>;
```

**错误映射**：在 `AppState` 中，`DbError` 自动转换为 `AppError`：

| DbError | AppError |
|---------|----------|
| NotFound | SkillNotFound |
| AlreadyExists | SkillAlreadyExists |
| QueryError | InternalError |
| ConnectionError | InternalError |
| ValidationError | ValidationError |

Sources: [error.rs](src/db/error.rs#L1-L24), [lib.rs](src/lib.rs#L100-L117)

## 新增迁移指南

要为系统添加新迁移，请遵循以下步骤：

### 1. 创建迁移文件

在 `src/db/migrations/` 目录下创建新文件：

```sql
-- src/db/migrations/012_your_migration_name.sql
-- Description: 迁移说明

ALTER TABLE skills ADD COLUMN new_field VARCHAR(255);
CREATE INDEX idx_skills_new_field ON skills(new_field);
```

### 2. 注册迁移

在 `src/db/migrations.rs` 的 `MIGRATIONS` 数组中添加条目：

```rust
const MIGRATIONS: &[(&str, &str)] = &[
    // ... 现有迁移 ...
    ("012_your_migration_name", include_str!("migrations/012_your_migration_name.sql")),
];
```

### 3. 幂等性设计原则

```sql
-- ✓ 推荐：使用 IF NOT EXISTS 或条件检查
CREATE INDEX IF NOT EXISTS idx_table_column ON table(column);

-- ✓ 推荐：使用 ON CONFLICT 处理种子数据
INSERT INTO admin_users (username, password_hash)
VALUES ('new_admin', 'hash')
ON CONFLICT (username) DO NOTHING;

-- ✓ 推荐：使用 ADD COLUMN IF NOT EXISTS
ALTER TABLE skills ADD COLUMN IF NOT EXISTS new_field TYPE;
```

Sources: [migrations.rs](src/db/migrations.rs#L11-L23)

## 运维注意事项

### 迁移执行监控

迁移执行时会输出结构化日志：

```rust
tracing::info!("Running migration: {}", name);  // 开始
tracing::info!("Migration {} completed", name);  // 完成
```

### 数据库健康检查

使用 `check_migrations()` 函数验证迁移状态：

```rust
pub async fn check_migrations(pool: &PgPool) -> DbResult<bool> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'agents'"
    )
    .fetch_one(pool)
    .await?;
    
    Ok(row.0 > 0)
}
```

### 生产环境建议

1. **备份优先**：在生产环境执行迁移前务必备份数据库
2. **幂等性验证**：确保迁移可安全重复执行
3. **大事务处理**：复杂迁移应使用显式事务包裹
4. **回滚方案**：保留回滚脚本或设计可逆迁移

---

## 相关文档

- [数据模型](14-shu-ju-mo-xing) - 完整的数据库实体关系说明
- [存储服务](16-cun-chu-fu-wu) - 持久化层设计与实现
- [系统架构](8-xi-tong-jia-gou) - 整体系统架构概览