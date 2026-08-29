# AionHive

> 企业级 AI Skills 共享平台

[![Version](https://img.shields.io/badge/version-0.2.0-blue.svg)](VERSION)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

---

## 愿景

**Skills 成为企业核心 AI 资产**

---

## 是什么

Anspire SkillGarden 是一个面向企业的 **Agent Skills 共享平台**，支持：

- **跨容器/跨服务器/跨云**的 Agent Skills 共享
- Agents **自维护** Skills（创建、更新、评价）
- Skills 作为企业 **可追溯、可审计** 的 AI 资产
- Skills 被 Agent 网络 **共同贡献和受益**

---

## 核心问题

ClawPool 中的每个 OpenClaw Agent 运行在独立容器中，分布在不同服务器或云服务商。这些 Agent 彼此隔离，无法感知对方的 Skills，导致：

- 经验无法积累
- Skills 重复开发
- 新 Agent 难以快速具备能力

**SkillGarden 要解决**：让这些真正隔离的 Agent 能够共享 Skills。

---

## 核心价值

| 价值 | 描述 |
|------|------|
| **降本** | 减少重复造轮子，Skills 复用率目标 > 30% |
| **增效** | Skills 即插即用，新 Agent 快速具备能力 |
| **进化** | Agent 使用后自动评价，驱动 Skills 优化 |
| **可控** | 企业级审计，管理员可查看、审核、禁用 Skills |

---

## 与 ClawPool 的关系

```
┌─────────────────────────────────────────────────────────────────┐
│                        ClawPool 生态                             │
│                                                                 │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │OpenClaw │  │OpenClaw │  │OpenClaw │  │OpenClaw │           │
│  │ Agent 1 │  │ Agent 2 │  │ Agent 3 │  │ Agent N │           │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘           │
│       │            │            │            │                   │
│       │            │            │            │                   │
│       │      跨容器/跨服务器/跨云 Skills 共享                    │
│       │            │            │            │                   │
│       └────────────┴────────────┴────────────┘                   │
│                    │                                              │
│                    ▼                                              │
│         ┌─────────────────────┐                                    │
│         │   SkillGarden       │                                    │
│         │   Skills 共享平台    │                                    │
│         └─────────────────────┘                                    │
└─────────────────────────────────────────────────────────────────┘
```

---

## 快速开始

### 1. Agent 接入

参考 [setup/setup.md](setup/setup.md) 进行 5 分钟快速接入。

### 2. 搜索 Skills

```bash
mcp__skillgarden__skills_search --query "browse,qa"
```

### 3. 安装 Skills

```bash
mcp__skillgarden__skills_install --skill_id "browse-v1.0.0"
```

### 4. 评价 Skills（结构化指标）

```bash
mcp__skillgarden__evaluate_skill \
  --skill_id "browse-v1.0.0" \
  --success true \
  --duration_ms 1150 \
  --tags "reliable,fast"
```

---

## 文档

- [设计文档](docs/DESIGN.md) - 项目概述、核心价值、MVP 范围
- [架构文档](docs/ARCHITECTURE.md) - 详细技术架构
- [MVP 规范](docs/MVP.md) - MVP 分阶段任务列表
- [安装指南](setup/setup.md) - Agent 接入指南

---

## MVP 阶段

| 阶段 | 时间 | 目标 |
|------|------|------|
| MVP 1 | Week 1-2 | Agent 侧核心（接入、搜索、安装） |
| MVP 2 | Week 3-4 | Agent 评价闭环（结构化评价） |
| MVP 3 | Week 5-6 | 核心假设验证（复用率 > 30%） |
| MVP 4 | 后续迭代 | 管理平台（审计、审核） |

---

## Skills 评价设计

**评价给 Agent 看，不是给人看**

传统设计：
```
Agent 评价 → 文本反馈 → 管理员阅读 → 判断质量
```

SkillGarden 设计：
```
Agent 评价 → 结构化指标 → 其他 Agent 读取 → 自动选择最佳 Skill
```

**结构化评价指标**：
- `success`：本次使用是否成功
- `duration_ms`：执行时间
- `error_type`：错误类型
- `tags`：标签（reliable/fast/stable/experimental）

---

## 验证指标

| 指标 | 目标 | 测量方式 |
|------|------|----------|
| Skills 复用率 | > 30% | 同一 Skill 被多个 Agent 安装 |
| Agent 接入时间 | < 5 分钟 | 从 setup.md 到能搜索 Skills |
| 任务成功率 | > 90% | 使用 Skills 的任务完成率 |

---

## 项目结构

```
aion-hive/
├── agents/
│   └── mcp-server/           # MCP Server 实现
├── skills/                    # Skills 资产仓库
│   ├── browse/
│   ├── review/
│   └── qa/
├── setup/                     # Agent 自接入
│   └── setup.md
├── admin/                     # 管理平台 (MVP 4)
├── docs/                      # 文档
├── data/                      # 数据存储
└── logs/                      # 日志
```

---

## 技术栈

| 组件 | 选择 |
|------|------|
| 语言 | TypeScript 5.x |
| 运行时 | Node.js 20.x |
| MCP SDK | @modelcontextprotocol/sdk 0.5.x |
| 存储 | JSON Files (MVP) |
| 测试 | Vitest |
| 构建 | tsup |

---

## 相关项目

- [ClawPool](https://github.com/aionui/clawpool) - 数字员工集群平台
- [OpenClaw](https://github.com/aionui/openclaw) - Agent 运行时
- [Hermes Agent](https://github.com/primaryshots/hermes) - 自进化 Agent 参考
- [MCP SDK](https://github.com/modelcontextprotocol/sdk) - 官方协议

---

## 许可证

[MIT](LICENSE)

---

## 版本

当前版本：0.2.0

详见 [CHANGELOG.md](CHANGELOG.md)
