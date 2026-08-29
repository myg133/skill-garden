# AionHive - 架构文档

> **版本**: v0.3.0
> **更新日期**: 2026-04-20
> **状态**: 工程评审完成

---

## 1. 系统概述

### 1.1 在 ClawPool 生态中的位置

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
│       │   跨容器/跨服务器/跨云的真正隔离环境              │
│       │            │            │            │                 │
│       └────────────┼────────────┼────────────┘                 │
│                    │                                          │
│                    ▼                                          │
│         ┌─────────────────────────┐                          │
│         │     AionHive           │                          │
│         │   (企业级 Skills 平台)   │                          │
│         │                         │                          │
│         │  Agent 1 的 Skills ───┼──→ Agent 2/3/N 可使用      │
│         │  Agent 2 的 Skills ───┼──→ Agent 1/3/N 可使用      │
│         │  Agent 3 的 Skills ───┼──→ Agent 1/2/N 可使用      │
│         │                         │                          │
│         │  ← 贡献 Skills  │  消费 Skills →                   │
│         └─────────────────────────┘                          │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 核心组件

| 组件 | 职责 | MVP 阶段 |
|------|------|----------|
| MCP Server (Rust) | 提供 Skills 访问协议，处理 SSE 和 streamable-http | MVP 1-3 |
| Tantivy Index | 全文搜索索引 | MVP 1-3 |
| Registry Service | Skills 元数据存储和管理 | MVP 1-3 |
| Evaluator Service | 收集评价、计算置信度权重 | MVP 2-3 |
| Svelte Frontend | Admin Dashboard（可选，MVP 4） | MVP 4 |

### 1.3 技术栈

| 组件 | 技术 | 版本 |
|------|------|------|
| 后端 | Rust | 1.70+ |
| Web 框架 | Axum | 0.7 |
| MCP 协议 | 官方 Rust crate | 支持 SSE + streamable-http |
| 序列化 | serde + serde_json | 1.x |
| 全文搜索 | Tantivy + tantivy-tokenizer | 0.22 |
| 文件锁 | fs2 | 0.4 |
| 日志 | tracing + tracing-subscriber | 0.1 / 0.3 |
| 错误处理 | anyhow + thiserror | 1 |
| 时间 | chrono | 0.4 |
| 前端 | Svelte | (MVP 4) |

---

## 2. 架构设计

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│              MCP Server (Rust + Axum)                            │
│                                                                 │
│  Transport Layer:                                                │
│  ┌──────────────────────┐  ┌─────────────────────────┐        │
│  │   streamable-http    │  │         SSE             │        │
│  │      (主要协议)       │  │    (Server-Sent Events) │        │
│  └──────────┬───────────┘  └───────────┬─────────────┘        │
│             │                            │                       │
│             └─────────┬──────────────────┘                       │
│                       ▼                                          │
│  ┌───────────────────────────────────────────────────────┐      │
│  │              MCP Protocol Handler                      │      │
│  │                                                       │      │
│  │  Tools:                                               │      │
│  │  ├── skills_search, skills_list, skills_info          │      │
│  │  ├── skills_install (下载)                           │      │
│  │  ├── skills_create, skills_update, skills_delete       │      │
│  │  ├── evaluate_skill, skills_stats                     │      │
│  │  └── health_check                                     │      │
│  └───────────────────────┬───────────────────────────────┘      │
│                          │                                       │
│  ┌──────────────────────▼───────────────────────────────┐      │
│  │                  Business Logic                       │      │
│  │                                                       │      │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐   │      │
│  │  │ Registry   │  │ Evaluator  │  │  Search    │   │      │
│  │  │ Service    │  │ Service    │  │  Service   │   │      │
│  │  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘   │      │
│  │        │               │               │           │      │
│  └────────┼───────────────┼───────────────┼───────────┘      │
│           │               │               │                   │
│  ┌────────▼───────────────▼───────────────▼───────────┐      │
│  │                   Data Layer                        │      │
│  │                                                       │      │
│  │  ┌────────────────┐  ┌────────────────────────┐     │      │
│  │  │ skills/        │  │  data/                 │     │      │
│  │  │ (源码)          │  │  ├── registry/         │     │      │
│  │  │                │  │  │   └── skills-index.json│    │      │
│  │  │                │  │  └── evaluations/        │     │      │
│  │  │                │  │      └── {skill-id}.json│     │      │
│  │  └────────────────┘  └────────────────────────┘     │      │
│  │                                                       │      │
│  │  ┌────────────────┐  ┌────────────────────────┐     │      │
│  │  │  Tantivy Index │  │  Lock Files            │     │      │
│  │  │  (搜索索引)     │  │  (.lock-{skill-id})    │     │      │
│  │  └────────────────┘  └────────────────────────┘     │      │
│  └───────────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 端点设计

```
POST /mcp         → streamable-http (主要协议)
GET  /mcp         → SSE (备选/实时推送)
GET  /health      → 健康检查
```

### 2.3 目录结构

```
aion-hive/
├── src/
│   ├── main.rs              # 入口
│   ├── lib.rs               # 库入口
│   ├── mcp/
│   │   ├── mod.rs           # MCP 模块
│   │   ├── server.rs        # MCP Server 实现
│   │   └── tools.rs         # MCP Tools 定义
│   ├── services/
│   │   ├── mod.rs
│   │   ├── registry.rs      # Skills 注册服务
│   │   ├── evaluator.rs     # 评价服务
│   │   ├── search.rs        # 搜索服务 (Tantivy)
│   │   └── storage.rs       # 文件存储服务
│   ├── models/
│   │   ├── mod.rs
│   │   ├── skill.rs         # Skill 模型
│   │   ├── evaluation.rs    # Evaluation 模型
│   │   └── error.rs         # 错误类型
│   ├── schemas/
│   │   ├── mod.rs
│   │   └── validation.rs    # 数据验证
│   └── utils/
│       ├── mod.rs
│       ├── rate_limiter.rs   # 限流
│       └── weight.rs         # 置信度权重
├── skills/                   # Skills 源码（版本控制）
│   ├── browse/
│   │   └── SKILL.md
│   ├── review/
│   │   └── SKILL.md
│   └── qa/
│       └── SKILL.md
├── data/                     # 运行时数据（不在版本控制）
│   ├── registry/
│   │   └── skills-index.json
│   └── evaluations/
│       └── {skill-id}.json
├── tests/
│   └── integration.sh        # 集成测试脚本
├── Cargo.toml
├── README.md
├── CHANGELOG.md
├── VERSION
├── LICENSE
└── .gitignore
```

---

## 3. 数据模型

### 3.1 Skill 模型

**文件**: `skills/{name}/SKILL.md`

```yaml
---
name: skill-name
description: Brief description (Agent-parseable)
tags: [tag1, tag2]
version: 1.0.0
author_agent_id: agent-xxx
created: 2026-04-20
updated: 2026-04-20
compatibility: ">=1.0.0"
dependencies: []
---

# Skill Body (Markdown)
```

**Rust 结构体**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,                    // "skill-{name}-{version}"
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub version: String,               // semver
    pub author_agent_id: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub compatibility: String,         // e.g., ">=1.0.0"
    pub dependencies: Vec<String>,
    pub content: String,               // SKILL.md body
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub version: String,
    pub author_agent_id: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub install_count: u32,
}
```

### 3.2 Evaluation 模型

**文件**: `data/evaluations/{skill-id}.json`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub id: String,
    pub skill_id: String,
    pub agent_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error_type: Option<ErrorType>,
    pub tags: Vec<EvalTag>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    Timeout,
    Crash,
    LogicError,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvalTag {
    Reliable,
    Fast,
    Stable,
    Experimental,
}
```

### 3.3 SkillStats 模型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStats {
    pub skill_id: String,
    pub success_rate: f64,           // 加权成功率 (0-1)
    pub avg_duration_ms: u64,        // 加权平均执行时间
    pub total_evaluations: u32,
    pub unique_agents: u32,         // 评价过的唯一 Agent 数
    pub confidence: f64,            // 置信度 (0-1)
    pub tags: Vec<String>,          // 聚合后的高频标签
    pub local_version: Option<String>,  // Agent 本地版本 (如果有)
    pub latest_version: String,     // 最新版本
    pub upgrade_available: bool,    // 是否有新版本
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    Low,      // < 3 评价 or 加权总分 < 0.5
    Medium,   // 3-10 评价
    High,     // > 10 评价，一致性好
}
```

---

## 4. API 设计

### 4.1 统一响应格式

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCode {
    // 通用
    Unknown,
    InternalError,

    // Skill 相关
    SkillNotFound,
    SkillAlreadyExists,
    SkillInstallFailed,
    SkillCreateFailed,
    SkillUpdateFailed,
    SkillInvalidFormat,
    SkillTooLarge,
    MaliciousContent,
    InvalidSkillName,
    TooManyTags,

    // Evaluation 相关
    EvaluationInvalid,
    EvaluationRateLimited,

    // Storage 相关
    RegistryReadFailed,
    RegistryWriteFailed,
    RegistryLockFailed,
    FileNotFound,

    // 验证相关
    ValidationError,
    InvalidVersion,
}
```

### 4.2 MCP Tools

#### 4.2.1 消费型 Tools

**health_check**
```rust
fn health_check() -> ApiResponse<HealthStatus>

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,          // "ok" | "degraded" | "error"
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub skills_count: u32,
}
```

**skills_search**
```rust
fn skills_search(query: String, tags: Vec<String>, limit: u32) -> ApiResponse<Vec<SkillMetadata>>
```

**skills_list**
```rust
fn skills_list() -> ApiResponse<Vec<SkillMetadata>>
```

**skills_info**
```rust
fn skills_info(skill_id: String) -> ApiResponse<SkillDetail>

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillDetail {
    pub metadata: SkillMetadata,
    pub content: String,         // SKILL.md 完整内容
    pub stats: Option<SkillStats>,
}
```

**skills_install**
```rust
fn skills_install(skill_id: String) -> ApiResponse<InstallResult>

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub skill_id: String,
    pub local_path: String,      // 本地保存路径
    pub content: String,        // SKILL.md 内容供 Agent 保存
}
```

#### 4.2.2 贡献型 Tools

**skills_create**
```rust
fn skills_create(
    name: String,
    description: String,
    tags: Vec<String>,
    content: String,            // SKILL.md 内容
) -> ApiResponse<Skill>
```

**skills_update**
```rust
fn skills_update(
    skill_id: String,
    updates: SkillUpdate,
) -> ApiResponse<Skill>

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}
```

**skills_delete**
```rust
fn skills_delete(skill_id: String) -> ApiResponse<()>
```

**evaluate_skill**
```rust
fn evaluate_skill(
    skill_id: String,
    success: bool,
    duration_ms: u64,
    error_type: Option<ErrorType>,
    tags: Vec<EvalTag>,
) -> ApiResponse<EvaluationResult>

#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub success: bool,
    pub evaluation_id: String,
    pub new_stats: SkillStats,
}
```

**skills_stats**
```rust
fn skills_stats(skill_id: String) -> ApiResponse<SkillStats>
```

---

## 5. 核心服务设计

### 5.1 Registry Service

**职责**: Skills 的 CRUD 操作

```rust
pub struct RegistryService {
    base_path: PathBuf,          // skills/ 目录
    data_path: PathBuf,          // data/registry/ 目录
}

impl RegistryService {
    // 列出所有 Skills
    pub fn list_skills(&self) -> Result<Vec<SkillMetadata>>;

    // 获取 Skill 详情
    pub fn get_skill(&self, id: &str) -> Result<Skill>;

    // 检查 Skill 是否存在
    pub fn exists(&self, name: &str, author_id: &str) -> bool;

    // 创建 Skill（带文件锁）
    pub fn create_skill(&self, skill: NewSkill, author_id: &str) -> Result<Skill>;

    // 更新 Skill（带文件锁）
    pub fn update_skill(&self, id: &str, updates: SkillUpdate, author_id: &str) -> Result<Skill>;

    // 删除 Skill
    pub fn delete_skill(&self, id: &str) -> Result<()>;
}
```

### 5.2 Evaluator Service

**职责**: 评价收集、置信度计算

```rust
pub struct EvaluatorService {
    data_path: PathBuf,          // data/evaluations/ 目录
    rate_limiter: RateLimiter,
    weight_calculator: WeightCalculator,
}

impl EvaluatorService {
    // 提交评价（带限流）
    pub fn evaluate(&self, eval: Evaluation) -> Result<EvaluationResult>;

    // 获取统计信息（加权）
    pub fn get_stats(&self, skill_id: &str) -> Result<SkillStats>;

    // 获取评价历史
    pub fn get_evaluations(&self, skill_id: &str, limit: u32) -> Result<Vec<Evaluation>>;
}
```

### 5.3 Search Service

**职责**: Tantivy 索引管理

```rust
pub struct SearchService {
    index: Index,
    schema: Schema,
}

impl SearchService {
    // 启动时重建索引
    pub fn rebuild_index(&self, skills: &[Skill]) -> Result<()>;

    // 增量添加索引
    pub fn index_skill(&self, skill: &Skill) -> Result<()>;

    // 增量更新索引（删除旧 + 添加新）
    pub fn reindex_skill(&self, skill: &Skill) -> Result<()>;

    // 搜索
    pub fn search(&self, query: &str, tags: &[String], limit: u32) -> Result<Vec<SkillMetadata>>;
}
```

### 5.4 Storage Service

**职责**: 文件系统操作、目录初始化

```rust
pub struct StorageService;

impl StorageService {
    // 启动时初始化目录
    pub fn ensure_directories(&self) -> Result<()>;

    // 读取 JSON 文件
    pub fn read_json<T: DeserializeOwned>(&self, path: &Path) -> Result<T>;

    // 写入 JSON 文件（原子操作）
    pub fn write_json<T: Serialize>(&self, path: &Path, data: &T) -> Result<()>;

    // 文件锁
    pub fn with_lock<F, T>(&self, path: &Path, f: F) -> Result<T>
    where F: FnOnce() -> Result<T>;
}
```

---

## 6. 关键机制

### 6.1 文件锁机制

**用途**: 防止并发写入冲突

```rust
use fs2::FileExt;

pub fn with_file_lock<F, T>(lock_path: &Path, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let lock_file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(lock_path)?;

    lock_file.lock_exclusive()?;

    let result = f();

    lock_file.unlock()?;

    result
}
```

### 6.2 置信度权重机制

**核心**: 评价数据的可信度根据多个因素动态调整

```rust
pub struct EvaluationWeight {
    base_weight: f64,           // 基础权重 1.0

    // 加分项
    has_success_history: bool,  // 该 Agent 之前对该 Skill 有成功评价
    recent: bool,              // 最近 24 小时内
    matches_majority: bool,     // 与大多数评价一致

    // 减分项
    is_singleton: bool,         // 唯一评价（孤证）
    too_fast: bool,            // 执行时间 < 1s（可能未真实使用）
    too_slow: bool,            // 执行时间 > 10x 平均值（异常）
}

pub fn calculate_weight(eval: &Evaluation, context: &EvalContext) -> f64 {
    let mut weight = 1.0;

    // 加分
    if context.has_success_history { weight += 0.2; }
    if context.is_recent { weight += 0.1; }
    if context.matches_majority { weight += 0.3; }

    // 减分
    if context.is_singleton { weight -= 0.5; }
    if context.too_fast { weight -= 0.3; }
    if context.too_slow { weight -= 0.2; }

    weight.max(0.1) // 最低权重 0.1
}
```

### 6.3 限流机制

**用途**: 防止 Agent 刷评价数据

```rust
pub struct RateLimiter {
    data: Mutex<HashMap<String, (u32, u64)>>, // key -> (count, reset_time)
}

impl RateLimiter {
    // 每个 Agent 对同一 Skill 每天最多 10 条评价
    pub fn check(&self, agent_id: &str, skill_id: &str) -> bool;

    const MAX_PER_DAY: u32 = 10;
    const WINDOW_SECS: u64 = 86400; // 24 hours
}
```

### 6.4 安全验证

**用途**: 防止恶意内容和大文件

```rust
const MAX_SKILL_SIZE: usize = 1_000_000; // 1MB
const MAX_NAME_LENGTH: usize = 100;
const MAX_TAG_COUNT: usize = 10;

pub fn validate_skill(name: &str, tags: &[String], content: &str) -> Result<()> {
    // 大小限制
    if content.len() > MAX_SKILL_SIZE {
        return Err(Error::SkillTooLarge(content.len()));
    }

    // 名称验证
    if name.is_empty() || name.len() > MAX_NAME_LENGTH {
        return Err(Error::InvalidSkillName);
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(Error::InvalidSkillName);
    }

    // 标签限制
    if tags.len() > MAX_TAG_COUNT {
        return Err(Error::TooManyTags);
    }

    // 恶意内容检测
    let malicious_patterns = [
        "<script", "javascript:", "onerror=", "onclick=",
        "/etc/passwd", "C:\\Windows", "..\\", "../",
    ];
    let lower = content.to_lowercase();
    for pattern in malicious_patterns {
        if lower.contains(pattern) {
            return Err(Error::MaliciousContent);
        }
    }

    Ok(())
}
```

### 6.5 索引一致性

**策略**: 两阶段提交 + 启动自愈

```rust
// 创建 Skill 流程:
// 1. 验证内容
// 2. 获取文件锁
// 3. 写入临时文件
// 4. 原子 rename
// 5. 更新 skills-index.json
// 6. 更新 Tantivy 索引
// 7. 释放锁

// 启动时自愈:
pub fn startup_reconcile(registry: &RegistryService, search: &SearchService) {
    let files = registry.list_all_skills();
    let indexed = search.list_indexed_ids();

    // 恢复丢失的索引
    for skill in files.difference(&indexed) {
        search.index_skill(skill);
    }

    // 清理孤立索引
    for id in indexed.difference(&files) {
        search.remove_from_index(id);
    }
}
```

---

## 7. 数据流

### 7.1 创建 Skill 流程

```
Agent                              MCP Server                      File System / Tantivy
 │                                     │                                │
 │── skills_create(...) ──────────────>│                                │
 │                                     │                                │
 │                                     │── 验证内容 ────────────────────>│
 │                                     │   (大小、名称、恶意内容)        │
 │                                     │                                │
 │                                     │── 获取文件锁 ─────────────────>│
 │                                     │   .lock-{name}                │
 │                                     │                                │
 │                                     │── 检查冲突 ────────────────────>│
 │                                     │                                │
 │                                     │── 写入 SKILL.md ───────────────>│
 │                                     │                                │
 │                                     │── 更新 skills-index.json ─────>│
 │                                     │                                │
 │                                     │── 增量索引到 Tantivy ─────────>│
 │                                     │                                │
 │                                     │── 释放锁 ─────────────────────>│
 │                                     │                                │
 │<── { success: true, skill: {...} } ─│                                │
```

### 7.2 搜索 Skill 流程

```
Agent                              MCP Server                      Tantivy Index
 │                                     │                                │
 │── skills_search("browse") ──────────>│                                │
 │                                     │                                │
 │                                     │── 构建查询 ─────────────────────>│
 │                                     │   name: browse                 │
 │                                     │   description: browse          │
 │                                     │   tags: browse                 │
 │                                     │   content: browse (全文)        │
 │                                     │                                │
 │                                     │── 执行搜索 ─────────────────────>│
 │                                     │<── TopDocs ─────────────────────│
 │                                     │                                │
 │<── { skills: [...] } ───────────────│                                │
```

### 7.3 安装 Skill 流程

```
Agent                              MCP Server                      File System
 │                                     │                                │
 │── skills_install("browse-v1.0.0") ─>│                                │
 │                                     │                                │
 │                                     │── 读取 SKILL.md ───────────────>│
 │                                     │<── 内容 ────────────────────────│
 │                                     │                                │
 │                                     │── 更新 install_count ─────────>│
 │                                     │                                │
 │<── { success: true, content: ... } ─│                                │
 │                                     │                                │
 │── Agent 保存到本地 ────────────────────────────────────────────────────>│
```

### 7.4 评价 Skill 流程

```
Agent                              MCP Server                      File System
 │                                     │                                │
 │── evaluate_skill(...) ─────────────>│                                │
 │                                     │                                │
 │                                     │── 限流检查 ────────────────────>│
 │                                     │                                │
 │                                     │── 获取写锁 ───────────────────>│
 │                                     │                                │
 │                                     │── 读取现有评价 ────────────────>│
 │                                     │<── evaluations.json ───────────│
 │                                     │                                │
 │                                     │── 计算新统计 ──────────────────│
 │                                     │   (加权成功率、平均时间、置信度) │
 │                                     │                                │
 │                                     │── 追加评价 ────────────────────│
 │                                     │                                │
 │                                     │── 写回 ───────────────────────>│
 │                                     │                                │
 │                                     │── 释放锁 ─────────────────────>│
 │                                     │                                │
 │<── { success: true, stats: {...} } ─│                                │
```

---

## 8. 存储设计

### 8.1 skills-index.json

```json
{
  "version": "1.0",
  "skills": [
    {
      "id": "skill-browse-v1.0.0",
      "name": "browse",
      "description": "Web browsing skill",
      "tags": ["web", "scrape"],
      "version": "1.0.0",
      "author_agent_id": "agent-abc123",
      "created": "2026-04-20T10:00:00Z",
      "updated": "2026-04-20T10:00:00Z",
      "install_count": 5
    }
  ]
}
```

### 8.2 evaluation 文件

**文件**: `data/evaluations/{skill-id}.json`

```json
{
  "skill_id": "skill-browse-v1.0.0",
  "evaluations": [
    {
      "id": "eval-uuid-1",
      "agent_id": "agent-xyz",
      "success": true,
      "duration_ms": 1234,
      "tags": ["reliable", "fast"],
      "timestamp": "2026-04-20T11:00:00Z"
    },
    {
      "id": "eval-uuid-2",
      "agent_id": "agent-abc",
      "success": false,
      "duration_ms": 5000,
      "error_type": "timeout",
      "timestamp": "2026-04-20T11:30:00Z"
    }
  ]
}
```

### 8.3 Tantivy Schema

```rust
let mut schema_builder = Schema::builder();
schema_builder.add_text_field("id", STRING | STORED);
schema_builder.add_text_field("name", TEXT | STORED);
schema_builder.add_text_field("description", TEXT | STORED);
schema_builder.add_text_field("tags", TEXT | STORED);
schema_builder.add_text_field("content", TEXT); // 全文检索，不存储
schema_builder.add_u64_field("install_count", INDEXED | STORED);
let schema = schema_builder.build();
```

---

## 9. 错误处理

### 9.1 错误响应示例

```json
{
  "success": false,
  "error": {
    "code": "SKILL_NOT_FOUND",
    "message": "Skill 'unknown-skill-v1.0.0' not found",
    "details": {
      "skill_id": "unknown-skill-v1.0.0"
    }
  }
}
```

### 9.2 错误码映射

| ErrorCode | HTTP Status | 说明 |
|-----------|-------------|------|
| Unknown | 500 | 未知错误 |
| InternalError | 500 | 内部错误 |
| SkillNotFound | 404 | Skill 不存在 |
| SkillAlreadyExists | 409 | Skill 已存在（同名）|
| SkillTooLarge | 413 | 内容超过 1MB |
| MaliciousContent | 400 | 恶意内容 |
| InvalidSkillName | 400 | 无效的名称 |
| EvaluationRateLimited | 429 | 评价超限 |
| RegistryLockFailed | 503 | 文件锁获取失败 |
| ValidationError | 400 | 数据验证失败 |

---

## 10. 部署

### 10.1 环境变量

```bash
# 服务配置
AION_HIVE_HOST=0.0.0.0
AION_HIVE_PORT=3000

# 路径配置
AION_HIVE_DATA_DIR=./data
AION_HIVE_SKILLS_DIR=./skills

# 日志
RUST_LOG=info
```

### 10.2 健康检查

```bash
curl http://localhost:3000/health
```

**响应**:
```json
{
  "status": "ok",
  "version": "0.3.0",
  "timestamp": "2026-04-20T12:00:00Z",
  "skills_count": 3
}
```

### 10.3 目录初始化

启动时自动创建：
- `data/registry/`
- `data/evaluations/`
- `data/registry/skills-index.json`（如不存在）

---

## 11. 未来扩展 (MVP 4+)

- [ ] Admin Dashboard (Svelte)
- [ ] REST API for Admin
- [ ] SQLite 存储（替代 JSON）
- [ ] 多租户权限
- [ ] Skills 版本历史
- [ ] Skills 依赖解析
- [ ] Webhook 通知

---

## 12. 参考

- [MCP Protocol Specification](https://modelcontextprotocol.io)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Tantivy Documentation](https://docs.rs/tantivy)
- [Axum Documentation](https://docs.rs/axum)
