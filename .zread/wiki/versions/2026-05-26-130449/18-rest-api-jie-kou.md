本文档详细介绍 SkillGarden/AionHive 平台提供的 REST API 接口，涵盖认证机制、端点规范、请求/响应格式以及错误处理。所有 API 均基于 HTTP/JSON 协议，遵循 REST 设计原则。

## 架构概览

SkillGarden 平台提供三类独立接口，分别服务于不同的使用者：

```mermaid
graph TB
    subgraph "SkillGarden 接口体系"
        A["MCP Protocol<br/>(stdio/HTTP+SSE)"] --> B["Agent (AI Agent)<br/>Skills 发现、获取、执行"]
        C["REST API<br/>(HTTP/JSON)"] --> D["Admin (人)<br/>平台管理、审计、审核"]
        E["Webhook<br/>(HTTP POST)"] --> F["Evaluator Agent<br/>评估回调"]
    end
    
    style C fill:#e1f5fe
    style D fill:#e1f5fe
```

REST API 采用 **Axum** 框架构建，默认运行于 `8080` 端口，提供两类认证方式：Agent 认证（JWT）和管理员认证（用户名/密码）。[Sources: src/main.rs#L131-L200](src/main.rs#L131-L200)

Sources: [src/api/routes.rs](src/api/routes.rs#L1-L44)

## 认证机制

### JWT Bearer Token 认证

大部分 API 端点需要通过 `Authorization: Bearer <token>` 头部传递 JWT Token。Token 通过 `/api/v1/auth/agent/token` 接口获取，有效期为 24 小时。

**Token 载荷结构**：
```json
{
  "agent_id": "string",
  "org_id": "uuid | null",
  "session_id": "uuid | null",
  "roles": ["string"],
  "scope": ["string"],
  "exp": 1234567890,
  "iat": 1234567890
}
```

JWT Secret 通过环境变量 `AION_HIVE_JWT_SECRET` 配置，默认为 `aion_hive_secret_key_change_in_production`。[Sources: src/api/jwt.rs#L15-L19](src/api/jwt.rs#L15-L19)

### Agent 注册与认证流程

```mermaid
sequenceDiagram
    participant Agent
    participant API as REST API
    participant DB as Database
    
    Agent->>API: POST /auth/agent/register
    Note over API: 生成随机 secret
    API->>DB: 存储 agent_id + hashed_secret
    DB-->>API: success
    API-->>Agent: {agent_id, secret}
    
    Note over Agent: 安全存储 secret
    Agent->>API: POST /auth/agent/token<br/>{agent_id, agent_secret}
    API->>DB: 验证 credentials
    DB-->>API: valid
    API-->>Agent: {token, expires_in: 86400}
```

Sources: [src/api/handlers.rs#L219-L270](src/api/handlers.rs#L219-L270)

### 管理员认证

管理员通过用户名/密码登录，获取带有 `admin` 角色的 JWT Token：

```bash
POST /api/v1/admin/login
Content-Type: application/json

{
  "username": "admin",
  "password": "password"
}
```

Sources: [src/api/handlers.rs#L272-L318](src/api/handlers.rs#L272-L318)

## 错误处理

API 统一使用 `ApiError` 枚举处理错误，返回标准 HTTP 状态码和 JSON 错误体：

| 错误类型 | HTTP 状态码 | 说明 |
|----------|-------------|------|
| `NotFound` | 404 | 资源不存在 |
| `BadRequest` | 400 | 请求参数错误 |
| `Unauthorized` | 401 | 认证失败或 Token 无效 |
| `Forbidden` | 403 | 权限不足 |
| `InternalError` | 500 | 服务器内部错误 |
| `Conflict` | 409 | 资源冲突 |

**错误响应格式**：
```json
{
  "error": "错误描述信息",
  "status": 404
}
```

Sources: [src/api/error.rs#L11-L52](src/api/error.rs#L11-L52)

## Skills API

Skills API 提供完整的 CRUD 操作，支持分页查询和全文搜索。

### 列出 Skills

```
GET /api/v1/skills
```

**查询参数**：

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `tag` | string | - | 按标签过滤 |
| `keyword` | string | - | 关键词搜索（匹配名称和描述） |
| `page` | integer | 1 | 页码（最小值 1） |
| `page_size` | integer | 20 | 每页数量（最大 100） |

**响应**：
```json
{
  "data": [
    {
      "id": "uuid",
      "name": "browse",
      "description": "网页浏览技能",
      "tags": ["web", "search"],
      "visibility": "marketplace",
      "version": "1.0.0",
      "author": "agent-1"
    }
  ],
  "total": 100,
  "page": 1,
  "page_size": 20
}
```

Sources: [src/api/handlers.rs#L30-L65](src/api/handlers.rs#L30-L65)

### 获取单个 Skill

```
GET /api/v1/skills/:id
```

**响应**：
```json
{
  "metadata": {
    "id": "uuid",
    "name": "browse",
    "description": "...",
    "tags": ["web"],
    "visibility": "marketplace",
    "version": "1.0.0",
    "author": "agent-1"
  },
  "content": "# SKILL.md 内容...",
  "stats": {
    "success_rate": 0.95,
    "total_evaluations": 150,
    "confidence": 0.88
  }
}
```

Sources: [src/api/handlers.rs#L67-L82](src/api/handlers.rs#L67-L82)

### 创建 Skill

```
POST /api/v1/skills
Authorization: Bearer <token>
```

**请求体**：
```json
{
  "name": "web-scraper",
  "description": "网页数据抓取技能",
  "tags": ["web", "data"],
  "content": "# SKILL.md 内容...",
  "version": "1.0.0",
  "git_url": "https://github.com/...",
  "visibility": "marketplace",
  "tools": ["fetch", "parse"]
}
```

**可见性选项**：
- `private`：仅创建者可见
- `org_visible`：组织内可见
- `marketplace`：市场公开
- `shared`：指定共享

Sources: [src/api/handlers.rs#L84-L117](src/api/handlers.rs#L84-L117)

### 更新 Skill

```
PUT /api/v1/skills/:id
Authorization: Bearer <token>
```

所有字段均为可选，只更新提供的字段。

Sources: [src/api/handlers.rs#L119-L150](src/api/handlers.rs#L119-L150)

### 删除 Skill

```
DELETE /api/v1/skills/:id
Authorization: Bearer <token>
```

Sources: [src/api/handlers.rs#L152-L164](src/api/handlers.rs#L152-L164)

### 获取 Skill 统计数据

```
GET /api/v1/skills/:id/stats
```

**响应**：
```json
{
  "success_rate": 0.95,
  "total_evaluations": 150,
  "avg_duration_ms": 1200,
  "confidence": 0.88,
  "reliable_count": 100,
  "fast_count": 80,
  "stable_count": 120,
  "experimental_count": 10
}
```

Sources: [src/api/handlers.rs#L166-L174](src/api/handlers.rs#L166-L174)

## Evaluations API

评价 API 用于记录 Skill 使用结果，驱动置信度权重机制。

### 创建评价

```
POST /api/v1/evaluations
Authorization: Bearer <token>
```

**请求体**：
```json
{
  "skill_id": "uuid",
  "success": true,
  "duration_ms": 1150,
  "error_type": null,
  "tags": ["reliable", "fast"]
}
```

**错误类型选项**：`timeout`、`crash`、`logic_error`、`other`

**标签选项**：`reliable`、`fast`、`stable`、`experimental`

**响应**：
```json
{
  "message": "Evaluation recorded successfully",
  "evaluation_id": "uuid",
  "new_stats": {
    "success_rate": 0.95,
    "total_evaluations": 151,
    "confidence": 0.89
  }
}
```

Sources: [src/api/handlers.rs#L176-L217](src/api/handlers.rs#L176-L217)

## 认证 API

### 注册 Agent

```
POST /api/v1/auth/agent/register
```

**请求体**：
```json
{
  "agent_id": "my-agent-1",
  "agent_name": "My Agent"
}
```

**响应**：
```json
{
  "agent_id": "my-agent-1",
  "secret": "生成的随机密钥",
  "message": "Agent registered successfully. Store the secret securely - it will not be shown again."
}
```

Sources: [src/api/handlers.rs#L219-L246](src/api/handlers.rs#L219-L246)

### 获取 Token

```
POST /api/v1/auth/agent/token
```

**请求体**：
```json
{
  "agent_id": "my-agent-1",
  "agent_secret": "之前获取的密钥"
}
```

**响应**：
```json
{
  "token": "eyJhbGc...",
  "token_type": "Bearer",
  "expires_in": 86400
}
```

Sources: [src/api/handlers.rs#L248-L270](src/api/handlers.rs#L248-L270)

## 管理 API

### 管理员登录

```
POST /api/v1/admin/login
```

**请求体**：
```json
{
  "username": "admin",
  "password": "password"
}
```

**响应**：
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

Sources: [src/api/handlers.rs#L272-L318](src/api/handlers.rs#L272-L318)

### 审计日志

```
GET /api/v1/admin/audit-logs
Authorization: Bearer <admin_token>
```

**查询参数**：

| 参数 | 类型 | 说明 |
|------|------|------|
| `agent_id` | string | 按 Agent 过滤 |
| `action` | string | 按操作类型过滤 |
| `resource_type` | string | 按资源类型过滤 |
| `limit` | integer | 返回数量（默认 50，最大 100） |
| `offset` | integer | 偏移量 |

Sources: [src/api/handlers.rs#L320-L370](src/api/handlers.rs#L320-L370)

### Skill 审核

```
POST /api/v1/admin/skills/:id/approve
Authorization: Bearer <admin_token>
```

将 Skill 状态更新为 `published`。

```
POST /api/v1/admin/skills/:id/reject
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "reason": "违反平台规范"
}
```

将 Skill 状态更新为 `rejected`，并记录拒绝原因到审计日志。

Sources: [src/api/handlers.rs#L410-L477](src/api/handlers.rs#L410-L477)

## 组织管理 API (v0.4)

多租户架构下的组织管理接口。

### 创建组织

```
POST /api/v1/organizations
```

**请求体**：
```json
{
  "name": "ACME Corp"
}
```

**响应** (HTTP 201)：
```json
{
  "id": "uuid",
  "name": "ACME Corp",
  "created_at": "2024-01-01T00:00:00Z"
}
```

Sources: [src/api/handlers.rs#L485-L494](src/api/handlers.rs#L485-L494)

### 列出组织

```
GET /api/v1/organizations
```

**查询参数**：`limit`（默认 20，最大 100）、`offset`

Sources: [src/api/handlers.rs#L507-L519](src/api/handlers.rs#L507-L519)

### 获取/更新/删除组织

| 方法 | 端点 | 说明 |
|------|------|------|
| `GET` | `/api/v1/organizations/:id` | 获取单个组织 |
| `PUT` | `/api/v1/organizations/:id` | 更新组织名称 |
| `DELETE` | `/api/v1/organizations/:id` | 删除组织 |

Sources: [src/api/handlers.rs#L496-L542](src/api/handlers.rs#L496-L542)

## 会话管理 API (v0.4)

管理 Agent 与平台的会话上下文。

### 创建会话

```
POST /api/v1/sessions
```

**请求体**：
```json
{
  "agent_id": "my-agent-1",
  "org_id": "uuid"
}
```

Sources: [src/api/handlers.rs#L546-L555](src/api/handlers.rs#L546-L555)

### 列出/获取会话

| 方法 | 端点 | 说明 |
|------|------|------|
| `GET` | `/api/v1/sessions` | 列出所有会话（支持按 `status` 过滤） |
| `GET` | `/api/v1/sessions/:id` | 获取单个会话 |

Sources: [src/api/handlers.rs#L557-L584](src/api/handlers.rs#L557-L584)

### 结束会话

```
POST /api/v1/sessions/:id/end
```

Sources: [src/api/handlers.rs#L586-L595](src/api/handlers.rs#L586-L595)

### 声明能力

```
POST /api/v1/sessions/:id/declare
```

**请求体**：
```json
{
  "capabilities": ["browse", "qa", "code-execute"]
}
```

用于 Agent 声明其具备的能力，Tool Router 据此路由工具请求。

Sources: [src/api/handlers.rs#L597-L607](src/api/handlers.rs#L597-L607)

## 组织工具 API (v0.4)

管理组织自定义工具的注册和审核。

### 注册组织工具

```
POST /api/v1/org-tools
```

**请求体**：
```json
{
  "org_id": "uuid",
  "tool_id": "custom-scraper",
  "name": "Custom Web Scraper",
  "description": "自定义网页抓取工具",
  "schema": {
    "type": "object",
    "properties": {
      "url": {"type": "string"}
    }
  },
  "implementation": {}
}
```

Sources: [src/api/handlers.rs#L611-L627](src/api/handlers.rs#L611-L627)

### 列出组织工具

| 方法 | 端点 | 说明 |
|------|------|------|
| `GET` | `/api/v1/org-tools` | 列出所有组织的工具 |
| `GET` | `/api/v1/org-tools/:org_id` | 列出指定组织的工具 |
| `GET` | `/api/v1/org-tools/:org_id?approved_only=true` | 仅列出已审核通过的工具 |

Sources: [src/api/handlers.rs#L629-L652](src/api/handlers.rs#L629-L652)

### 审核组织工具

```
POST /api/v1/org-tools/:id/approve
Authorization: Bearer <admin_token>
```

```
POST /api/v1/org-tools/:id/reject
Authorization: Bearer <admin_token>
```

Sources: [src/api/handlers.rs#L654-L675](src/api/handlers.rs#L654-L675)

## 输入验证规则

API 层接收请求后会进行以下验证：

| 字段 | 最大长度 | 规则 |
|------|----------|------|
| Skill 名称 | 100 字符 | 仅允许字母、数字、连字符、下划线 |
| 标签 | 50 字符/个，最多 10 个 | 仅允许字母、数字、连字符、下划线 |
| 描述 | 2000 字符 | - |
| 内容 | 500KB | 禁止路径遍历、恶意脚本 |
| 版本号 | - | 遵循 SemVer (x.y.z) |
| 执行时长 | 1 小时 | 评价记录的最大 duration_ms |

**安全验证**：自动检测并拒绝包含 `<script`、`javascript:`、`../` 等恶意模式的内容请求。

Sources: [src/schemas/validation.rs#L1-L120](src/schemas/validation.rs#L1-L120)

## 完整端点列表

| 方法 | 端点 | 认证 | 说明 |
|------|------|------|------|
| `GET` | `/health` | 无 | 健康检查 |
| `POST` | `/mcp` | 无 | MCP JSON-RPC |
| `GET` | `/sse` | 无 | SSE 连接 |
| `POST` | `/sse/:session_id` | 无 | SSE 消息 |
| `GET` | `/api/v1/skills` | Bearer | 列出 Skills |
| `POST` | `/api/v1/skills` | Bearer | 创建 Skill |
| `GET` | `/api/v1/skills/:id` | Bearer | 获取 Skill |
| `PUT` | `/api/v1/skills/:id` | Bearer | 更新 Skill |
| `DELETE` | `/api/v1/skills/:id` | Bearer | 删除 Skill |
| `GET` | `/api/v1/skills/:id/stats` | Bearer | 获取统计 |
| `POST` | `/api/v1/evaluations` | Bearer | 创建评价 |
| `POST` | `/api/v1/auth/agent/register` | 无 | 注册 Agent |
| `POST` | `/api/v1/auth/agent/token` | 无 | 获取 Token |
| `POST` | `/api/v1/admin/login` | 无 | 管理员登录 |
| `GET` | `/api/v1/admin/audit-logs` | Admin | 审计日志 |
| `POST` | `/api/v1/admin/skills/:id/approve` | Admin | 审核通过 |
| `POST` | `/api/v1/admin/skills/:id/reject` | Admin | 审核拒绝 |
| `POST` | `/api/v1/organizations` | Bearer | 创建组织 |
| `GET` | `/api/v1/organizations` | Bearer | 列出组织 |
| `GET` | `/api/v1/organizations/:id` | Bearer | 获取组织 |
| `PUT` | `/api/v1/organizations/:id` | Bearer | 更新组织 |
| `DELETE` | `/api/v1/organizations/:id` | Bearer | 删除组织 |
| `POST` | `/api/v1/sessions` | Bearer | 创建会话 |
| `GET` | `/api/v1/sessions` | Bearer | 列出会话 |
| `GET` | `/api/v1/sessions/:id` | Bearer | 获取会话 |
| `POST` | `/api/v1/sessions/:id/end` | Bearer | 结束会话 |
| `POST` | `/api/v1/sessions/:id/declare` | Bearer | 声明能力 |
| `POST` | `/api/v1/org-tools` | Bearer | 注册工具 |
| `GET` | `/api/v1/org-tools` | Bearer | 列出所有工具 |
| `GET` | `/api/v1/org-tools/:org_id` | Bearer | 列出组织工具 |
| `POST` | `/api/v1/org-tools/:id/approve` | Admin | 工具审核通过 |
| `POST` | `/api/v1/org-tools/:id/reject` | Admin | 工具审核拒绝 |

Sources: [src/api/routes.rs](src/api/routes.rs#L1-L44)

## 环境配置

运行 API 服务前需配置环境变量：

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DATABASE_URL` | `postgres://postgres:password@localhost:5432/aionhive` | PostgreSQL 连接字符串 |
| `AION_HIVE_HTTP_PORT` | `8080` | HTTP 服务器端口 |
| `AION_HIVE_DATA_DIR` | `./data` | 数据存储目录 |
| `AION_HIVE_SKILLS_DIR` | `./skills` | Skills 存储目录 |
| `AION_HIVE_JWT_SECRET` | `aion_hive_secret_key_change_in_production` | JWT 签名密钥 |

Sources: [.env.example](.env.example#L1-L31)

---

## 相关页面

- [MCP 协议接口](17-mcp-xie-yi-jie-kou) - Agent 使用的 MCP 协议接口
- [认证与授权](19-ren-zheng-yu-shou-quan) - 详细的权限控制机制
- [系统架构](8-xi-tong-jia-gou) - 整体技术架构
- [组织管理](20-zu-zhi-guan-li) - 多租户组织管理详解
- [会话管理](21-hui-hua-guan-li) - Agent 会话生命周期