# Skill Garden CLI 构建与分发

本目录包含 Skill Garden CLI 的跨平台构建脚本。构建产物输出到 `cli-dist/{version}/{os}-{arch}/`，供服务端 `cli.setup` 工具分发给 Agent。

## 目录结构

```
deploy/                     ← 构建脚本（本目录）
├── build-cli.ps1           ← Windows 构建脚本
├── build-cli.sh            ← Linux/macOS 构建脚本
└── README.md               ← 本文

cli-dist/                   ← CLI 分发目录（构建产物 + 资源模板）
├── SKILL.md                ← AI Agent 技能定义模板（{server_url} 等占位符）
├── instructions.md         ← 安装说明模板
├── install.sh              ← Linux/macOS 安装脚本模板
├── install.ps1             ← Windows 安装脚本模板
└── {version}/              ← 构建产物
    ├── windows-x86_64/
    │   └── skill-garden.exe
    ├── windows-aarch64/
    │   └── skill-garden.exe
    ├── linux-x86_64/
    │   └── skill-garden
    ├── linux-aarch64/
    │   └── skill-garden
    ├── macos-x86_64/
    │   └── skill-garden
    └── macos-aarch64/
        └── skill-garden
```

## 重要提醒

CLI 和服务端共用 `default = ["server"]` feature。构建 CLI 时**必须**加 `--no-default-features`，否则 sqlx、tantivy、axum 等服务端依赖会一并编译，导致编译时间和二进制体积大幅膨胀。

```powershell
# 正确 ✅
cargo build --release --no-default-features --features cli

# 错误 ❌ （会把 server 所有依赖一起编译）
cargo build --release --features cli
```

## 支持的目标平台

| 目标名 | Rust triple | 二进制名 |
|--------|-------------|----------|
| `windows-x86_64` | `x86_64-pc-windows-msvc` | `skill-garden.exe` |
| `windows-aarch64` | `aarch64-pc-windows-msvc` | `skill-garden.exe` |
| `linux-x86_64` | `x86_64-unknown-linux-gnu` | `skill-garden` |
| `linux-aarch64` | `aarch64-unknown-linux-gnu` | `skill-garden` |
| `macos-x86_64` | `x86_64-apple-darwin` | `skill-garden` |
| `macos-aarch64` | `aarch64-apple-darwin` | `skill-garden` |

## 使用方法

### Windows（PowerShell）

```powershell
# 构建当前平台（自动检测 OS + 架构）
.\deploy\build-cli.ps1

# 构建指定目标（逗号分隔）
.\deploy\build-cli.ps1 -Targets "windows-x86_64,linux-x86_64"

# 指定输出目录（默认 cli-dist）
.\deploy\build-cli.ps1 -DataDir "custom/output"
```

### Linux / macOS（Bash）

```bash
# 构建当前平台
./deploy/build-cli.sh

# 构建指定目标
./deploy/build-cli.sh linux-x86_64,macos-aarch64

# 指定输出目录
DATA_DIR=/path/to/output ./deploy/build-cli.sh
```

## 前置条件

1. **Rust 工具链**：已安装 `rustup` 和 `cargo`
2. **目标平台 toolchain**：交叉编译需要安装对应的 target

```bash
# 安装交叉编译 target（按需）
rustup target add x86_64-unknown-linux-gnu    # Linux x86_64
rustup target add aarch64-unknown-linux-gnu   # Linux ARM64
rustup target add x86_64-apple-darwin         # macOS x86_64
rustup target add aarch64-apple-darwin        # macOS ARM64 (Apple Silicon)
rustup target add aarch64-pc-windows-msvc     # Windows ARM64
```

**交叉编译限制**：
- 从 Windows 交叉编译 Linux 需要 MSYS2 + MinGW GCC 工具链
- 从 Linux 交叉编译 Windows 需要 `mingw-w64`
- macOS target 通常只能从 macOS 编译（需要 Apple SDK）

**推荐做法**：在每个平台的原生环境中分别构建各自的 target，或者使用 GitHub Actions 多 runner 矩阵编译。

## 与服务端的集成

服务端 `cli.setup` MCP 工具读取 `cli-dist/` 目录：

```
Agent 请求 cli.setup(platform="linux-x86_64", arch="x86_64")
                │
                ▼
服务端读取 cli-dist/{version}/linux-x86_64/skill-garden
                │
                ▼
打成 tar.gz（包含: binary + config.toml + install.sh + SKILL.md）
                │
                ▼
返回 download_url 给 Agent
```

`cli-dist/` 是一个独立目录，不依赖 `AION_HIVE_DATA_DIR` 环境变量。服务端始终从项目根目录的 `cli-dist/` 读取 CLI 二进制文件。

## 本地开发测试

如果只需要本地测试 CLI 功能，直接编译当前平台即可：

```powershell
# Windows
cargo build --release --no-default-features --features cli
.\target\release\skill-garden.exe whoami
```

```bash
# Linux/macOS
cargo build --release --no-default-features --features cli
./target/release/skill-garden whoami
```

首次使用需要登录或手动配置 `~/.skill-garden/config.toml`：

```toml
server = "https://your-server.com"
token = "sk_xxx"
```
