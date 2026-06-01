# Group 权限设计方案

## 一、现状分析

### 1.1 当前权限体系架构

项目有 **5 个角色层级**，每个层级有若干角色名，通过 `role_permissions` 表绑定权限码：

| 层级 | 角色 | 典型权限 |
|------|------|---------|
| `system` | `super_admin` | 全局所有权限（tenant CRUD、marketplace、audit 等） |
| `system` | `marketplace_admin` | 市场管理权限 |
| `organization` | `owner` | 组织内全部 42 项权限 |
| `organization` | `admin` | 组织管理（无 delete、transfer） |
| `organization` | `reviewer` | 技能评审 + 只读 |
| `organization` | `developer` | 技能创建/安装/提审 + 只读 |
| `organization` | `member` | 只读 + 安装技能 |
| `group` | `lead` | 组内全部 21 项权限（CRUD 组、管理成员、技能操作） |
| `group` | `member` | 组内 9 项权限（只读、安装技能、提交评审） |
| `personal` | `user` | 个人技能管理 |

### 1.2 权限评估流程

入口在 `PermissionService.build_context()` → `PermissionService.has_permission()`：

```
build_context(identity_id):
  1. 查 system_role_assignments → system_roles
  2. 查 org_memberships → org_roles  (org_id, role_name)
  3. group_roles = Vec::new()  ← 目前为空！
```

```
has_permission(ctx, permission_code, resource):
  1. super_admin → 直接通过
  2. 遍历所有 role_entries（system + org + group）
  3. 对每个 entry，查 role_permissions 表获取该角色的权限列表
  4. 匹配 permission_code，并验证 scope_restriction
  5. 对于 group 级角色，还会查 group_permission_overrides 看是否有覆盖
```

### 1.3 已存在但未启用的基础设施

| 组件 | 状态 | 说明 |
|------|------|------|
| `memberships` 表 | ✅ 已有 | `(identity_id, group_id, role)` 存储组成员关系 |
| `group_permission_overrides` 表 | ✅ 已有 | `(group_id, role_name, permission_code, granted)` 组级权限覆盖 |
| `GroupPermissionOverrideRepository` | ✅ 已有 | 完整的 CRUD 方法 |
| `PermissionService` | ✅ 已有 | `has_permission` 已写好了 group 角色检查逻辑 |
| `PermissionContext.group_roles` | ❌ 空向量 | `build_context()` 中固定为 `Vec::new()` |
| API 路由 | ❌ 缺失 | 无任何管理 group_permission_overrides 的接口 |
| 前端页面 | ❌ 缺失 | 无 group 权限管理 UI |

### 1.4 核心问题

1. **`build_context()` 未加载 group_roles**：组成员身份的角色信息从未被查询，导致 group 级权限（lead/member）永远不会生效
2. **无 API 管理 group_permission_overrides**：虽然有表结构和仓库层，但无法通过接口增删改
3. **创建 group 时无默认权限设置**：创建组后，其 lead/member 角色的权限用的是全局 `role_permissions` 表默认值，但无法自定义
4. **组内 vs 组外权限边界模糊**：没有明确区分"组成员通过 group 角色获得的权限"和"组织成员通过 org 角色获得的权限"

---

## 二、组内权限 vs 组外权限

### 2.1 权限来源

一个用户对 group 资源的访问权限来自 **两个来源**：

| 来源 | 范围 | 示例 |
|------|------|------|
| **组织身份** | 对该组织下所有 group 生效 | org owner/admin 可以管理组织内任何 group |
| **组成员身份** | 仅对该 group 生效 | group lead 只能管理自己所在的 group |

### 2.2 权限合并规则

**取并集**：用户的最终权限 = org 角色权限 ∪ group 角色权限（含 overrides）

```
用户 A 在 org-X 中是 member（org 角色）
用户 A 在 group-Y 中是 lead（group 角色）

对 group-Y 的操作：
  - group:read     ← org member 有 + group lead 有 → ✅
  - group:update   ← org member 无 + group lead 有 → ✅
  - group:delete   ← org member 无 + group lead 有 → ✅
  - org:delete     ← org member 无 + group lead 无 → ❌
```

### 2.3 权限覆盖（Overrides）

`group_permission_overrides` 表允许**针对特定 group 的特定角色**调整权限：

| 场景 | 操作 |
|------|------|
| 限制 group lead 不能删除 group | `INSERT (group_id, 'lead', 'group:delete', false)` |
| 允许 group member 创建 skill | `INSERT (group_id, 'member', 'skill:create', true)` |

覆盖规则：
- `granted = true`：即使全局 `role_permissions` 里没有，也授予该权限
- `granted = false`：即使全局 `role_permissions` 里有，也拒绝该权限

### 2.4 scope_restriction 的 group 含义

当 `role_permissions` 中 `scope_restriction = 'group'` 时，表示该权限**仅在用户所属 group 范围内生效**。例如：

- group lead 有 `skill:delete` 且 scope_restriction='group'
  → lead 只能删除**关联到该 group 的 skill**，不能删除组织内其他 skill

### 2.5 scope_restriction = "group" 的校验实现

`has_permission()` 中 `scope_restriction = "group"` 分支原有空实现 `{}`，已于 Phase 5 修复。

#### 修复前的问题

```rust
// 问题 1: scope_restriction = "group" 为空实现，不校验任何资源归属
"group" => {}  // 任何 group lead 都可以操作任意 group

// 问题 2: override 检查遍历所有 group_roles，而非当前 scope_id 对应的 group
for (group_id, group_role) in &ctx.group_roles {
    // 如果任意 group 有 grant override，就通过
    // 这导致 group-A 的 override 影响 group-B 的权限判断
}
```

场景示例：用户在 group-A 是 lead，在 group-B 是 member。当检查 `group:delete` 权限时：
- 问题 1：即使操作目标是 group-B，只要用户是 group-A 的 lead 就通过（scope 不隔离）
- 问题 2：如果用户在任何 group 有 grant override，就直接返回 true（跨组污染）

#### 修复方案

```rust
// 修复 1: scope_restriction = "group" 校验资源归属
"group" => {
    if let Some(scope_group_id) = scope_id {
        // 优先匹配 resource.group_id（显式传入的 group ID）
        if let Some(resource_group_id) = resource.group_id {
            if *scope_group_id != resource_group_id {
                continue;  // 资源不属于当前 group，跳过该角色
            }
        // 回退匹配 resource.owner_id（当 owner_type = "group" 时）
        } else if let Some(owner_id) = resource.owner_id {
            if *scope_group_id != owner_id {
                continue;
            }
        }
    }
}

// 修复 2: override 检查仅针对当前 group
if role_level == "group" {
    if let Some(current_group_id) = scope_id {
        let override_result = self
            .group_perm_override_repo
            .find_by_group_role_permission(
                *current_group_id,  // 仅查询当前 group 的覆盖
                role_name,
                permission_code,
            ).await?;
        // ...
    }
}
```

#### ResourceScope 新增字段

为支持 group 级 scope 校验，`ResourceScope` 新增 `group_id` 字段：

```rust
pub struct ResourceScope {
    pub owner_type: Option<String>,
    pub owner_id: Option<Uuid>,
    pub author_identity_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub group_id: Option<Uuid>,  // 新增：目标 group 的 ID
}
```

Handler 中构造 `ResourceScope` 时按需传入 `group_id`：
```rust
let resource = ResourceScope {
    owner_type: Some("group".to_string()),
    owner_id: Some(target_group_id),
    group_id: Some(target_group_id),
    // ...
    ..Default::default() // 其他字段为 None
};
```

---

## 三、设计方案

### 3.1 修复 `build_context()` 加载 group_roles

**文件**：`src/services/permission.rs`

```rust
// 当前代码
let group_roles: Vec<(Uuid, String)> = Vec::new();

// 修改为
let group_roles = self
    .group_membership_repo  // 新增
    .list_user_groups(identity_id)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;
```

需要新增 `GroupMembershipRepository` 或复用 `GroupRepository` 的 `list_members` 方法，按 `identity_id` 反查其所有 group 成员关系。

### 3.2 新增 API 路由

在 `routes.rs` 的 group 路由组下新增：

```rust
// Group 权限覆盖管理
.route("/api/v1/admin/groups/:id/permissions", get(list_group_permissions_handler))
.route("/api/v1/admin/groups/:id/permissions", put(update_group_permission_handler))
.route("/api/v1/admin/groups/:id/permissions/:code", delete(delete_group_permission_handler))
```

### 3.3 新增 Handler 方法

| Handler | 功能 |
|---------|------|
| `list_group_permissions_handler` | 返回 group 的全部 permission_overrides（按 role 分组） |
| `update_group_permission_handler` | upsert 一条 override（body: `{role_name, permission_code, granted}`） |
| `delete_group_permission_handler` | 删除一条 override |

### 3.4 创建 Group 时设置默认权限

`create_group_handler` 不需要改动——默认权限由 `role_permissions` 表的 `group` 层级数据提供，创建 group 时无需额外写入。

但如果需要**创建时自定义权限**，可以扩展 `CreateGroupRequest`：

```rust
pub struct CreateGroupRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub group_type: Option<String>,
    pub organization_id: Uuid,
    // 新增
    pub permissions: Option<Vec<GroupPermissionOverrideInput>>,
}
```

### 3.5 前端 GroupDetail 权限管理 Tab

在 `GroupDetail.svelte` 中新增一个 **Permissions** 标签页，展示当前 group 的权限覆盖配置：

```
┌─ Group Detail ───────────────────────────────┐
│ [Info] [Members] [Permissions] ← tabs        │
│                                               │
│  Role: lead                                   │
│  ┌─────────────────────────────────────────┐  │
│  │ Permission          Default   Override   │  │
│  │ group:read          ✅        -          │  │
│  │ group:update        ✅        -          │  │
│  │ group:delete        ✅        🔴 Denied  │  │
│  │ skill:create        ❌        🟢 Granted │  │
│  │ ...                                      │  │
│  └─────────────────────────────────────────┘  │
│                                               │
│  Role: member                                 │
│  ┌─────────────────────────────────────────┐  │
│  │ ...                                      │  │
│  └─────────────────────────────────────────┘  │
└───────────────────────────────────────────────┘
```

### 3.6 权限检查流程（完整版）

```
用户请求操作 group 资源
        │
        ▼
┌─ has_permission(ctx, "group:delete", resource) ─┐
│                                                   │
│  1. super_admin? → ✅ 直接放行                     │
│                                                   │
│  2. 遍历 org_roles:                               │
│     org owner/admin 有 group:delete?              │
│     → 检查 scope_restriction 是否匹配             │
│     → 有则 ✅                                      │
│                                                   │
│  3. 遍历 group_roles:                             │
│     group lead/member 有 group:delete?            │
│     → 检查 group_permission_overrides 是否拒绝    │
│     → 检查 scope_restriction 是否匹配             │
│     → 有则 ✅                                      │
│                                                   │
│  4. 都不满足 → ❌                                  │
└───────────────────────────────────────────────────┘
```

---

## 四、实施步骤

### Phase 1：修复后端权限加载（1 个文件）

- [x] `src/services/permission.rs`：`build_context()` 中加载 group_roles
- [x] 新增 `GroupRepository::list_user_group_memberships()` 方法

### Phase 2：新增 API 路由（3 个文件）

- [x] `src/api/handlers.rs`：新增 3 个 handler（list/update/delete permissions）
- [x] `src/api/models.rs`：新增请求/响应体
- [x] `src/api/routes.rs`：注册 3 条路由

### Phase 3：前端 Group 权限管理页面（2 个文件）

- [x] `admin/src/routes/GroupDetail.svelte`：新增 Permissions 标签页
- [x] `admin/src/lib/api.js`：新增 API 调用方法

### Phase 4：创建 Group 时可选权限（按需）

- [x] `src/api/models.rs`：扩展 `CreateGroupBody`，增加 `PermissionOverrideInput`
- [x] `src/api/handlers.rs`：`create_group_handler` 中批量写入 overrides，添加默认权限查询 handler
- [x] `src/api/routes.rs`：注册 `GET /api/v1/groups/default-permissions`
- [x] `admin/src/routes/Groups.svelte`：创建组弹窗中可配置权限

### Phase 5：修复 scope_restriction = "group" 的组隔离校验（1 个文件）

- [x] `src/services/permission.rs`：
  - `ResourceScope` 新增 `group_id` 字段
  - `scope_restriction = "group"` 分支实现资源归属校验
  - override 检查改为仅查询当前 `scope_id` 对应的 group，而非遍历所有 groups

---

## 五、API 接口设计

### 5.1 获取 Group 权限配置

```
GET /api/v1/admin/groups/:id/permissions

Response:
{
  "data": {
    "lead": [
      { "permission_code": "group:read", "granted": true, "is_default": true },
      { "permission_code": "group:delete", "granted": false, "is_default": true }
    ],
    "member": [
      { "permission_code": "group:read", "granted": true, "is_default": true }
    ]
  }
}
```

### 5.2 设置 Group 权限覆盖

```
PUT /api/v1/admin/groups/:id/permissions
Body:
{
  "role_name": "lead",
  "permission_code": "group:delete",
  "granted": false
}

Response: 200 OK
```

### 5.3 删除 Group 权限覆盖（恢复默认）

```
DELETE /api/v1/admin/groups/:id/permissions/group:delete?role_name=lead

Response: 200 OK
```

---

## 六、关键设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 权限合并方式 | 取并集 | 用户加入 group 应该获得额外权限，不应被 org 角色限制 |
| 覆盖粒度 | role + permission_code | 精确控制，防止误操作 |
| 默认权限 | 使用 role_permissions 全局定义 | 保持一致性，减少创建 group 时的配置负担 |
| 未覆盖时行为 | 使用全局默认值 | 简单直观，只有在需要定制时才写入 overrides |
| 前端展示 | 合并视图（默认值 + 覆盖状态） | 一目了然看到哪些权限被修改了 |