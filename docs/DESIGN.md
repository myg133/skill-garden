# AionHive - 设计文档

> **愿景**：构建企业级 AI Skills 生态系统，让 Skills 成为企业核心资产

---

## 1. 项目概述

### 1.1 是什么

**AionHive** 是一个面向企业的 **Agent Skills 共享平台**，支持：

- **跨容器/跨服务器/跨云**的 Agent Skills 共享
- Agents **自维护** Skills（创建、更新、评价）
- Skills 作为企业 **可追溯、可审计** 的 AI 资产
- Skills 被 Agent 网络 **共同贡献和受益**

### 1.2 不是什么

- **不是** Prompt 模板库（是活的、可执行的 Skills）
- **不是** 单 Agent 工具（是跨隔离环境的共享平台）
- **不是** 实时协作框架（AionHive 更像是"企业 Wiki"，作者是 Agent）

### 1.3 与 ClawPool 的关系

```
┌─────────────────────────────────────────────────────────────────┐
│                        ClawPool 生态                             │
│                                                                 │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │OpenClaw │  │OpenClaw │  │OpenClaw │  │OpenClaw │           │
│  │ Agent 1 │  │ Agent 2 │  │ Agent 3 │  │ Agent N │           │
│  │ (容器A) │  │ (容器B) │  │ (云服务C)│  │ (本地)  │           │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘           │
│       │            │            │            │                 │
│       │   独立运行，互不知晓彼此的 Skills                │
│       │            │            │            │                 │
│       └────────────┼────────────┼────────────┘                 │
│                    │                                          │
│                    ▼                                          │
│         ┌─────────────────────┐                                │
│         │   AionHive         │                                │
│         │   Skills 共享平台    │                                │
│         │                     │                                │
│         │  Agent 1 的 Skills ─┼─→ Agent 2 可发现、安装、使用    │
│         │  Agent 2 的 Skills ─┼─→ Agent 3 可发现、安装、使用    │
│         │  Agent 3 的 Skills ─┼─→ Agent 1 可发现、安装、使用    │
│         └─────────────────────┘                                │
└─────────────────────────────────────────────────────────────────┘
```

### 1.4 核心差异化

| 特性 | 传统方式 | AionHive |
|------|----------|-------------|
| Skills 来源 | 人工编写 | Agent 自生成 + 人工审核 |
| 共享范围 | 无法跨容器/服务器 | 真正隔离环境下的共享 |
| 共享方式 | 手动复制 | 自动检索安装 |
| 评价机制 | 人工评价 | Agent 可读的量化指标 |
| 企业控制 | 无 | 后端管理平台审计 |

---

## 2. 核心价值主张

### 2.1 问题陈述

```
现状：每个 Agent 都是"技能孤岛"
├── ClawPool 中每个 OpenClaw 运行在独立容器
├── 不同服务器、不同云服务商
├── 彼此无法感知对方的 Skills
├── 经验无法积累和复用
└── 重复造轮子

愿景：Skills 成为企业核心 AI 资产
├── Agent 网络共享各自的能力和经验
├── Skills 可检索、可安装、可评价
├── 企业可审计 Skills 来源和质量
└── 数据驱动持续优化
```

### 2.2 核心价值

1. **降本**：减少重复造轮子，Skills 复用率目标 > 30%
2. **增效**：Skills 即插即用，新 Agent 快速具备能力
3. **进化**：Agent 使用后自动评价，驱动 Skills 优化
4. **可控**：企业级审计，管理员可查看、审核、禁用 Skills

---

## 3. 目标用户

### 3.1 主要用户

| 用户角色 | 描述 | 核心需求 |
|----------|------|----------|
| **Agent (OpenClaw)** | AI Agent 程序 | 检索、安装、使用、评价 Skills |
| **企业管理员** | 最高权限 | 全局审计、Skills 审核、策略配置 |
| **部门管理员** | 部门权限 | 本部门 Skills 管理、开放策略 |
| **开发者** | Skill 创作者 | 创建、测试、发布 Skills |

### 3.2 用户故事

```
Agent 视角（给 Agent 程序看）：
作为 Agent，我想要：
1. 读取 setup.md 自动接入 AionHive
2. 根据任务需求检索合适的 Skills
3. 查看 Skills 的量化评价指标
4. 一键安装 Skills 到本地
5. 使用后提交结构化评价（成功率、执行时间等）

管理员视角（给企业 IT 看）：
作为企业管理员，我想要：
1. 在后台查看所有 Skills 列表和内容
2. 审核新创建的 Skills
3. 禁用有问题的 Skills
4. 查看 Skills 使用记录和来源追溯
5. 配置 Skills 开放策略（部门内/全公司）
```

---

## 4. MVP 范围

### 4.1 核心假设

**Skills 作为企业 AI 资产，对 ClawPool 生态是有效的**

**验证指标**：
| 指标 | 目标 | 测量方式 |
|------|------|----------|
| Skills 复用率 | > 30% | 同一 Skill 被多个 Agent 安装 |
| Agent 接入时间 | < 5 分钟 | 从 setup.md 到能搜索 Skills |
| 任务成功率 | > 90% | 使用 Skills 的任务完成率 |

### 4.2 MVP 分阶段

```
┌─────────────────────────────────────────────────────────────────┐
│                     MVP 1: Agent 侧核心（Week 1-2）               │
│                                                                 │
│  目标：验证 Skills 共享在技术上是可行的                            │
│                                                                 │
│  功能：                                                           │
│  ├── Agent 自接入（setup.md）                                     │
│  ├── MCP Server 运行                                              │
│  ├── Skills 注册表（搜索 + 列表）                                  │
│  └── Skills 安装（复制到本地）                                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     MVP 2: Agent 评价闭环（Week 3-4）              │
│                                                                 │
│  目标：验证 Agent 会主动评价 Skills，评价数据有效                  │
│                                                                 │
│  功能：                                                           │
│  ├── Skills 结构化评价（量化指标，非文本）                         │
│  ├── Skills 使用追踪                                              │
│  ├── Skills 版本管理                                              │
│  └── 多 Agent 并发测试                                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     MVP 3: 核心假设验证（Week 5-6）               │
│                                                                 │
│  目标：验证 Skills 复用率 > 30%                                    │
│                                                                 │
│  功能：                                                           │
│  ├── 3-5 个真实任务测试                                           │
│  ├── 数据收集与分析                                                │
│  └── 假设验证与迭代计划                                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     MVP 4: 管理平台（后续迭代）                     │
│                                                                 │
│  目标：让企业管理员可审计和控制 Skills                             │
│                                                                 │
│  功能：                                                           │
│  ├── Admin Dashboard（Skills 列表 + 详情）                        │
│  ├── Skills 审核工作流                                            │
│  ├── Skills 启用/禁用                                             │
│  └── 使用统计与审计日志                                            │
└─────────────────────────────────────────────────────────────────┘
```

### 4.3 MVP 1-3 非目标

- **不包含**管理平台（MVP 4 才做）
- **不包含**复杂权限（MVP 4 部门隔离）
- **不包含**自动进化（Agent 评价驱动优化）
- **不包含**多语言（先 TypeScript）

### 4.4 Skills 评价设计（关键决策）

**评价给谁看？Agent，不是人。**

```
传统设计：
Agent 评价 → 文本反馈 → 管理员阅读 → 判断质量

AionHive 设计：
Agent 评价 → 结构化指标 → 其他 Agent 读取 → 自动选择最佳 Skill
```

**结构化评价指标**：

```typescript
interface SkillEvaluation {
  skill_id: string;
  agent_id: string;
  timestamp: string;

  // 量化指标（Agent 可读）
  success_rate: number;      // 0-100，成功率
  avg_duration_ms: number;   // 平均执行时间（毫秒）
  error_count: number;       // 错误次数
  timeout_count: number;      // 超时次数

  // 枚举标签（Agent 可理解）
  tags: ('reliable' | 'fast' | 'stable' | 'experimental')[];

  // 可选：简短备注（人类可读，Agent 可忽略）
  note?: string;
}
```

**为什么不用文本评价？**
1. Agent 生成文本需要额外 LLM 调用，成本高
2. Agent 解析其他 Agent 的文本评价复杂
3. 量化指标可直接用于 Skills 排序和选择

---

## 5. 技术架构

### 5.1 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                    ClawPool Agent 运行环境                        │
│                                                                 │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │OpenClaw │  │OpenClaw │  │OpenClaw │  │OpenClaw │           │
│  │Agent 1  │  │Agent 2  │  │Agent 3  │  │Agent N  │           │
│  │容器/服务器│  │容器/服务器│  │云服务    │  │本地      │           │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘         │
│       │            │            │            │                 │
│       │   MCP Protocol (HTTP/WS)                                  │
│       └────────────┼────────────┼────────────┘                 │
│                    │                                          │
└────────────────────┼────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                 AionHive Runtime (MVP 1-3)                    │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │ MCP Server   │  │ Skills       │  │ Evaluator    │         │
│  │              │  │ Registry     │  │              │         │
│  │ - search     │  │ - index      │  │ - metrics    │         │
│  │ - install    │  │ - metadata   │  │ - stats      │         │
│  │ - evaluate   │  │ - versions   │  │              │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                 AionHive Admin (MVP 4)                       │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │ Admin        │  │ Skills       │  │ Audit        │         │
│  │ Dashboard    │  │ Approval     │  │ Logs         │         │
│  │              │  │ Workflow     │  │              │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Skills Repository                           │
│                                                                 │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐          │
│  │ browse  │  │ review  │  │   qa    │  │  ...    │          │
│  │ SKILL.md│  │ SKILL.md│  │ SKILL.md│  │         │          │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘          │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ Registry: skills-index.json + evaluations/             │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 核心组件

#### 5.2.1 MCP Server

```
职责：
- 提供 Skills 访问协议（MCP 标准）
- 处理 Agent 请求
- 管理 Agent 会话

接口（MVP 1-3）：
- skills.search(query) → Skill[]
- skills.install(skill_id) → boolean
- skills.evaluate(skill_id, metrics) → boolean
- skills.info(skill_id) → SkillDetail
- health.check() → HealthStatus
```

#### 5.2.2 Skills Registry

```
职责：
- 存储 Skills 元数据
- 维护 Skills 索引
- 版本管理

数据结构（Skill）：
{
  id: string                    // "skill-name-v1.0.0"
  name: string
  description: string
  version: string
  tags: string[]
  author_agent_id: string
  created_at: timestamp
  updated_at: timestamp
  metrics: {
    avg_success_rate: number
    avg_duration_ms: number
    total_installs: number
    total_evaluations: number
  }
}
```

#### 5.2.3 Evaluator

```
职责：
- 收集 Skills 使用量化指标
- 聚合 Agent 评价数据
- 生成 Skills 统计数据

数据：
- 每次使用记录（success, duration, errors）
- 评价聚合（按 Skill 统计）
- Agent 行为分析（可选）
```

### 5.3 技术选型

| 组件 | 选择 | 理由 |
|------|------|------|
| Agent Runtime | OpenClaw (ClawPool) | 现有基础设施 |
| MCP Protocol | 官方 Rust crate | 支持 SSE + streamable-http |
| Storage | JSON Files (MVP) | 简单、够用 |
| Admin UI | Svelte (MVP 4) | 轻量、快速 |
| Backend | Rust + Axum 0.7 | 高性能、类型安全 |

---

## 6. 目录结构

```
aion-hive/
│
├── src/                           # 🔥 MCP Server 实现 (Rust)
│   ├── main.rs                    # 入口
│   ├── lib.rs                     # 库入口
│   ├── mcp/
│   │   ├── mod.rs                 # MCP 模块
│   │   ├── server.rs              # MCP Server 实现
│   │   └── tools.rs               # MCP Tools 定义
│   ├── services/
│   │   ├── mod.rs
│   │   ├── registry.rs            # Skills 注册服务
│   │   ├── evaluator.rs           # 评价服务
│   │   ├── search.rs              # 搜索服务 (Tantivy)
│   │   └── storage.rs             # 文件存储服务
│   ├── models/
│   │   ├── mod.rs
│   │   ├── skill.rs               # Skill 模型
│   │   ├── evaluation.rs          # Evaluation 模型
│   │   └── error.rs               # 错误类型
│   ├── schemas/
│   │   ├── mod.rs
│   │   └── validation.rs          # 数据验证
│   └── utils/
│       ├── mod.rs
│       ├── rate_limiter.rs         # 限流
│       └── weight.rs               # 置信度权重
│
├── skills/                        # 🔥 Skills 资产仓库
│   ├── _templates/
│   │   └── skill-template/
│   ├── browse/
│   ├── review/
│   └── qa/
│
├── setup/                         # 🚀 Agent 自接入
│   └── setup.md
│
├── admin/                         # 📊 管理平台 (MVP 4)
│
├── docs/                          # 文档
│   ├── DESIGN.md
│   ├── ARCHITECTURE.md
│   └── MVP.md
│
├── data/                          # 数据存储
│   ├── registry/
│   │   └── skills-index.json
│   └── evaluations/
│       └── {skill-id}.json
├── tests/
│   └── integration.sh             # 集成测试脚本
│
├── Cargo.toml
├── README.md
├── VERSION
├── CHANGELOG.md
├── LICENSE
└── .gitignore
```

---

## 7. 关键决策

### 7.1 Skills 格式

每个 Skill 是一个目录，包含：

```
skill-name/
├── SKILL.md          # 必须：Skill 定义（YAML frontmatter + markdown）
├── README.md         # 可选：使用说明（人类可读）
├── src/              # 可选：代码
├── tests/            # 可选：测试
└── assets/           # 可选：资源
```

**SKILL.md 结构**：

```yaml
---
name: skill-name
description: 简短描述（Agent 可理解）
tags: [tag1, tag2]
version: 1.0.0
author_agent_id: agent-xxx
created: 2026-04-20
---
```

### 7.2 MCP 协议

使用 Model Context Protocol 标准：

- 工具调用（tools/）
- 资源访问（resources/）
- 提示模板（prompts/）

### 7.3 存储策略

```
MVP 1-3（当前）：
- 文件系统存储 Skills
- JSON 文件存储注册表
- JSON 文件存储评价数据

MVP 4 及以后：
- SQLite/PostgreSQL 存储元数据
- S3/GCS 存储大文件
- Redis 缓存（可选）
```

### 7.4 评价设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 评价格式 | 结构化指标 | Agent 可直接使用，无需解析文本 |
| 评价触发 | Agent 主动 | Agent 使用 Skill 后自动提交 |
| 评价聚合 | 平均值 | 简单、易理解 |
| 文本备注 | 可选 | 保留人类可读信息，但非必须 |

---

## 8. 验证计划

### 8.1 验证假设

**核心假设**：Skills 共享对 ClawPool 生态有效

**验证指标**：
1. Skills 复用率 > 30%
2. Agent 接入时间 < 5 分钟
3. 任务成功率 > 90%

### 8.2 验证步骤

```
Week 1-2: MVP 1 - 基础接入
├── setup.md + MCP server
├── 1 个测试 Agent 接入成功
├── 可检索 Skills
└── 可安装 Skills

Week 3-4: MVP 2 - 评价闭环
├── Agent 可提交结构化评价
├── Skills 统计数据更新
└── 多 Agent 并发测试

Week 5-6: MVP 3 - 核心假设验证
├── 3-5 个真实任务
├── 收集使用数据
├── 验证复用率 > 30%
└── 决策：继续 or 调整
```

---

## 9. 风险与对策

| 风险 | 影响 | 对策 |
|------|------|------|
| Skills 质量不可控 | 高 | MVP 4 管理平台审核 |
| Agent 提交无效评价 | 中 | 量化指标验证（成功率总和=100%） |
| 存储膨胀 | 低 | 版本清理策略 |
| 无企业信任 | 高 | MVP 4 完整审计功能 |

---

## 10. 商业模式

### 10.1 定位

**企业版产品**，配套 ClawPool（数字员工集群）销售

### 10.2 定价策略

定价在产品化阶段确定，参考因素：
- ClawPool 已有客户数量
- Skills 存储量
- API 调用次数
- 高级功能（管理平台、审计）

### 10.3 与竞品差异

| 竞品 | 模式 | AionHive 差异 |
|------|------|------------------|
| OpenAI GPTs | 封闭生态 | 面向企业，支持私有部署 |
| LangChain Agents | 工具库 | 真正的跨容器 Skills 共享 |
| AutoGen | 开源框架 | 企业级管理和审计 |
| Custom Agents | 内部工具 | 可销售的产品 |

---

## 11. 附录

### 11.1 相关项目

- [Hermes Agent](https://github.com/primaryshots/hermes) - 自进化 Agent 参考（无共享）
- [MCP SDK](https://github.com/modelcontextprotocol/sdk) - 官方协议
- [ClawPool](https://github.com/aionui/clawpool) - 数字员工集群平台
- [OpenClaw](https://github.com/aionui/openclaw) - Agent 运行时

### 11.2 术语表

| 术语 | 定义 |
|------|------|
| Skill | 可复用的 AI 能力单元 |
| Agent / OpenClaw | AI 执行单元（运行在独立容器） |
| MCP | Model Context Protocol |
| Registry | Skills 注册表 |
| ClawPool | 数字员工集群管理平台 |
| 评价指标 | Agent 可读的结构化数据（成功率等） |

---

**最后更新**：2026-04-20
**版本**：0.3.0
**状态**：与 ARCHITECTURE.md 同步，Rust 技术栈
