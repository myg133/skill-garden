# SkillGarden API 设计文档

> Version: 0.4.0
> 状态: 设计中

---

## 1. 架构概览

### 1.1 三类接口

SkillGarden 平台有三类独立的接口，分别服务于不同的角色：

| 接口类型 | 协议 | 使用者 | 用途 |
|----------|------|--------|------|
| **MCP Protocol** | MCP (stdio/HTTP+SSE) | Agent (AI Agent) | Skills 发现、获取、执行 |
| **REST API** | HTTP/JSON | Admin (人) | 平台管理、组织管理、审核 |
| **Webhook** | HTTP POST | Evaluator Agent | 评估回调 |

### 1.2 核心概念

```
Skill = Prompt 工作流定义（不是工具）
Tool = 可执行工具（本地/平台/沙箱）
Session = Agent 与平台的会话上下文
```

---

## 2. MCP Protocol 接口

### 2.1 设计原则

- MCP 接口服务于 Agent (AI Agent)
- Agent 通过 MCP 接口发现、学习、执行 Skills
- `skills.delete` 不在 MCP 接口中，删除是 Admin 职责

### 2.2 Skills 接口

#### skills.list

列出当前 Agent 有权限访问的 Skills。

```yaml
name: skills.list
description: 列出我有权限的 Skills（shortDescription 概述）
arguments:
  session_id: string  # Session ID，用于过滤 org 内的 Skills
returns:
  - id: string
    name: string
    shortDescription: string  # 简短描述，用于列表展示
    tags: string[]
    visibility: "private" | "org_visible" | "marketplace" | "shared"
    version: string
    author: string
```

#### skills.get

获取完整 Skill 内容。

```yaml
name: skills.get
description: 获取完整 Skill 内容（包含 SKILL.md）
arguments:
  session_id: string
  skill_id: string
returns:
  id: string
  name: string
  description: string
  content: string          # SKILL.md 完整内容
  tools: string[]          # Skill 引用的工具列表，如 ["browse", "qa"]
  git_url: string | null   # 可选，源码地址
  tags: string[]
  visibility: string
  version: string
  author: string
  stats:
    success_rate: number
    total_evaluations: number
    confidence: number
```

#### skills.search

搜索 Skills。

```yaml
name: skills.search
description: 搜索 Skills
arguments:
  query: string
  tags: string[] | null
  limit: number  # 默认 10
returns:
  - id: string
    name: string
    shortDescription: string
    score: number        # 相关性分数
    tags: string[]
```

#### skills.create

创建新 Skill（Agent 作为贡献者）。

```yaml
name: skills.create
description: 创建 Skill（Agent 作为贡献者）
arguments:
  name: string
  description: string
  tags: string[]
  content: string          # SKILL.md 内容
  visibility: string       # "private" | "org_visible" | "marketplace" | "shared"
returns:
  skill_id: string
  git_url: string         # 平台分配的 Git 仓库地址
```

#### skills.update

更新已有 Skill。

```yaml
name: skills.update
description: 更新 Skill
arguments:
  skill_id: string
  content: string | null
  description: string | null
  tags: string[] | null
returns:
  updated: true
```

### 2.3 Tools 接口

#### tools.execute

通过 Tool Router 执行工具。

```yaml
name: tools.execute
description: 执行工具（通过 Tool Router 路由到本地/平台/组织工具）
arguments:
  session_id: string
  cmd: string              # 工具名，如 "browse", "qa"
  args: object             # 工具参数
returns:
  status: "local" | "platform" | "org_tool"
  result: object           # 工具执行结果
  error: string | null     # 错误信息（如果有）
```

#### 执行流程

```
Agent 调用 tools.execute(cmd="browse", args={url: "..."})
  → Tool Router 查找 session 的路由配置
  → 根据 cmd 匹配路由目标：
    - local: 返回给 Agent 本地执行
    - platform: 路由到平台 Sandbox 执行
    - org_tool: 路由到组织的自定义工具执行
```

### 2.3.1 Sandbox 执行环境

当 Tool Router 将请求路由到 `platform` 时，平台使用 Sandbox 执行工具。

#### Sandbox 模型

```yaml
Sandbox:
  id: string                    # Sandbox 实例 ID
  session_id: string            # 关联的 Session
  container_id: string          # Docker 容器 ID
  image: string                 # 工具执行镜像
  status: "starting" | "ready" | "busy" | "stopped"
  created_at: timestamp
  last_used: timestamp
```

#### Sandbox 执行流程

```
tools.execute(cmd="browse", args={url: "..."})
  │
  ├─→ Tool Router 返回 status: "platform"
  │
  └─→ Platform 查找/创建 Sandbox for session
          │
          ├─→ Sandbox 存在且 ready → 直接执行
          │
          ├─→ Sandbox 存在但 busy → 等待或创建新 Sandbox
          │
          └─→ Sandbox 不存在 → 创建新 Sandbox
          
  → 在 Sandbox 中执行工具
  → 返回执行结果
```

#### Sandbox 生命周期

| 阶段 | 触发条件 | 操作 |
|------|----------|------|
| 创建 | 首次路由到 platform 的工具 | 启动容器 |
| 就绪 | 容器启动完成 | 标记 ready |
| 执行 | tools.execute 路由到 platform | 在容器中执行 |
| 复用 | 后续请求 | 复用已有容器 |
| 清理 | Session 结束 / 超时 | 停止并删除容器 |

#### Sandbox 配置

```yaml
# 工具执行镜像
sandbox:
  image: "aion-hive/tool-executor:latest"
  
  # 容器资源配置
  resources:
    cpu: "1"
    memory: "512Mi"
    
  # Session 容器复用策略
  reuse:
    enabled: true
    max_idle_time: 300  # 5分钟无活动后停止
    
  # 容器超时
  timeout:
    max_execution: 300  # 单次执行最多 5 分钟
    max_lifetime: 3600  # 容器最多存活 1 小时
```

#### 工具注册（平台工具）

平台预置的工具需要在 Sandbox 镜像中实现：

```yaml
# 平台工具注册表
platform_tools:
  browse:
    image: "aion-hive/tool-browse:latest"
    sandbox_required: true
    timeout: 30
    
  qa:
    image: "aion-hive/tool-qa:latest"
    sandbox_required: true
    timeout: 60
    
  exec:
    image: "aion-hive/tool-exec:latest"
    sandbox_required: true
    timeout: 120
```

### 2.4 Session 接口

#### session.info

获取当前 Session 信息。

```yaml
name: session.info
description: 获取当前 Session 信息
arguments:
  session_id: string
returns:
  session_id: string
  org_id: string
  agent_id: string
  capabilities: string[]     # Agent 声明的能力
  tool_router:
    tool_name: "local" | "platform" | "org_tool"
```

#### session.declare

Agent 声明自己的能力，影响 Tool Router 配置。

```yaml
name: session.declare
description: Agent 声明自己的能力
arguments:
  session_id: string
  capabilities: string[]    # Agent 拥有的工具列表，如 ["browse", "qa"]
returns:
  tool_router:              # 更新后的路由配置
    browse: "local"
    qa: "platform"
    exec: "platform"
```

**注意**: Session 创建由程序控制（认证后自动创建），不是 Agent 调用。

### 2.5 Evaluations 接口

#### evaluations.submit

提交 Skill 使用评价。

```yaml
name: evaluations.submit
description: 提交 Skill 使用评价
arguments:
  skill_id: string
  success: boolean
  duration_ms: number
  error_type: "timeout" | "crash" | "logic_error" | null
  tags: string[]           # "reliable" | "fast" | "stable" | "experimental"
returns:
  evaluation_id: string
  new_stats:
    success_rate: number
    avg_duration_ms: number
    total_evaluations: number
    confidence: number
```

---

## 3. REST API 接口

### 3.1 设计原则

- REST API 服务于 Admin (人)
- 所有管理操作都在 REST API 中
- 需要 Bearer Token 认证

### 3.2 Admin 认证

#### POST /api/admin/login

管理员登录。

```http
POST /api/admin/login
Content-Type: application/json

{
  "username": "admin",
  "password": "..."
}
```

```json
{
  "token": "eyJhbGc...",
  "token_type": "Bearer",
  "expires_in": 86400,
  "user": {
    "id": "uuid",
    "username": "admin",
    "display_name": "Administrator"
  }
}
```

#### GET /api/admin/me

获取当前管理员信息。

```http
GET /api/admin/me
Authorization: Bearer <token>
```

### 3.3 Skills 管理

#### DELETE /api/skills/:id

删除 Skill（Admin only）。

```http
DELETE /api/skills/:id
Authorization: Bearer <token>
```

```json
{
  "deleted": "skill-id"
}
```

#### POST /api/skills/:id/approve

审核通过，发布 Skill。

```http
POST /api/skills/:id/approve
Authorization: Bearer <token>
```

```json
{
  "approved": "skill-id",
  "status": "published"
}
```

#### POST /api/skills/:id/reject

审核拒绝。

```http
POST /api/skills/:id/reject
Authorization: Bearer <token>

{
  "reason": "Skill contains inappropriate content"
}
```

```json
{
  "rejected": "skill-id",
  "reason": "Skill contains inappropriate content"
}
```

### 3.4 Organizations 管理

#### GET /api/admin/organizations

列出所有组织。

```http
GET /api/admin/organizations
Authorization: Bearer <token>
```

```json
{
  "organizations": [
    {
      "id": "uuid",
      "name": "Acme Corp",
      "members_count": 15,
      "skills_count": 42,
      "created_at": "2024-01-15T10:30:00Z"
    }
  ],
  "total": 10
}
```

#### POST /api/admin/organizations

创建组织。

```http
POST /api/admin/organizations
Authorization: Bearer <token>

{
  "name": "New Organization",
  "settings": {}
}
```

#### GET /api/admin/organizations/:id

获取组织详情。

#### PUT /api/admin/organizations/:id

更新组织。

#### DELETE /api/admin/organizations/:id

删除组织。

### 3.5 Organization 成员管理

#### GET /api/admin/orgs/:org_id/members

列出组织成员（Agents）。

```http
GET /api/admin/orgs/:org_id/members
Authorization: Bearer <token>
```

```json
{
  "members": [
    {
      "agent_id": "agent-xxx",
      "name": "Browse Agent",
      "capabilities": ["browse", "qa"],
      "joined_at": "2024-01-15T10:30:00Z"
    }
  ]
}
```

#### POST /api/admin/orgs/:org_id/members

添加成员到组织。

```http
POST /api/admin/orgs/:org_id/members
Authorization: Bearer <token>

{
  "agent_id": "agent-yyy",
  "name": "New Agent"
}
```

#### DELETE /api/admin/orgs/:org_id/members/:agent_id

从组织移除成员。

### 3.6 审计日志

#### GET /api/admin/audit-logs

获取审计日志。

```http
GET /api/admin/audit-logs
Authorization: Bearer <token>
?limit=50&offset=0&action=skill.create
```

```json
{
  "logs": [
    {
      "id": "uuid",
      "actor_id": "agent-xxx",
      "actor_type": "agent",
      "action": "skill.create",
      "resource_type": "skill",
      "resource_id": "skill-browse-1.0.0",
      "details": {},
      "timestamp": "2024-01-15T10:30:00Z"
    }
  ],
  "total": 150
}
```

### 3.7 平台统计

#### GET /api/admin/stats

获取平台统计。

```http
GET /api/admin/stats
Authorization: Bearer <token>
```

```json
{
  "total_skills": 150,
  "total_agents": 45,
  "total_organizations": 10,
  "total_evaluations": 1200,
  "average_success_rate": 0.87
}
```

#### GET /api/admin/orgs/:org_id/stats

获取组织统计。

---

## 4. Webhook 接口 (Evaluator Agent)

### 4.1 设计原则

- Evaluator Agent 注册一个 HTTP 端点
- SkillGarden 在评估触发时回调该端点
- 支持轻量评估和深度评估两种触发

### 4.2 Evaluator 注册

#### POST /api/evaluators/register

注册 Evaluator Agent。

```http
POST /api/evaluators/register
Content-Type: application/json

{
  "name": "Quality Evaluator",
  "webhook_url": "https://evaluator.example.com/hook",
  "capabilities": ["lightweight", "depth"],
  "org_id": "uuid"  # 可选，限制为某组织
}
```

```json
{
  "evaluator_id": "uuid",
  "api_key": "eva_xxxxxxxxxxxx"
}
```

#### DELETE /api/evaluators/:id

注销 Evaluator。

#### GET /api/evaluators

列出已注册的 Evaluators。

### 4.3 评估触发回调

#### 轻量评估回调

由 Agent 每次使用 Skill 后触发。

```http
POST <evaluator_webhook_url>
Headers:
  X-Evaluator-Key: <api_key>
  X-Trigger-Type: lightweight
  Content-Type: application/json

{
  "event_id": "uuid",
  "skill_id": "skill-browse-1.0.0",
  "agent_id": "agent-xxx",
  "session_id": "session-yyy",
  "timestamp": "2024-01-15T10:30:00Z",
  "data": {
    "success": true,
    "duration_ms": 1150,
    "error_type": null,
    "tags": ["reliable", "fast"]
  }
}
```

#### 深度评估回调

由 Platform 在特定条件下触发。

```http
POST <evaluator_webhook_url>
Headers:
  X-Evaluator-Key: <api_key>
  X-Trigger-Type: depth
  Content-Type: application/json

{
  "event_id": "uuid",
  "skill_id": "skill-browse-1.0.0",
  "timestamp": "2024-01-15T10:30:00Z",
  "data": {
    "skill_content": "<full SKILL.md content>",
    "recent_evaluations": [
      { "success": true, "duration_ms": 1150 },
      { "success": false, "duration_ms": 5000, "error_type": "timeout" }
    ],
    "usage_stats": {
      "total_uses": 150,
      "success_rate": 0.87,
      "avg_duration_ms": 1200,
      "unique_agents": 12
    },
    "context": {
      "org_id": "org-xxx",
      "requesting_agent_id": "agent-yyy"
    }
  }
}
```

#### Evaluator 回调响应

```json
{
  "status": "received",
  "evaluation_id": "uuid",
  "quality_score": 0.85,
  "suggestions": ["consider adding timeout handling"],
  "new_version": "1.1.0"
}
```

### 4.4 触发条件

深度评估的触发条件（由 Platform 配置）：

| 条件 | 描述 |
|------|------|
| first_use | Agent 首次使用某 Skill |
| cumulative_count | 累计使用 N 次后 |
| high_error_rate | 错误率超过阈值 |
| staleness | 30 天无更新 |

---

## 5. 数据模型

### 5.1 Skill

```json
{
  "id": "skill-browse-1.0.0",
  "name": "browse",
  "description": "网页浏览和爬取",
  "content": "# SKILL.md\n...",
  "tools": ["browse"],
  "tags": ["web", "scraping"],
  "visibility": "marketplace",
  "version": "1.0.0",
  "author": "agent-xxx",
  "git_url": "https://git.example.com/skills/browse",
  "status": "published",
  "stats": {
    "success_rate": 0.92,
    "total_evaluations": 150,
    "confidence": 0.85
  },
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-20T15:00:00Z"
}
```

### 5.2 Organization

```json
{
  "id": "uuid",
  "name": "Acme Corp",
  "settings": {},
  "created_at": "2024-01-15T10:30:00Z"
}
```

### 5.3 Agent

```json
{
  "id": "uuid",
  "agent_id": "agent-xxx",
  "name": "Browse Agent",
  "org_id": "uuid",
  "capabilities": ["browse", "qa"],
  "created_at": "2024-01-15T10:30:00Z"
}
```

### 5.4 Session

```json
{
  "id": "uuid",
  "agent_id": "agent-xxx",
  "org_id": "uuid",
  "status": "active",
  "tool_router": {
    "browse": "local",
    "qa": "platform",
    "exec": "platform"
  },
  "created_at": "2024-01-15T10:30:00Z",
  "ended_at": null
}
```

### 5.5 Evaluation

```json
{
  "id": "uuid",
  "skill_id": "skill-browse-1.0.0",
  "agent_id": "agent-xxx",
  "success": true,
  "duration_ms": 1150,
  "error_type": null,
  "tags": ["reliable", "fast"],
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### 5.6 Sandbox

```json
{
  "id": "sandbox-xxx",
  "session_id": "session-yyy",
  "org_id": "org-zzz",
  "container_id": "container-abc",
  "image": "aion-hive/tool-executor:latest",
  "status": "ready",
  "tools": ["browse", "qa"],
  "created_at": "2024-01-15T10:30:00Z",
  "last_used": "2024-01-15T11:00:00Z"
}
```

### 5.7 OrgTool

组织注册的自定义工具。

```json
{
  "id": "uuid",
  "org_id": "org-zzz",
  "tool_id": "company-api-tool",
  "name": "Company API Tool",
  "description": "访问公司内部 API",
  "schema": {
    "type": "object",
    "properties": {
      "endpoint": { "type": "string" },
      "params": { "type": "object" }
    }
  },
  "implementation": {
    "type": "docker",
    "image": "company/tool-api:latest"
  },
  "status": "approved",
  "created_at": "2024-01-15T10:30:00Z"
}
```

---

## 6. 角色权限矩阵

| 操作 | Platform Admin | Org Admin | Agent | Evaluator |
|------|--------------|-----------|-------|------------|
| **Skills** |
| skills.list | ✅ | ✅ (本组织) | ✅ | - |
| skills.get | ✅ | ✅ | ✅ | - |
| skills.search | ✅ | ✅ | ✅ | - |
| skills.create | ✅ | ✅ | ✅ | - |
| skills.update | ✅ | ✅ | ✅ (作者) | - |
| skills.delete | ✅ | - | - | - |
| **Organizations** |
| 创建组织 | ✅ | - | - | - |
| 删除组织 | ✅ | - | - | - |
| 成员管理 | ✅ | ✅ (本组织) | - | - |
| **Tools** |
| tools.execute | - | - | ✅ | - |
| **OrgTools** |
| org_tool.register | - | ✅ (本组织) | - | - |
| org_tool.approve | ✅ | - | - | - |
| org_tool.reject | ✅ | - | - | - |
| **Sandbox** |
| Sandbox 管理 | ✅ | - | - | - |
| **Evaluations** |
| 接收回调 | - | - | - | ✅ |

---

## 7. 附录

### 7.1 MCP 传输模式

- **Stdio 模式**: 用于本地 Agent 连接
- **HTTP+SSE 模式**: 用于远程 Agent 连接

### 7.2 错误码

| 错误码 | 含义 |
|--------|------|
| 400 | 请求参数错误 |
| 401 | 未认证 |
| 403 | 无权限 |
| 404 | 资源不存在 |
| 429 | 请求过于频繁 |
| 500 | 服务器内部错误 |

### 7.3 环境变量

| 变量 | 默认值 | 描述 |
|------|--------|------|
| AION_HIVE_TRANSPORT | stdio | 传输模式 |
| AION_HIVE_HTTP_PORT | 8080 | HTTP 端口 |
| AION_HIVE_DATA_DIR | data | 数据目录 |
| AION_HIVE_EVAL_WEBHOOK_URLS | - | Evaluator Webhook URL（逗号分隔） |

### 7.4 Sandbox 配置

| 变量 | 默认值 | 描述 |
|------|--------|------|
| AION_HIVE_SANDBOX_IMAGE | aion-hive/tool-executor:latest | 工具执行镜像 |
| AION_HIVE_SANDBOX_REUSE_ENABLED | true | 是否复用 Sandbox 容器 |
| AION_HIVE_SANDBOX_MAX_IDLE_SECONDS | 300 | 容器最大空闲时间（秒） |
| AION_HIVE_SANDBOX_MAX_EXECUTION_SECONDS | 300 | 单次执行最大时间（秒） |
| AION_HIVE_SANDBOX_MAX_LIFETIME_SECONDS | 3600 | 容器最大生命周期（秒） |

---

## 8. Agent 认证协议

### 8.1 认证流程

```
┌─────────┐                           ┌──────────────┐                           ┌─────────┐
│  Agent  │                           │   SkillGarden │                           │   DB    │
└────┬────┘                           └──────┬───────┘                           └────┬────┘
     │                                      │                                       │
     │  1. POST /auth/agent (agent_id + secret)                                      │
     │─────────────────────────────────────►│                                       │
     │                                      │  2. Validate credentials               │
     │                                      │──────────────────────────────────────►│
     │                                      │                                       │
     │                                      │  3. Return agent info + org_id        │
     │                                      │◄──────────────────────────────────────│
     │                                      │                                       │
     │  4. Generate JWT (agent_id, org_id, exp)                                      │
     │◄─────────────────────────────────────│                                       │
     │                                      │                                       │
     │  5. MCP connect with JWT             │                                       │
     │─────────────────────────────────────►│  6. Verify JWT, create Session        │
     │                                      │──────────────────────────────────────►│
     │                                      │                                       │
     │  7. Session info (session_id, tools) │                                       │
     │◄─────────────────────────────────────│                                       │
```

### 8.2 认证请求

**Endpoint**: `POST /api/auth/agent`

**Request Body**:
```json
{
  "agent_id": "agent-001",
  "agent_secret": "sk_live_xxxxxxxxxxxxxxxxxxxx",
  "agent_name": "browse-agent-prod",
  "capabilities": ["browse", "qa", "code-review"]
}
```

**Response**:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 86400,
  "agent_id": "agent-001",
  "org_id": "org-123"
}
```

### 8.3 JWT Token 结构

**Header**:
```json
{
  "alg": "HS256",
  "typ": "JWT"
}
```

**Payload**:
```json
{
  "sub": "agent-001",
  "org_id": "org-123",
  "capabilities": ["browse", "qa"],
  "iat": 1704067200,
  "exp": 1704153600,
  "jti": "token-uuid"
}
```

### 8.4 MCP 连接认证

**Stdio 模式**:
```bash
# Agent 使用 JWT 作为 Authorization header
echo '{"jsonrpc":"2.0","method":"initialize",...}' | \
  AUTH_TOKEN="eyJhbGciOiJIUzI1NiIs..." cargo run
```

**HTTP 模式**:
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize",...}'
```

### 8.5 Token 刷新

**Endpoint**: `POST /api/auth/agent/refresh`

**Request**:
```json
{
  "refresh_token": "rt_xxxxxxxxxxxxx"
}
```

**Response**:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_in": 86400
}
```

---

## 9. Session 自动创建机制

### 9.1 Session 创建流程

```
Agent MCP Initialize
        │
        ▼
┌───────────────────┐
│ 验证 JWT Token     │
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ 提取 agent_id      │
│ 提取 org_id       │
│ 提取 capabilities │
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ 构造 Session       │
│ session_id=uuid   │
│ status=active     │
│ started_at=now()  │
└────────┬──────────┘
         │
         ▼
┌───────────────────┐     ┌───────────────┐
│ 存储 Session      │────►│  PostgreSQL   │
└────────┬──────────┘     └───────────────┘
         │
         ▼
┌───────────────────┐
│ 返回 session.info │
│ 给 Agent         │
└───────────────────┘
```

### 9.2 Session 数据模型

```rust
struct Session {
    session_id: Uuid,           // 唯一标识
    agent_id: String,           // Agent ID (来自 JWT)
    org_id: String,            // 组织 ID (来自 JWT)
    capabilities: Vec<String>,  // Agent 声明的能力
    status: SessionStatus,     // active | idle | terminated
    created_at: DateTime<Utc>,  // 创建时间
    last_active_at: DateTime<Utc>, // 最后活跃时间
    terminated_at: Option<DateTime<Utc>>, // 终止时间
    metadata: JsonValue,        // 扩展元数据
}

enum SessionStatus {
    Active,    // 正在处理请求
    Idle,      // 空闲（无请求超过阈值）
    Terminated // 已终止
}
```

### 9.3 Session 生命周期

| 状态 | 进入条件 | 退出条件 | Agent 可感知 |
|------|----------|----------|--------------|
| Active | 收到 MCP 请求 | 无请求 60s | 是 |
| Idle | Active 超时 60s | 收到新请求 | 是 |
| Terminated | Idle 超时 5min | - | 否 |

**Idle 触发**:
```rust
// Agent 无请求 60 秒后进入 Idle
async fn check_idle_sessions() {
    let idle_threshold = Utc::now() - Duration::seconds(60);
    // 更新 status = "idle"
}

// Idle 超过 5 分钟，终止 Session
async fn terminate_idle_sessions() {
    let terminate_threshold = Utc::now() - Duration::minutes(5);
    // 更新 status = "terminated", terminated_at = now()
}
```

### 9.4 Session 续期

当 Agent 收到 `session.info` 响应后，最后活跃时间更新：

```json
// MCP 响应
{
  "session_id": "sess_abc123",
  "status": "active",
  "tools": [...],
  "server_info": {...}
}
```

### 9.5 Session 主动终止

**Endpoint**: `DELETE /api/sessions/:session_id` (Admin only)

**Request**:
```http
DELETE /api/sessions/sess_abc123 HTTP/1.1
Authorization: Bearer <admin_token>
```

**Response**:
```json
{
  "success": true,
  "session_id": "sess_abc123",
  "terminated_at": "2025-05-15T12:00:00Z"
}
```

---

## 10. Tool Router 路由算法

### 10.1 路由决策流程

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tool Call Request                          │
│  { "name": "browser_navigate", "arguments": { "url": "..." } }  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Step 1: 权限检查                           │
│  检查 Agent org_id 是否有权限调用此 tool                         │
└─────────────────────────────────────────────────────────────────┘
                                │
                    ┌───────────┴───────────┐
                    │                       │
                  允许                    拒绝
                    │                       │
                    ▼                       ▼
┌─────────────────────────────────┐   ┌─────────────────────────┐
│    Step 2: Capability 匹配       │   │   返回 Error             │
│  Agent.capabilities 包含        │   │   { "code": 403 }       │
│  tool.required_capabilities?    │   └─────────────────────────┘
└─────────────────────────────────┘
                    │
          ┌─────────┴─────────┐
          │                   │
        是                   否
          │                   │
          ▼                   ▼
┌─────────────────┐   ┌─────────────────────────────────────────┐
│ Step 3A:       │   │         Step 3B: Sandbox 执行            │
│ 本地执行        │   │  检查 tool.type = "platform"            │
│ tool.type =    │   │  或 tool.type = "org_tool"             │
│ "builtin"      │   │  或 tool.type = "remote"               │
└─────────────────┘   └─────────────────────────────────────────┘
                                │
                    ┌───────────┴───────────┐
                    │                       │
                  本地                   远程
                  (Tool Executor)        (Sandbox)
                    │                       │
                    ▼                       ▼
              ┌───────────┐           ┌───────────────┐
              │ 执行成功   │           │ 调用 Sandbox  │
              │ 返回结果   │           │ 返回结果      │
              └───────────┘           └───────────────┘
```

### 10.2 路由规则表

| Tool Type | 执行位置 | 认证方式 | 延迟 | 示例 |
|-----------|----------|----------|------|------|
| `builtin` | 本地 (Rust) | 无 | <1ms | `echo`, `env` |
| `platform` | Sandbox | JWT + 工具白名单 | 10-50ms | `browser_navigate` |
| `org_tool` | Sandbox | JWT + 组织授权 | 10-50ms | `custom_api_call` |
| `remote` | 远程服务 | JWT + 远程验证 | 50-200ms | `external_service` |

### 10.3 Capability 匹配算法

```rust
fn matches_capabilities(
    agent_caps: &[String],
    required_caps: &[String]
) -> bool {
    // 全部 required 都在 agent_caps 中
    required_caps.iter().all(|r| agent_caps.contains(r))
}

// 示例
let agent = Agent { capabilities: vec!["browse".into(), "qa".into()] };
let tool = Tool { required_capabilities: vec!["browse".into()] };

assert!(matches_capabilities(&agent.capabilities, &tool.required_capabilities));
```

### 10.4 路由缓存

```rust
struct ToolRouteCache {
    cache: RwLock<HashMap<String, CachedRoute>>, // key: tool_name
}

struct CachedRoute {
    route: RouteResult,
    cached_at: DateTime<Utc>,
    ttl_seconds: u64,
}

// 缓存有效期 5 分钟
const ROUTE_CACHE_TTL: u64 = 300;
```

### 10.5 路由优先级

```
1. Builtin Tools (最高优先级)
   └─ 始终在本地执行

2. Platform Tools
   └─ 检查 AION_HIVE_PLATFORM_TOOLS 白名单

3. Org Tools
   └─ 检查 Organization.org_tools 授权

4. Remote Tools
   └─ 检查 tool.remote_url 可达性
```

### 10.6 路由失败处理

| 错误类型 | 处理策略 | 重试 |
|----------|----------|------|
| `tool_not_found` | 返回 404 | 否 |
| `capability_mismatch` | 返回 403 + 缺少的 capability | 否 |
| `org_not_authorized` | 返回 403 | 否 |
| `sandbox_timeout` | 返回 504 | 是 (3 次) |
| `sandbox_error` | 返回 500 | 否 |

---

## 11. 数据库 Schema

### 11.1 ER 图

```
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│   Organization  │       │     Agent       │       │      Skill      │
├─────────────────┤       ├─────────────────┤       ├─────────────────┤
│ id (PK)         │◄──────│ org_id (FK)     │       │ id (PK)         │
│ name            │       │ agent_id (PK)   │       │ name            │
│ created_at      │       │ agent_secret    │       │ version         │
│ updated_at      │       │ name            │       │ description     │
│ settings        │       │ capabilities    │       │ prompt          │
└────────┬────────┘       │ created_at      │       │ source_url      │
         │                └────────┬────────┘       │ author_agent_id │
         │                         │                │ status          │
         │                ┌────────┴────────┐        │ approved_at     │
         │                │                 │        │ approved_by     │
         │                ▼                 ▼        │ created_at      │
┌─────────────────┐  ┌───────────┐  ┌───────────┐   │ updated_at      │
│   OrgTool       │  │  Session  │  │ Evaluation│   └────────┬────────┘
├─────────────────┤  ├─────────────────────────────┤            │
│ id (PK)         │  │ session_id (PK)│            │            │
│ org_id (FK)     │  │ agent_id (FK)  │            │            │
│ tool_name       │  │ status         │            │            │
│ tool_config     │  │ capabilities   │            │            │
│ created_at      │  │ created_at     │            │            │
│ created_by      │  │ last_active_at │            │            │
└─────────────────┘  │ terminated_at  │            │            │
                     └────────┬────────┘            │            │
                              │                    │            │
                              └──────────┬──────────┘            │
                                         │                       │
                                         ▼                       │
                              ┌───────────────────┐              │
                              │   SkillVersion    │◄─────────────┘
                              ├───────────────────┤
                              │ id (PK)           │
                              │ skill_id (FK)     │
                              │ version           │
                              │ prompt            │
                              │ changelog         │
                              │ created_at        │
                              └───────────────────┘
```

### 11.2 DDL

```sql
-- Organizations 表
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    settings JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_organizations_name ON organizations(name);

-- Agents 表
CREATE TABLE agents (
    agent_id VARCHAR(255) PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    agent_secret_hash VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    capabilities TEXT[] DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_agents_org_id ON agents(org_id);
CREATE INDEX idx_agents_capabilities ON agents USING GIN(capabilities);

-- Skills 表
CREATE TABLE skills (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    version VARCHAR(50) NOT NULL,
    description TEXT,
    prompt TEXT NOT NULL,
    source_url VARCHAR(1024),
    author_agent_id VARCHAR(255),
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    approved_at TIMESTAMPTZ,
    approved_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(name, version)
);

CREATE INDEX idx_skills_name ON skills(name);
CREATE INDEX idx_skills_status ON skills(status);
CREATE INDEX idx_skills_author ON skills(author_agent_id);

-- Skill Versions 表
CREATE TABLE skill_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    prompt TEXT NOT NULL,
    changelog TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(skill_id, version)
);

CREATE INDEX idx_skill_versions_skill_id ON skill_versions(skill_id);

-- Sessions 表
CREATE TABLE sessions (
    session_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id VARCHAR(255) NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    capabilities TEXT[] DEFAULT '{}',
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_active_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    terminated_at TIMESTAMPTZ
);

CREATE INDEX idx_sessions_agent_id ON sessions(agent_id);
CREATE INDEX idx_sessions_org_id ON sessions(org_id);
CREATE INDEX idx_sessions_status ON sessions(status);
CREATE INDEX idx_sessions_last_active ON sessions(last_active_at);

-- Evaluations 表
CREATE TABLE evaluations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    session_id UUID REFERENCES sessions(session_id) ON DELETE SET NULL,
    agent_id VARCHAR(255) NOT NULL,
    success BOOLEAN NOT NULL,
    duration_ms INTEGER,
    error_type VARCHAR(100),
    tags TEXT[],
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_evaluations_skill_id ON evaluations(skill_id);
CREATE INDEX idx_evaluations_agent_id ON evaluations(agent_id);
CREATE INDEX idx_evaluations_created_at ON evaluations(created_at);

-- Organization Tools 表
CREATE TABLE org_tools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    tool_name VARCHAR(255) NOT NULL,
    tool_config JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    UNIQUE(org_id, tool_name)
);

CREATE INDEX idx_org_tools_org_id ON org_tools(org_id);

-- Audit Logs 表
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    actor_id VARCHAR(255) NOT NULL,
    actor_type VARCHAR(50) NOT NULL,
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id VARCHAR(255),
    details JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_logs_org_id ON audit_logs(org_id);
CREATE INDEX idx_audit_logs_actor_id ON audit_logs(actor_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);

-- Organization Members 表
CREATE TABLE org_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL DEFAULT 'member',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(org_id, user_id)
);

CREATE INDEX idx_org_members_org_id ON org_members(org_id);
CREATE INDEX idx_org_members_user_id ON org_members(user_id);
```

### 11.3 索引策略

| 表 | 索引类型 | 用途 |
|----|----------|------|
| skills | GIN (capabilities) | 技能搜索 |
| agents | GIN (capabilities) | Agent 能力查询 |
| sessions | BTree (last_active_at) | 空闲 Session 清理 |
| evaluations | BTree (created_at) | 评价趋势分析 |
| audit_logs | BTree (created_at) | 审计查询 |

### 11.4 软删除策略

```sql
-- 使用 deleted_at 实现软删除
ALTER TABLE skills ADD COLUMN deleted_at TIMESTAMPTZ;

-- 查询未删除的 Skills
SELECT * FROM skills WHERE deleted_at IS NULL;

-- 批量清理（保留 90 天）
DELETE FROM skills WHERE deleted_at < NOW() - INTERVAL '90 days';
```

---

## 12. API 规范补充

### 12.1 统一错误响应格式

所有 API（REST + MCP）使用统一的错误响应格式：

```json
{
  "error": {
    "code": "SKILL_NOT_FOUND",
    "message": "Skill with id 'xxx' not found",
    "details": {
      "skill_id": "xxx",
      "suggestion": "Use skills.search to find available skills"
    },
    "request_id": "req_abc123"
  }
}
```

**错误码体系**：

| 错误码 | HTTP 状态码 | 说明 |
|--------|-------------|------|
| `INVALID_REQUEST` | 400 | 请求参数错误 |
| `UNAUTHORIZED` | 401 | 未认证或 Token 过期 |
| `FORBIDDEN` | 403 | 无权限访问 |
| `SKILL_NOT_FOUND` | 404 | Skill 不存在 |
| `AGENT_NOT_FOUND` | 404 | Agent 不存在 |
| `SESSION_NOT_FOUND` | 404 | Session 不存在 |
| `SKILL_ALREADY_EXISTS` | 409 | Skill 已存在（创建时冲突） |
| `CAPABILITY_MISMATCH` | 403 | Agent 缺少必要 Capability |
| `ORG_NOT_AUTHORIZED` | 403 | 组织未被授权使用此 Tool |
| `RATE_LIMITED` | 429 | 请求过于频繁 |
| `SANDBOX_TIMEOUT` | 504 | Sandbox 执行超时 |
| `INTERNAL_ERROR` | 500 | 服务器内部错误 |

**MCP 错误响应**：

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32603,
    "message": "Internal error",
    "data": {
      "code": "SANDBOX_TIMEOUT",
      "request_id": "req_abc123"
    }
  }
}
```

### 12.2 分页格式

所有 List 端点支持标准分页参数：

**请求参数**：

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `page` | integer | 1 | 页码（从 1 开始） |
| `page_size` | integer | 20 | 每页数量（最大 100） |
| `cursor` | string | null | 游标分页（优先于 page） |
| `sort_by` | string | created_at | 排序字段 |
| `sort_order` | string | desc | 排序方向：`asc` 或 `desc` |

**响应格式**：

```json
{
  "data": [
    { "id": "1", "name": "Skill A" },
    { "id": "2", "name": "Skill B" }
  ],
  "pagination": {
    "page": 1,
    "page_size": 20,
    "total_items": 156,
    "total_pages": 8,
    "has_next": true,
    "has_prev": false,
    "next_cursor": "eyJpZCI6IjIwIn0=",
    "prev_cursor": null
  }
}
```

**游标分页（推荐）**：

```json
{
  "data": [...],
  "pagination": {
    "cursor": {
      "next": "eyJpZCI6IjIwIn0=",
      "prev": "eyJpZCI6IjEifQ=="
    },
    "has_more": true
  }
}
```

### 12.3 REST API 完整路径

#### 认证相关

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/auth/admin/login` | Admin 登录 |
| POST | `/api/v1/auth/admin/refresh` | 刷新 Admin Token |
| POST | `/api/v1/auth/agent` | Agent 认证 |
| POST | `/api/v1/auth/agent/refresh` | 刷新 Agent Token |

#### Skills 管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/skills` | 列出 Skills（分页） |
| GET | `/api/v1/skills/:id` | 获取 Skill 详情 |
| DELETE | `/api/v1/skills/:id` | 删除 Skill |
| POST | `/api/v1/skills/:id/approve` | 审核通过 |
| POST | `/api/v1/skills/:id/reject` | 审核拒绝 |
| POST | `/api/v1/skills/:id/versions` | 创建新版本 |

#### Organizations 管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/organizations` | 列出组织 |
| POST | `/api/v1/organizations` | 创建组织 |
| GET | `/api/v1/organizations/:id` | 获取组织详情 |
| PUT | `/api/v1/organizations/:id` | 更新组织 |
| DELETE | `/api/v1/organizations/:id` | 删除组织 |

#### Organization Members

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/organizations/:id/members` | 列出成员 |
| POST | `/api/v1/organizations/:id/members` | 添加成员 |
| DELETE | `/api/v1/organizations/:id/members/:user_id` | 移除成员 |
| PUT | `/api/v1/organizations/:id/members/:user_id/role` | 更新成员角色 |

#### Sessions 管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/sessions` | 列出 Sessions |
| GET | `/api/v1/sessions/:id` | 获取 Session 详情 |
| DELETE | `/api/v1/sessions/:id` | 终止 Session |

#### Evaluations

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/skills/:id/evaluations` | 获取 Skill 评价统计 |

#### Audit Logs

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/audit-logs` | 获取审计日志（分页） |
| GET | `/api/v1/audit-logs/:id` | 获取日志详情 |

#### Health

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 健康检查 |
| GET | `/health/ready` | 就绪检查 |

### 12.4 详细 Request/Response Schema

#### Skill 对象

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["id", "name", "version", "status", "created_at"],
  "properties": {
    "id": {
      "type": "string",
      "format": "uuid",
      "description": "Skill 唯一标识"
    },
    "name": {
      "type": "string",
      "pattern": "^[a-z0-9][a-z0-9-]*$",
      "minLength": 2,
      "maxLength": 64,
      "description": "Skill 名称（小写字母、数字、连字符）"
    },
    "version": {
      "type": "string",
      "pattern": "^\\d+\\.\\d+\\.\\d+$",
      "example": "1.0.0",
      "description": "语义化版本号"
    },
    "description": {
      "type": "string",
      "maxLength": 500,
      "description": "简短描述"
    },
    "content": {
      "type": "string",
      "description": "SKILL.md 完整内容"
    },
    "tags": {
      "type": "array",
      "items": { "type": "string" },
      "maxItems": 10,
      "description": "标签列表"
    },
    "visibility": {
      "type": "string",
      "enum": ["private", "org_visible", "shared", "marketplace"],
      "description": "可见性"
    },
    "status": {
      "type": "string",
      "enum": ["draft", "pending_review", "published", "rejected", "deprecated"],
      "description": "状态"
    },
    "author_agent_id": {
      "type": "string",
      "description": "作者 Agent ID"
    },
    "git_url": {
      "type": ["string", "null"],
      "format": "uri",
      "description": "Git 仓库地址"
    },
    "stats": {
      "type": "object",
      "properties": {
        "success_rate": { "type": "number", "minimum": 0, "maximum": 1 },
        "total_evaluations": { "type": "integer" },
        "confidence": { "type": "number", "minimum": 0, "maximum": 1 }
      }
    },
    "approved_at": { "type": ["string", "null"], "format": "date-time" },
    "approved_by": { "type": ["string", "null"] },
    "created_at": { "type": "string", "format": "date-time" },
    "updated_at": { "type": "string", "format": "date-time" }
  }
}
```

#### Organization 对象

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["id", "name", "created_at"],
  "properties": {
    "id": {
      "type": "string",
      "format": "uuid"
    },
    "name": {
      "type": "string",
      "minLength": 1,
      "maxLength": 255
    },
    "settings": {
      "type": "object",
      "properties": {
        "default_visibility": { "type": "string" },
        "allow_marketplace": { "type": "boolean" },
        "custom_tools": { "type": "array" }
      }
    },
    "created_at": { "type": "string", "format": "date-time" },
    "updated_at": { "type": "string", "format": "date-time" }
  }
}
```

#### Session 对象

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["session_id", "agent_id", "org_id", "status", "created_at"],
  "properties": {
    "session_id": {
      "type": "string",
      "format": "uuid"
    },
    "agent_id": {
      "type": "string"
    },
    "org_id": {
      "type": "string",
      "format": "uuid"
    },
    "capabilities": {
      "type": "array",
      "items": { "type": "string" }
    },
    "status": {
      "type": "string",
      "enum": ["active", "idle", "terminated"]
    },
    "created_at": { "type": "string", "format": "date-time" },
    "last_active_at": { "type": "string", "format": "date-time" },
    "terminated_at": { "type": ["string", "null"], "format": "date-time" }
  }
}
```

#### Evaluation 对象

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["id", "skill_id", "agent_id", "success", "created_at"],
  "properties": {
    "id": { "type": "string", "format": "uuid" },
    "skill_id": { "type": "string", "format": "uuid" },
    "session_id": { "type": ["string", "null"], "format": "uuid" },
    "agent_id": { "type": "string" },
    "success": { "type": "boolean" },
    "duration_ms": { "type": "integer", "minimum": 0 },
    "error_type": { "type": ["string", "null"] },
    "tags": {
      "type": "array",
      "items": { "type": "string" }
    },
    "created_at": { "type": "string", "format": "date-time" }
  }
}
```

### 12.5 MCP Protocol Request/Response 格式

#### Initialize 请求

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "roots": { "listChanged": true },
      "sampling": {}
    },
    "clientInfo": {
      "name": "agent-browse-v1",
      "version": "1.0.0"
    }
  }
}
```

#### Initialize 响应

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": { "listChanged": true }
    },
    "serverInfo": {
      "name": "skillgarden",
      "version": "0.4.0"
    },
    "session_id": "sess_abc123",
    "tools": [
      { "name": "skills.list", "description": "...", "inputSchema": {...} },
      { "name": "skills.get", "description": "...", "inputSchema": {...} },
      { "name": "skills.search", "description": "...", "inputSchema": {...} },
      { "name": "skills.create", "description": "...", "inputSchema": {...} },
      { "name": "skills.update", "description": "...", "inputSchema": {...} },
      { "name": "tools.execute", "description": "...", "inputSchema": {...} },
      { "name": "session.info", "description": "...", "inputSchema": {...} },
      { "name": "session.declare", "description": "...", "inputSchema": {...} },
      { "name": "evaluations.submit", "description": "...", "inputSchema": {...} }
    ]
  }
}
```

#### tools.execute 请求

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools.execute",
  "params": {
    "name": "browser_navigate",
    "arguments": {
      "url": "https://example.com",
      "wait_until": "networkidle"
    }
  }
}
```

#### tools.execute 响应

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"success\": true, \"url\": \"https://example.com\", \"title\": \"Example Domain\"}"
      }
    ],
    "is_error": false
  }
}
```

#### tools.execute 错误响应

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "error": {
    "code": -32603,
    "message": "Tool execution failed",
    "data": {
      "code": "CAPABILITY_MISMATCH",
      "details": {
        "required": ["browse"],
        "agent_capabilities": ["qa"]
      }
    }
  }
}
```

### 12.6 版本控制策略

**API 版本**：

- 当前版本：`v1`
- 路径前缀：`/api/v1/`
- 版本升级：创建新版本路径，不破坏旧版本（至少 12 个月维护期）

**Breaking Changes**：

- 删除端点
- 删除或重命名字段
- 改变字段类型
- 改变认证要求

**Non-Breaking Changes**：

- 添加新端点
- 添加新字段（响应中）
- 添加新可选参数

**版本检测**：

```http
GET /api/v1/skills HTTP/1.1
Accept: application/json
API-Version: 2024-05-15
```

响应头：
```http
HTTP/1.1 200 OK
Content-Type: application/json
API-Version: v1
API-Deprecation: true
API-Sunset: Sat, 01 Jan 2026 00:00:00 GMT
```
