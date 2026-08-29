# AionHive - Claude Code 上下文

> 本文件为 Claude Code 提供项目上下文

---

## 项目概述

**Anspire SkillGarden** 是一个面向企业的 **Agent Skills 共享平台**。

### 核心价值

```
Skills 成为企业核心 AI 资产
├── 跨容器/跨服务器/跨云的 Agent Skills 共享
├── Agents 自维护 Skills（创建、更新、评价）
├── Skills 可追溯、可审计
└── 企业级管理控制
```

### 与 ClawPool 的关系

SkillGarden 是 ClawPool（数字员工集群）的配套产品，解决 OpenClaw Agent 之间的 Skills 孤岛问题。

---

## 目录结构

```
aion-hive/
│
├── agents/                      # Agent 运行时
│   └── mcp-server/              # MCP Server 实现 (MVP 1-3)
│
├── skills/                      # 🔥 Skills 资产仓库
│   ├── _templates/
│   │   └── skill-template/
│   ├── browse/
│   ├── review/
│   └── qa/
│
├── setup/                       # 🚀 Agent 自接入
│   └── setup.md
│
├── admin/                       # 📊 管理平台 (MVP 4)
│
├── tests/                       # 测试
│   └── e2e/                     # E2E 测试
│       └── mcp_e2e_test.ts     # MCP HTTP E2E 测试
│
├── docs/                        # 文档
│   ├── DESIGN.md                # 设计文档
│   ├── ARCHITECTURE.md          # 架构文档
│   └── MVP.md                   # MVP 规范
│
├── data/                        # 数据存储
│   └── registry/
│
├── logs/                        # 日志
│
├── CLAUDE.md                    # 本文件
├── README.md
├── VERSION
├── CHANGELOG.md
├── LICENSE
├── start-http-server.ps1        # HTTP 服务器启动脚本
├── start-sse-server.ps1        # SSE 服务器启动脚本
└── .gitignore
```

---

## 关键概念

### Skill

可复用的 AI 能力单元，包含：
- `SKILL.md`：Skill 定义（YAML frontmatter + markdown）
- 可选：`src/`、`tests/`、`assets/`

### Skill 评价（关键设计）

**评价给 Agent 看，不是给人看**

```typescript
// 结构化评价
interface Evaluation {
  skill_id: string;
  agent_id: string;
  success: boolean;           // 是否成功
  duration_ms: number;         // 执行时间
  error_type?: 'timeout' | 'crash' | 'logic_error' | 'other';
  tags?: ('reliable' | 'fast' | 'stable' | 'experimental')[];
}
```

### MCP Server

提供 Skills 访问协议，实现以下工具：
- `skills_search` - 搜索 Skills
- `skills_install` - 安装 Skills
- `evaluate_skill` - 评价 Skills
- `skills_stats` - 获取统计数据

#### MCP Transport Modes

| Mode | Endpoint | Description |
|------|----------|-------------|
| stdio | - | 标准输入输出（默认） |
| HTTP | `POST /mcp` | HTTP POST 请求/响应 |
| SSE | `GET /sse` + `POST /sse/:session_id` | Server-Sent Events 双向通信 |

启动命令：
```bash
# Stdio（默认）
cargo run

# HTTP/SSE 模式
$env:AION_HIVE_TRANSPORT="http"
$env:AION_HIVE_HTTP_PORT="8080"
cargo run
```

---

## MVP 阶段

```
MVP 1 (Week 1-2): Agent 侧核心
MVP 2 (Week 3-4): Agent 评价闭环
MVP 3 (Week 5-6): 核心假设验证
MVP 4 (后续):     管理平台
```

详细规范见 `docs/MVP.md`

---

## 验证指标

| 指标 | 目标 |
|------|------|
| Skills 复用率 | > 30% |
| Agent 接入时间 | < 5 分钟 |
| 任务成功率 | > 90% |

---

## 相关项目

- [ClawPool](https://github.com/aionui/clawpool) - 数字员工集群平台
- [OpenClaw](https://github.com/aionui/openclaw) - Agent 运行时
- [Hermes Agent](https://github.com/primaryshots/hermes) - 自进化 Agent 参考

---

## CLI 命令

```bash
# 开发
npm run dev          # 启动 MCP Server

# 构建
npm run build         # 构建
npm run build:watch  # 监听模式

# 测试
npm test             # 运行测试
npm run test:watch   # 监听模式
npm run test:coverage # 覆盖率

# 代码质量
npm run lint         # ESLint
npm run format       # Prettier
```

---

## 测试

### E2E Tests (Deno)

```bash
# HTTP Transport E2E Tests
.\start-http-server.ps1 -Port 8080
deno test --allow-all --filter "MCP E2E" tests/e2e/mcp_e2e_test.ts

# SSE Transport E2E Tests (需要 Node.js 环境)
# MCP SDK 的 SSE 客户端依赖 EventSource，在 Deno 中不兼容
```

### Rust Integration Tests

```bash
cargo test --test integration
```

---

## 常用命令

- `/help` - 显示帮助
- `/plan-ceo-review` - CEO 级别战略评审
- `/office-hours` - YC Office Hours 头脑风暴

---

**最后更新**：2026-04-21
**版本**：0.3.0
