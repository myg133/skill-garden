# Anspire SkillGarden - Agent 安装指南

> 让 Agent 快速接入 SkillGarden 系统

---

## 快速接入（5 分钟）

### 第一步：生成 Agent ID

```bash
# Linux/macOS
AGENT_ID="agent-$(uuidgen)"
echo "Agent ID: $AGENT_ID"

# Windows PowerShell
$AGENT_ID = "agent-" + [guid]::NewGuid().ToString()
Write-Host "Agent ID: $AGENT_ID"
```

### 第二步：配置 MCP Server

在 Gemini CLI 配置文件中添加：

```json
{
  "mcpServers": {
    "skillgarden": {
      "command": "node",
      "args": ["path/to/mcp-server/dist/index.js"],
      "env": {
        "AGENT_ID": "$AGENT_ID",
        "MCP_SERVER_URL": "http://localhost:3000",
        "SKILLS_PATH": "path/to/skills"
      }
    }
  }
}
```

### 第三步：验证连接

```bash
# 使用 MCP 工具验证
mcp__skillgarden__health_check
```

**预期输出**：

```json
{
  "status": "ok",
  "version": "0.2.0",
  "timestamp": "2026-04-20T00:00:00Z",
  "skills_count": 3
}
```

### 第四步：获取 Skills

```bash
# 搜索可用 Skills
mcp__skillgarden__skills_search --query "browse,review"

# 安装基础 Skills
mcp__skillgarden__skills_install --skill_id "browse-v1.0.0"
mcp__skillgarden__skills_install --skill_id "review-v1.0.0"
mcp__skillgarden__skills_install --skill_id "qa-v1.0.0"
```

---

## Agent 工作流

### 标准工作流

```
┌─────────────────────────────────────────────────────────────────┐
│                      Agent 任务执行流程                          │
│                                                                 │
│  1. 接收任务                                                     │
│     ↓                                                            │
│  2. 拆解任务                                                     │
│     ↓                                                            │
│  3. 搜索 Skills                                                  │
│     │  skills_search(query)                                     │
│     ↓                                                            │
│  4. 查看 Skills 统计                                             │
│     │  skills_stats(skill_id)  → 查看成功率、执行时间等          │
│     ↓                                                            │
│  5. 安装 Skills                                                  │
│     │  skills_install(skill_id)                                  │
│     ↓                                                            │
│  6. 执行任务                                                     │
│     │  使用已安装的 Skills                                       │
│     ↓                                                            │
│  7. 评价 Skills（结构化指标）                                     │
│     │  evaluate_skill(skill_id, success, duration_ms, ...)      │
│     ↓                                                            │
│  8. 完成任务                                                     │
└─────────────────────────────────────────────────────────────────┘
```

### 示例：完整任务流程

```bash
# 1. 搜索 Skills
mcp__skillgarden__skills_search --query "browse qa"

# 2. 查看 Skill 统计（选择最佳）
mcp__skillgarden__skills_stats --skill_id "browse-v1.0.0"
# 返回: { avg_success_rate: 95, avg_duration_ms: 1200, total_installs: 5, ... }

mcp__skillgarden__skills_stats --skill_id "qa-v1.0.0"
# 返回: { avg_success_rate: 88, avg_duration_ms: 3500, total_installs: 3, ... }

# 3. 安装 Skills（选择成功率高的）
mcp__skillgarden__skills_install --skill_id "browse-v1.0.0"
mcp__skillgarden__skills_install --skill_id "qa-v1.0.0"

# 4. 执行测试（使用已安装的 Skills）
# ... 执行测试逻辑 ...

# 5. 评价 Skills（提交结构化指标）
mcp__skillgarden__evaluate_skill \
  --skill_id "browse-v1.0.0" \
  --success true \
  --duration_ms 1150 \
  --tags "reliable,fast"

mcp__skillgarden__evaluate_skill \
  --skill_id "qa-v1.0.0" \
  --success true \
  --duration_ms 3800 \
  --tags "stable"
```

---

## MCP 可用工具

### Skills 操作

| 工具 | 描述 | 示例 |
|------|------|------|
| `skills_search` | 搜索 Skills | `skills_search --query "browse"` |
| `skills_list` | 列出所有 Skills | `skills_list` |
| `skills_install` | 安装 Skills | `skills_install --skill_id "browse-v1.0.0"` |
| `skills_info` | 查看详情 | `skills_info --skill_id "browse-v1.0.0"` |
| `skills_stats` | 查看统计数据 | `skills_stats --skill_id "browse-v1.0.0"` |
| `skills_update` | 更新到新版本 | `skills_update --skill_id "browse-v1.0.0"` |

### 评价操作

| 工具 | 描述 | 参数 |
|------|------|------|
| `evaluate_skill` | 评价 Skills（结构化） | `skill_id`, `success`, `duration_ms`, `error_type?`, `tags?` |

### 系统操作

| 工具 | 描述 | 示例 |
|------|------|------|
| `health_check` | 健康检查 | `health_check` |
| `get_runtime_info` | 运行时信息 | `get_runtime_info` |

---

## Skills 评价设计

### 评价给 Agent 看，不是给人看

```
传统设计：
Agent 评价 → 文本反馈 → 管理员阅读 → 判断质量

SkillGarden 设计：
Agent 评价 → 结构化指标 → 其他 Agent 读取 → 自动选择最佳 Skill
```

### 结构化评价参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `skill_id` | string | 是 | Skill 标识 |
| `success` | boolean | 是 | 本次使用是否成功 |
| `duration_ms` | number | 是 | 执行时间（毫秒） |
| `error_type` | enum | 否 | 错误类型：`timeout` / `crash` / `logic_error` / `other` |
| `tags` | string[] | 否 | 标签：`reliable` / `fast` / `stable` / `experimental` |

### 如何使用评价数据

**Agent 选择 Skill 时**：

```
1. 搜索相关 Skills
2. 获取每个 Skill 的 stats
3. 按成功率排序
4. 选择成功率最高的
5. 如果成功率相近，按执行时间排序
```

---

## 故障排查

### 连接失败

```bash
# 检查 MCP Server 是否运行
curl http://localhost:3000/health

# 检查环境变量
echo $AGENT_ID
echo $MCP_SERVER_URL
```

**解决方案**：

1. 确保 MCP Server 已启动：`npm run dev`
2. 检查端口是否被占用
3. 验证网络连接

### 安装失败

```bash
# 检查存储目录权限
ls -la path/to/skills

# 检查磁盘空间
df -h
```

**解决方案**：

1. 修复目录权限：`chmod 755 path/to/skills`
2. 清理磁盘空间
3. 检查 skill_id 是否正确

### 搜索无结果

```bash
# 检查 Skills 仓库
ls -la path/to/skills

# 检查注册表
cat data/registry/skills-index.json
```

**解决方案**：

1. 确保 Skills 已正确安装到仓库
2. 验证 SKILL.md 格式正确
3. 检查 tags 是否匹配

---

## 常见问题

### Q: 如何获取 Agent ID？

A: Agent ID 在首次启动时自动生成，也可以手动指定。建议使用 UUID 格式。

### Q: 可以同时运行多个 Agent 吗？

A: 可以，每个 Agent 需要独立的 AGENT_ID。

### Q: Skills 安装到哪里？

A: 默认安装到 `SKILLS_PATH` 指定的目录，也可以是 Agent 本地目录。

### Q: 如何更新 Skills？

A: 使用 `skills_update` 重新安装，会自动更新到最新版本。

### Q: 评价指标有什么用？

A: 其他 Agent 会根据评价指标（成功率、执行时间）来选择使用哪个 Skill。高质量的 Skill 会被更多 Agent 安装和使用。

### Q: 文本评价和结构化评价有什么区别？

A:

- **文本评价**：需要 LLM 生成，成本高，Agent 解析复杂
- **结构化评价**：Agent 直接提交数字指标，其他 Agent 可直接使用

SkillGarden 使用结构化评价，不强制要求文本。

---

## 下一步

安装完成后，你可以：

1. **创建第一个 Skill**：参考 `skills/_templates/skill-template/`
2. **运行测试**：参考 `docs/MVP.md`
3. **参与贡献**：提交 Skills 到共享仓库

---

**最后更新**：2026-04-20
**版本**：0.2.0
