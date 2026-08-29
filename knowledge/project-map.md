# 项目知识库

> 探索时间: 2024-08-29
> 版本: 0.3.0
> 状态: 持续更新

---

## 1. 项目概览

### 1.1 基本信息

| 属性 | 值 |
|------|------|
| 项目名 | AionHive / SkillGarden |
| 定位 | 企业级 AI Skills 共享平台 |
| 协议 | MCP (Model Context Protocol) |
| 技术栈 | Rust + Axum 0.7 + PostgreSQL + Tantivy |
| 代码路径 | `code/` (worktree) |
| 入口 | `code/src/main.rs` |

### 1.2 核心概念

```
Skill  = Prompt 工作流定义（不是工具）
Tool   = 可执行工具（本地/平台/沙箱）
Session = Agent 与平台的会话上下文
```

### 1.3 三类接口

| 接口类型 | 协议 | 使用者 |
|----------|------|--------|
| MCP Protocol | MCP (stdio/HTTP+SSE) | Agent (AI Agent) |
| REST API | HTTP/JSON | Admin (人) |
| Webhook | HTTP POST | Evaluator Agent |

---

## 2. 代码地图

### 2.1 核心目录结构

```
code/src/
├── main.rs              # 入口
├── lib.rs               # AppState 定义
├── mcp/
│   └── server.rs        # MCP Server 实现 (2000+ 行)
├── api/
│   ├── routes.rs        # API 路由配置
│   ├── handlers/        # 35+ 个 handler
│   │   ├── skills.rs    # Skills CRUD
│   │   ├── marketplace.rs
│   │   ├── sessions.rs
│   │   ├── orgs.rs
│   │   ├── groups.rs
│   │   └── ...
│   ├── jwt.rs           # JWT 认证
│   └── models.rs
├── services/            # 业务逻辑层
│   ├── registry.rs      # Skills 注册
│   ├── evaluator.rs     # 评价系统
│   ├── search.rs        # Tantivy 搜索
│   ├── permission.rs    # 权限系统
│   ├── sandbox.rs       # Docker 沙箱
│   ├── git_proxy.rs     # Git 代理
│   └── admin/          # 管理服务
├── models/             # 数据模型
│   ├── skill.rs
│   ├── session.rs
│   ├── organization.rs
│   ├── role.rs
│   └── ...
├── db/
│   ├── migrations.rs   # 数据库迁移
│   └── repositories/   # Repository 模式
│       ├── skill.rs
│       ├── session.rs
│       └── ...
└── schemas/           # 验证
```

### 2.2 关键文件速查

| 功能 | 文件路径 | 关键函数/结构 |
|------|----------|---------------|
| MCP Server | `src/mcp/server.rs` | `McpServer`, `call_tool_internal` |
| API 路由 | `src/api/routes.rs` | `create_api_router` |
| Skills 注册 | `src/services/registry.rs` | `create_skill`, `update_skill` |
| 权限检查 | `src/services/permission.rs` | `PermissionService`, `check_skill_permission` |
| 搜索 | `src/services/search.rs` | `SearchService`, `rebuild_from_skills` |
| 评价 | `src/services/evaluator.rs` | `EvaluatorService`, `add_evaluation` |
| AppState | `src/lib.rs` | `AppState::new` |
| Skill 模型 | `src/models/skill.rs` | `Skill`, `SkillMetadata` |

---

## 3. 功能清单

### 3.1 MCP Tools (Agent 调用)

| Tool | 功能 | 认证 |
|------|------|------|
| `health_check` | 健康检查 | ❌ 不需要 |
| `skills.search` | 搜索 Skills | ✅ API Key |
| `skills.list` | 列出 Skills | ✅ API Key |
| `skills.info` | 获取详情 | ✅ API Key |
| `skills.create` | 创建 Skill | ✅ API Key |
| `skills.update` | 更新 Skill | ✅ API Key |
| `skills.install` | 安装 Skill | ✅ API Key |
| `skills.versions` | 版本列表 | ✅ API Key |
| `skills.popular` | 人气排行 | ✅ API Key |
| `skills.stats` | 统计信息 | ✅ API Key |
| `evaluate_skill` | 提交评价 | ✅ API Key |
| `session.info` | 会话信息 | ✅ API Key |
| `session.declare` | 声明能力 | ✅ API Key |
| `tools.list` | 组织工具列表 | ✅ API Key |
| `tools.execute` | 执行组织工具 | ✅ API Key |
| `tools.platform.execute` | 执行平台工具 | ✅ API Key |
| `cli.setup` | CLI 下载安装 | ✅ API Key |

### 3.2 REST API 分类

#### Skills 管理 (~20 端点)
```
GET    /api/v1/skills
POST   /api/v1/skills
GET    /api/v1/skills/:id
PUT    /api/v1/skills/:id
DELETE /api/v1/skills/:id
POST   /api/v1/skills/:id/approve
POST   /api/v1/skills/:id/reject
POST   /api/v1/skills/:id/publish
POST   /api/v1/skills/:id/submit-to-marketplace
GET    /api/v1/skills/:id/stats
GET    /api/v1/skills/:id/files
POST   /api/v1/skills/:name/rollback
POST   /api/v1/skills/:name/sync
GET    /api/v1/skills/:name/versions
```

#### Marketplace
```
GET    /api/v1/marketplace
POST   /api/v1/admin/marketplace/:id/approve
POST   /api/v1/admin/marketplace/:id/reject
POST   /api/v1/admin/marketplace/:id/relist
POST   /api/v1/admin/marketplace/:id/delist
```

#### Organization 管理
```
GET    /api/v1/organizations
POST   /api/v1/organizations
GET    /api/v1/orgs/:slug
GET    /api/v1/orgs/:slug/skills
GET    /api/v1/orgs/:slug/members
POST   /api/v1/orgs/:slug/members/invite
```

#### Group 管理 (RBAC)
```
GET    /api/v1/orgs/:slug/groups
POST   /api/v1/orgs/:slug/groups
GET    /api/v1/orgs/:slug/groups/:id
PUT    /api/v1/orgs/:slug/groups/:id
DELETE /api/v1/orgs/:slug/groups/:id
GET    /api/v1/groups/:id/members
POST   /api/v1/groups/:id/members
GET    /api/v1/groups/:id/permissions
PUT    /api/v1/groups/:id/permissions
```

#### Admin 管理 (~25 端点)
```
GET    /api/v1/admin/stats
GET    /api/v1/admin/tenants
POST   /api/v1/admin/tenants
GET    /api/v1/admin/identities
POST   /api/v1/admin/identities
GET    /api/v1/admin/roles
POST   /api/v1/admin/roles
GET    /api/v1/admin/api-keys
POST   /api/v1/admin/api-keys
GET    /api/v1/admin/audit-logs
```

---

## 4. 数据模型

### 4.1 核心实体

```
Tenant (租户)
  └── Organization (组织)
        ├── Identity (身份/用户/Agent)
        │     ├── ApiKey
        │     └── SystemRoleAssignment
        ├── Group (分组)
        │     ├── GroupPermissionOverride
        │     └── GroupSkill
        ├── Skill (技能)
        │     ├── Evaluation
        │     └── SkillPolicy
        └── OrgTool (组织工具)
```

### 4.2 Skill 可见性

| 可见性 | 说明 |
|--------|------|
| `private` | 仅创建者可见 |
| `org_visible` | 组织内可见 |
| `shared` | 共享（组织间） |
| `marketplace` | 市场发布 |

### 4.3 Skill 状态

| 状态 | 说明 |
|------|------|
| `draft` | 草稿 |
| `pending_review` | 待审核 |
| `published` | 已发布 |
| `rejected` | 已拒绝 |

---

## 5. 权限系统

### 5.1 角色层级

```
System Role (系统级)
  └── super_admin

Tenant Role (租户级)
  └── tenant_admin

Organization Role (组织级)
  └── 自定义角色 + 权限矩阵
```

### 5.2 权限缓存

- **TTL**: 5 秒
- **缓存键**: identity_id
- **用途**: 减少高频权限查询的 DB 负载

---

## 6. 关键技术点

### 6.1 搜索

- **引擎**: Tantivy 0.22
- **分词**: jieba (中文支持)
- **索引时机**: 启动时自动重建空索引

### 6.2 评价权重

| 因素 | 权重说明 |
|------|----------|
| 一致性 | 相同结果的次数越多，权重越高 |
| 时间衰减 | 近期评价权重更高 |
| 评估数量 | 越多越接近真实值 |

### 6.3 MCP 认证

- **环境变量**: `AION_HIVE_JWT_TOKEN` / `AION_HIVE_AUTH_TOKEN`
- **API Key 前缀**: `sk_`
- **Session 自动创建**: API Key 认证后自动创建

---

## 7. 更新日志

| 日期 | 更新内容 |
|------|----------|
| 2024-08-29 | 初始版本，记录项目概览、代码地图、功能清单 |
