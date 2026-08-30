AionHive 的配置体系采用**环境变量驱动**模式——所有运行时配置均通过环境变量注入，配合 `.env` 文件实现开发环境的便捷管理。服务启动时，`dotenvy` 库自动从项目根目录加载 `.env` 文件，覆盖系统环境变量中未定义的条目。配置项按功能分为 7 大类别，涵盖数据库连接、HTTP 服务、安全密钥、文件存储、外部集成（GitLab/Docker/GitProxy）、评价 Webhook 以及 CLI 工具加密密钥。以下逐一展开。

Sources: [main.rs](src/main.rs#L352-L354), [.env.example](.env.example#L1-L74)

---

## 配置总览

所有环境变量及其默认值完整记录在项目根目录的 `.env.example` 文件中。该文件是**权威参考源**，每次新增或修改配置项时需同步更新。`.env` 文件本身已被 `.gitignore` 排除，确保密钥不会误提交到版本库。

Sources: [.env.example](.env.example#L1-L74), [.gitignore](.gitignore#L10-L11)

### 配置项一览表

| 环境变量 | 必填 | 默认值 | 用途 |
|---|---|---|---|
| **DATABASE** | | | |
| `DATABASE_URL` | 是 | `postgres://postgres:password@localhost:5432/aionhive` | PostgreSQL 连接字符串 |
| **HTTP SERVER** | | | |
| `AION_HIVE_TRANSPORT` | 否 | `http` | 传输模式：`http`（REST+SSE）或 `stdio` |
| `AION_HIVE_HTTP_PORT` | 否 | `8080` | HTTP 服务监听端口 |
| **DIRECTORIES** | | | |
| `AION_HIVE_DATA_DIR` | 否 | `./data` | 数据根目录（registry、search_index、evaluations 等子目录自动创建于此） |
| **SECURITY** | | | |
| `AION_HIVE_JWT_SECRET` | **强烈建议** | 随机生成（每次重启后旧 token 失效） | JWT 签名密钥 |
| `AION_HIVE_JWT_EXPIRY_HOURS` | 否 | `24` | JWT Token 过期时间（小时） |
| `AION_HIVE_CLI_ENCRYPTION_KEY` | 否 | 不设置则明文存储 | CLI API Key 的 AES-256-GCM 加密密钥（32 字节 hex，64 位十六进制字符） |
| **SKILL DOWNLOAD** | | | |
| `AION_HIVE_PUBLIC_URL` | 否 | `http://localhost:{AION_HIVE_HTTP_PORT}` | 对外公开的下载链接基础 URL |
| `AION_HIVE_DOWNLOAD_SECRET` | 否 | 回退到 `AION_HIVE_JWT_SECRET` | 下载 Token 签名 HMAC 密钥 |
| **EVALUATION WEBHOOKS** | | | |
| `AION_HIVE_EVAL_WEBHOOK_URLS` | 否 | 空 | 评价结果推送 URL 列表（逗号分隔） |
| **GITLAB INTEGRATION** | | | |
| `GITLAB_URL` | 否 | `https://gitlab.com` | GitLab 实例地址 |
| `GITLAB_GROUP` | 否 | `skill-garden` | GitLab Group 路径 |
| `GITLAB_TOKEN` | 否 | 空 | GitLab Personal Access Token（需 `api` + `read_repository` + `write_repository` 权限） |
| `GITLAB_PUSH_ENABLED` | 否 | `false` | 是否自动推送到远程 GitLab |
| `GITLAB_WEBHOOK_SECRET` | 否 | `skill-garden-webhook` | GitLab Webhook 验证密钥（X-Gitlab-Token 头） |
| **DOCKER SANDBOX** | | | |
| `DOCKER_HOST` | 否 | `http://localhost:2375` | Docker 守护进程 TCP 端点 |
| **GIT PROXY** | | | |
| `GIT_PROXY_API_BASE` | 否 | `http://localhost:8081` | Git Proxy 服务 API 地址 |

---

## 数据库连接配置

`DATABASE_URL` 是系统运行的**唯一必需配置项**。服务启动时，`AppState::new()` 方法通过 `sqlx::PgPool::connect()` 建立连接池，若环境变量未设置则回退到 `postgres://localhost:5432/aionhive` 的默认值。连接建立后，系统自动执行数据库迁移（`db::migrations::run_migrations`），确保所有表结构处于最新状态。

```mermaid
flowchart LR
    A[.env 文件] -->|dotenvy::dotenv| B[环境变量]
    B -->|DATABASE_URL| C[PgPool::connect]
    C --> D[运行数据库迁移]
    D --> E[初始化 Repository 层]
    B -->|AION_HIVE_DATA_DIR| F[创建数据目录]
    F --> G[registry/ search_index/ evaluations/]
```

`run_http_server` 函数中会再次读取 `DATABASE_URL` 创建独立的连接池，用于构建 HTTP 路由层的 Repository 实例。这种设计将核心服务（`AppState`）和 HTTP 层（`AppRouterState`）的数据库连接分离，各司其职。

Sources: [lib.rs](src/lib.rs#L137-L146), [main.rs](src/main.rs#L216-L227)

---

## JWT 安全认证体系

JWT（JSON Web Token）是 AionHive 的**核心认证机制**，所有 API 请求（MCP 工具调用、REST 管理接口）均通过 JWT 验证调用方身份。

### JWT 密钥加载机制

`AION_HIVE_JWT_SECRET` 通过 `OnceLock` 静态变量实现**惰性初始化**——首次调用 `get_jwt_secret()` 时读取环境变量并缓存，后续请求直接使用缓存值，避免重复读取环境变量。若环境变量未设置，系统会生成一个随机 UUID 作为密钥，并记录 `tracing::error!` 告警。**生产环境必须配置固定密钥**，否则每次服务重启后所有已签发的 JWT Token 都会失效。

```rust
// jwt.rs - 密钥加载逻辑
fn get_jwt_secret() -> &'static str {
    JWT_SECRET.get_or_init(|| {
        match std::env::var("AION_HIVE_JWT_SECRET") {
            Ok(secret) if !secret.is_empty() => secret,
            _ => {
                let fallback = format!("auto_generated_{}", Uuid::new_v4());
                tracing::error!("AION_HIVE_JWT_SECRET 未设置！...");
                fallback
            }
        }
    })
}
```

### Token 过期时间

`AION_HIVE_JWT_EXPIRY_HOURS` 控制签发 Token 的有效期，默认 24 小时。通过 `generate_token()` 和 `generate_identity_token()` 函数调用 `get_jwt_expiry_hours()` 获取配置值，与当前时间相加后写入 JWT 的 `exp` 声明字段。

Sources: [api/jwt.rs](src/api/jwt.rs#L12-L36)

---

## CLI Token 加密方案

### 痛点：API Key 明文存储

CLI 工具（`skill-garden`）通过 `cli.setup` 命令获取 API Key 后，将其保存在 `~/.skill-garden/config.toml` 文件中。若不加密，任何能读取该文件的人都能获取 API Key。

### 解决方案：AES-256-GCM 加密

`AION_HIVE_CLI_ENCRYPTION_KEY` 是一个 **32 字节（64 位十六进制字符）** 的 AES-256 密钥，用于加密 CLI 配置文件中的 API Key。加密后的 Token 以 `skc_` 前缀标识，格式为：

```
skc_<base64(nonce(12字节) || ciphertext + tag(16字节))>
```

| 组件 | 长度 | 说明 |
|---|---|---|
| `skc_` | 4 字符 | 加密 Token 标识前缀，用于区分明文和密文 |
| `nonce` | 12 字节 | 每次加密随机生成，确保同一明文每次密文不同 |
| `ciphertext + tag` | 变长 + 16 字节 | AES-256-GCM 加密输出，认证标签确保完整性 |

### 密钥生成与使用

```bash
# 生成 32 字节随机密钥（64 位十六进制字符）
openssl rand -hex 32
```

加密流程：
1. 服务端收到 `cli.setup` 请求后，用该密钥加密 API Key
2. 加密后的 `skc_` Token 写入 `config.toml`
3. CLI 每次请求时，读取 Token 并解密得到原始 API Key
4. 未设置此密钥时，API Key 以明文形式写入 `config.toml`

**安全要点**：服务端和 CLI 端使用**同一密钥**进行加解密。若密钥丢失，已加密的 Token 无法恢复，用户需重新执行 `cli.setup`。

Sources: [utils/cli_token.rs](src/utils/cli_token.rs#L1-L90)

---

## 外部集成配置

### GitLab 集成

Skill Git 服务支持将技能仓库推送到 GitLab 实现远程版本管理。配置项通过 `GitRemoteConfig::from_env()` 一次性从环境变量加载：

```rust
// skill_git.rs - GitLab 配置加载
impl GitRemoteConfig {
    pub fn from_env() -> Self {
        Self {
            gitlab_url: std::env::var("GITLAB_URL").unwrap_or_else(|_| "https://gitlab.com".to_string()),
            gitlab_group: std::env::var("GITLAB_GROUP").unwrap_or_else(|_| "skill-garden".to_string()),
            gitlab_token: std::env::var("GITLAB_TOKEN").unwrap_or_default(),
            push_enabled: std::env::var("GITLAB_PUSH_ENABLED")
                .map(|v| v == "true" || v == "1").unwrap_or(false),
        }
    }
}
```

远程仓库 URL 构造规则：`https://oauth2:{token}@{gitlab_url}/{group}/{repo_name}.git`。使用 `oauth2` 作为认证用户名，GitLab Token 作为密码，避免 HTTPS 基本认证中的明文密码暴露。

`GITLAB_PUSH_ENABLED` 设置为 `true` 时，每次技能上传/ZIP 解压后自动推送到远程；`false` 时仅本地 Git 管理。

Sources: [services/skill_git.rs](src/services/skill_git.rs#L38-L63)

### Docker Sandbox 集成

Sandbox 服务通过 Docker 容器提供隔离执行环境。`DOCKER_HOST` 环境变量指定 Docker 守护进程的 TCP 端点，默认 `http://localhost:2375`。若通过 Unix Socket 通信，可设置为 `unix:///var/run/docker.sock`。

Sandbox 配置包含多项并发控制参数，可通过 `SandboxConfig::default()` 查看默认值：

| 参数 | 默认值 | 说明 |
|---|---|---|
| 默认超时 | 30 秒 | 单次工具执行超时 |
| 容器最大生存期 | 3600 秒（1 小时） | 容器被回收前的最长存活时间 |
| 最大空闲时间 | 600 秒（10 分钟） | 空闲容器自动释放 |
| 全局最大并发容器数 | 50 | 所有工具共享 |
| 单工具最大池化容器数 | 5 | 每个工具独立池化 |
| 队列等待超时 | 60 秒 | 请求在 FIFO 队列中等待的最长时间 |

Sources: [services/sandbox.rs](src/services/sandbox.rs#L28-L57)

### Git Proxy 集成

`GIT_PROXY_API_BASE` 配置 Git Proxy 代理服务的 API 地址，用于技能仓库的远程 Git 操作（分支管理、文件读取、diff 查询等）。默认指向 `http://localhost:8081`。

Sources: [services/git_proxy.rs](src/services/git_proxy.rs#L60-L76)

---

## 评价 Webhook 配置

`AION_HIVE_EVAL_WEBHOOK_URLS` 支持**多个 Webhook URL**，用逗号分隔。每次收到新的评价时，`EvaluatorService` 将 `EvaluationResult` 以 HTTP POST 请求转发到所有配置的 URL。若未配置（空字符串），则不触发任何转发。

```rust
// evaluator.rs - Webhook URL 解析
let webhook_urls = std::env::var("AION_HIVE_EVAL_WEBHOOK_URLS")
    .map(|s| s.split(',').map(str::trim).map(String::from).collect())
    .unwrap_or_default();
```

Sources: [services/evaluator.rs](src/services/evaluator.rs#L27-L31)

---

## 数据目录结构

`AION_HIVE_DATA_DIR` 指定所有数据文件的根目录，默认为 `./data`。服务启动时自动创建以下子目录：

| 子目录 | 用途 |
|---|---|
| `registry/` | 技能注册信息、文件存储 |
| `evaluations/` | 评价数据持久化 |
| `search_index/` | Tantivy 全文搜索引擎索引文件 |

若 `AION_HIVE_DATA_DIR` 为相对路径，系统会将其转为**绝对路径**（基于当前工作目录），避免异步线程中工作目录变化导致路径解析错误。

Sources: [main.rs](src/main.rs#L365-L380)

---

## CLI 配置文件

CLI 工具的配置独立于服务端，存储在 `~/.skill-garden/config.toml` 文件中。这是一个 TOML 格式的配置文件，包含三个可选字段：

```toml
# ~/.skill-garden/config.toml
server = "https://skill-garden.example.com"    # 服务端地址
token = "skc_..."                               # API Key（加密后以 skc_ 开头）
skills_dir = "/home/user/.agent/skills"         # 技能安装目录
```

CLI 配置通过 `CliConfig` 结构体管理，提供 `load()`、`save()`、`delete()` 三个操作：
- `login` 命令：验证 API Key 后保存配置
- `logout` 命令：删除配置文件
- `whoami` 命令：读取配置并显示当前身份信息

Sources: [cli/config.rs](src/cli/config.rs#L1-L67), [cli/commands.rs](src/cli/commands.rs#L11-L57)

---

## 安全最佳实践

### 密钥管理

1. **生产环境必须设置 `AION_HIVE_JWT_SECRET`**：使用固定密钥确保服务重启后 Token 持续有效。推荐使用 `openssl rand -base64 32` 生成强随机密钥。
2. **CLI 加密密钥单独配置**：`AION_HIVE_CLI_ENCRYPTION_KEY` 与 JWT 密钥分离，降低密钥泄露影响面。
3. **下载签名密钥隔离**：`AION_HIVE_DOWNLOAD_SECRET` 默认回退到 JWT 密钥，但建议独立配置，实现最小权限原则。

### 文件安全

- `.env` 文件已加入 `.gitignore`，防止密钥被提交到版本库
- `.pem`、`.key` 证书文件和 `secrets/` 目录同样被 Git 忽略
- CLI 配置文件 `~/.skill-garden/config.toml` 默认权限为当前用户可读写

### 密钥轮换

JWT 密钥轮换后，所有已签发的 Token 自动失效，用户需重新登录。CLI 加密密钥轮换后，已加密的 `config.toml` 无法解密，用户需重新执行 `cli.setup` 生成新的加密 Token。

Sources: [.gitignore](.gitignore#L10-L11), [.gitignore](.gitignore#L47-L49), [api/jwt.rs](src/api/jwt.rs#L14-L29)

---

## 快速配置指南

### 开发环境初始化

```bash
# 1. 复制配置文件
cp .env.example .env

# 2. 编辑 .env 文件，修改数据库连接
# DATABASE_URL=postgres://postgres:your_password@localhost:5432/aionhive

# 3. 生成 CLI 加密密钥
openssl rand -hex 32

# 4. 将生成的密钥填入 .env
# AION_HIVE_CLI_ENCRYPTION_KEY=<上一步生成的64位hex字符串>

# 5. 启动服务（自动加载 .env）
cargo run
```

### 生产环境检查清单

| 检查项 | 操作 |
|---|---|
| JWT 密钥 | 设置 `AION_HIVE_JWT_SECRET` 为固定值 |
| CLI 加密密钥 | 设置 `AION_HIVE_CLI_ENCRYPTION_KEY` |
| 数据库密码 | 修改 `DATABASE_URL` 中的密码为强密码 |
| 公开 URL | 设置 `AION_HIVE_PUBLIC_URL` 为实际域名 |
| GitLab Token | 如需 GitLab 集成，设置 `GITLAB_TOKEN` |
| Docker 安全 | 确认 `DOCKER_HOST` 指向可信端点 |

---

## 下一步阅读

完成环境配置后，推荐按以下路径继续：

- [快速启动指南](2-kuai-su-qi-dong-zhi-nan) — 完成服务启动与第一个 API 调用
- [PostgreSQL 数据库迁移与初始化](4-postgresql-shu-ju-ku-qian-yi-yu-chu-shi-hua) — 深入了解数据库表结构与迁移机制
- [整体架构：Rust 后端 + Svelte 管理后台 + CLI 工具链](5-zheng-ti-jia-gou-rust-hou-duan-svelte-guan-li-hou-tai-cli-gong-ju-lian) — 理解各模块的配置依赖关系