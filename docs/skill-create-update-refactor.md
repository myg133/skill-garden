# Skill 创建与更新工作流重构方案

> 状态：草案，待讨论  
> 日期：2026-07-22

## 一、当前实现 vs 目标对比

### 1.1 当前实现

```
上传 ZIP
    │
    ├─ 解压验证
    ├─ 自动递增版本号 (patch+1)
    ├─ git add -A + git commit + git tag v{version}  ← 上传时立即 commit
    ├─ 写入 skills 表 (status = pending_review)
    ├─ 写入 skill_versions 表
    └─ sync_skill_files_from → skills/{name}/

审核流程：
    draft → 提交审核(pending_review) → 审核通过(approved) → 发布(published)

问题：
1. Git commit + tag 发生在上传时，而不是审核通过后
2. 如果审核驳回，Git 中已有无效的 commit
3. 审核通过后发布只是改 status，没有 Git 操作
4. skills/{name}/ 目录在上传时就被覆盖，审核期间用户看到的是未审核内容
```

### 1.2 目标流程

```
上传 ZIP
    │
    ├─ 解压验证
    ├─ 暂存到临时目录（不进入 Git）
    ├─ 预览确认后写入 skills 表 (status = draft)
    └─ 文件暂存在 git-repos/skill-{name}/.pending/ 目录

提交审核：
    draft → pending_review

审核通过后：
    ├─ 计算版本号 (首次 1.0.0，后续 patch+1 或用户指定)
    ├─ git add -A + git commit + git tag v{version}  ← 审核通过后才 commit
    ├─ 写入 skill_versions 表
    ├─ sync_skill_files_from → skills/{name}/
    └─ status → approved

发布：
    approved → published
    此时文件已在 skills/{name}/，可直接安装/下载
```

---

## 二、详细设计

### 2.1 版本号管理

| 场景 | 规则 |
|---|---|
| 首次上传 | 默认 `1.0.0`，用户可在 SKILL.md 中指定 |
| 已有版本更新 | patch+1 自动递增（如 1.0.3 → 1.0.4） |
| 用户指定版本 | SKILL.md frontmatter 中的 `version` 字段优先 |
| 版本冲突 | 如果指定版本已存在（在 skills 表或 Git tag 中），返回错误 |

**版本号在审核通过时确定**，而非上传时。

### 2.2 Git 管理变更

**当前**：
```
上传 → git commit + tag（审核前）
```

**目标**：
```
上传 → git commit（不打 tag，生成 commit hash 可追溯）
审核通过 → git tag v{version}（打在已有 commit 上）
审核驳回 → git reset --soft HEAD~1（撤销 commit）
```

**为什么上传时就 commit**：

| 好处 | 说明 |
|---|---|
| **可看变更** | `git diff HEAD~1` 对比新旧版本，管理员审核时可看 diff |
| **可追溯** | 每次上传都有 commit hash，即使驳回也有记录 |
| **不污染 tag** | tag 只在审核通过后才打，Git tag 列表 = 已发布版本列表 |
| **驳回可撤销** | `git reset --soft HEAD~1` 撤销 commit，文件回到暂存区 |

**完整流程**：

```
初始状态（已发布 v1.0.3）：
    git-repos/skill-browse/
    ├── .git/                    # tags: v1.0.0, v1.0.1, v1.0.2, v1.0.3
    ├── SKILL.md (v1.0.3)
    └── src/...

用户上传新版本 ZIP：
    ├─ 解压到临时目录
    ├─ 确定版本号 v1.0.4
    ├─ clean_working_dir → 清空工作目录
    ├─ copy 新文件到工作目录
    ├─ git add -A
    └─ git commit -m "v1.0.4: browse (pending review)"
        （不打 tag，HEAD 指向新 commit）

审核期间：
    ├─ 管理员查看 diff：git diff v1.0.3 HEAD
    ├─ 管理员查看文件：git show HEAD:SKILL.md
    └─ 管理员查看文件列表：git ls-tree -r HEAD

审核通过：
    ├─ git tag -a v1.0.4 -m "Approved version"
    ├─ 写入 skill_versions 表
    ├─ git archive v1.0.4 | gzip → releases/browse/v1.0.4.tar.gz
    ├─ sync_skill_files_from → skills/browse/
    └─ status → approved

审核驳回：
    ├─ git reset --soft HEAD~1（撤销 commit，文件回到暂存区）
    ├─ 工作目录恢复到上一个已发布版本的状态
    └─ status → rejected
```

**文件覆盖策略**：

审核通过后直接覆盖 `skills/{name}/`，这是安全的：
- 审核通过后才执行，`skills/{name}/` 始终是已审核内容
- 旧版本文件在 Git 历史中可恢复
- `releases/{name}/v{old}.tar.gz` 保留了旧版本 tarball

### 2.3 审核流程调整

```
┌──────────┐     提交审核     ┌──────────────┐     审核通过      ┌──────────┐
│  draft   │ ──────────────→ │pending_review │ ──────────────→ │ approved │
└──────────┘                 └──────────────┘                  └────┬─────┘
                                                                   │
                                          ┌────────────────────────┘
                                          │ 审核通过时执行：
                                          │ 1. 确定版本号
                                          │ 2. git commit + tag
                                          │ 3. 写入 skill_versions
                                          │ 4. sync_skill_files_from
                                          │ 5. status → approved
                                          ▼
                                     ┌──────────┐     发布      ┌───────────┐
                                     │ approved │ ──────────→ │ published │
                                     └──────────┘             └───────────┘
```

### 2.4 文件目录规划

```
data/
├── git-repos/                     # Git 仓库（版本管理）
│   └── skill-{name}/
│       ├── .git/                  # 只包含已审核通过的版本
│       ├── SKILL.md               # 当前已发布版本的文件
│       ├── src/...
│       └── .pending/              # 待审核文件（临时）
│           ├── SKILL.md
│           └── src/...
│
├── skills/                        # 安装/下载用（与 Git 工作目录同步）
│   └── {name}/
│       ├── SKILL.md
│       └── src/...
│
├── releases/                      # 【新增】版本 tarball 归档
│   └── {name}/
│       ├── v1.0.0.tar.gz
│       ├── v1.0.1.tar.gz
│       └── latest.tar.gz → v1.0.1.tar.gz
│
└── search/                        # Tantivy 搜索索引
```

### 2.5 Tarball 生成与下载

**生成时机**：审核通过后（git commit + tag 完成时）

**生成逻辑**：
```rust
fn generate_release_tarball(name: &str, version: &str) -> Result<PathBuf> {
    let release_dir = data_dir.join("releases").join(name);
    let tarball_path = release_dir.join(format!("v{}.tar.gz", version));
    
    // 1. 从 Git 导出指定 tag 的文件
    // git archive --format=tar v{version} | gzip > v{version}.tar.gz
    Command::new("git")
        .current_dir(&repo_dir)
        .args(["archive", "--format=tar", &format!("v{}", version)])
        .stdout(Stdio::piped())
        .output()?;
    
    // 2. gzip 压缩
    // 3. 写入 releases/{name}/v{version}.tar.gz
    
    // 4. 更新 latest.tar.gz 软链接
    symlink(tarball_path, release_dir.join("latest.tar.gz"))?;
}
```

**下载流程**（替代当前 `download_skill_handler`）：

```
GET /api/v1/skills/:name/download/:version?token=...
    │
    ├─ 验证 token（现有逻辑不变）
    ├─ 读取 releases/{name}/v{version}.tar.gz
    └─ 直接返回文件（无需实时打包）
```

**优势**：
- 审核通过时一次性生成，下载时直接返回静态文件
- 不依赖 `skills/{name}/` 目录的实时状态
- 支持任意历史版本下载
- 减轻下载时的服务器压力

### 2.6 安装流程调整

```
skills.install(skill_id)
    │
    ├─ 获取 skill 的 name + version
    ├─ 读取 releases/{name}/v{version}.tar.gz
    ├─ 解压到目标目录
    └─ 返回安装结果
```

替代当前从 `skills/{name}/` 目录打包的方式。

---

## 三、与现有功能的兼容

### 3.1 需要改动的

| 模块 | 改动 |
|---|---|
| `process_upload` | 不再 git commit + tag；文件暂存到 `.pending/` 目录 |
| `confirm_upload_from_preview` | 同上 |
| `approve_org_skill_handler` | 增加：确定版本号 → git commit + tag → 写入 skill_versions → 生成 tarball → sync_skill_files_from |
| `download_skill_handler` | 改为从 `releases/{name}/` 读取 tarball |
| MCP `skills.install` | 改为从 `releases/{name}/` 读取 tarball |
| `sync_skill_files_from` | 调整时机（审核通过后调用，而非上传时） |
| `skill_versions` 写入 | 从上传时改为审核通过时 |

### 3.2 不受影响的

| 模块 | 原因 |
|---|---|
| 搜索索引 | 审核通过后才 add_skill，不受影响 |
| 市场审核流程 | marketplace_status 状态机不变 |
| 编辑更新（draft_content） | 编辑和上传是两个独立路径 |
| 回退 | 基于 Git tag，审核通过后 tag 已存在，可正常回退 |
| 版本 diff | 基于 Git tag，同上 |
| `list_skill_files` / `get_skill_file` | 基于 Git tag，同上 |

### 3.3 需要新增的

| 项目 | 说明 |
|---|---|
| `releases/{name}/` 目录 | 存放审核通过后生成的 tarball |
| `generate_release_tarball()` | `git archive v{version} \| gzip` 生成 tarball |
| 审核通过后的 Git 操作 | `git tag` + 写入 `skill_versions` + 生成 tarball + `sync_skill_files_from`，整合到 `approve_org_skill_handler` |
| 审核驳回后的 Git 操作 | `git reset --soft HEAD~1` 撤销 commit |

---

## 四、实施步骤

### Phase 1：调整上传流程

1. `process_upload` / `confirm_upload_from_preview` 中：
   - 保留 `git commit`（不打 tag）← **改动**
   - 移除 `git tag` ← **改动**
   - 移除 `sync_skill_files_from` 调用 ← **改动**
   - 移除 `skill_versions` 表写入 ← **改动**
   - 保留 DB 写入（skills 表，status=pending_review）

### Phase 2：调整审核通过/驳回逻辑

1. `approve_org_skill_handler` 中增加：
   - `git tag v{version}` 打在当前 HEAD 上
   - 写入 `skill_versions` 表
   - `generate_release_tarball`
   - `sync_skill_files_from`
2. 审核驳回时：
   - `git reset --soft HEAD~1` 撤销 commit
   - status → rejected

### Phase 3：调整下载/安装

1. 修改 `download_skill_handler` 从 `releases/` 读取
2. 修改 MCP `skills.install` 从 `releases/` 读取
3. 保留 `skills/{name}/` 目录用于文件预览（`list_skill_files` / `get_skill_file` 用 Git）

### Phase 4：数据迁移

1. 为现有已发布 Skill 生成 tarball（`git archive v{version} \| gzip`）
2. 写入 `releases/{name}/v{version}.tar.gz`

---

## 五、业务场景覆盖检查

### 5.1 审核期间的文件预览

| 场景 | 实现方式 |
|---|---|
| 作者预览自己上传的新版本 | `list_skill_files` / `get_skill_file` 通过 `git show HEAD:{path}` 读取（HEAD 指向审核中的 commit） ✅ |
| 管理员审核时预览文件 | 同上 ✅ |
| 管理员审核时看 diff | `git diff v{old} HEAD` 对比新旧版本 ✅ |
| 普通用户查看市场 Skill | `skills/{name}/` 目录仍是旧版本（审核通过前未覆盖） ✅ |

### 5.2 审核期间作者再次修改

| 场景 | 处理方式 |
|---|---|
| 作者在 pending_review 时重新上传 ZIP | 覆盖当前 commit：`git reset --soft HEAD~1` + 重新 `git commit` |
| 作者在 pending_review 时编辑 tags/description | 直接更新 DB（不影响 Git，因为 tags 不在 Git 中） |
| 作者取消提交 | 撤销 commit + status → draft |

### 5.3 首次上传 vs 更新上传

| 场景 | 区别 |
|---|---|
| 首次上传 | Git 仓库不存在 → `git init` → `git commit`（无旧 tag 可对比） |
| 更新上传 | Git 仓库已存在 → `clean_working_dir` → `git commit`（可 `git diff v{latest}` 对比） |
| 版本号 | 首次默认 1.0.0，更新 patch+1 |

### 5.4 市场审核

| 场景 | 处理方式 |
|---|---|
| 提交到市场 | 当前 `marketplace_status` 状态机不变，`submit_to_marketplace` → `pending_review` |
| 市场审核通过 | 不涉及 Git 操作（文件已在内部审核通过时处理完毕），只改 `marketplace_status → listed` |
| 市场审核驳回 | 同上，只改 `marketplace_status → rejected` |

**注意**：内部审核通过时文件已覆盖 `skills/{name}/`，市场审核是独立的元数据审核。

### 5.5 组织 Skill 审核

| 场景 | 处理方式 |
|---|---|
| 组织 Developer 上传 | 同个人 Skill，`git commit` + status=pending_review |
| 组织 Reviewer 审核通过 | `git tag` + `sync_skill_files_from` + 生成 tarball |
| 组织 Reviewer 审核驳回 | `git reset --soft HEAD~1` + status=rejected |

### 5.6 并发上传

| 场景 | 处理方式 |
|---|---|
| 同一 Skill 同时有 pending_review 又有人上传 | 拒绝第二次上传，提示「已有待审核版本」 |
| 同一 Skill 不同人上传 | 同一个人 Skill 只有 owner 能上传（权限已控制），组织 Skill 用文件锁 |

### 5.7 删除 Skill

| 场景 | 处理方式 |
|---|---|
| 删除 draft 状态的 Skill | 直接删除 DB 记录 + 撤销 Git commit |
| 删除 published 的 Skill | 必须先下架（delist），再删除。Git 历史保留，`releases/` 中的 tarball 可选清理 |
| 删除 archived 的 Skill | 直接删除 DB 记录 |

### 5.8 版本回退

| 场景 | 处理方式 |
|---|---|
| 回退到旧版本 | 基于旧 Git tag 创建新 commit + 新 tag（如 v1.0.2 → v1.0.5），走审核流程 |
| 回退后旧 tarball | 直接使用已有的 `releases/{name}/v1.0.2.tar.gz` 重命名为新版本 |

---

## 六、已确认决策

1. ✅ **上传时 git commit，不打 tag** — 可看 diff，可追溯，tag 列表干净
2. ✅ **审核通过时 git tag** — tag 列表 = 已发布版本
3. ✅ **审核驳回时 git reset --soft HEAD~1** — 撤销 commit
4. ✅ **tarball 审核通过时预生成** — `git archive v{version} | gzip`
5. ✅ **下载/安装从 `releases/` 读取** — 不依赖 `skills/{name}/` 实时状态
6. ✅ **文件覆盖在审核通过时** — `skills/{name}/` 始终是已审核内容

---

## 七、已废弃方案

- ❌ `.pending/` 暂存目录 — 改为 git commit（不打 tag）方案，更简洁
- ❌ 审核通过后才 git commit — 无法看 diff，不利于审核
