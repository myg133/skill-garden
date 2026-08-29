# Skill 更新工作流设计方案

> 状态：已确认，待实现  
> 日期：2026-07-21

## 一、背景与问题

### 1.1 当前行为

| 场景 | 当前行为 |
|---|---|
| 作者编辑 tags | 直接更新 DB + 搜索索引，不影响 status/marketplace_status |
| 作者重新上传 ZIP | 创建新版本（patch+1），status = pending_review，需重新走审核 |
| 已上架市场 Skill 编辑 tags | 直接生效，**无需重新审核** |

### 1.2 核心问题

1. **已上架市场的 Skill** 被作者更新（tags/description/content）后直接生效，用户可能注入不当内容
2. **直接编辑** 和 **上传新版本** 的行为不一致——前者不审核，后者审核
3. 没有「版本草稿」机制，上传即进入审核队列，作者无法在上传后继续修改再提交

---

## 二、设计目标

1. **市场 Skill 任何内容变更都需审核** — 已 listed 的 Skill 更新 tags/description/content 后，变更不应直接生效
2. **编辑和上传行为一致** — 两种更新方式走同一套审核流程
3. **不影响非市场 Skill** — 个人/组织内部的 Skill 编辑保持现有行为
4. **不破坏已有版本** — 审核期间，市场用户看到的仍是旧版本内容
5. **最小改动** — 尽量复用现有 `marketplace_status` 状态机

---

## 三、设计方案

### 3.1 核心思路：引入「更新草稿」机制

在 `skills` 表中增加一个 `draft_content` JSONB 列，用于暂存待审核的更新内容。

```
已上架市场 Skill (listed)
    │
    ├─ 作者编辑 tags/description/content
    │     │
    │     └─ 不直接更新主字段，写入 draft_content
    │         marketplace_status → pending_update
    │
    ├─ 市场 Admin/Reviewer 审核
    │     ├─ 通过：draft_content → 合并到主字段，marketplace_status → listed
    │     └─ 驳回：清空 draft_content，marketplace_status → listed（恢复）
    │
    ├─ 作者取消更新
    │     └─ 清空 draft_content，marketplace_status → listed
    │
    └─ 作者上传新版本 ZIP
          │
          └─ 创建新版本行，status = draft
              marketplace_status = pending_review（走市场审核）
              审核通过后旧版本自动 delisted
```

### 3.2 数据模型变更

#### 3.2.1 `skills` 表新增字段

```sql
ALTER TABLE skills ADD COLUMN draft_content JSONB DEFAULT NULL;
-- draft_content 结构示例：
-- {
--   "description": "新描述",
--   "tags": ["new", "tags"],
--   "content": "新的内容..."
-- }
```

#### 3.2.2 `marketplace_status` 新增值

```sql
-- 在已有 pending_delist 基础上，再增加 pending_update：
ALTER TABLE skills DROP CONSTRAINT IF EXISTS chk_marketplace_status;
ALTER TABLE skills ADD CONSTRAINT chk_marketplace_status CHECK (
    marketplace_status IS NULL
    OR marketplace_status IN (
        'pending_review',   -- 新 Skill 等待市场审核
        'listed',           -- 已上架
        'rejected',         -- 审核驳回
        'delisted',         -- 已下架
        'unlisted',         -- 已取消上架
        'pending_delist',   -- 申请下架等待审核（已实现）
        'pending_update'    -- 【新增】更新等待审核
    )
);
```

### 3.3 状态流转

```
                      ┌─────────────┐
                      │   listed     │ ← 市场可见
                      └──┬───┬───┬──┘
                         │   │   │
          作者编辑 tags/  │   │   │  作者上传新版本
          desc/content    │   │   │
                         ▼   │   ▼
               ┌────────────┐│ ┌──────────────────┐
               │pending_update│ │ 新版本行(status=draft) │
               │(draft_content│ │ marketplace_status=  │
               │ 暂存变更)    ││ │ pending_review       │
               └──┬──┬──┬──┘│ └────────┬──────────┘
                  │  │  │   │          │
         Admin审核│  │  │   │    Admin审核│
           ┌──────┴┐ │  │   │     ┌─────┴──────┐
           ▼       ▼ ▼  ▼   │     ▼            ▼
         通过    驳回 取消/   │   通过          驳回
           │       │ 上传ZIP │     │            │
           │       │ 申请下架│     ▼            ▼
           ▼       ▼ 自动取消│  新版本listed   新版本rejected
      draft_content 清空    │  旧版本→archived  旧版本仍listed
      合并到主字段 draft_content │  搜索索引更新
      → listed   → listed     │  skills/{name}/覆盖
                               │
               作者申请下架     │
                    │          │
                    ▼          │
             pending_delist    │
                 ┌──┴──┐       │
                 ▼     ▼       │
              delisted 恢复listed│
```

### 3.4 规则矩阵

| 当前 marketplace_status | 操作 | 结果 |
|---|---|---|
| `NULL`（未提交市场） | 编辑 tags/desc/content | **直接生效**（现有行为不变） |
| `rejected` | 编辑 tags/desc/content | **直接生效**（现有行为不变） |
| `delisted` | 编辑 tags/desc/content | **直接生效**（已下架，无需审核） |
| `listed` | 编辑 tags/desc/content | → `pending_update`，变更写入 `draft_content` |
| `listed` | 上传新版本 ZIP | 创建新版本行，`status=draft`，`marketplace_status=pending_review`；审核通过后旧版本自动清理 |
| `pending_review` | 编辑 tags/desc/content | **直接生效**（还在审核中，无需额外审核） |
| `pending_update` | 再次编辑 tags/desc/content | 覆盖 `draft_content`，不创建新的审核记录 |
| `pending_update` | 作者取消 | 清空 `draft_content`，`marketplace_status` → `listed` |
| `pending_update` | 上传新版本 ZIP | 自动取消 pending_update（清空 draft_content，恢复 listed），然后正常创建新版本行 |
| `pending_update` | 申请下架 | 自动取消 pending_update（清空 draft_content，恢复 listed），然后进入 pending_delist 流程 |
| `pending_delist` | 编辑 tags/desc/content | 拒绝，提示「有待处理的下架申请」 |
| `pending_delist` | 上传新版本 ZIP | 拒绝，提示「有待处理的下架申请」 |

### 3.5 API 变更

#### 3.5.1 修改 `update_skill_handler`

```
PUT /api/v1/skills/:id

当前逻辑：
  → 直接更新 DB 主字段

新逻辑：
  if marketplace_status == "listed":
      → 写入 draft_content
      → 设 marketplace_status = "pending_update"
      → 返回 { message: "Update submitted for review", marketplace_status: "pending_update" }
  else if marketplace_status == "pending_update":
      → 覆盖 draft_content（允许再次编辑）
      → 返回 { message: "Draft updated", marketplace_status: "pending_update" }
  else:
      → 直接更新 DB 主字段（现有行为不变）
```

#### 3.5.2 新增 API

```
POST /api/v1/admin/marketplace/:id/approve-update   — 批准更新
POST /api/v1/admin/marketplace/:id/reject-update     — 驳回更新
POST /api/v1/skills/:id/cancel-update                — 作者取消更新
```

**approve-update 逻辑**：
```
1. require_marketplace_admin（Admin / Reviewer 均可）
2. 检查 marketplace_status == "pending_update"
3. 读取 draft_content
4. 将 draft_content 中的字段合并到 skills 主字段（description/tags/content）
5. 清空 draft_content
6. marketplace_status → "listed"
7. 更新搜索索引
8. 审计日志
```

**reject-update 逻辑**：
```
1. require_marketplace_admin（Admin / Reviewer 均可）
2. 检查 marketplace_status == "pending_update"
3. 清空 draft_content
4. marketplace_status → "listed"
5. 审计日志
```

**cancel-update 逻辑**：
```
1. 检查当前用户是 Skill 作者（owner）
2. 检查 marketplace_status == "pending_update"
3. 清空 draft_content
4. marketplace_status → "listed"
5. 审计日志
```

#### 3.5.3 修改读取接口

`GET /api/v1/skills/:id` 和 `GET /api/v1/marketplace` 等**市场公开接口**：
- 始终返回主字段内容（不受 `draft_content` 影响），确保市场用户看到的是稳定版本
- **市场列表接口**（`list_marketplace_listed`）查询条件改为 `marketplace_status IN ('listed', 'pending_update')`，确保 `pending_update` 期间 Skill 不会从市场消失

`GET /api/v1/skills/:id`（作者/管理员查看自己 Skill 详情时）：
- 如果 `marketplace_status == "pending_update"` 且有 `draft_content`，在响应中额外返回 `draft` 字段：
  ```json
  {
    "metadata": { ... },
    "content": "旧内容",
    "draft": {
      "description": "新描述",
      "tags": ["new", "tags"],
      "content": "新内容"
    }
  }
  ```

#### 3.5.4 上传新版本 ZIP 与旧版本清理

`POST /api/v1/skills/upload` 当前行为已是创建新版本 + `status=pending_review`，**无需修改核心流程**。

需要增加：

**a) 新版本审核通过时的清理逻辑**（修改 `marketplace_review_approve_handler`）：

```
1. 当前 Skill 审核通过 → marketplace_status = "listed"
2. 查找同一 skill_name 的旧 listed 版本（id != 当前 id，marketplace_status = 'listed'）
3. 如果存在旧版本：
   a. 旧版本 marketplace_status → "delisted"
   b. 旧版本 visibility → pre_marketplace_visibility（或 "private"）
   c. 旧版本从搜索索引中删除
   d. 旧版本 status → "archived"（新增状态，标记为已归档）
4. sync_skill_files_from — 用新版本文件覆盖 skills/{name}/ 目录
5. 更新搜索索引（新版本内容）
```

**b) 旧版本归档状态**：

新增 `status` 值 `"archived"`，表示该版本已被新版本替代：

```sql
-- 扩展 skills.status CHECK 约束
ALTER TABLE skills DROP CONSTRAINT IF EXISTS chk_skills_status;
ALTER TABLE skills ADD CONSTRAINT chk_skills_status CHECK (
    status IN ('draft', 'pending_review', 'approved', 'rejected', 'published', 'archived')
);
```

同时更新 Rust 代码中 `VALID_STATUSES` 常量（`src/db/repositories/skill.rs`）：
```rust
const VALID_STATUSES: [&str; 6] = [
    "draft", "pending_review", "approved", "rejected", "published", "archived"
];
```

**c) 市场列表无需去重**：

旧版本 `status=archived` + `marketplace_status=delisted` 后，`list_marketplace_listed` 的查询条件 `WHERE status='published' AND marketplace_status IN ('listed','pending_update')` 自动过滤掉旧版本，**不需要 `DISTINCT ON`**。

**d) 旧版本数据保留与恢复**：

旧版本**不删除**，保留在 `skills` 表和 `skill_versions` 表中，用途：
- 管理员可查看 Skill 的完整版本历史（`GET /api/v1/skills/:name/versions`）
- 如果新版本有问题，可**从旧版本回退**：
  1. 从 `skill_versions` 表找到旧版本的 `git_tag`
  2. `git checkout {tag}` 恢复旧版本文件
  3. 创建新的 patch 版本行（`status=draft`, `marketplace_status=pending_review`）
  4. 走市场审核流程
  5. 审核通过后新行 `listed`，当前的 `archived` 行保持不变

**e) 清理后各组件状态**：

| 组件 | 旧版本 | 新版本 |
|---|---|---|
| `skills` 表 | status=archived, marketplace_status=delisted | status=published, marketplace_status=listed |
| 搜索索引 | 已删除 | 已更新 |
| `skills/{name}/` 文件 | 已覆盖（不存在） | 最新文件 |
| `git-repos/` Git 仓库 | 保留（tag 历史可查） | 最新 commit + tag |
| 市场列表 | 不展示 | 展示 |
| 下载 | 不可用 | 可用 |

### 3.6 前端变更

| 页面 | 变更 |
|---|---|
| **SkillDetail** | `listed` 状态时编辑 tags/desc/content → 确认弹窗「变更将提交市场审核，审核通过后生效」 |
| **SkillDetail** | `pending_update` 状态时显示「Update Pending Review」标签，展示 draft 内容预览 |
| **SkillDetail** | `pending_update` 状态时允许再次编辑（覆盖 draft）和取消（恢复 listed） |
| **SkillDetail** | `pending_delist` 状态时禁用编辑按钮 |
| **Review 页面** | 新增「Pending Updates」审核队列 tab |
| **Review 页面** | 审核时展示 diff：旧内容 vs draft_content |

### 3.7 搜索索引处理

- `pending_update` 期间，搜索索引**不变**（仍索引旧内容）
- 审核通过后，用合并后的新内容更新搜索索引
- 驳回 / 取消后搜索索引不变

### 3.8 审计日志

所有操作写入审计日志：

| 操作 | action 字段 |
|---|---|
| 提交更新 | `marketplace_update_submitted` |
| 批准更新 | `marketplace_update_approved` |
| 驳回更新 | `marketplace_update_rejected` |
| 取消更新 | `marketplace_update_cancelled` |

### 3.9 现有代码兼容性改造

以下现有代码需要同步修改，否则新增状态会引发 bug：

#### 3.9.1 `status` 过滤需排除 `archived`

**文件**：`src/db/repositories/skill.rs`

| 方法 | 行号 | 当前代码 | 修改 |
|---|---|---|---|
| `list_sorted` | 273 | `WHERE s.status != 'rejected'` | → `WHERE s.status NOT IN ('rejected', 'archived')` |
| `count` | 296 | `WHERE status != 'rejected'` | → `WHERE status NOT IN ('rejected', 'archived')` |

理由：`archived` 状态的旧版本不应出现在任何用户可见列表中。

#### 3.9.2 `VALID_STATUSES` 常量更新

**文件**：`src/db/repositories/skill.rs` 第 10 行

```rust
// 当前
const VALID_STATUSES: [&str; 5] = ["draft", "pending_review", "approved", "rejected", "published"];
// 改为
const VALID_STATUSES: [&str; 6] = ["draft", "pending_review", "approved", "rejected", "published", "archived"];
```

#### 3.9.3 delist 相关 handler 的前置条件扩展

以下 handler 的 `marketplace_status` 检查需要从 `== "listed"` 扩展为 `IN ("listed", "pending_update")`：

| Handler | 行号 | 原因 |
|---|---|---|
| `marketplace_delist_handler` | 3728 | `pending_update` 状态也应能下架（自动取消 pending_update） |
| `admin_unpublish_skill_handler` | 3279 | 同上 |
| `request_marketplace_delist_handler` | 7341 | 作者在 `pending_update` 期间应能申请下架 |

#### 3.9.4 `process_upload` 区分首次/更新上传

**文件**：`src/services/skill_git.rs` 第 247-259 行

```rust
// 当前：每次都使用 ZIP 中的 tags/description
let new_skill = NewSkill {
    description: metadata.description.clone(),
    tags: metadata.tags.clone(),
    ...
};

// 改为：首次上传用 ZIP 值，更新上传继承 DB 值
let (desc, tgs) = if latest_version.is_none() {
    // 首次上传：使用 ZIP 中的值
    (metadata.description.clone(), metadata.tags.clone())
} else {
    // 更新上传：从 DB 继承当前最新版本的 tags/description
    let current = skill_repo.find_latest_by_name(&metadata.name).await?;
    (current.map(|s| s.description).unwrap_or_default(),
     current.map(|s| s.tags).unwrap_or_default())
};
```

#### 3.9.5 `SkillRow` / `SkillMetadataRow` 加 `draft_content` 字段

所有 SELECT 查询的列列表需要加上 `s.draft_content`，`Skill` / `SkillMetadata` 结构体也需要对应字段。

---

## 四、文件改动清单

| 文件 | 改动 |
|---|---|
| `src/db/migrations/035_add_draft_content.sql` | **新增** — draft_content 列 + marketplace_status/status CHECK 约束扩展 |
| `src/db/repositories/skill.rs` | SkillRow/SkillMetadataRow 加 `draft_content`；VALID_STATUSES 加 `archived`；`list_sorted`/`count` 排除 `archived`；所有 SELECT 加 `draft_content` 列；新增 `save_draft_content`、`clear_draft_content`、`apply_draft_content` 方法 |
| `src/api/handlers.rs` | 修改 `update_skill_handler`；新增 `approve_update_handler`、`reject_update_handler`、`cancel_update_handler`；修改 `marketplace_review_approve_handler` 增加旧版本清理；修改 `marketplace_delist_handler`/`admin_unpublish_skill_handler`/`request_delist` 前置条件兼容 `pending_update` |
| `src/api/routes.rs` | 新增 3 条路由 |
| `src/services/registry.rs` | `update_skill_internal` 增加 `listed` 判断 |
| `src/services/skill_git.rs` | `process_upload` 区分首次/更新上传，更新时继承 DB 的 tags/description |
| `src/models/skill.rs` | Skill/SkillMetadata 增加 `draft_content` 字段 |
| `admin/src/routes/SkillDetail.svelte` | 编辑区分 listed/pending_update 状态；新增取消更新按钮；pending_delist 禁用编辑 |
| `admin/src/routes/Review.svelte` | 新增 pending_update 审核队列 tab |
| `admin/src/lib/api.js` | 新增 `approveUpdate`、`rejectUpdate`、`cancelUpdate` |

---

## 五、ZIP 上传更新与文件预览

### 5.1 问题

用户通过 ZIP 上传新版本时，`skills/{name}/` 目录仍被旧版本占用（用于市场 install/download）。审核期间新版本文件只存在于 Git 仓库中（通过 tag 可访问），但用户无法下载测试。

### 5.2 当前文件读取架构

| 接口 | 文件来源 | 依赖 `skills/{name}/` |
|---|---|---|
| `list_skill_files` | `git ls-tree {tag}` | ❌ 不依赖 |
| `get_skill_file` | `git show {tag}:{path}` | ❌ 不依赖 |
| `download_skill` | `skills/{name}/` 目录 | ✅ 依赖 |
| MCP install | `skills/{name}/` 目录 | ✅ 依赖 |

### 5.3 方案：全部走 Git，不新建文件夹

**结论：Git 方案更高效，不需要新建文件夹。**

理由：
- `list_skill_files` / `get_skill_file` 已通过 Git tag 读取，任意版本随时可预览，零额外成本
- 新建文件夹需要管理生命周期、磁盘翻倍、并发控制，复杂度高

**唯一需改动的**：`download_skill_handler` 增加从 Git 按 tag 打包的能力：

```
if 版本是 listed 状态:
    → 读 skills/{name}/ 目录打包（现有逻辑，不变）
else（审核中的新版本）:
    → git checkout {tag} 到临时目录 → 打包 → 清理临时目录
```

这样审核期间作者也能下载新版本测试，审核通过后 `sync_skill_files_from` 覆盖 `skills/{name}/` 恢复正常。

**ZIP 上传不解析 tags/description**：

上传 ZIP 时，`name` 从 SKILL.md frontmatter 提取（用于确定 skill_name 和版本递增），但 `tags` 和 `description` 的处理逻辑：
- **首次上传**（该 skill_name 无任何版本）：正常解析 tags/description，作为初始值
- **已有旧版本**（更新场景）：不覆盖 tags/description，继承 DB 中当前最新版本的值

原因是上传是更新文件内容，tags/description 应该由用户通过编辑接口单独管理。Git 仓库正常提交所有文件（包括 SKILL.md），但 DB 中的 tags/description 保持不变。

---

## 六、已确认决策

1. ✅ **`pending_update` 期间允许作者再次编辑** — 覆盖 `draft_content`，不创建新的审核记录
2. ✅ **`pending_update` 期间允许作者取消** — 清空 `draft_content`，恢复 `listed`，新增 `cancel-update` API
3. ✅ **新版本审核通过后旧版本清理**：
   - 旧版本 `marketplace_status` → `delisted`，`status` → `archived`
   - 旧版本从搜索索引删除
   - 旧版本数据保留在 DB 中（`skills` 表 + `skill_versions` 表），可通过版本历史查看
   - 市场列表无需去重（`archived` + `delisted` 自动过滤）
   - `skills/{name}/` 目录用新版本文件覆盖
   - 如需回退，从 `skill_versions` 找到旧版本 Git tag → `git checkout` → 创建新版本走审核
4. ✅ **审核通过时记录 changelog** — 在 `skill_versions` 表中记录变更内容摘要
5. ✅ **回退操作需走审核** — `rollback_skill_handler` 回退创建的新版本不再直接设为 `published + marketplace`，改为 `pending_review` 走市场审核

---

## 七、不做的

- ❌ 不改变非市场 Skill 的编辑行为（直接生效）
- ❌ 不引入 Git 分支管理
- ❌ 不在 `pending_update` 期间阻塞其他独立操作（上传新版本/申请下架自动取消 pending_update）
