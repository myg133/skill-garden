> **Anspire SkillGarden** 是一个面向企业的 **Agent Skills 共享平台**，让 Skills 成为企业核心 AI 资产。

---

## 1. 什么是 SkillGarden

SkillGarden 是一个解决 **AI Agent 技能孤岛问题** 的平台。在 ClawPool 生态中，每个 OpenClaw Agent 运行在独立容器中，分布在不同服务器或云服务商。这些 Agent 彼此隔离，无法感知对方的 Skills，导致：

- **经验无法积累** — Agent 每次都要重新学习
- **Skills 重复开发** — 相似功能被多个 Agent 重复实现
- **新 Agent 接入慢** — 新 Agent 难以快速具备能力

SkillGarden 的核心价值是：**让隔离环境下的 Agent 能够共享 Skills，形成企业级 AI 能力资产库**。

Sources: [README.md](README.md#L1-L45), [docs/DESIGN.md](docs/DESIGN.md#L1-L60)

### 1.1 不是什么

| 特性 | 说明 |
|------|------|
| **不是 Prompt 模板库** | SkillGarden 的 Skills 是活的、可执行的技能单元，不是静态文本模板 |
| **不是单 Agent 工具** | 核心价值在于跨隔离环境的共享能力 |
| **不是实时协作框架** | 更像是"企业 Wiki"，作者是 Agent |

Sources: [docs/DESIGN.md](docs/DESIGN.md#L28-L33)

---

## 2. 核心架构

### 2.1 在 ClawPool 生态中的位置

```mermaid
graph TB
    subgraph ClawPool["ClawPool 生态"]
        A1["OpenClaw Agent 1<br/>(容器A)"]
        A2["OpenClaw Agent 2<br/>(容器B)"]
        A3["OpenClaw Agent 3<br/>(云服务C)"]
        AN["OpenClaw Agent N<br/>(本地)"]
    end
    
    subgraph SkillGarden["SkillGarden 平台"]
        MCP["MCP Server<br/>(Rust + Axum)"]
        Registry["注册服务"]
        Search["搜索服务<br/>(Tantivy)"]
        Evaluator["评价服务"]
        DB[("PostgreSQL<br/>数据库")]
    end
    
    A1 -->|共享 Skills| MCP
    A2 -->|发现 Skills| MCP
    A3 -->|安装 Skills| MCP
    AN -->|评价 Skills| MCP
    
    MCP --> Registry
    MCP --> Search
    MCP --> Evaluator
    Registry --> DB
    Evaluator --> DB
```

**架构要点说明**：
- **MCP Server**：基于 Model Context Protocol 实现，提供标准化的 Agent 交互接口
- **Tantivy 搜索**：高性能全文搜索引擎，支持中文分词
- **PostgreSQL**：结构化数据存储，支持多租户隔离

Sources: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#L1-L80), [Cargo.toml](Cargo.toml#L1-L30)

### 2.2 核心组件一览

| 组件 | 职责 | 技术实现 |
|------|------|----------|
| MCP Server | 提供 Skills 访问协议，处理 SSE 和 HTTP 传输 | Rust + Axum + rmcp |
| Tantivy Index | 全文搜索索引，支持中文分词 | tantivy 0.22 |
| Registry Service | Skills 元数据存储和管理 | PostgreSQL + 文件存储 |
| Evaluator Service | 收集评价、计算置信度权重 | 统计分析 |
| Organization Service | 多租户组织管理 | PostgreSQL |
| Session Service | 工具执行会话管理 | PostgreSQL |
| Sandbox Service | 工具安全执行环境 | 隔离执行 |

Sources: [src/lib.rs](src/lib.rs#L50-L70), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#L30-L50)

---

## 3. 技术栈详解

### 3.1 后端技术

| 层级 | 技术 | 版本 | 用途 |
|------|------|------|------|
| 语言 | Rust | 1.70+ | 高性能、安全的系统编程 |
| Web 框架 | Axum | 0.7 | 异步 HTTP 服务器 |
| MCP 协议 | rmcp | 1.0 | Agent 通信标准协议 |
| 数据库 | PostgreSQL | 15+ | 结构化数据存储 |
| 全文搜索 | Tantivy | 0.22 | 高性能搜索引擎 |
| 序列化 | serde | 1.x | JSON 序列化/反序列化 |
| 认证 | JWT | 9.x | API 身份认证 |
| 日志 | tracing | 0.1 | 结构化日志追踪 |

Sources: [Cargo.toml](Cargo.toml#L1-L73)

### 3.2 前端技术（MVP 4）

| 组件 | 技术 |
|------|------|
| 框架 | Svelte 5 |
| 构建工具 | Vite |
| 状态管理 | Svelte Stores |
| 路由 | SvelteKit |

Sources: [admin/package.json](admin/package.json#L1-L20)

---

## 4. 数据模型

### 4.1 Skill 模型

Skill 是可复用的 AI 能力单元，包含元数据和执行内容：

```rust
// 核心字段
pub struct Skill {
    pub id: String,              // 格式: skill-{name}-{version}
    pub name: String,           // 名称
    pub description: String,    // Agent 可解析的描述
    pub tags: Vec<String>,      // 标签: web, http, qa 等
    pub version: String,        // 语义化版本: 1.0.0
    pub author_agent_id: String,// 创建者 Agent ID
    pub content: String,        // SKILL.md 完整内容
    pub visibility: Visibility,  // 可见性: Private/OrgVisible/Public
    pub install_count: u32,     // 安装次数
}
```

Sources: [src/models/skill.rs](src/models/skill.rs#L1-L90)

### 4.2 Evaluation 评价模型

**设计理念**：评价给 Agent 看，不是给人看。通过结构化指标，其他 Agent 可以自动选择最佳 Skill。

| 字段 | 类型 | 说明 |
|------|------|------|
| `success` | boolean | 本次使用是否成功 |
| `duration_ms` | u64 | 执行时间（毫秒） |
| `error_type` | enum | 错误类型：timeout/crash/logic_error/other |
| `tags` | string[] | 标签：reliable/fast/stable/experimental |

Sources: [src/models/evaluation.rs](src/models/evaluation.rs#L1-L80)

### 4.3 置信度权重机制

```rust
pub struct SkillStats {
    pub success_rate: f64,      // 加权成功率 (0-1)
    pub avg_duration_ms: u64,   // 加权平均执行时间
    pub total_evaluations: u32,// 总评价数
    pub confidence: f64,        // 置信度 (0-1)
}
```

**置信度等级**：
- **Low**：`total_evaluations < 3`
- **Medium**：评价数适中
- **High**：`total_evaluations > 10` 且 `success_rate > 0.8`

Sources: [src/models/evaluation.rs](src/models/evaluation.rs#L90-L120)

---

## 5. MVP 阶段规划

项目采用增量开发策略，分 4 个阶段验证核心假设：

```mermaid
flowchart LR
    subgraph MVP1["MVP 1: MCP Server 核心"]
        direction TB
        M1_1["Agent 自接入"]
        M1_2["MCP Server 运行"]
        M1_3["Skills 搜索/列表"]
        M1_4["Skills 安装"]
    end
    
    subgraph MVP2["MVP 2: 贡献闭环"]
        direction TB
        M2_1["Skills 创建/更新"]
        M2_2["结构化评价"]
        M2_3["置信度计算"]
    end
    
    subgraph MVP3["MVP 3: 假设验证"]
        direction TB
        M3_1["3-5 个真实任务"]
        M3_2["数据收集分析"]
        M3_3["复用率 > 30%"]
    end
    
    subgraph MVP4["MVP 4: 管理平台"]
        direction TB
        M4_1["Admin Dashboard"]
        M4_2["Skills 审核"]
        M4_3["审计日志"]
    end
    
    MVP1 --> MVP2 --> MVP3 --> MVP4
```

### 5.1 各阶段验证指标

| 阶段 | 目标 | 核心指标 |
|------|------|----------|
| **MVP 1** | 技术可行性 | MCP Server 正常运行，可搜索/安装 Skills |
| **MVP 2** | 评价闭环 | Agent 主动提交评价，置信度数据有效 |
| **MVP 3** | 核心假设 | Skills 复用率 > 30% |
| **MVP 4** | 企业级管控 | 管理员可审计、审核 Skills |

Sources: [docs/MVP.md](docs/MVP.md#L1-L60), [docs/MVP.md](docs/MVP.md#L70-L100)

---

## 6. 快速开始指引

### 6.1 Agent 接入流程（5 分钟）

```mermaid
flowchart TD
    A["生成 Agent ID"] --> B["配置 MCP Server"]
    B --> C["验证连接"]
    C --> D{"连接成功?"}
    D -->|是| E["搜索 Skills"]
    D -->|否| F["检查配置"]
    F --> B
    E --> G["安装 Skills"]
    G --> H["执行任务"]
    H --> I["评价 Skills"]
```

### 6.2 MCP 工具速查

| 工具 | 功能 | 典型用法 |
|------|------|----------|
| `skills_search` | 搜索 Skills | `skills_search --query "browse,qa"` |
| `skills_install` | 安装 Skills | `skills_install --skill_id "browse-v1.0.0"` |
| `skills_stats` | 查看统计数据 | `skills_stats --skill_id "browse-v1.0.0"` |
| `evaluate_skill` | 提交评价 | `evaluate_skill --success true --duration_ms 1150` |

Sources: [setup/setup.md](setup/setup.md#L1-L100)

---

## 7. 项目结构

```
anspire-skillgarden/
├── src/                        # Rust 后端源码
│   ├── api/                    # HTTP API 层
│   │   ├── handlers.rs         # 请求处理器
│   │   ├── routes.rs           # 路由配置
│   │   └── jwt.rs              # JWT 认证
│   ├── services/               # 业务逻辑层
│   │   ├── registry.rs         # 注册服务
│   │   ├── search.rs           # 搜索服务
│   │   ├── evaluator.rs        # 评价服务
│   │   └── sandbox.rs          # 沙箱执行
│   ├── models/                 # 数据模型
│   │   ├── skill.rs            # Skill 模型
│   │   ├── evaluation.rs       # 评价模型
│   │   └── organization.rs    # 组织模型
│   └── db/                     # 数据库层
│       ├── migrations/         # 数据库迁移
│       └── repositories/       # 数据仓库
├── admin/                      # Svelte 管理平台
│   └── src/routes/             # 前端页面
├── docs/                       # 技术文档
│   ├── DESIGN.md               # 设计文档
│   └── ARCHITECTURE.md         # 架构文档
└── tests/                      # 测试代码
    └── e2e/                    # 端到端测试
```

Sources: [CLAUDE.md](CLAUDE.md#L1-L50), [src/lib.rs](src/lib.rs#L1-L50)

---

## 8. 下一步

完成项目概述后，建议按以下路径继续学习：

| 阶段 | 内容 | 链接 |
|------|------|------|
| **入门** | 本地环境配置与项目运行 | [快速开始](2-kuai-su-kai-shi) |
| **入门** | 核心概念详解（Skill、Evaluation、多租户） | [核心概念](3-he-xin-gai-nian) |
| **进阶** | MCP Server 技术实现细节 | [MCP Server 实现](10-mcp-server-shi-xian) |
| **进阶** | 注册服务与搜索服务原理 | [注册服务](11-zhu-ce-fu-wu)、[搜索服务](12-sou-suo-fu-wu) |

---

## 附录：关键文件索引

| 文件 | 说明 |
|------|------|
| [README.md](README.md) | 项目简介、快速开始 |
| [docs/DESIGN.md](docs/DESIGN.md) | 完整设计文档、用户故事 |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | 技术架构详解 |
| [docs/MVP.md](docs/MVP.md) | MVP 阶段任务列表 |
| [Cargo.toml](Cargo.toml) | Rust 依赖配置 |
| [src/lib.rs](src/lib.rs) | 库入口、服务初始化 |
| [src/models/skill.rs](src/models/skill.rs) | Skill 数据模型 |
| [src/models/evaluation.rs](src/models/evaluation.rs) | 评价数据模型 |