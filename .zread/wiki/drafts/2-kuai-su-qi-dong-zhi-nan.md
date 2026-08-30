本文将引导你在本地环境中完整搭建 AionHive（SkillGarden）平台，包含 Rust 后端服务、Svelte 管理后台和 CLI 命令行工具三个核心组件。阅读本文前，建议先了解[项目概述与核心价值](1-xiang-mu-gai-shu-yu-he-xin-jie-zhi)，对平台的整体定位形成基本认知。完成本指南后，你将拥有一个可运行的本机开发环境，能够注册 Skills、通过管理后台审核、并通过 CLI 搜索和安装。

---

## 系统概览

AionHive 采用**三组件分离架构**：Rust 后端作为核心服务层，暴露 REST API 和 MCP 协议接口；Svelte 管理后台面向管理员和普通用户，提供 Web 管理界面；CLI 命令行工具面向 Agent 终端用户，实现 Skills 搜索、安装和评价。三者的关系如下图所示：

```mermaid
flowchart LR
    subgraph 用户端
        CLI["skill-garden CLI\n（Rust 二进制）"]
        Admin["管理后台\n（Svelte SPA）"]
    end

    subgraph 服务端["AionHive Server（Rust + Axum）"]
        API["REST API\n/api/v1/*"]
        MCP["MCP JSON-RPC\n/mcp"]
        SSE["SSE 实时通信\n/sse"]
        DB[("PostgreSQL\n数据库")]
        FS[("本地文件系统\nSkills / 索引")]
    end

    CLI -->|HTTP| MCP
    Admin -->|HTTP| API
    API --> DB
    MCP --> DB
    API --> FS
    MCP --> FS
```

Sources: [main.rs](src/main.rs#L1-L30), [Cargo.toml](Cargo.toml#L1-L10)

---

## 前提条件

在开始安装之前，请确保本地环境已安装以下依赖：

| 依赖 | 版本要求 | 用途 |
|------|---------|------|
| **Rust** | 1.70+（推荐使用 rustup 安装） | 编译后端服务与 CLI 工具 |
| **PostgreSQL** | 14+ | 主数据存储，Skills、身份、权限等全部持久化数据 |
| **Node.js** | 18+ | 运行 Svelte 管理后台的开发服务器与构建 |
| **npm** | 9+ | 管理前端依赖 |
| **Docker**（可选） | 最新版 | Sandbox 容器隔离执行，仅在需要沙箱功能时安装 |

**Windows 环境特别说明**：本仓库在 Windows 上经过测试，PostgreSQL 可通过 [EDB 安装包](https://www.enterprisedb.com/downloads/postgresql) 或 `winget install PostgreSQL.PostgreSQL` 安装。Rust 建议使用 [rustup.rs](https://rustup.rs/) 安装，它会自动处理 Visual Studio Build Tools 依赖。

Sources: [Cargo.toml](Cargo.toml#L9-L10), [admin/package.json](admin/package.json#L1-L24)

---

## 第一步：获取代码与环境配置

### 克隆仓库

将项目克隆到本地工作目录：

```bash
git clone https://github.com/aionui/aion-hive.git
cd aion-hive
```

### 配置环境变量

项目根目录包含 `.env.example` 文件，其中列出了所有可配置的环境变量。请复制该文件为 `.env` 并根据本地环境修改：

```bash
copy .env.example .env    # Windows
# 或
cp .env.example .env      # Linux / macOS
```

关键配置项说明如下：

| 环境变量 | 默认值 | 说明 |
|---------|--------|------|
| `DATABASE_URL` | `postgres://postgres:password@localhost:5432/aionhive` | PostgreSQL 连接字符串 |
| `AION_HIVE_HTTP_PORT` | `8080` | HTTP 服务监听端口 |
| `AION_HIVE_DATA_DIR` | `./data` | 数据存储目录（Skills 文件、搜索索引、评价数据） |
| `AION_HIVE_JWT_SECRET` | `change_this_secret_in_production` | JWT 签名密钥，**生产环境务必修改** |
| `AION_HIVE_JWT_EXPIRY_HOURS` | `24` | JWT 令牌过期时间（小时） |
| `AION_HIVE_PUBLIC_URL` | `http://localhost:8080` | 对外公开的下载链接基础 URL |
| `AION_HIVE_CLI_ENCRYPTION_KEY` | （示例值） | CLI 配置中 API Key 的加密密钥（32 字节 hex） |

**最低配置改动**：只需确保 `DATABASE_URL` 指向你的本地 PostgreSQL 实例即可启动。初次启动时，服务会自动运行数据库迁移（共 40 个迁移文件），无需手动导入 SQL。

Sources: [.env.example](.env.example#L1-L74), [src/db/migrations.rs](src/db/migrations.rs#L178-L238)

---

## 第二步：启动 PostgreSQL 并创建数据库

确保 PostgreSQL 服务正在运行，然后创建目标数据库：

```bash
# 通过 psql 创建数据库
psql -U postgres -c "CREATE DATABASE aionhive;"
```

如果使用 Docker，可以快速启动一个 PostgreSQL 实例：

```bash
docker run -d \
  --name aionhive-pg \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=aionhive \
  -p 5432:5432 \
  postgres:16
```

> 数据库迁移是**自动执行**的，无需手动运行任何 SQL 文件。服务启动时 `AppState::new()` 会调用 `db::migrations::run_migrations()`，按顺序应用 `src/db/migrations/001_initial_schema.sql` 到 `040_remove_market_admin_tenant_read.sql` 中的所有迁移。已执行的迁移记录在 `_migrations` 表中，重复启动不会重复执行。
> 
> Sources: [src/lib.rs](src/lib.rs#L137-L147), [src/db/migrations.rs](src/db/migrations.rs#L178-L238)

---

## 第三步：启动 Rust 后端服务

### 编译并运行

后端以 `server` 为默认 feature，直接使用 Cargo 即可编译运行：

```bash
# 编译并启动（默认使用 server feature）
cargo run --release
```

首次编译会下载所有依赖（包括 axum、sqlx、tantivy、bollard 等），可能需要 3-5 分钟。编译完成后，控制台将输出类似以下信息：

```
2025-01-01T12:00:00.000Z INFO  AionHive v0.3.0
2025-01-01T12:00:00.100Z INFO  AionHive initialized successfully
2025-01-01T12:00:00.100Z INFO  Registry: 0 skills
2025-01-01T12:00:00.100Z INFO  Starting MCP server with streamable-http + SSE transport on port 8080
2025-01-01T12:00:00.100Z INFO  Starting HTTP server on http://0.0.0.0:8080
```

### 验证服务状态

服务启动后，通过健康检查端点验证：

```bash
curl http://localhost:8080/health
```

预期返回：

```json
{"status":"OK","version":"0.3.0","skills_count":0}
```

### 服务启动时自动完成的工作

`main.rs` 中的初始化流程按顺序执行以下操作：

1. 加载 `.env` 环境变量（`dotenvy::dotenv()`）
2. 初始化日志系统（`tracing-subscriber`，支持 `RUST_LOG` 环境变量控制日志级别）
3. 创建数据目录（`data/registry`、`data/evaluations`、`data/search_index`）
4. 连接 PostgreSQL 并自动运行所有未应用的迁移
5. 初始化各个服务（Registry、Search、Evaluator、Session、Sandbox 等）
6. 如果搜索索引为空，自动从数据库全量重建索引
7. 启动 HTTP 监听，绑定路由（REST API + MCP + SSE）
8. 启动后台任务：SSE 会话清理（每 60 秒）和数据库会话清理（每 120 秒）

Sources: [src/main.rs](src/main.rs#L351-L399), [src/main.rs](src/main.rs#L215-L349), [src/lib.rs](src/lib.rs#L137-L200)

---

## 第四步：启动管理后台（Svelte）

管理后台是一个独立的 Svelte SPA 项目，位于 `admin/` 目录下。它通过 Vite 开发服务器运行，并自动将 `/api/*` 请求代理到后端。

### 安装依赖

```bash
cd admin
npm install
```

### 启动开发服务器

```bash
npm run dev
```

Vite 开发服务器默认在 `http://localhost:5174` 启动。Vite 配置中已设置代理规则，所有 `/api/*` 请求会被转发到 `http://localhost:8080`（即后端服务地址），因此前端开发时无需处理跨域问题。

Sources: [admin/vite.config.js](admin/vite.config.js#L1-L24), [admin/package.json](admin/package.json#L1-L24)

### 注册管理员账号

管理后台启动后，打开浏览器访问 `http://localhost:5174`。首次使用需要注册管理员账号：

1. 点击 **Register** 进入注册页面
2. 填写用户名和密码
3. 注册成功后，系统会自动分配 `system:admin` 角色，获得完整的系统管理权限

> 注册流程对应的 API 路由为 `POST /api/v1/auth/register`，由 `user_register_handler` 处理。在管理后台中，注册表单通过 `api.adminLogin` 或 `api.userRegister` 调用后端接口。
> 
> Sources: [src/api/routes.rs](src/api/routes.rs#L104-L106), [admin/src/lib/api.js](admin/src/lib/api.js#L186-L200)

### 管理后台功能概览

登录后，根据用户权限不同，导航栏会动态展示不同的功能入口。管理员可以看到完整的导航菜单：

| 导航分组 | 包含页面 | 所需权限 |
|---------|---------|---------|
| **Overview** | Dashboard | `system:admin:access` |
| **Users** | Identities, API Keys | `system:admin:access` |
| **Organizations** | Organizations, Groups, Org Tools | `org:read` / `tenant:read` |
| **Content** | Marketplace, Skills, Review, Marketplace Roles | 部分公开 |
| **Account** | My Profile, My API Keys | 公开 |
| **System** | Tenants, System Roles, Sessions, Audit Logs, Settings | 多种权限 |
| **Infrastructure** | Sandboxes | `system:admin:access` |

普通用户（非管理员）则看到简化的用户侧导航：Dashboard、Marketplace、My Skills、Submissions 等。

Sources: [admin/src/config/nav-routes.js](admin/src/config/nav-routes.js#L1-L96)

---

## 第五步：构建并使用 CLI 工具

CLI 命令行工具 `skill-garden` 是面向 Agent 和终端用户的核心工具，支持搜索、安装、评价 Skills。

### 构建 CLI 二进制

```bash
# 使用 cli feature 编译（跳过 server 依赖，编译更快）
cargo build --release --no-default-features --features cli
```

编译产物位于 `target/release/skill-garden.exe`（Windows）或 `target/release/skill-garden`（Linux/macOS）。

### 安装 CLI 到系统路径

项目提供了两种安装方式：

**方式一：手动安装**（推荐开发环境）

```bash
# 将二进制文件复制到 PATH 目录
# Windows
copy target\release\skill-garden.exe C:\Users\<用户名>\.skill-garden\bin\
# 然后将该目录加入 PATH 环境变量

# Linux/macOS
cp target/release/skill-garden /usr/local/bin/
```

**方式二：使用安装脚本**（面向分发场景）

CLI 构建产物目录 `cli-dist/` 中包含了安装脚本，支持 Windows（`install.ps1`）和 Linux/macOS（`install.sh`）。生产环境中，CLI 二进制通过 `POST /api/v1/cli/download/:version/:target` 端点分发，由 `download_cli_handler` 处理。

Sources: [src/bin/cli.rs](src/bin/cli.rs#L1-L213), [cli-dist/install.ps1](cli-dist/install.ps1#L1-L30), [cli-dist/install.sh](cli-dist/install.sh#L1-L29)

### 完整的 CLI 使用流程

```bash
# 1. 登录（使用 API Key）
skill-garden login http://localhost:8080

# 2. 查看当前身份
skill-garden whoami

# 3. 搜索 Skills
skill-garden search "browse"

# 4. 列出所有 Skills
skill-garden list

# 5. 查看 Skill 详情
skill-garden info browse-v1.0.0

# 6. 安装 Skill 到本地
skill-garden install browse-v1.0.0

# 7. 查看版本历史
skill-garden versions browse

# 8. 查看热门 Skills
skill-garden popular --limit 10

# 9. 查看 Skill 统计
skill-garden stats browse-v1.0.0

# 10. 登出（清除本地凭证）
skill-garden logout
```

CLI 的配置存储在 `~/.skill-garden/config.toml`（Linux/macOS）或 `%USERPROFILE%\.skill-garden\config.toml`（Windows），包含服务端地址、API Key 和默认安装目录三项配置。

Sources: [src/cli/config.rs](src/cli/config.rs#L1-L68), [src/cli/commands.rs](src/cli/commands.rs#L1-L347)

### 获取 API Key

API Key 是 CLI 认证的核心凭证，其格式为 `sk_` 前缀。获取方式有两种：

- **管理后台**：登录后进入 Account → My API Keys 页面创建
- **Agent 注册**：通过 `POST /api/v1/auth/agent/register` 接口注册 Agent 后自动获取

API Key 由 `ApiKeyService` 管理，支持创建、撤销、状态更新等操作，后端通过 `AION_HIVE_CLI_ENCRYPTION_KEY` 环境变量决定是否对配置文件中存储的 Key 进行加密。

Sources: [src/api/routes.rs](src/api/routes.rs#L129-L132), [.env.example](.env.example#L29-L33)

---

## 完整启动流程总结

以下流程图展示了从零到可用的完整启动顺序：

```mermaid
flowchart TD
    A[安装 PostgreSQL] --> B[创建数据库 aionhive]
    B --> C[复制 .env.example 为 .env\n并修改 DATABASE_URL]
    C --> D[运行 cargo run --release\n启动后端服务]
    D --> E{服务启动成功？}
    E -->|是| F[验证: curl localhost:8080/health]
    E -->|否| G[检查 PostgreSQL 连接\n检查 .env 配置]
    G --> D
    
    F --> H[cd admin && npm install\n启动管理后台]
    H --> I[打开 localhost:5174\n注册管理员账号]
    
    F --> J[编译 CLI: cargo build --release\n--no-default-features --features cli]
    J --> K[登录 CLI: skill-garden login\nhttp://localhost:8080]
    
    I --> L[完成：可创建 Skill\n并通过管理后台审核]
    K --> M[完成：可搜索和安装 Skills]
```

---

## 下一步阅读

完成本指南后，你的本地开发环境已就绪。建议按以下顺序深入理解各个模块：

- **架构层面**：[整体架构文档](5-zheng-ti-jia-gou-rust-hou-duan-svelte-guan-li-hou-tai-cli-gong-ju-lian) 全面了解各组件间的协作关系
- **环境完善**：[环境变量与密钥配置](3-huan-jing-bian-liang-yu-mi-yao-pei-zhi) 详细了解每个配置项的含义和推荐值
- **数据库初始化**：[PostgreSQL 数据库迁移与初始化](4-postgresql-shu-ju-ku-qian-yi-yu-chu-shi-hua) 深入了解迁移体系
- **核心概念**：[Skill 资产模型](6-skill-zi-chan-mo-xing-sheng-ming-zhou-qi-ban-ben-ke-jian-xing-yu-shi-chang-zhuang-tai) 理解 Skills 从草稿到市场发布的完整生命周期
- **权限体系**：[RBAC 权限模型](8-rbac-quan-xian-mo-xing-system-tenant-org-group-si-ji-jiao-se-ti-xi) 理解多层级权限控制机制
- **CLI 深入**：[CLI 命令行工具](25-cli-ming-ling-xing-gong-ju-sou-suo-an-zhuang-ping-jie-skills) 详细查看每个命令的用法和参数