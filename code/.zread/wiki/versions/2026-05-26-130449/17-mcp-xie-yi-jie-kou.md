本文档详细描述 Anspire SkillGarden 项目中 MCP (Model Context Protocol) 协议的接口实现。该协议为 AI Agent 提供标准化的 Skills 访问机制，支持工具调用、搜索、注册等核心功能。

Sources: [src/mcp/server.rs](src/mcp/server.rs#L1-L781)
Sources: [src/main.rs](src/main.rs#L1-L253)

## 协议概述

MCP 协议是基于 JSON-RPC 2.0 的通信协议，采用**工具调用**（Tool Calling）模式实现 AI Agent 与 Skills 平台之间的交互。项目使用 `rmcp 1.0` 库实现协议栈，支持标准输入输出（stdio）和 HTTP/SSE 两种传输方式。

MCP 协议的核心价值在于为 AI Agent 提供统一的 Skills 调用接口，使得 Agent 能够动态发现、搜索、安装和评估平台上的 Skills，实现跨 Agent 的知识与能力共享。

Sources: [Cargo.toml](Cargo.toml#L20-L21)

## 协议架构

MCP 协议在系统中的位置可通过以下架构图说明：

```mermaid
flowchart TB
    subgraph "MCP Client Layer"
        A["AI Agent<br/>(Claude/GPT等)"]
    end
    
    subgraph "Transport Layer"
        B["Stdio Transport"]
        C["HTTP Transport"]
        D["SSE Transport"]
    end
    
    subgraph "MCP Server"
        E["McpServer Handler"]
        F["Tool Router"]
    end
    
    subgraph "Service Layer"
        G["RegistryService<br/>Skills注册"]
        H["SearchService<br/>全文搜索"]
        I["EvaluatorService<br/>评价统计"]
        J["SessionService<br/>会话管理"]
    end
    
    subgraph "Data Layer"
        K["PostgreSQL"]
        L["Tantivy Index"]
    end
    
    A --> B
    A --> C
    A --> D
    B --> E
    C --> E
    D --> E
    E --> F
    F --> G
    F --> H
    F --> I
    F --> J
    G --> K
    H --> L
    I --> K
    J --> K
```

协议遵循标准的请求-响应模式，每个请求包含 `method` 字段标识要调用的工具，`params` 字段传递参数，`id` 用于请求-响应匹配。

Sources: [src/mcp/server.rs](src/mcp/server.rs#L94-L200)

## 工具清单

MCP Server 提供以下 12 个工具，按功能分为三类：

### 健康检查与系统工具

| 工具名称 | 描述 | 必需参数 |
|---------|------|---------|
| `health_check` | 检查 MCP 服务健康状态 | 无 |

### Skills 管理工具

| 工具名称 | 描述 | 必需参数 |
|---------|------|---------|
| `skills.list` | 列出所有可用的 Skills | limit (可选) |
| `skills.info` | 获取指定 Skill 的详细信息 | skill_id |
| `skills.search` | 根据关键词和标签搜索 Skills | query |
| `skills.create` | 创建新的 Skill | name, description, content |
| `skills.update` | 更新现有 Skill | skill_id |
| `skills.install` | 标记 Skill 为已安装 | skill_id |
| `skills.stats` | 获取 Skill 的评价统计 | skill_id |

### 评价与会话工具

| 工具名称 | 描述 | 必需参数 |
|---------|------|---------|
| `evaluate_skill` | 提交 Skill 执行评价 | skill_id, agent_id, success, duration_ms |
| `session.info` | 获取当前会话信息 | session_id |
| `session.declare` | 声明 Agent 会话能力 | session_id, capabilities |

Sources: [src/mcp/server.rs](src/mcp/server.rs#L351-L540)

## 请求格式

所有 MCP 请求均遵循 JSON-RPC 2.0 格式：

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "工具名称",
    "arguments": {
      "参数1": "值1",
      "参数2": "值2"
    }
  }
}
```

### 初始化请求

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {
      "name": "example-client",
      "version": "1.0.0"
    }
  }
}
```

服务端响应：

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "serverInfo": {
      "name": "aion-hive",
      "version": "0.3.0"
    }
  }
}
```

### 工具列表请求

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}
```

Sources: [src/mcp/server.rs](src/mcp/server.rs#L109-L150)

## 工具参数详解

### skills.search — 搜索 Skills

```json
{
  "name": "skills.search",
  "arguments": {
    "query": "web scraping",
    "tags": ["web", "http"],
    "limit": 10
  }
}
```

**参数说明：**

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|-----|------|-----|-------|-----|
| query | string | 是 | - | 搜索关键词 |
| tags | string[] | 否 | null | 按标签过滤 |
| limit | number | 否 | 10 | 最大返回数量 |

**响应示例：**

```json
{
  "isError": false,
  "content": [
    {
      "text": "[{\"id\":\"skill-web-scraper-1.0.0\",\"name\":\"web-scraper\",...}]"
    }
  ]
}
```

### skills.create — 创建 Skill

```json
{
  "name": "skills.create",
  "arguments": {
    "name": "my-skill",
    "description": "A useful skill for data processing",
    "tags": ["data", "processing"],
    "content": "# SKILL.md\n\nThis skill provides...",
    "version": "1.0.0"
  }
}
```

**参数说明：**

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|-----|------|-----|-------|-----|
| name | string | 是 | - | Skill 名称（唯一） |
| description | string | 是 | - | Skill 描述 |
| tags | string[] | 否 | [] | 标签列表 |
| content | string | 是 | - | SKILL.md 内容 |
| version | string | 否 | "1.0.0" | 版本号（semver） |

### evaluate_skill — 评价 Skill

```json
{
  "name": "evaluate_skill",
  "arguments": {
    "skill_id": "skill-web-scraper-1.0.0",
    "agent_id": "agent-001",
    "success": true,
    "duration_ms": 1500,
    "error_type": null,
    "tags": ["reliable", "fast"]
  }
}
```

**参数说明：**

| 参数 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| skill_id | string | 是 | Skill 标识符 |
| agent_id | string | 是 | 评价 Agent ID |
| success | boolean | 是 | 执行是否成功 |
| duration_ms | number | 是 | 执行耗时（毫秒） |
| error_type | string | 否 | 错误类型：timeout/crash/logic_error/other |
| tags | string[] | 否 | 评价标签：reliable/fast/stable/experimental |

Sources: [src/mcp/server.rs](src/mcp/server.rs#L201-L350)

## 认证机制

MCP 服务支持两种认证方式：

### 标准输入输出模式（Stdio）

通过环境变量传递 JWT Token：

```bash
export AION_HIVE_JWT_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
./aion-hive --mode stdio
```

服务启动时自动从环境变量提取并验证 Token，构建 `AgentContext` 上下文。

Sources: [src/mcp/server.rs](src/mcp/server.rs#L41-L60)

### HTTP 传输模式

通过 `Authorization: Bearer <token>` 请求头传递：

```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call",...}'
```

### JWT Claims 结构

认证后的 Token 包含以下 Claims：

| Claim | 类型 | 说明 |
|-------|------|------|
| agent_id | string | Agent 唯一标识 |
| org_id | uuid | 组织 ID（可选） |
| session_id | uuid | 会话 ID（可选） |
| roles | string[] | 角色列表 |
| scope | string[] | 权限范围 |
| exp | i64 | 过期时间戳 |
| iat | i64 | 签发时间戳 |

Sources: [src/api/jwt.rs](src/api/jwt.rs#L18-L30)

## 传输层接口

### HTTP 传输

**端点：** `POST /mcp`

接受原始 JSON-RPC 请求体，返回 JSON-RPC 响应。

```bash
# 健康检查
curl -X POST http://127.0.0.1:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"health_check","arguments":{}}}'
```

Sources: [src/main.rs](src/main.rs#L32-L40)

### SSE 传输

SSE（Server-Sent Events）传输支持持久化会话，适合需要保持状态的场景。

**建立会话：**
```
GET /sse
```

服务端返回 SSE 流，包含 `endpoint` 事件指示消息端点。

**发送消息：**
```
POST /sse/{session_id}
```

```bash
# 建立 SSE 会话
curl -N http://127.0.0.1:8080/sse

# 发送 MCP 请求（从另一个终端）
curl -X POST http://127.0.0.1:8080/sse/abc123 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"health_check","arguments":{}}}'
```

Sources: [src/main.rs](src/main.rs#L42-L85)

### 标准输入输出传输

适用于本地进程调用，通过子进程方式启动 MCP 服务：

```bash
# 环境变量配置
export AION_HIVE_JWT_TOKEN="..."
export DATABASE_URL="postgres://..."

# 启动 stdio 服务
./aion-hive --mode stdio

# 服务从 stdin 读取请求，写入 stdout
```

Sources: [src/mcp/server.rs](src/mcp/server.rs#L69-L73)

## 错误处理

### 错误响应格式

MCP 错误响应遵循 JSON-RPC 2.0 错误规范：

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32603,
    "message": "Internal error: skill not found"
  }
}
```

### 错误码定义

| 错误码 | 含义 | 说明 |
|-------|------|------|
| -32600 | Invalid Request | 请求格式错误 |
| -32601 | Method not found | 未知方法 |
| -32602 | Invalid params | 参数无效 |
| -32603 | Internal error | 服务器内部错误 |
| -32000 | Authentication required | 需要认证 |
| -32001 | Permission denied | 权限不足 |

### 工具级错误

工具执行失败时，通过 `isError: true` 标记：

```json
{
  "isError": true,
  "content": [
    {
      "type": "text",
      "text": "{\"error\": \"skill_id is required\"}"
    }
  ]
}
```

Sources: [src/mcp/server.rs](src/mcp/server.rs#L95-L102)

## SDK 使用示例

### TypeScript (Deno) 使用 MCP SDK

```typescript
import { Client } from "https://esm.run/@modelcontextprotocol/sdk@1.29.0/client";
import { StreamableHTTPClientTransport } from "https://esm.run/@modelcontextprotocol/sdk@1.29.0/client/streamableHttp.js";

const MCP_SERVER_URL = "http://127.0.0.1:8080/mcp";

async function createClient() {
  const client = new Client({ name: "my-agent", version: "1.0.0" });
  const transport = new StreamableHTTPClientTransport(MCP_SERVER_URL);
  await client.connect(transport);
  return client;
}

// 搜索 Skills
async function searchSkills(query: string) {
  const client = await createClient();
  const result = await client.callTool({
    name: "skills_search",
    arguments: { query, limit: 10 },
  });
  
  if (result.isError) {
    throw new Error("Search failed");
  }
  
  const text = (result.content[0] as { text?: string }).text!;
  return JSON.parse(text);
}

// 创建 Skill
async function createSkill(name: string, description: string, content: string) {
  const client = await createClient();
  const result = await client.callTool({
    name: "skills_create",
    arguments: { name, description, content, version: "1.0.0" },
  });
  
  if (result.isError) {
    throw new Error("Create failed");
  }
  
  const text = (result.content[0] as { text?: string }).text!;
  return JSON.parse(text);
}
```

Sources: [tests/e2e/mcp_e2e_test.ts](tests/e2e/mcp_e2e_test.ts#L1-L100)

### SSE 传输示例

```typescript
import { Client } from "https://esm.run/@modelcontextprotocol/sdk@1.29.0/client";
import { SSEClientTransport } from "https://esm.run/@modelcontextprotocol/sdk@1.29.0/client/sse.js";

const SSE_SERVER_URL = "http://127.0.0.1:8080/sse";

async function createSseClient() {
  const client = new Client({ name: "my-agent", version: "1.0.0" });
  const transport = new SSEClientTransport(SSE_SERVER_URL);
  await client.connect(transport);
  return client;
}

// 多次请求共享同一会话
async function multiRequestDemo() {
  const client = await createSseClient();
  
  const result1 = await client.callTool({
    name: "health_check",
    arguments: {},
  });
  
  const result2 = await client.callTool({
    name: "skills_list",
    arguments: { limit: 5 },
  });
  
  // 会话保持，两次请求共享状态
}
```

Sources: [tests/e2e/mcp_sse_e2e_test.ts](tests/e2e/mcp_sse_e2e_test.ts#L1-L80)

## 协议版本兼容性

| 协议版本 | 支持状态 | 说明 |
|---------|---------|------|
| 2024-11-05 | ✅ 当前版本 | 使用的标准版本 |
| 2024-10-05 | ⚠️ 可能兼容 | 需测试验证 |

服务端在初始化时声明支持的协议版本，客户端应使用匹配版本进行通信。

Sources: [src/mcp/server.rs](src/mcp/server.rs#L118-L128)

## 下一步

- 深入了解 [REST API 接口](18-rest-api-jie-kou) — MCP 的 RESTful 替代方案
- 探索 [认证与授权](19-ren-zheng-yu-shou-quan) 机制
- 查看 [会话管理](21-hui-hua-guan-li) 了解多租户支持