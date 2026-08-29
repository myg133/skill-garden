# Skill Garden CLI

## 这是什么

`skill-garden` 是一个命令行工具，让你可以在终端中直接与 Skill Garden {server_url} 交互。
通过 `cli.setup` MCP 工具安装后，已自动配置好 API Key，开箱即用。

## 安装方式

由 AI Agent 通过 MCP 调用 `cli.setup` 工具自动完成：
1. Agent 调用 `cli.setup` → 获得下载链接和安装指令
2. 下载 tar.gz → 解压 → 运行 install.sh/install.ps1
3. install 脚本自动完成：
   - 将 `skill-garden` 二进制复制到 `~/.skill-garden/bin/`
   - 将 `config.toml`（含有效 API Key）写入 `~/.skill-garden/config.toml`
   - 将可执行目录加入 PATH

安装后此 SKILL.md 位于 `~/.skill-garden/SKILL.md`，Agent 可通过读取它了解 CLI 用法。

## 何时使用 CLI

| 场景 | 用 CLI 还是 MCP |
|------|----------------|
| 搜索技能 | 都可以，CLI 输出更可读 |
| 批量安装多项技能 | **CLI**（可管道串联） |
| 查看技能统计/排行 | **CLI** |
| 脚本化/自动化操作 | **CLI** |
| 离线/本地查看已安装技能 | **CLI** |
| 执行 MCP 工具（sandbox/evaluate） | **MCP**（CLI 不支持） |

## 可用命令

```bash
# 搜索
skill-garden search "<query>" [--limit N]

# 浏览
skill-garden list [--page N] [-n N]
skill-garden popular [--limit N]

# 详情
skill-garden info <skill-id>
skill-garden versions <skill-name>
skill-garden stats <skill-id>

# 安装
skill-garden install <skill-id> [--dir <path>]

# 身份
skill-garden whoami
skill-garden login <server> [--token <key>]
skill-garden logout

# 配置
skill-garden config show
skill-garden config set <key> <value>
```

## 配置位置

| 文件 | 路径 |
|------|------|
| 可执行文件 | `~/.skill-garden/bin/skill-garden` |
| 配置文件 | `~/.skill-garden/config.toml` |
| SKILL.md | `~/.skill-garden/SKILL.md` |
| 已安装技能 | `./skills/`（当前工作目录下） |

## config.toml 格式

```toml
server = "{server_url}"
token = "sk_xxxxxxxx"
```

通过 `cli.setup` 安装后 token 已自动填充，无需手动 login。

## 验证安装

```bash
{verify}
```

## 平台信息

- 平台：{os}/{arch}
- 版本：v{version}
- 服务端：{server_url}
