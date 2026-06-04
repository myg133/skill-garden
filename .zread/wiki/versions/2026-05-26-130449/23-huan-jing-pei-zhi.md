本文档详细介绍 SkillGarden 项目的开发环境配置，涵盖从基础依赖安装到服务启动的完整流程。文档面向具备一定开发经验的中级工程师，假设读者熟悉命令行操作和现代开发工作流程。

---

## 1. 前置依赖

SkillGarden 是采用 Rust 后端 + Svelte 前端的全栈项目，需要配置两类运行时环境。

### 1.1 Rust 工具链

项目要求 Rust 1.70 或更高版本，采用 Cargo 作为构建工具链。安装方式推荐使用 `rustup` 官方安装器，它会自动处理版本管理和交叉编译支持。

```bash
# 验证 Rust 环境
rustc --version    # 应显示 1.70+
cargo --version    # 应显示最新稳定版

# 推荐更新到最新稳定版
rustup update stable
```

Sources: [Cargo.toml](Cargo.toml#L4)

### 1.2 Node.js 环境

前端管理平台使用 Svelte 框架构建，需要 Node.js 18+ 环境。推荐使用 Node.js 20.x LTS 版本以获得更好的性能和稳定性。

```bash
# 验证 Node.js 环境
node --version     # 应显示 v18+ 或 v20+
npm --version     # 验证 npm 包管理器可用
```

Sources: [admin/package.json](admin/package.json#L1-L20)

### 1.3 PostgreSQL 数据库

项目使用 PostgreSQL 14+ 作为持久化存储。数据库存储 Skills 元数据、评价记录、组织信息和会话状态。

```bash
# 验证 PostgreSQL 安装
psql --version

# 创建数据库
createdb aionhive
# 或使用 SQL 命令
# CREATE DATABASE aionhive;
```

Sources: [src/lib.rs](src/lib.rs#L49)

---

## 2. 环境变量配置

### 2.1 配置模板

项目根目录提供 `.env.example` 文件作为环境变量配置模板。在开发环境中，需要将其复制为 `.env` 文件并修改相应值。

```bash
# 复制配置模板
copy .env.example .env
```

Sources: [.env.example](.env.example#L1-L31)

### 2.2 核心配置项

下表详细说明各环境变量的用途、默认值和配置建议：

| 配置项 | 用途 | 默认值 | 生产环境要求 |
|--------|------|--------|-------------|
| `DATABASE_URL` | PostgreSQL 连接字符串 | `postgres://postgres:password@localhost:5432/aionhive` | 必须修改密码，使用 SSL 连接 |
| `AION_HIVE_HTTP_PORT` | HTTP 服务器监听端口 | `8080` | 可根据服务端口规划调整 |
| `AION_HIVE_DATA_DIR` | 数据存储根目录 | `./data` | 使用绝对路径，分配足够磁盘空间 |
| `AION_HIVE_SKILLS_DIR` | Skills 资产目录 | `./skills` | 建议使用独立磁盘分区 |
| `AION_HIVE_JWT_SECRET` | JWT 令牌签名密钥 | `change_this_secret_in_production` | **必须**在生产环境使用强随机密钥 |
| `AION_HIVE_EVAL_WEBHOOK_URLS` | 评价结果 Webhook 回调地址 | 空 | 留空表示禁用，支持多 URL 逗号分隔 |

Sources: [.env.example](.env.example#L1-L31)

### 2.3 安全配置建议

JWT 密钥是系统安全的关键防线。在生产部署时，必须使用密码学安全的随机字符串生成器生成新密钥：

```bash
# Linux/macOS 生成强随机密钥
openssl rand -base64 32

# Windows PowerShell 生成强随机密钥
[Convert]::ToBase64String((1..32 | ForEach-Object { Get-Random -Maximum 256 }))
```

---

## 3. 项目构建

### 3.1 克隆与依赖安装

首先克隆项目仓库，然后使用 Cargo 下载并编译 Rust 依赖。首次构建会耗时较长，因为需要编译所有依赖库。

```bash
# 克隆项目
git clone https://github.com/aionui/anspire-skillgarden.git
cd anspire-skillgarden

# 首次构建（编译所有依赖）
cargo build --release

# 开发构建（编译更快但运行稍慢）
cargo build
```

### 3.2 数据库迁移

项目启动时会自动执行数据库迁移。迁移脚本定义在 `src/db/migrations.rs` 中，包括 11 个迁移文件，用于创建组织、会话、工具路由等核心表结构。

```rust
// 迁移文件列表
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial_schema", include_str!("migrations/001_initial_schema.sql")),
    ("002_add_skill_status", include_str!("migrations/002_add_skill_status.sql")),
    // ... 共 11 个迁移
];
```

Sources: [src/db/migrations.rs](src/db/migrations.rs#L8-L18)

迁移包含的核心表结构：

| 迁移 | 功能 |
|------|------|
| `001_initial_schema` | 核心 Skills 元数据表 |
| `004_add_organizations` | 多租户组织表 |
| `005_add_sessions` | Agent 会话表 |
| `006_add_org_tools` | 组织级工具注册表 |
| `010_add_admin_users` | 管理员用户表 |

### 3.3 前端构建

管理平台采用 Vite + Svelte 构建，需要先安装前端依赖：

```bash
cd admin

# 安装依赖
npm install

# 开发模式启动
npm run dev

# 生产构建
npm run build
```

Sources: [admin/vite.config.js](admin/vite.config.js#L1-L16)

---

## 4. 启动模式

SkillGarden 支持三种 MCP 传输模式，适用于不同的集成场景。

### 4.1 模式对比

| 模式 | 适用场景 | 端口 | 配置复杂度 |
|------|----------|------|------------|
| **Stdio** | 本地开发、直接集成 | 无 | 最低 |
| **HTTP** | 远程 Agent、服务化部署 | 8080 | 中等 |
| **SSE** | 实时推送、长连接场景 | 8081 | 较高 |

Sources: [CLAUDE.md](CLAUDE.md#L78-L89)

### 4.2 Stdio 模式（默认）

Stdio 模式通过标准输入输出进行通信，适用于本地开发或与本地 Agent 直接集成。启动命令最为简单：

```bash
# 直接运行
cargo run

# 或使用构建后的可执行文件
./target/debug/aion-hive.exe
```

Sources: [CLAUDE.md](CLAUDE.md#L90-L96)

### 4.3 HTTP 模式

HTTP 模式通过 REST API 处理 MCP 请求，适用于需要网络访问的远程 Agent。使用 PowerShell 脚本启动：

```powershell
# 启动 HTTP 服务器（默认端口 8080）
.\start-http-server.ps1

# 自定义端口和数据目录
.\start-http-server.ps1 -Port 9090 -DataDir "D:\data\skillgarden"
```

服务器会启动在指定端口，提供以下端点：

- `GET /health` - 健康检查
- `POST /mcp` - MCP JSON-RPC 处理
- `GET /api/v1/skills` - REST API 列表
- `POST /api/v1/evaluations` - 评价提交

Sources: [start-http-server.ps1](start-http-server.ps1#L1-L14)

### 4.4 SSE 模式

SSE（Server-Sent Events）模式支持服务端主动推送消息，适用于需要实时更新通知的场景：

```powershell
# 启动 SSE 服务器（默认端口 8081）
.\start-sse-server.ps1

# 自定义配置
.\start-sse-server.ps1 -Port 8082 -DataDir "test-data"
```

SSE 端点包括：

- `GET /sse` - 建立 SSE 连接，返回 session_id
- `POST /sse/:session_id` - 通过 session 发送消息

Sources: [start-sse-server.ps1](start-sse-server.ps1#L1-L14)

---

## 5. 前端开发配置

### 5.1 开发服务器

管理平台开发服务器配置在 `vite.config.js` 中，默认监听端口 5174，并代理 API 请求到后端服务器：

```javascript
export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 5174,
    proxy: {
      '/api': {
        target: 'http://localhost:8081',  // 代理到后端 HTTP 端口
        changeOrigin: true
      }
    }
  }
});
```

Sources: [admin/vite.config.js](admin/vite.config.js#L6-L15)

### 5.2 前后端联调

开发时需要同时运行后端服务器和前端开发服务器：

```powershell
# 终端 1：启动后端（HTTP 模式）
.\start-http-server.ps1 -Port 8081

# 终端 2：启动前端开发服务器
cd admin
npm run dev
```

访问 `http://localhost:5174` 即可使用管理平台，所有 API 请求会自动代理到后端。

---

## 6. 环境验证

### 6.1 后端健康检查

服务启动后，可通过健康检查端点验证环境配置是否正确：

```bash
# Windows PowerShell
Invoke-RestMethod -Uri "http://localhost:8080/health" -Method Get

# Linux/macOS
curl http://localhost:8080/health
```

正常响应示例：

```json
{
  "status": "OK",
  "version": "0.3.0",
  "skills_count": 0
}
```

Sources: [src/main.rs](src/main.rs#L57-L64)

### 6.2 数据库连接验证

检查数据库迁移是否成功执行：

```sql
-- 连接到 PostgreSQL
psql $DATABASE_URL

-- 查看迁移记录
SELECT * FROM _migrations ORDER BY applied_at;

-- 验证核心表是否存在
\dt
```

---

## 7. 常见问题

### 7.1 编译错误

如果遇到编译错误，首先确保 Rust 工具链版本正确：

```bash
# 检查版本要求
cat Cargo.toml | findstr "rust-version"

# 更新 Rust
rustup update
```

### 7.2 数据库连接失败

检查 PostgreSQL 服务状态和连接字符串格式：

```powershell
# 验证 PostgreSQL 服务运行中
Get-Service | Where-Object {$_.Name -like "*postgres*"}

# 测试数据库连接
psql "postgres://postgres:password@localhost:5432/aionhive" -c "SELECT 1;"
```

### 7.3 端口占用

如果启动时提示端口被占用，检查并释放端口：

```powershell
# 查看端口占用
netstat -ano | findstr "8080"

# 结束占用进程（替换 <PID> 为实际进程 ID）
taskkill /PID <PID> /F
```

---

## 8. 下一步

完成环境配置后，建议按以下顺序阅读文档：

1. [项目概述](1-xiang-mu-gai-shu) - 了解 SkillGarden 的核心价值和应用场景
2. [快速开始](2-kuai-su-kai-shi) - 体验完整的 Agent Skills 共享流程
3. [系统架构](8-xi-tong-jia-gou) - 深入理解技术架构设计

---

*文档版本：基于项目 v0.3.0，更新时间：2026-04-20*