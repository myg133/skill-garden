本页面详细介绍 **MVP 1（Week 1-2）** 的核心交付物：MCP Server 的完整实现。MVP 1 的核心目标是验证 Skills 共享在技术上的可行性，为后续阶段奠定基础架构。

## 技术架构总览

MVP 1 采用分层架构设计，核心组件包括 MCP Server、注册服务、搜索服务和存储服务。以下架构图展示了各组件之间的交互关系：

```mermaid
flowchart TB
    subgraph Client["客户端层"]
        Agent["Agent 客户端"]
    end
    
    subgraph Transport["传输层"]
        Stdio["Stdio Transport"]
        HTTP["HTTP Transport"]
        SSE["SSE Transport"]
    end
    
    subgraph Server["MCP Server"]
        Handler["ServerHandler"]
        Tools["MCP Tools"]
    end
    
    subgraph Services["服务层"]
        Registry["RegistryService<br/>Skills CRUD"]
        Search["SearchService<br/>Tantivy 搜索"]
        Evaluator["EvaluatorService<br/>评价统计"]
        Storage["StorageService<br/>文件操作"]
    end
    
    subgraph Data["数据层"]
        DB[(PostgreSQL)]
        Index["search_index/"]
        RegistryFile["registry/"]
    end
    
    Agent --> Stdio
    Agent --> HTTP
    Agent --> SSE
    
    Stdio --> Handler
    HTTP --> Handler
    SSE --> Handler
    
    Handler --> Tools
    Tools --> Registry
    Tools --> Search
    Tools --> Evaluator
    
    Registry --> Storage
    Registry --> DB
    Search --> Index
    Evaluator --> RegistryFile
    
    Storage --> DB
```

架构设计遵循以下原则：

| 设计原则 | 说明 |
|---------|------|
| **传输无关** | 统一的 MCP 协议层，支持 stdio、HTTP、SSE 三种传输模式 |
| **服务分层** | 清晰的业务逻辑与数据访问分离 |
| **文件安全** | 使用文件锁防止并发写入冲突，原子写入保证数据一致性 |
| **全文搜索** | Tantivy 提供高性能的中文全文搜索能力 |

Sources: [src/mcp/server.rs](src/mcp/server.rs#L1-L40), [src/lib.rs](src/lib.rs#L1-L100)

## MCP Server 实现

### 核心结构

McpServer 是整个系统的核心入口点，负责处理 MCP 协议请求并调用相应的服务。它依赖以下关键服务：

```mermaid
classDiagram
    class McpServer {
        +RegistryService registry
        +SearchService search
        +EvaluatorService evaluator
        +SessionService session
        +OrgToolService org_tool
        +ToolRouterService tool_router
        +Option~AgentContext~ agent_context
        +new() McpServer
        +run() Result
        +handle_jsonrpc() Result
        +call_tool_internal() Value
    }
    
    class AgentContext {
        +String agent_id
        +Option~Uuid~ org_id
        +Option~Uuid~ session_id
        +Vec~String~ roles
        +Vec~String~ scope
    }
    
    McpServer --> AgentContext
```

McpServer 从环境变量 `AION_HIVE_JWT_TOKEN` 中提取 JWT Token 并验证身份，支持多租户场景下的会话隔离。

Sources: [src/mcp/server.rs](src/mcp/server.rs#L9-L80)

### MCP Tools 定义

MVP 1 提供了以下 MCP Tools，覆盖 Skills 的发现、安装和基本管理功能：

| 工具名称 | 功能描述 | 核心参数 |
|---------|---------|---------|
| `health_check` | 健康检查 | 无 |
| `skills.search` | 搜索 Skills | `query`, `tags`（可选）, `limit`（默认 10） |
| `skills.list` | 列出所有 Skills | `limit`（默认 100） |
| `skills.info` | 获取 Skill 详情 | `skill_id`（必填） |
| `skills.create` | 创建新 Skill | `name`, `description`, `tags`, `content`, `version` |
| `skills.update` | 更新 Skill | `skill_id`, `description`, `tags`, `content` |
| `skills.install` | 标记安装（简化实现） | `skill_id` |
| `skills.stats` | 获取统计数据 | `skill_id` |
| `session.info` | 获取会话信息 | `session_id` |
| `session.declare` | 声明能力 | `session_id`, `capabilities` |

每个 Tool 的输入模式（Input Schema）都遵循 JSON Schema 规范，确保客户端可以正确构建请求。

Sources: [src/mcp/server.rs](src/mcp/server.rs#L400-L560)

### 传输模式

系统支持三种 MCP 传输模式，适用于不同的部署场景：

```mermaid
flowchart LR
    subgraph StdioMode["Stdio 模式（默认）"]
        Stdin["stdin"]
        Stdout["stdout"]
    end
    
    subgraph HttpMode["HTTP 模式"]
        Client["HTTP Client"] --> POST[/mcp]
    end
    
    subgraph SSEMode["SSE 模式"]
        Client2["HTTP Client"] --> GET[/sse]
        Client2 --> POST2[/sse/:id]
    end
```

| 模式 | 端点 | 适用场景 | 配置方式 |
|------|------|---------|---------|
| Stdio | 标准输入输出 | 本地开发、直接调用 | 默认，无需配置 |
| HTTP | `POST /mcp` | Web 服务、容器化部署 | `AION_HIVE_TRANSPORT=http` |
| SSE | `GET /sse` + `POST /sse/:id` | 实时双向通信 | `AION_HIVE_TRANSPORT=sse` |

Sources: [src/main.rs](src/main.rs#L1-L150)

## 注册服务 (Registry Service)

### 核心功能

RegistryService 负责 Skills 的完整生命周期管理，包括创建、更新、删除和查询操作。服务采用文件锁机制保护并发写入，确保数据一致性。

```mermaid
flowchart TB
    Create["创建 Skill"] --> Validate["输入验证"]
    Validate --> Lock["获取文件锁"]
    Lock --> Write["写入文件"]
    Write --> Index["更新索引"]
    Index --> Unlock["释放锁"]
    
    subgraph FileOps["文件操作"]
        Atomic["原子写入"]
        IndexFile["skills-index.json"]
    end
    
    Write --> Atomic
    Atomic --> IndexFile
```

### 关键方法

| 方法 | 功能 | 文件锁 |
|------|------|--------|
| `create_skill` | 创建新 Skill | ✓ |
| `update_skill` | 更新现有 Skill | ✓ |
| `get_skill` | 获取 Skill 详情 | - |
| `list_skills` | 列出所有 Skills | - |
| `delete_skill` | 删除 Skill | ✓ |

服务层通过数据库（PostgreSQL）持久化 Skill 数据，同时维护本地文件系统和 Tantivy 索引的同步。

Sources: [src/services/registry.rs](src/services/registry.rs#L1-L200)

## 搜索服务 (Search Service)

### Tantivy 全文搜索

SearchService 基于 Tantivy 实现高性能全文搜索能力，支持中文分词和灵活的查询语法。

```mermaid
flowchart LR
    Query["搜索查询"] --> Parser["QueryParser"]
    Parser --> Searcher["IndexSearcher"]
    Searcher --> Fields["搜索字段"]
    
    subgraph Schema["索引 Schema"]
        Name["name - TEXT"]
        Desc["description - TEXT"]
        Tags["tags - TEXT"]
        Content["content - TEXT"]
    end
    
    Fields --> Schema
```

### 搜索能力

| 功能 | 说明 | 示例 |
|------|------|------|
| 多字段搜索 | name、description、tags、content | `"web scraping"` |
| 标签过滤 | 支持 AND/OR 组合 | `tags:python AND tags:api` |
| 排序 | 按相关性评分 | 默认 |
| 增量更新 | 支持添加/删除文档 | `add_skill`, `delete_skill` |

搜索结果返回 Skill ID、相关性评分和安装次数，便于客户端进行排序和展示。

Sources: [src/services/search.rs](src/services/search.rs#L1-L150)

## 存储服务 (Storage Service)

### 原子写入

StorageService 提供安全的文件操作能力，核心是原子写入机制：

```
临时文件写入 → 同步到磁盘 → 原子重命名
```

这种方式确保即使在写入过程中发生崩溃，也不会产生不完整或损坏的文件。

### 文件锁机制

使用 `fs2` 库实现跨进程的文件锁，防止多个 Agent 同时修改同一 Skill：

```rust
// 获取 Skill 级别的锁
let lock = get_skill_lock("skill-name", &data_dir)?;
// ... 执行写操作 ...
drop(lock); // 自动释放
```

锁的作用域是单个 Skill，允许不同 Skill 的并发写入，同时保证同一 Skill 的写入顺序执行。

Sources: [src/services/storage.rs](src/services/storage.rs#L1-L150)

## 数据模型

### Skill 模型

```mermaid
classDiagram
    class Skill {
        +String id
        +String name
        +String description
        +Vec~String~ tags
        +String version
        +String author_agent_id
        +DateTime created
        +DateTime updated
        +String compatibility
        +Vec~String~ dependencies
        +String content
        +u32 install_count
        +Option~String~ git_url
        +Visibility visibility
        +Vec~String~ tools
    }
    
    class SkillMetadata {
        +String id
        +String name
        +String description
        +Vec~String~ tags
        +u32 install_count
    }
    
    class SkillDetail {
        +SkillMetadata metadata
        +String content
        +Option~SkillStats~ stats
    }
    
    Skill --> SkillMetadata : 转换
    Skill --> SkillDetail : 转换
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | 唯一标识，格式：`skill-{name}-{version}` |
| `name` | String | Skill 名称 |
| `description` | String | 描述（Agent 可解析） |
| `tags` | Vec\<String\> | 标签列表 |
| `version` | String | 语义化版本 |
| `content` | String | SKILL.md 完整内容 |
| `visibility` | Visibility | 可见性：Private/OrgVisible/Marketplace |

Sources: [src/models/skill.rs](src/models/skill.rs#L1-L100)

## 快速开始

### 环境要求

| 组件 | 最低版本 |
|------|---------|
| Rust | 1.70+ |
| PostgreSQL | 14+ |
| 操作系统 | Windows/Linux/macOS |

### 启动方式

```powershell
# 1. 配置环境变量
$env:DATABASE_URL = "postgres://user:pass@localhost:5432/aionhive"
$env:AION_HIVE_TRANSPORT = "http"  # 可选：stdio, http, sse

# 2. 启动 HTTP 服务器
.\start-http-server.ps1 -Port 8080
```

```bash
# Stdio 模式（默认）
cargo run

# HTTP 模式
AION_HIVE_TRANSPORT=http cargo run

# SSE 模式
AION_HIVE_TRANSPORT=sse cargo run
```

### 运行测试

```powershell
# 启动服务器
.\start-http-server.ps1 -Port 8080

# 运行 MCP E2E 测试
deno test --allow-all --filter "MCP E2E" tests/e2e/mcp_e2e_test.ts
```

Sources: [tests/e2e/mcp_e2e_test.ts](tests/e2e/mcp_e2e_test.ts#L1-L50)

## 验证检查清单

MVP 1 交付前需确认以下里程碑已完成：

| 检查项 | 状态 |
|-------|------|
| MCP Server 可通过 HTTP 访问 | ✓ |
| `skills.search` 返回预期结果 | ✓ |
| `skills.list` 列出所有 Skills | ✓ |
| `skills.create` 可创建新 Skill | ✓ |
| `skills.update` 可更新现有 Skill | ✓ |
| `skills_install` 可标记安装 | ✓ |
| `health_check` 接口可用 | ✓ |
| Tantivy 索引正常工作 | ✓ |
| 文件锁保护并发写入 | ✓ |
| 单元测试覆盖 > 80% | - |

Sources: [docs/MVP.md](docs/MVP.md#L60-L80)

## 下一步

完成 MVP 1 后，项目进入 [MVP 2: Skills 贡献闭环](5-mvp-2-skills-gong-xian-bi-huan)，将实现：

- 结构化评价系统
- 置信度权重计算
- 限流机制
- 多 Agent 并发测试

如需了解更多技术细节，可参考：
- [MCP Server 实现](10-mcp-server-shi-xian) - 深度解析 MCP 协议层
- [注册服务](11-zhu-ce-fu-wu) - RegistryService 完整实现
- [搜索服务](12-sou-suo-fu-wu) - Tantivy 配置与优化