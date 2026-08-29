# AionHive SkillGarden

> MCP 驱动的 AI Agent 技能市场与管理系统，支持多租户 RBAC、审计日志和 API Keys。

本仓库采用 **Agent Workspace 工作流**（参见 `.pi/skills/agent-workspace-v2/`），
使用 `workspace` 分支作为根容器，**所有 worktree 在仓库根平铺**（无中间目录层）。

## 目录 ↔ 分支对应表

| 路径 | 分支 | 角色 | 说明 |
|------|------|------|------|
| `./` | `workspace` | **根容器** | **只跟踪 README.md + .gitignore**（白名单防御） |
| `code/` | `develop` | CI 主工作区 | 主开发分支，CI 构建源；`.gitignore` 由 develop 跟踪 |
| `BA/` | `demand` | BA Agent | 需求管理、迭代计划、Agent 注册；`.gitignore` 由 demand 跟踪 |
| `Deploy/` | `deploy` | Deploy Agent | helm/k8s/部署配置，不做构建；`.gitignore` 由 deploy 跟踪 |
| `feature-REQ-xxx/` | `feature/REQ-xxx` | Dev Agent | 需求开发，按需创建，合并后清理 |
| `hotfix-xxx/` | `hotfix/xxx` | Dev Agent | 紧急修复，按需创建 |

**所有 worktree 跟 `README.md` 同级**，在仓库根平铺，没有中间目录层（如 `workspaces/`）。

**关于 `.gitignore`**：
- 仓库根 `.gitignore` 使用**双层防御机制**：
  - **白名单层**：`!README.md` + `!.gitignore` + `*` + `!*/`
  - **`/*/` 屏蔽层**：忽略所有第一层子目录（worktree 都在第一层）
- 即使误操作 `git add .` 也不会污染 workspace 分支，worktree 完全不被影响
- 其他 worktree 各自的 `.gitignore`（如 `code/.gitignore`）由对应工作分支管理，workspace 一概不管

## 快速上手

### 1. 克隆仓库

```bash
git clone git@github.com:myg133/skill-garden.git
cd skill-garden
# 默认在 workspace/ 主 worktree，看到这份 README
```

### 2. 注册主 worktree（如已部署过可跳过）

```bash
git worktree add code develop
git worktree add BA demand
git worktree add Deploy deploy
```

### 3. 接到需求后

在仓库根跑：

```bash
git worktree add feature-REQ-001 -b feature/REQ-001 develop
```

或在 `BA/` 里跑（用 `../` 回到仓库根，再平铺）：

```bash
cd BA
git worktree add ../feature-REQ-001 -b feature/REQ-001 develop
```

新 worktree 跟 `code/` `BA/` 在仓库根平铺，**没有中间目录层**。

### 4. 提交 PR

在 `feature-REQ-xxx/` 里编码、测试、提交：

```bash
git commit -m "[Dev] {描述} (关联: REQ-xxx)"
git push -u origin feature/REQ-xxx
# 在 GitHub/GitLab 创建 PR → develop
```

PR 合并后 Dev Agent 自动清理 worktree（详见 `.pi/skills/agent-workspace-v2/lifecycle/worktree-cleanup.md`）。

## 各角色 Agent 入口

- **BA Agent**（主 agent）→ 进入 `BA/` 目录
- **Dev Agent**（子 agent）→ 进入 `feature-REQ-xxx/` 目录
- **QA Agent**（子 agent）→ Pre-merge 进 `feature-REQ-xxx/`，Post-merge 进 `code/` + staging
- **Deploy Agent**（子 agent）→ 进入 `Deploy/` 目录

新 agent 启动时必须加载 `.pi/skills/agent-workspace-v2/SKILL.md`。

## 硬规则

- ❌ `workspace` 分支**只跟踪 `README.md` + `.gitignore`**（`git ls-files` 必须只有这两个）
- ✅ `.gitignore` 使用**双层防御**：白名单层 + `/*/` 屏蔽所有第一层子目录
- ❌ 不向 `workspace` 分支提交 PR
- ❌ 不在仓库根加中间目录层（禁止 `workspaces/` `agents/` 等中间层放 worktree）
- ❌ 不做 worktree 二级嵌套（禁止 `feature-xxx/code/`）
- ✅ 各 worktree 自己的 `.gitignore` 由对应 agent 在对应工作分支维护（workspace 不管）
- ✅ BA Agent 每次启动时执行 workspace 巡检

## 详细文档

- 工作流规范：`.pi/skills/agent-workspace-v2/SKILL.md`
- 初始化：`.pi/skills/agent-workspace-v2/init/`
- 生命周期：`.pi/skills/agent-workspace-v2/lifecycle/`
- 模板：`.pi/skills/agent-workspace-v2/templates/`
- 各角色 README：见对应 worktree 根目录（如 `BA/README.md`）

## 项目技术栈

- **语言**: Rust (Tokio async runtime)
- **Web 框架**: Axum 0.7
- **数据库**: PostgreSQL via sqlx 0.8
- **搜索**: Tantivy 0.22 (full-text search)
- **协议**: MCP (Model Context Protocol) via `rmcp`
- **Admin UI**: Svelte app in `admin/`

## 环境变量

| 变量 | 默认 | 说明 |
|------|------|------|
| `AION_HIVE_TRANSPORT` | `stdio` | 传输模式: `stdio` 或 `http` |
| `AION_HIVE_HTTP_PORT` | `8080` | HTTP 服务器端口 |
| `AION_HIVE_DATA_DIR` | `data` | 数据目录 |
| `AION_HIVE_SKILLS_DIR` | `skills` | Skills 资源目录 |
| `DATABASE_URL` | `postgres://localhost:5432/aionhive` | PostgreSQL 连接串 |

## 构建与运行

```bash
# Debug 构建
cargo build

# Release 构建
cargo build --release

# 运行 (stdio 模式，默认)
cargo run

# 运行 (HTTP 模式)
$env:AION_HIVE_TRANSPORT="http"
$env:AION_HIVE_HTTP_PORT="8080"
cargo run

# 运行测试
cargo test --test integration
```
