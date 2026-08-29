# AionHive / SkillGarden

> 企业级 AI Skills 共享平台 — 基于 MCP 协议的多租户 Skill 管理与评价系统

本仓库采用 **Agent Workspace 工作流**（参见 `.pi/skills/agent-workspace-v2/`），
使用 `workspace` 分支作为根容器，**所有 worktree 在仓库根平铺**（无中间目录层）。

## 目录 ↔ 分支对应表

| 路径 | 分支 | 角色 | 说明 |
|------|------|------|------|
| `./` | `workspace` | **根容器** | **只跟踪 README.md + .gitignore**（白名单防御） |
| `code/` | `develop` | CI 主工作区 | Rust/Axum MCP Server + Admin UI；`.gitignore` 由 develop 跟踪 |
| `BA/` | `demand` | BA Agent | 需求管理、迭代计划、Agent 注册；`.gitignore` 由 demand 跟踪 |
| `Deploy/` | `deploy` | Deploy Agent | helm/k8s/部署配置，不做构建；`.gitignore` 由 deploy 跟踪 |
| `feature-REQ-xxx/` | `feature/REQ-xxx` | Dev Agent | 需求开发，按需创建，合并后清理 |
| `hotfix-xxx/` | `hotfix/xxx` | Dev Agent | 紧急修复，按需创建 |

**所有 worktree 跟 `README.md` 同级**，在仓库根平铺，没有中间目录层。

**关于 `.gitignore`**：
- 仓库根 `.gitignore` 使用**双层防御机制**：
  - **白名单层**：`!README.md` + `!.gitignore` + `*` + `!*/`
  - **`/*/` 屏蔽层**：忽略所有第一层子目录（worktree 都在第一层）
- 其他 worktree 各自的 `.gitignore` 由对应工作分支管理，workspace 一概不管

## 快速上手

### 1. 克隆仓库

```bash
git clone git@github.com:myg133/skill-garden.git
cd skill-garden
# 默认在 workspace/ 主 worktree，看到这份 README
```

### 2. 注册 worktree

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

### 4. 开发 & 提交 PR

```bash
cd feature-REQ-001
# 编码、测试
git commit -m "[Dev] {描述} (关联: REQ-001)"
git push -u origin feature/REQ-001
# 在 GitHub 创建 PR → develop
```

## 各角色 Agent 入口

- **BA Agent**（主 agent）→ 进入 `BA/` 目录
- **Dev Agent**（子 agent）→ 进入 `feature-REQ-xxx/` 目录
- **QA Agent**（子 agent）→ Pre-merge 进 `feature-REQ-xxx/`，Post-merge 进 `code/` + staging
- **Deploy Agent**（子 agent）→ 进入 `Deploy/` 目录

## 技术栈

- **后端**: Rust / Axum 0.7 / Tokio / sqlx 0.8
- **数据库**: PostgreSQL
- **搜索**: Tantivy 0.22
- **协议**: MCP (Model Context Protocol) via `rmcp`
- **Admin UI**: Svelte (独立 SPA)

## 硬规则

- ❌ `workspace` 分支**只跟踪 `README.md` + `.gitignore`**
- ❌ 不向 `workspace` 分支提交 PR
- ❌ 不在仓库根加中间目录层
- ❌ 不做 worktree 二级嵌套
- ✅ BA Agent 每次启动时执行 workspace 巡检

## 详细文档

- 工作流规范：`.pi/skills/agent-workspace-v2/SKILL.md`
- 初始化：`.pi/skills/agent-workspace-v2/init/`
- 生命周期：`.pi/skills/agent-workspace-v2/lifecycle/`
- 模板：`.pi/skills/agent-workspace-v2/templates/`
- 开发指南：`code/README.md`
- BA 工作区：`BA/README.md`
- Deploy 工作区：`Deploy/README.md`
