Skill Garden 是一个三组件架构系统（Rust 后端 + Svelte 管理后台 + CLI 命令行工具），每个组件有独立的构建流程和分发策略。本文档系统性地解析从源码到可交付产物的完整构建管道，以及配套的安装脚本设计。

## 一、整体构建架构概览

项目的构建体系围绕三个核心组件展开，各组件共享同一个版本号（当前 `0.3.0`），但构建入口和依赖特征完全不同：

```mermaid
flowchart TD
    subgraph 源码层
        C["Cargo.toml<br/>(version: 0.3.0)<br/>feature: default = server"]
        A["admin/package.json<br/>(version: 0.1.0)"]
    end

    subgraph 构建产物
        S["target/release/server<br/>(Rust HTTP Server)"]
        CLI["cli-dist/{version}/{os}-{arch}/<br/>skill-garden(.exe)<br/>(CLI 二进制)"]
        AD["admin/dist/<br/>(Svelte 静态文件)"]
    end

    subgraph 分发目录
        CLI_DIST["cli-dist/<br/>├── {version}/<br/>│   ├── windows-x86_64/<br/>│   ├── linux-x86_64/<br/>│   └── macos-aarch64/<br/>├── install.ps1<br/>├── install.sh<br/>├── SKILL.md<br/>└── instructions.md"]
    end

    C -- "cargo build (default features)" --> S
    C -- "cargo build --no-default-features --features cli" --> CLI
    A -- "npm run build (vite)" --> AD
    CLI --> CLI_DIST
```

**构建入口定位**：
- 服务端二进制：`src/main.rs`，通过 `[[bin]] name = "server"` 声明，依赖 `server` feature 组
- CLI 二进制：`src/bin/cli.rs`，通过 `[[bin]] name = "skill-garden"` 声明，依赖 `cli` feature 组
- 管理后台：`admin/` 目录，独立 Svelte + Vite 项目

Sources: [Cargo.toml](Cargo.toml#L1-L118), [src/main.rs](src/main.rs#L1-L10), [src/bin/cli.rs](src/bin/cli.rs#L1-L30)

---

## 二、Rust 后端构建：服务端二进制

### 2.1 默认构建方式

服务端是 `default` feature 集合的产物，使用标准 Rust 构建命令即可：

```bash
cargo build --release
```

因为 `Cargo.toml` 中声明了 `default = ["server"]`，上述命令等价于 `cargo build --release --features server`。`server` feature 聚合了**所有服务端依赖**：axum（HTTP 框架）、sqlx（数据库 ORM）、tantivy（全文索引引擎）、bollard（Docker 沙箱客户端）、rmcp（MCP 协议实现）等数十个重量级 crate。

### 2.2 构建产物配置

`Cargo.toml` 通过 `[[bin]]` 表定义了两个独立二进制入口：

| 二进制名称 | 源文件路径 | 所需 feature | 职责 |
|-----------|-----------|-------------|------|
| `server` | `src/main.rs` | `server` | HTTP 服务端，包含 REST API、SSE、MCP 协议 |
| `skill-garden` | `src/bin/cli.rs` | `cli` | 命令行工具，仅通过 HTTP 与服务端通信 |

**重要约束**：`required-features` 字段确保错误的 feature 组合会导致编译失败。例如，不带 `server` feature 编译 `server` 二进制会报错，防止因缺少 SQLx 等依赖导致运行期崩溃。

### 2.3 启动流程

服务端启动时执行以下初始化序列（定义在 `src/main.rs` 的 `main()` 函数中）：

1. 加载 `.env` 环境变量（通过 `dotenvy`）
2. 初始化日志系统（`tracing_subscriber` + `EnvFilter`）
3. 解析 `AION_HIVE_DATA_DIR` 环境变量（默认 `./data`），创建 `registry/`、`evaluations/`、`search_index/` 子目录
4. 构造 `AppState`：连接 PostgreSQL、执行数据库迁移、初始化所有 Service 层
5. 启动 HTTP 服务器（端口由 `AION_HIVE_HTTP_PORT` 指定，默认 `8080`）
6. 注册路由：`/health`（健康检查）、`/mcp`（JSON-RPC 端点）、`/sse` / `/sse/:session_id`（SSE 实时通信）、以及 `create_api_router()` 生成的全部 REST API 路由
7. 启动后台任务：SSE 会话清理（60 秒间隔）、数据库会话清理（120 秒间隔）

Sources: [Cargo.toml](Cargo.toml#L18-L26), [src/main.rs](src/main.rs#L300-L402), [src/lib.rs](src/lib.rs#L1-L9)

---

## 三、CLI 构建：跨平台二进制分发

CLI 的构建是整个项目中**最复杂、最易出错**的部分，因为必须正确处理 feature 隔离和跨平台交叉编译。

### 3.1 Feature 隔离原则

CLI 编译的**核心约束**：必须使用 `--no-default-features --features cli`，否则 server 组件的所有依赖（sqlx、tantivy、axum 等）会被一并编译，导致：

- 编译时间从 ~30 秒膨胀到 ~5 分钟
- 二进制体积从 ~5 MB 膨胀到 ~50 MB
- 引入不必要的运行时依赖（如 Docker daemon、PostgreSQL 连接）

`src/lib.rs` 通过条件编译模块实现了 feature 隔离：

```rust
// 始终可用的模块
pub mod models;

// cli feature 专用
#[cfg(feature = "cli")]
pub mod cli;

// server feature 专用
#[cfg(feature = "server")]
pub mod api;
#[cfg(feature = "server")]
pub mod db;
#[cfg(feature = "server")]
pub mod mcp;
// ... 更多 server 模块
```

CLI 二进制仅依赖 `cli` feature 下的 `src/cli/` 模块，该模块包含三个子模块：`client`（MCP JSON-RPC 客户端）、`commands`（命令实现逻辑）、`config`（`~/.skill-garden/config.toml` 配置管理）。CLI 通过 `POST /mcp` 端点与服务端通信，使用 JSON-RPC 2.0 协议，因此不依赖任何数据库或服务端基础设施。

Sources: [src/lib.rs](src/lib.rs#L1-L16), [src/cli/mod.rs](src/cli/mod.rs#L1-L9)

### 3.2 跨平台构建脚本

`deploy/` 目录下提供了两个同功能的构建脚本（Windows PowerShell 和 Linux/macOS Bash），支持以下 6 个目标平台：

| 目标名 | Rust triple | 二进制名 | 典型使用场景 |
|--------|-------------|---------|-------------|
| `windows-x86_64` | `x86_64-pc-windows-msvc` | `skill-garden.exe` | Windows 桌面/服务器 |
| `windows-aarch64` | `aarch64-pc-windows-msvc` | `skill-garden.exe` | Windows ARM 设备 |
| `linux-x86_64` | `x86_64-unknown-linux-gnu` | `skill-garden` | Linux 服务器/桌面 |
| `linux-aarch64` | `aarch64-unknown-linux-gnu` | `skill-garden` | ARM Linux（树莓派、AWS Graviton） |
| `macos-x86_64` | `x86_64-apple-darwin` | `skill-garden` | Intel Mac |
| `macos-aarch64` | `aarch64-apple-darwin` | `skill-garden` | Apple Silicon Mac |

构建脚本的自动化流程：

```mermaid
flowchart LR
    A["读取 Cargo.toml<br/>提取 version"] --> B["确定构建目标"]
    B --> C{"指定目标？"}
    C -- 是 --> D["使用指定目标列表"]
    C -- 否 --> E["自动检测当前OS+arch<br/>构建原生目标"]
    D --> F["遍历每个目标"]
    E --> F
    F --> G{"target 已安装？"}
    G -- 否 --> H["rustup target add"]
    H --> I["cargo build --release<br/>--no-default-features<br/>--features cli<br/>--target {triple}"]
    G -- 是 --> I
    I --> J["复制到 cli-dist/{version}/{name}/"]
    J --> K["输出构建摘要"]
```

**关键设计决策**：
- 版本号从 `Cargo.toml` 自动解析，与 `VERSION` 文件（内容为 `0.3.0`）保持一致
- 默认构建原生目标（当前 OS + 架构），避免不必要的交叉编译工具链安装
- `rustup target list --installed` 检测是否已安装目标，缺失则自动安装
- 构建失败不影响其他目标（`continue` 继续处理下一个）
- 输出目录结构为 `cli-dist/{version}/{os}-{arch}/skill-garden(.exe)`，版本号作为目录名便于服务端分发

Sources: [deploy/build-cli.ps1](deploy/build-cli.ps1#L1-L165), [deploy/build-cli.sh](deploy/build-cli.sh#L1-L130), [VERSION](VERSION#L1)

### 3.3 交叉编译限制

**重要**：交叉编译存在平台限制，脚本不自动解决：

| 编译平台 → 目标平台 | 可行性 | 所需额外工具 |
|-------------------|-------|------------|
| Windows → Linux | 需要 MSYS2 + MinGW | `x86_64-unknown-linux-gnu` target |
| Linux → Windows | 需要 mingw-w64 | `x86_64-pc-windows-msvc` target |
| macOS → Linux | 受限 | 需要交叉编译工具链 |
| 非 macOS → macOS | **不可行** | 需要 Apple SDK（Xcode） |

**推荐做法**：在每个平台的原生环境中分别构建各自的 target，或使用 GitHub Actions 多 runner 矩阵编译。当前项目尚未配置 CI workflow，这是可扩充的部署能力之一。

### 3.4 本地开发测试

如果只需要本地验证 CLI 功能，无需完整跨平台构建：

```bash
# 构建当前平台 CLI
cargo build --release --no-default-features --features cli

# 验证
./target/release/skill-garden whoami
```

首次使用需登录或手动配置 `~/.skill-garden/config.toml`：

```toml
server = "https://your-server.com"
token = "sk_xxx"
```

配置文件的完整路径由 `dirs::home_dir()` 确定，位于 `~/.skill-garden/config.toml`。`CliConfig` 结构体通过 `serde` 序列化/反序列化，支持 `load()`、`save()`、`delete()` 三个核心操作。

Sources: [src/cli/config.rs](src/cli/config.rs#L1-L68), [deploy/README.md](deploy/README.md#L1-L146)

---

## 四、管理后台构建：Svelte 前端

### 4.1 构建配置

管理后台是一个独立的 Svelte + Vite 项目，构建配置在 `admin/vite.config.js` 中：

```javascript
export default defineConfig({
  plugins: [svelte()],
  css: {
    postcss: {
      plugins: [tailwindcss(), autoprefixer()],
    },
  },
  server: {
    host: '0.0.0.0',
    port: 5174,
    proxy: {
      '^/api/': {
        target: 'http://localhost:8080',
        changeOrigin: true
      }
    }
  }
});
```

关键配置点：
- **开发服务器**：端口 5174，代理 `/api/` 路径到后端 8080 端口，解决跨域问题
- **CSS 处理**：Tailwind CSS + PostCSS + Autoprefixer，最终编译为纯 CSS
- **构建输出**：`npm run build` 生成 `admin/dist/` 目录，包含静态 HTML/JS/CSS

### 4.2 构建命令

```bash
cd admin
npm install          # 安装依赖（JSZip、svelte-routing 等）
npm run build        # 生产构建 → admin/dist/
npm run dev          # 开发模式（热重载 + API 代理）
```

### 4.3 部署集成

管理后台的构建产物需要与 Rust 服务端集成。有两种部署策略：

| 策略 | 实现方式 | 适用场景 |
|------|---------|---------|
| **反向代理** | Nginx/Caddy 将 `/` 路由到 `admin/dist/`，`/api/` 路由到后端 8080 端口 | 生产环境，性能最佳 |
| **嵌入式静态文件** | Rust 服务端通过 `serve_dir` 中间件提供 `admin/dist/` 内容 | 单进程部署，简化运维 |

当前项目架构中，后端专注于提供 REST API 和 MCP 协议，前端静态文件推荐由反向代理托管。

Sources: [admin/vite.config.js](admin/vite.config.js#L1-L24), [admin/package.json](admin/package.json#L1-L24)

---

## 五、CLI 分发与安装管道

### 5.1 分发目录结构

构建完成的 CLI 二进制存储在 `cli-dist/` 目录中，该目录同时包含安装脚本和技能定义模板：

```
cli-dist/
├── SKILL.md              ← AI Agent 技能定义模板（含 {server_url} 占位符）
├── instructions.md       ← 安装说明模板（含 {version}、{os}、{url} 占位符）
├── install.sh            ← Linux/macOS 安装脚本模板
├── install.ps1           ← Windows 安装脚本模板
└── {version}/            ← 构建产物（版本号目录）
    ├── windows-x86_64/
    │   └── skill-garden.exe
    ├── linux-x86_64/
    │   └── skill-garden
    └── ...（其他平台）
```

**设计要点**：`cli-dist/` 是一个独立目录，不依赖 `AION_HIVE_DATA_DIR` 环境变量。服务端始终从项目根目录的 `cli-dist/` 读取 CLI 二进制文件。这确保了 CLI 分发路径的确定性，无论数据目录配置如何变化。

### 5.2 安装脚本工作机制

服务端提供 `cli.setup` MCP 工具，供 AI Agent 调用以获取适合其平台的 CLI 安装包。安装管道如下：

```mermaid
sequenceDiagram
    participant Agent as AI Agent
    participant Server as Skill Garden 服务端
    participant Files as cli-dist/

    Agent->>Server: cli.setup(platform="linux-x86_64")
    Server->>Files: 读取 cli-dist/{version}/linux-x86_64/skill-garden
    Server->>Files: 读取 install.sh、SKILL.md、config.toml
    Server->>Server: 打包为 tar.gz
    Server-->>Agent: 返回 download_url + 安装指令
    Agent->>Agent: 下载、解压、执行 install.sh
    Agent->>Agent: skill-garden 已加入 PATH
```

**install.sh**（Linux/macOS）的安装逻辑：
1. 检查 `/usr/local/bin` 是否可写
2. 可写 → 创建符号链接（`ln -sf`）到系统目录
3. 不可写 → 复制到 `~/.local/bin/`，并将该目录加入 `~/.bashrc` 的 PATH

**install.ps1**（Windows）的安装逻辑：
1. 检查是否以管理员身份运行
2. 管理员 → 复制到 `C:\Program Files\skill-garden\`，加入系统 PATH
3. 非管理员 → 复制到 `%USERPROFILE%\.skill-garden\bin\`，加入用户 PATH

这两个安装脚本被设计为**模板**，服务端在分发时会将占位符替换为实际值（版本号、平台、服务器 URL、API Key 等）。

Sources: [cli-dist/install.sh](cli-dist/install.sh#L1-L29), [cli-dist/install.ps1](cli-dist/install.ps1#L1-L30), [cli-dist/instructions.md](cli-dist/instructions.md#L1-L18), [cli-dist/SKILL.md](cli-dist/SKILL.md#L1-L88)

### 5.3 安全设计：CLI Token 加密

CLI 配置中存储的 API Key 通过环境变量 `AION_HIVE_CLI_ENCRYPTION_KEY` 进行加密保护。该变量需要 32 字节（64 位十六进制字符串）的密钥，可通过 `openssl rand -hex 32` 生成。如果未设置此密钥，API Key 将以明文形式写入 `config.toml`。

---

## 六、环境变量驱动的部署配置

整个部署过程由环境变量控制，通过 `.env` 文件（基于 `.env.example` 模板）配置：

| 环境变量 | 默认值 | 影响范围 | 说明 |
|---------|--------|---------|------|
| `DATABASE_URL` | `postgres://postgres:password@localhost:5432/aionhive` | 服务端 | PostgreSQL 连接字符串 |
| `AION_HIVE_HTTP_PORT` | `8080` | 服务端 | HTTP 服务监听端口 |
| `AION_HIVE_DATA_DIR` | `./data` | 服务端 | 数据存储根目录 |
| `AION_HIVE_JWT_SECRET` | `change_this_secret_in_production` | 服务端 | JWT 签名密钥 |
| `AION_HIVE_CLI_ENCRYPTION_KEY` | 未设置 | 服务端 + CLI | CLI 配置中 API Key 加密密钥 |
| `AION_HIVE_PUBLIC_URL` | `http://localhost:8080` | 服务端 | 下载链接的公开访问地址 |
| `GITLAB_URL` | `https://gitlab.com` | 服务端 | GitLab 集成配置 |
| `DOCKER_HOST` | 未设置 | 服务端 | Docker 沙箱端点 |

**生产环境部署清单**：
1. 编译服务端二进制：`cargo build --release`
2. 编译 CLI 二进制：`./deploy/build-cli.sh`（或对应平台的脚本）
3. 构建管理后台：`cd admin && npm install && npm run build`
4. 配置 `.env` 文件：**必须**修改 `JWT_SECRET`、`CLI_ENCRYPTION_KEY`，配置正确的 `DATABASE_URL`
5. 启动服务端：`./target/release/server`
6. 配置反向代理（Nginx/Caddy）：前端静态文件 + API 反向代理

Sources: [.env.example](.env.example#L1-L74)

---

## 七、构建流程最佳实践与常见问题

### 7.1 构建速查表

| 目标 | 命令 | 预期产物 |
|------|------|---------|
| 服务端（开发） | `cargo run` | 监听 8080 端口 |
| 服务端（生产） | `cargo build --release && ./target/release/server` | 优化后的二进制 |
| CLI（当前平台） | `cargo build --release --no-default-features --features cli` | `target/release/skill-garden(.exe)` |
| CLI（跨平台分发） | `.\deploy\build-cli.ps1` 或 `./deploy/build-cli.sh` | `cli-dist/{version}/` 下各平台二进制 |
| 管理后台（开发） | `cd admin && npm run dev` | 5174 端口热重载 |
| 管理后台（生产） | `cd admin && npm run build` | `admin/dist/` 静态文件 |

### 7.2 常见错误排查

**症状**：CLI 构建报错，提示缺少 sqlx 或 tantivy 依赖
**原因**：编译 CLI 时忘记加 `--no-default-features`，导致 server feature 的依赖被引入
**解决**：使用 `cargo build --release --no-default-features --features cli`

**症状**：`rustup target add` 失败
**原因**：目标平台需要额外的系统库（如 Linux 交叉编译需要 GCC 工具链）
**解决**：在目标平台上原生构建，或安装对应工具链（Windows 需要 MSYS2，Linux 需要 mingw-w64）

**症状**：服务端启动后无法连接数据库
**原因**：`DATABASE_URL` 配置错误，或数据库迁移未运行
**解决**：检查 `.env` 配置，确保 PostgreSQL 已启动且数据库已创建

Sources: [deploy/README.md](deploy/README.md#L50-L80)

---

## 下一步阅读

本文档重点覆盖了构建与部署流程。要深入理解系统的其他核心模块，建议按以下顺序阅读：

- 了解如何配置和初始化数据库 → [PostgreSQL 数据库迁移与初始化](4-postgresql-shu-ju-ku-qian-yi-yu-chu-shi-hua)
- 理解 CLI 的命令行交互设计 → [CLI 命令行工具：搜索、安装、评价 Skills](25-cli-ming-ling-xing-gong-ju-sou-suo-an-zhuang-ping-jie-skills)
- 掌握服务端路由和认证机制 → [API 路由设计与认证机制（JWT + API Key）](10-api-lu-you-she-ji-yu-ren-zheng-ji-zhi-jwt-api-key)
- 查看完整的项目架构 → [整体架构：Rust 后端 + Svelte 管理后台 + CLI 工具链](5-zheng-ti-jia-gou-rust-hou-duan-svelte-guan-li-hou-tai-cli-gong-ju-lian)