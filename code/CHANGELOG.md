# Changelog

所有重要的项目变更都将记录在此文件中。

## [0.4.0] - 2026-06-03

### 新增功能

#### License 配额管理 (REQ-003 Phase 2 Stage 4)

企业私有部署模式下，支持通过 license.json 文件限制租户配额：

- **License 文件读取**: 从 `LICENSE_FILE` 环境变量读取 license.json
- **配额检查**: 创建租户前检查当前租户数量是否达到 max_tenants 上限
- **友好错误**: 配额达到上限时返回清晰的错误信息

##### License 文件格式

```json
{
  "max_tenants": 5,
  "features": ["sso", "audit"]
}
```

##### 相关文件

- `src/services/license.rs` - License 服务模块
- `src/api/handlers/tenants.rs` - 租户创建增加配额检查
- `src/db/repositories/tenant.rs` - 添加 count() 方法
- `src/services/admin/tenant.rs` - 添加 count() 方法

### 变更

- `src/lib.rs` - 导出 LicenseService 和 LicenseQuotaExceeded
- `src/services/mod.rs` - 导出 license 模块

---

## [0.3.0] - 2026-04-20

### 重大更新

#### 技术栈确认

- **后端**: Rust + Axum
- **前端**: Svelte (MVP 4)
- **协议**: MCP (官方 Rust crate, SSE + streamable-http)
- **搜索**: Tantivy + tantivy-tokenizer (jieba 分词)
- **文件锁**: fs2

#### 工程架构评审完成

通过 plan-eng-review 的完整评审，确认以下架构决策：

| 决策 | 方案 |
|------|------|
| 并发写入 | fs2 文件锁 |
| 全文搜索 | Tantivy 嵌入式索引 |
| 错误处理 | 统一 Response + ErrorCode |
| 数据验证 | Rust 结构体验证 + 安全扫描 |
| SKILL.md 格式 | 添加 updated, compatibility, dependencies 字段 |
| 目录结构 | src/ + data/ + skills/ 分离 |
| 索引同步 | 新建/更新时增量 |
| Skill 冲突 | 同一 Agent 可覆盖自己的 |
| 评价真实性 | 置信度权重机制 |
| 版本升级 | 智能提示 |
| 安全检查 | 大小限制 + 恶意内容检测 |
| 竞态条件 | 文件锁保护 |
| 索引一致性 | 两阶段提交 + 启动自愈 |

#### 目录结构标准化

```
src/
├── main.rs
├── lib.rs
├── mcp/          # MCP 协议实现
├── services/     # 业务逻辑
├── models/       # 数据模型
├── schemas/      # 验证
└── utils/        # 工具类
```

#### 核心机制实现

- **置信度权重**: 基于历史一致性、最近时间、多数一致性等因素
- **限流机制**: 每个 Agent 对同一 Skill 每天最多 10 条评价
- **安全验证**: 1MB 大小限制、恶意内容检测

### 文档更新

- `docs/ARCHITECTURE.md` - 完整技术架构 + 数据流 + 存储设计
- `docs/MVP.md` - 详细任务列表 + Rust 技术栈

### 新增验证指标

- 评价数据质量：置信度 > 0.7

---

## [0.2.0] - 2026-04-20

### 重大更新

#### 设计重构

- **核心叙事更新**：从"新 Agent 继承老 Agent 能力"改为"Agent 网络共享各自的能力和经验"
- **定位明确**：配套 ClawPool（数字员工集群）销售
- **上下文清晰**：针对跨容器/跨服务器/跨云的真正隔离环境

#### Skills 评价设计

- **结构化评价**：从文本评价改为量化指标评价
- **评价对象明确**：评价给 Agent 看，不是给人看
- **新评价参数**：success, duration_ms, error_type, tags

#### MVP 分阶段

- **MVP 1-3**：Agent 侧核心（不含管理平台）
- **MVP 4**：管理平台（后续迭代）
- **验收标准细化**：Skills 复用率 > 30%

### 文档更新

- `docs/DESIGN.md` - 完全重写，反映新理解
- `docs/MVP.md` - MVP 阶段重新划分
- `docs/ARCHITECTURE.md` - 技术架构细化
- `setup/setup.md` - Agent 工作流更新
- `CLAUDE.md` - 上下文更新
- `README.md` - 重新组织

### 术语更新

| 旧术语 | 新术语 | 说明 |
|--------|--------|------|
| 协作 | 共享 | 更准确描述跨隔离环境的 Skills 传递 |
| 文本评价 | 结构化评价 | Agent 可直接使用的量化指标 |

---

## [0.1.0] - 2026-04-20

### 初始版本

- 项目初始化
- 设计文档创建
- MVP 规划
- 基础 Skills 模板（browse, review, qa）
- setup.md 初稿

### 创建的文件

```
docs/
├── DESIGN.md
├── ARCHITECTURE.md
└── MVP.md

skills/
├── _templates/skill-template/
├── browse/
├── review/
└── qa/

setup/
└── setup.md
```
