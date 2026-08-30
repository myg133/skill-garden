本文档深入解析 AionHive 的数据库迁移体系与初始化流程。迁移系统采用**自建版本管理**（而非 sqlx 内置的 migrate! 宏），通过 `_migrations` 追踪表精确控制 40 个 SQL 迁移文件的执行顺序与幂等性。数据库初始化是应用启动的核心环节，在 `AppState::new()` 中作为首要步骤完成。

## 迁移架构设计

### 设计原则

迁移系统遵循三条核心原则：**幂等性**（每个迁移文件可安全重复执行）、**不可逆演进**（所有迁移仅做 ADD/ALTER/CREATE，不做破坏性 DROP）、**模块化分阶段**（40 个迁移按功能域划分为 6 个演进阶段）。系统不依赖外部迁移工具，完全内建在应用代码中。

### 核心组件

迁移引擎由三个组件构成：`MIGRATIONS` 静态数组（在编译时通过 `include_str!` 嵌入所有 SQL 文件）、`_migrations` 追踪表（记录已执行迁移的名称和时间戳）、以及 `split_sql_statements` 函数（按分号拆分 SQL 语句逐条执行）。这种设计避免了外部文件依赖，确保二进制部署时迁移始终可用。

```rust
// 编译时嵌入所有 SQL 文件
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial_schema", include_str!("migrations/001_initial_schema.sql")),
    // ... 共 40 个迁移
];
```
Sources: [migrations.rs](src/db/migrations.rs#L7-L97)

### 执行流程

```mermaid
flowchart TD
    A[AppState::new 启动] --> B[创建 PgPool 连接]
    B --> C[run_migrations]
    C --> D[CREATE TABLE IF NOT EXISTS _migrations]
    D --> E{遍历 MIGRATIONS 数组}
    E --> F[检查 _migrations 表\n该迁移是否已执行?]
    F -->|是| G[跳过]
    F -->|否| H[逐条执行 SQL 语句]
    H --> I[INSERT INTO _migrations 记录]
    I --> J{还有下一个迁移?}
    J -->|是| E
    J -->|否| K[迁移完成]
    K --> L[初始化 Repository 层]
    L --> M[重建 Tantivy 搜索索引]
```

Sources: [lib.rs](src/lib.rs#L166-L170), [migrations.rs](src/db/migrations.rs#L99-L142)

## 40 个迁移的演进路线

### 阶段一：初始架构 (001-012)

这一阶段建立了平台的核心数据模型：Agent 认证体系、Skill 资产表（含标签、依赖、评价）、审计日志、组织架构、会话管理和工具注册。迁移 001 创建了初始的 `agents`、`skills`、`evaluations`、`audit_logs` 四张核心表，并启用了 `uuid-ossp` 扩展。迁移 003 插入了默认的 admin agent。迁移 004 引入 `organizations` 表，为后续多租户奠定基础。

```sql
-- 001_initial_schema: 核心表结构
CREATE TABLE agents (
    agent_id VARCHAR(255) PRIMARY KEY,
    agent_secret_hash VARCHAR(255) NOT NULL,
    roles TEXT[] DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```
Sources: [001_initial_schema.sql](src/db/migrations/001_initial_schema.sql#L1-L80)

### 阶段二：多租户与 RBAC 基础 (013-020)

这是一个关键的架构转折点。迁移 013 引入了完整的多租户体系：`tenants`、`groups` 表，并在 `organizations` 表上增加了 `tenant_id`、`slug` 等字段。迁移 014 建立了身份抽象层 `identities`（统一 User/Agent）、角色 `roles`、权限点 `permissions`，以及成员关系 `memberships` 和 `organization_identities` 表。迁移 015 增加了 `api_keys`、`audit_log_entries`、`skill_evaluations` 三张新表。迁移 018 是 RBAC 体系的完善，引入 `role_permissions`、`group_permission_overrides`、`group_skills`、`licenses` 四张表，并定义了 50+ 个细粒度权限点。

```sql
-- 014: 角色-权限模型的核心
INSERT INTO roles (name, role_type, scope_level, permissions, description) VALUES
('super_admin', 'system', 'global', '["*"]', 'Super Administrator'),
('marketplace_admin', 'system', 'global', '["skill:approve", "skill:publish", ...]', 'Marketplace Administrator'),
('tenant_admin', 'tenant', 'tenant', '["tenant:manage", "org:manage", ...]', 'Tenant Administrator'),
('org_admin', 'organization', 'org', '["org:manage", "org:configure", ...]', 'Organization Administrator'),
('skill_developer', 'organization', 'org', '["skill:create", "skill:read", ...]', 'Skill Developer');
```
Sources: [013_add_tenants.sql](src/db/migrations/013_add_tenants.sql#L1-L55), [014_add_identities_and_roles.sql](src/db/migrations/014_add_identities_and_roles.sql#L1-L128), [018_add_rbac_and_group_skills.sql](src/db/migrations/018_add_rbac_and_group_skills.sql#L1-L389)

### 阶段三：身份合并与版本管理 (021-025)

迁移 021 将 `admin_users` 表合并到 `identities` 表，统一了身份模型。迁移 022 引入 `skill_versions` 表，开始追踪 Skill 版本历史与 Git 提交的关联。迁移 023 增加了 `git_remote_url` 字段。迁移 024 增强了 Agent 模型。迁移 025 修复了 Sessions 与 Identity 的关联关系。

| 迁移 | 核心变更 | 目的 |
|------|---------|------|
| 021 | `admin_users` → `identities` 合并 | 统一身份模型，消除数据冗余 |
| 022 | 创建 `skill_versions` 表 | 版本历史追踪，Git 集成基础 |
| 023 | 添加 `git_remote_url` 字段 | 支持远程 Git 仓库 URL |
| 024 | 增强 agents 表字段 | Agent 元数据扩展 |
| 025 | 修复 sessions.identity_id 引用 | 数据一致性修正 |

Sources: [022_add_skill_versions.sql](src/db/migrations/022_add_skill_versions.sql#L1-L27)

### 阶段四：RBAC 完善与下载凭证 (026-031)

迁移 026 使 `api_keys.organization_id` 可为空（支持个人用户 API Key），种入默认 `skill_user` 角色，并创建 `download_tokens` 表用于技能下载审计。迁移 028 增加 `admin_unpublished` 字段。迁移 029 统一了管理后台认证。迁移 030 引入 `tenant_role_assignments`。迁移 031 使用固定 UUID `00000000-0000-0000-0000-000000000001` 种入初始 super_admin 身份（默认密码 `admin/admin123`，首次登录后必须修改）。

```sql
-- 031: 初始化超级管理员
INSERT INTO identities (
    id, identity_type, name, username, display_name, email,
    password_hash, is_system_admin, status
) VALUES (
    '00000000-0000-0000-0000-000000000001', 'user', 'admin', 'admin', 'Super Admin',
    'admin@aionhive.local',
    '$2b$12$LJ3m4ys3GZfnYMz8kVsKaOlSiWhU2wZFPm./bCv4xJvK5pTM1XhKm',
    true, 'active'
);
```
Sources: [026_rbac_and_download_tokens.sql](src/db/migrations/026_rbac_and_download_tokens.sql#L1-L56), [031_seed_admin_user.sql](src/db/migrations/031_seed_admin_user.sql#L1-L31)

### 阶段五：Marketplace 双轨制 (032-037)

这一阶段实现了 Skill 可见性与 Marketplace 状态的分离。迁移 032 引入 `marketplace_status` 和 `pre_marketplace_visibility` 字段，使 Skill 的本地发布状态与市场上市状态解耦。迁移 033 增加市场相关权限点。迁移 034 清理了 27 个遗留权限码。迁移 035 增加 `pending_delist` 状态。迁移 036 增加 `draft_content` 字段并扩展了状态约束。迁移 037 移除了 marketplace_admin 的内部审核权限。

```sql
-- 032: marketplace_status 字段定义
ALTER TABLE skills ADD COLUMN marketplace_status VARCHAR(50) DEFAULT NULL;
ALTER TABLE skills ADD CONSTRAINT chk_marketplace_status CHECK (
    marketplace_status IS NULL
    OR marketplace_status IN ('pending_review', 'listed', 'rejected', 'delisted', 'unlisted')
);
```
Sources: [032_add_marketplace_status.sql](src/db/migrations/032_add_marketplace_status.sql#L1-L39), [034_cleanup_legacy_permissions.sql](src/db/migrations/034_cleanup_legacy_permissions.sql#L1-L30)

### 阶段六：细粒度优化与清理 (038-040)

迁移 038 引入 `is_current` 标志位，实现多版本 Skill 的"当前版本"语义——当新版本通过审核发布后，旧版本的 `is_current` 自动设为 false。同时为 `tenant_admin` 角色增加了跨组织的管理权限点。迁移 039 清理了不再使用的 `organization_identities` 表。迁移 040 移除了 `marketplace_admin` 角色的 `tenant:read` 权限，解决了管理后台侧边栏中"Organizations"分组标题对 marketplace_admin 用户可见的问题。

```sql
-- 038: is_current 标志位
ALTER TABLE skills ADD COLUMN IF NOT EXISTS is_current BOOLEAN NOT NULL DEFAULT true;

-- 数据迁移：将每个 skill name 的最新版本设为 current
WITH latest AS (
    SELECT DISTINCT ON (name) id, name
    FROM skills
    ORDER BY name, created_at DESC
)
UPDATE skills s SET is_current = (s.id = latest.id)
FROM latest WHERE s.name = latest.name;
```
Sources: [038_add_is_current_and_tenant_perms.sql](src/db/migrations/038_add_is_current_and_tenant_perms.sql#L1-L70), [040_remove_market_admin_tenant_read.sql](src/db/migrations/040_remove_market_admin_tenant_read.sql#L1-L12)

## 启动初始化流程

### AppState 初始化序列

当应用启动时，`AppState::new()` 方法按以下顺序完成初始化：

```mermaid
sequenceDiagram
    participant App as main.rs
    participant State as AppState::new()
    participant Pool as PgPool
    participant Migrations as run_migrations()
    participant Registry as RegistryService
    participant Search as SearchService

    App->>State: 传入 data_dir
    State->>Pool: 连接 DATABASE_URL
    State->>Migrations: 执行 40 个迁移
    Migrations->>Migrations: 检查 _migrations 表
    Migrations->>Migrations: 执行未应用迁移
    Migrations-->>State: 数据库就绪
    State->>Registry: 初始化 RegistryService
    State->>Search: 初始化 SearchService
    Search->>Search: doc_count == 0?
    Search->>Search: 全量重建 Tantivy 索引
    State-->>App: 返回 AppState
```

Sources: [lib.rs](src/lib.rs#L160-L200)

### 数据库连接配置

数据库连接通过 `DATABASE_URL` 环境变量配置，默认值为 `postgres://postgres:password@localhost:5432/aionhive`。应用使用 sqlx 的 `PgPool` 连接池进行连接管理。连接池的创建在 `AppState::new()` 和 `run_http_server()` 中均有发生——前者是全局状态初始化，后者是 HTTP 服务层为独立 Repository 实例创建额外连接。

```rust
// 标准连接建立
let pool = sqlx::PgPool::connect(
    &std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost:5432/aionhive".to_string()),
).await?;
```
Sources: [lib.rs](src/lib.rs#L166-L170), [.env.example](.env.example#L7-L8)

### 搜索索引重建

迁移完成后，系统会检查 Tantivy 全文搜索索引是否为空。如果 `doc_count()` 返回 0，则从数据库中加载所有 Skill 记录，调用 `search.rebuild_from_skills()` 重建索引。这一机制确保了新部署或数据目录重置后，搜索功能仍能正常运作。

```rust
if search.doc_count().unwrap_or(1) == 0 {
    let all_skills = skill_repo.list_all().await?;
    let models: Vec<crate::models::Skill> = all_skills.into_iter().map(|s| { /* 字段映射 */ }).collect();
    match search.rebuild_from_skills(&models) {
        Ok(n) => tracing::info!("Index rebuild complete: {} published skills indexed", n),
        Err(e) => tracing::error!("Index rebuild failed: {}", e),
    }
}
```
Sources: [lib.rs](src/lib.rs#L172-L200)

## Repository 模式与数据访问层

### 分层架构

数据库访问层采用 Repository 模式，分为三层：**Repository 实现**（src/db/repositories/ 下每个模块对应一个数据表）、**Traits 接口**（src/db/traits.rs 定义注入接口）、**Service 业务层**（通过 Repository 组合完成业务逻辑）。每个 Repository 都通过 `sqlx::PgPool` 进行操作，支持事务管理。

| 层 | 职责 | 示例 |
|----|------|------|
| Repository | 单表 CRUD、SQL 查询 | `SkillRepository::find_by_id()` |
| Trait | 依赖注入接口定义 | `SkillRepositoryTrait` |
| Service | 跨表业务编排 | `PermissionService` 组合 6 个 Repository |

Sources: [traits.rs](src/db/traits.rs#L1-L88), [repositories 目录](src/db/repositories/)

### 错误处理

数据库错误通过 `DbError` 枚举统一处理，包含五种变体：`ConnectionError`（连接失败）、`QueryError`（SQL 执行错误）、`NotFound`（记录不存在）、`AlreadyExists`（唯一约束冲突）、`ValidationError`（数据校验失败）。`DbError` 实现了到 `AppError` 的自动转换，使错误可以在服务层向上传播。

```rust
pub enum DbError {
    ConnectionError(String),
    QueryError(String),
    NotFound(String),
    AlreadyExists(String),
    ValidationError(String),
}
```
Sources: [error.rs](src/db/error.rs#L1-L24)

## 最佳实践与注意事项

### 迁移开发规范

1. **幂等性优先**：所有迁移 SQL 应使用 `IF NOT EXISTS`、`IF EXISTS`、`ON CONFLICT DO NOTHING` 等幂等语法，确保重复执行安全
2. **单文件单职责**：每个迁移文件聚焦一个功能变更，避免"大爆炸式"迁移
3. **命名规范**：文件前缀为三位数字编号（001-040），名称用下划线分隔的英文短语描述变更内容
4. **数据迁移需谨慎**：如迁移 038 的 `is_current` 数据填充，使用 `DISTINCT ON` 确保只取每个 skill name 的最新版本

### 常见问题

| 问题 | 原因 | 解决 |
|------|------|------|
| 迁移失败后重试 | 部分语句已执行 | 确保 SQL 使用幂等语法，或手动修复后重新执行 |
| `_migrations` 表损坏 | 表被误删或手动修改 | 根据实际数据库状态手动重建 `_migrations` 记录 |
| 字段已存在错误 | 迁移未使用 `IF NOT EXISTS` | 添加 `IF NOT EXISTS` 子句后重试 |
| 外键约束失败 | 数据依赖顺序问题 | 确认被引用的表/数据已先行就位 |

## 下一步阅读

- 深入理解 Repository 实现细节：[Repository 模式：PostgreSQL 数据访问与事务管理](27-repository-mo-shi-postgresql-shu-ju-fang-wen-yu-shi-wu-guan-li)
- 查看完整的迁移演进总结：[数据库迁移体系：从 001 到 040 的演进路线](28-shu-ju-ku-qian-yi-ti-xi-cong-001-dao-040-de-yan-jin-lu-xian)
- 了解环境变量配置流程：[环境变量与密钥配置](3-huan-jing-bian-liang-yu-mi-yao-pei-zhi)
- 整体架构概览：[整体架构：Rust 后端 + Svelte 管理后台 + CLI 工具链](5-zheng-ti-jia-gou-rust-hou-duan-svelte-guan-li-hou-tai-cli-gong-ju-lian)