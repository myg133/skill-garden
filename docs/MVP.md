# AionHive - MVP 规范

> **版本**: v0.3.0
> **更新日期**: 2026-04-20
> **目标**: 验证 Skills 共享对 ClawPool 生态是否有效

---

## 1. MVP 概述

### 1.1 背景

ClawPool 中的每个 OpenClaw Agent 运行在独立容器中，分布在不同服务器或云服务商。这些 Agent 彼此隔离，无法感知对方的 Skills，导致：

- 经验无法积累
- Skills 重复开发
- 新 Agent 难以快速具备能力

### 1.2 核心假设

**Skills 作为企业 AI 资产，对 ClawPool 生态是有效的**

### 1.3 验证指标

| 指标 | 目标 | 测量方式 |
|------|------|----------|
| Skills 复用率 | > 30% | 同一 Skill 被多个 Agent 安装 |
| Agent 接入时间 | < 5 分钟 | 从 setup.md 到能搜索 Skills |
| 任务成功率 | > 90% | 使用 Skills 的任务完成率 |
| 评价数据质量 | 置信度 > 0.7 | 评价数 >= 3，且加权成功率 > 0.6 |

### 1.4 MVP 分阶段

```
MVP 1 (Week 1-2): MCP Server 核心
├── Rust 项目初始化
├── MCP Server (SSE + streamable-http)
├── Skills 注册表（搜索 + 列表）
├── Skills 安装（下载）
└── Tantivy 全文搜索

MVP 2 (Week 3-4): Skills 贡献闭环
├── Skills 创建/更新/删除
├── Skills 结构化评价
├── 置信度权重计算
├── 限流机制
└── 多 Agent 并发测试

MVP 3 (Week 5-6): 核心假设验证
├── 3-5 个真实任务
├── 数据收集
└── 假设验证

MVP 4 (后续迭代): 管理平台
├── Svelte Admin Dashboard
├── REST API
├── Skills 审核工作流
└── 审计日志
```

---

## 2. MVP 1: MCP Server 核心（Week 1-2）

### 2.1 目标

验证 Skills 共享在技术上是可行的

### 2.2 技术栈

| 组件 | 技术 |
|------|------|
| 语言 | Rust 1.70+ |
| Web 框架 | Axum 0.7 |
| MCP 协议 | 官方 Rust crate |
| 序列化 | serde + serde_json |
| 全文搜索 | Tantivy 0.22 |
| 文件锁 | fs2 |
| 测试 | cargo test |

### 2.3 任务列表

#### T1.1: Rust 项目初始化

```
负责人：开发者
预估时间：2 小时

验收标准：
- [ ] Cargo.toml 配置
- [ ] 项目结构创建（src/, tests/）
- [ ] 基础依赖引入
- [ ] main.rs 入口
- [ ] 编译通过
```

#### T1.2: 目录初始化模块

```
负责人：开发者
预估时间：1 小时

验收标准：
- [ ] data/registry/ 目录创建
- [ ] data/evaluations/ 目录创建
- [ ] skills-index.json 初始化
- [ ] 启动时自动初始化
```

#### T1.3: MCP Server 核心

```
负责人：开发者
预估时间：8 小时

验收标准：
- [ ] Axum HTTP 服务器可启动
- [ ] streamable-http 传输层实现
- [ ] SSE 传输层实现
- [ ] health_check 接口可用
- [ ] 基础错误处理
```

#### T1.4: Models 模块

```
负责人：开发者
预估时间：4 小时

验收标准：
- [ ] Skill 模型定义
- [ ] Evaluation 模型定义
- [ ] ApiResponse 包装
- [ ] ErrorCode 枚举
- [ ] 序列化/反序列化测试
```

#### T1.5: Validation 模块

```
负责人：开发者
预估时间：4 小时

验收标准：
- [ ] Skill 验证（大小限制、名称、标签）
- [ ] Evaluation 验证
- [ ] 恶意内容检测
- [ ] 单元测试通过
```

#### T1.6: Registry Service

```
负责人：开发者
预估时间：6 小时

验收标准：
- [ ] skills_search 接口实现
- [ ] skills_list 接口实现
- [ ] skills_info 接口实现
- [ ] skills_install 接口实现（下载）
- [ ] JSON 文件存储
- [ ] 文件锁保护写入
```

#### T1.7: Search Service (Tantivy)

```
负责人：开发者
预估时间：6 小时

验收标准：
- [ ] Tantivy 索引创建
- [ ] 中文分词配置（jieba）
- [ ] 索引构建（name, description, tags, content）
- [ ] 搜索查询实现
- [ ] 启动时索引重建
- [ ] 增量索引更新
```

#### T1.8: Storage Service

```
负责人：开发者
预估时间：4 小时

验收标准：
- [ ] 文件读取/写入
- [ ] 原子写入（临时文件 + rename）
- [ ] 文件锁封装
- [ ] 单元测试
```

#### T1.9: MCP Tools 定义

```
负责人：开发者
预估时间：4 小时

验收标准：
- [ ] skills_search tool
- [ ] skills_list tool
- [ ] skills_info tool
- [ ] skills_install tool
- [ ] health_check tool
```

#### T1.10: 集成测试

```
负责人：开发者
预估时间：4 小时

验收标准：
- [ ] MCP Server 启动测试
- [ ] 搜索接口测试
- [ ] 安装接口测试
- [ ] 端到端流程测试
```

#### T1.11: 部署文档

```
负责人：开发者
预估时间：2 小时

验收标准：
- [ ] README.md 快速启动
- [ ] 环境变量说明
- [ ] 健康检查验证
```

### 2.4 MVP 1 里程碑检查

- [ ] MCP Server 可通过 HTTP 访问
- [ ] skills_search 返回预期结果
- [ ] skills_install 可下载 SKILL.md
- [ ] 单元测试覆盖 > 80%

---

## 3. MVP 2: Skills 贡献闭环（Week 3-4）

### 3.1 目标

Agent 可以创建、评价 Skills，形成完整闭环

### 3.2 任务列表

#### T2.1: Skills 创建功能

```
负责人：开发者
预估时间：6 小时

验收标准：
- [ ] skills_create tool
- [ ] 文件锁保护并发
- [ ] 同名 Skill 冲突处理（同一 Agent 可覆盖自己的）
- [ ] 安全验证（大文件、恶意内容）
- [ ] Tantivy 增量索引
```

#### T2.2: Skills 更新功能

```
负责人：开发者
预估时间：4 小时

验收标准：
- [ ] skills_update tool
- [ ] 版本管理
- [ ] Tantivy 增量更新
- [ ] 智能升级提示
```

#### T2.3: Skills 删除功能

```
负责人：开发者
预估时间：2 小时

验收标准：
- [ ] skills_delete tool
- [ ] 索引清理
- [ ] 评价数据保留
```

#### T2.4: Evaluator Service

```
负责人：开发者
预估时间：8 小时

验收标准：
- [ ] evaluate_skill 接口
- [ ] 限流机制（每个 Agent 每 Skill 每天 10 条）
- [ ] 评价数据存储
- [ ] 置信度权重计算
- [ ] 加权统计（成功率、平均时间）
- [ ] skills_stats 接口
```

#### T2.5: 多 Agent 并发测试

```
负责人：开发者
预估时间：6 小时

验收标准：
- [ ] 并发创建测试
- [ ] 并发评价测试
- [ ] 文件锁无死锁
- [ ] 数据一致性验证
```

### 3.3 MVP 2 里程碑检查

- [ ] Agent 可创建新 Skill
- [ ] Agent 可评价 Skill
- [ ] skills_stats 返回置信度信息
- [ ] 并发测试通过

---

## 4. MVP 3: 核心假设验证（Week 5-6）

### 4.1 目标

用真实任务验证 "Skills 作为企业 AI 资产是有效的"

### 4.2 任务列表

#### T3.1: 真实任务测试

```
负责人：开发者
预估时间：8 小时

任务 1: QA 测试
- Agent A 使用 browse skill 访问网站
- Agent A 使用 qa skill 进行测试
- Agent A 提交评价

任务 2: 代码审查
- Agent B 使用 review skill 审查代码
- Agent B 提交评价

任务 3: 混合任务
- Agent C 搜索多个 Skills
- Agent C 组合使用
- Agent C 评价每个 Skill
```

#### T3.2: 数据收集

```
负责人：开发者
预估时间：4 小时

收集指标：
- [ ] Skills 复用率
- [ ] Agent 接入时间
- [ ] 任务成功率
- [ ] 评价数据质量
```

#### T3.3: 假设验证

```
负责人：开发者
预估时间：4 小时

验证标准：
- [ ] Skills 复用率 > 30%？
- [ ] 置信度 > 0.7？
- [ ] 任务成功率 > 90%？
```

#### T3.4: 迭代报告

```
负责人：开发者
预估时间：2 小时

输出：
- [ ] MVP 报告
- [ ] 核心假设验证结论
- [ ] 后续建议
```

### 4.3 MVP 3 里程碑检查

- [ ] 3-5 个真实任务完成
- [ ] 数据收集完成
- [ ] 核心假设验证结论

---

## 5. MVP 4: 管理平台（后续迭代）

### 5.1 目标

提供企业级管理能力

### 5.2 任务列表

#### T4.1: Svelte 前端初始化

```
负责人：开发者
预估时间：4 小时

验收标准：
- [ ] Svelte + Vite 项目
- [ ] 路由配置
- [ ] 基础组件库
```

#### T4.2: REST API

```
负责人：开发者
预估时间：8 小时

端点：
- [ ] GET /api/skills
- [ ] GET /api/skills/:id
- [ ] PUT /api/skills/:id
- [ ] DELETE /api/skills/:id
- [ ] GET /api/skills/:id/stats
```

#### T4.3: Admin Dashboard

```
负责人：开发者
预估时间：16 小时

页面：
- [ ] Skills 列表
- [ ] Skill 详情
- [ ] 统计仪表板
- [ ] 搜索过滤
```

#### T4.4: 审核工作流

```
负责人：开发者
预估时间：8 小时

功能：
- [ ] Skills 待审核队列
- [ ] 审核通过/拒绝
- [ ] 审核历史
```

#### T4.5: 审计日志

```
负责人：开发者
预估时间：6 小时

功能：
- [ ] 操作日志记录
- [ ] 日志查询
- [ ] 导出功能
```

---

## 6. 验收标准

### 6.1 MVP 1 验收

| 验收项 | 标准 | 测试方式 |
|--------|------|----------|
| MCP Server 启动 | 3 秒内启动 | 手动计时 |
| health_check | 返回正确格式 | curl |
| skills_search | 返回 Skills 列表 | 集成测试 |
| skills_install | 返回 SKILL.md 内容 | 集成测试 |
| 单元测试 | > 80% 覆盖 | cargo test |

### 6.2 MVP 2 验收

| 验收项 | 标准 | 测试方式 |
|--------|------|----------|
| skills_create | 成功创建 | 集成测试 |
| 冲突处理 | 同 Agent 可覆盖 | 集成测试 |
| 限流 | 第 11 条评价被拒绝 | 集成测试 |
| 置信度 | 正确计算 | 单元测试 |

### 6.3 MVP 3 验收

| 验收项 | 标准 | 测试方式 |
|--------|------|----------|
| Skills 复用率 | > 30% | 数据统计 |
| 任务成功率 | > 90% | 任务统计 |
| 置信度 | > 0.7 | 数据统计 |

---

## 7. 风险与缓解

| 风险 | 概率 | 影响 | 缓解策略 |
|------|------|------|----------|
| Skills 质量失控 | 高 | 高 | 置信度权重自动筛选 |
| Agent 提交脏数据 | 中 | 中 | 限流 + 置信度权重 |
| 并发写入冲突 | 中 | 高 | 文件锁保护 |
| 搜索结果不准确 | 低 | 中 | Tantivy + 启动自愈 |

---

## 8. 里程碑

```
Week 0: 项目初始化
├── 项目结构创建
└── 依赖配置

Week 1: MCP Server 核心
├── Models 定义
├── Storage Service
├── Registry Service
└── MCP 协议实现

Week 2: 搜索与测试
├── Tantivy 搜索
├── 集成测试
└── 部署文档

Week 3: 贡献功能
├── skills_create
├── skills_update
└── 安全验证

Week 4: 评价功能
├── Evaluator Service
├── 置信度权重
└── 并发测试

Week 5-6: 验证
├── 真实任务测试
├── 数据收集
└── 假设验证
```

---

## 9. 参考

- [ARCHITECTURE.md](./ARCHITECTURE.md) - 完整架构文档
- [DESIGN.md](./DESIGN.md) - 设计文档
- [Cargo.toml 模板](#10-cargo-dependencies)

---

## 10. Cargo 依赖

```toml
[package]
name = "aion-hive"
version = "0.3.0"
edition = "2021"

[dependencies]
# Web 框架
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# MCP 协议
mcp = "0.1"

# JSON / 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 全文搜索
tantivy = "0.22"
tantivy-tokenizer = { version = "0.22", features = ["jieba"] }

# 文件操作
fs2 = "0.4"

# 日志
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# 错误处理
anyhow = "1"
thiserror = "1"

# 时间
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tokio-test = "0.4"
```

---

## 附录 A: MCP Tools 快速参考

### 消费型

| Tool | 输入 | 输出 |
|------|------|------|
| health_check | - | HealthStatus |
| skills_search | query, tags, limit | Vec<SkillMetadata> |
| skills_list | - | Vec<SkillMetadata> |
| skills_info | skill_id | SkillDetail |
| skills_install | skill_id | InstallResult |

### 贡献型

| Tool | 输入 | 输出 |
|------|------|------|
| skills_create | name, description, tags, content | Skill |
| skills_update | skill_id, updates | Skill |
| skills_delete | skill_id | - |
| evaluate_skill | skill_id, success, duration_ms, ... | EvaluationResult |
| skills_stats | skill_id | SkillStats |
