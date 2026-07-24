# 前端权限体系全面接入方案

> 版本: 1.0 | 日期: 2026-07-17 | 对应后端版本: 0.3.0

---

## 一、现状分析

### 当前数据流

```
登录 POST /api/v1/auth/login
  ↓ 后端返回
{
  token: "eyJ...",           // JWT，roles=["user"]，不含系统角色
  user: {
    id, username, is_admin,  // ← 前端只取 is_admin 布尔值
    organizations: [{id, name, slug, role}]  // ← 前端未使用！
  }
}
  ↓ 前端存储
auth = { token, username, is_admin: true/false }  // 只有这3个字段
```

### 核心矛盾

| 后端已有的信息 | 前端是否使用 |
|---------------|:--:|
| JWT `roles`（system级角色） | ❌ 未解析 |
| JWT `identity_id` | ❌ 未使用 |
| 登录响应 `organizations`（含 org role） | ❌ 完全丢弃 |
| 登录响应 `is_admin` → 映射 super_admin | ⚠️ 仅用于是否进入 admin 布局 |

---

## 二、后端改动（总计 3 处）

### 2.1 登录响应增加 `system_roles` + `tenant_roles`

**文件：** `src/api/models.rs` — `UserLoginResponse` / `UserInfoResponse`

```rust
pub struct UserInfoResponse {
    // ... 现有字段保持不变 ...
    pub is_admin: bool,
    pub organizations: Vec<UserOrgInfo>,
    // 🆕 新增
    pub system_roles: Vec<String>,          // ["super_admin"] / ["marketplace_admin"] / []
    pub tenant_roles: Vec<TenantRoleInfo>,  // [{tenant_id, role_name}]
}

pub struct TenantRoleInfo {
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub role_name: String,  // "tenant_admin"
}
```

**文件：** `src/api/handlers.rs` — `user_login_handler`

登录时调用 `PermissionService::build_context()` 获取当前用户的所有角色，写入响应。

### 2.2 新增 `/users/me/permissions` 端点（权限刷新）

**用途：** 页面刷新后重新获取权限上下文（不重新登录）

```
GET /api/v1/users/me/permissions
  ↓ 返回
{
  system_roles: ["super_admin"],
  tenant_roles: [{tenant_id, tenant_name, role_name}],
  org_roles: [{org_id, org_name, role_name}],
  group_roles: [{group_id, group_name, role_name}],
  permissions: ["skill:create", "skill:read", ...]
}
```

### 2.3 JWT 放宽 admin 路由校验

当前 admin 路由通过 `roles` 中的 `"admin"` 判断，迁移到 `PermissionService::has_permission(ctx, "system:admin:access", None)`，使得 `super_admin` / `marketplace_admin` 都能正确通过。

**文件：** `src/api/jwt.rs` — `AgentContext::require_admin()` 改为调用 PermissionService。

---

## 三、前端改动

### 3.1 核心架构：数据驱动的 tab 渲染

#### 设计原则

**不引入 Guard 组件、不写逐页 `{#if}`，而是用声明式配置表驱动一切：**

```
权限配置表 (纯数据)
  ↓
layout 引擎根据用户权限自动过滤
  ↓
只渲染用户有权访问的 tab + 内容
```

**好处：**
- 新增页面只需加一行配置，不需要改任何 `.svelte` 逻辑
- 所有权限判断集中在一处，不会散落在 20 个文件中
- 页面内部只知道「我有数据就渲染」，不需要关心权限

#### 3.1.1 导航/路由配置表 (`src/config/nav-routes.js`)

```js
// 每个路由项声明自己需要什么权限，layout 自动过滤
export const navRoutes = [
  {
    // ===== Super Admin 专属 =====
    id: 'overview',
    label: 'Overview',
    icon: 'dashboard',
    tabs: [
      { href: '/stats', label: 'Dashboard', permission: null, systemRoles: ['super_admin'] },
    ]
  },
  {
    id: 'system',
    label: 'System',
    icon: 'settings',
    tabs: [
      { href: '/tenants',     label: 'Tenants',     permission: 'tenant:read' },
      { href: '/identities',  label: 'Identities',  permission: 'system:admin:access' },
      { href: '/api-keys',    label: 'API Keys',    permission: 'apikey:read' },
      { href: '/sessions',    label: 'Sessions',    permission: 'system:admin:access' },
      { href: '/audit',       label: 'Audit Logs',  permission: 'audit:read_global' },
      { href: '/settings',    label: 'Settings',    permission: 'system:admin:access' },
      { href: '/sandboxes',   label: 'Sandboxes',   permission: 'system:admin:access' },
    ]
  },
  {
    id: 'marketplace',
    label: 'Marketplace',
    icon: 'store',
    tabs: [
      { href: '/roles',       label: 'Roles',       permission: 'marketplace:manage' },
      { href: '/review',      label: 'Review',      permission: 'skill:approve_review' },
      { href: '/skills',      label: 'Skills',      permission: 'skill:read' },
    ]
  },
  {
    id: 'organizations',
    label: 'Organizations',
    icon: 'org',
    tabs: [
      { href: '/organizations', label: 'Organizations', permission: 'org:read' },
      { href: '/org-tools',     label: 'Org Tools',     permission: 'org:read' },
    ]
  },
  {
    id: 'groups',
    label: 'Groups',
    icon: 'group',
    tabs: [
      { href: '/groups', label: 'Groups', permission: 'group:read' },
    ]
  },
  {
    id: 'personal',
    label: 'Personal',
    icon: 'user',
    tabs: [
      { href: '/profile',     label: 'Profile',     permission: 'profile:read' },
      { href: '/my-api-keys', label: 'My API Keys', permission: 'apikey:create' },
    ]
  },
];
```

#### 3.1.2 权限 Store (`src/stores/permission.js`)

```js
// 仅存储 + 一个 hasPermission() 函数
import { writable, derived, get } from 'svelte/store';

const store = writable({
  systemRoles: [],        // ["super_admin"] / ["marketplace_admin"] / []
  tenantRoles: [],        // [{tenant_id, tenant_name, role}]
  orgRoles: [],           // [{org_id, org_name, role}]
  groupRoles: [],         // [{group_id, group_name, role}]
  permissions: new Set(), // 所有 permission_code 的集合
});

// ==================== 导出的纯函数 ====================

export function hasPermission(code) {
  if (!code) return true; // null permission = 不需要权限
  const s = get(store);
  if (s.systemRoles.includes('super_admin')) return true;
  return s.permissions.has(code);
}

export function hasSystemRole(role) {
  if (!role) return true;
  const s = get(store);
  if (s.systemRoles.includes('super_admin')) return true;
  return s.systemRoles.includes(role);
}

export function hasOrgRole(orgId, ...roles) {
  if (!orgId || roles.length === 0) return true;
  const s = get(store);
  if (s.systemRoles.includes('super_admin')) return true;
  return s.orgRoles.some(r => r.org_id === orgId && roles.includes(r.role));
}

// 过滤导航配置
export function filterNavRoutes(navRoutes) {
  const result = [];
  for (const group of navRoutes) {
    const visibleTabs = group.tabs.filter(tab => {
      if (tab.systemRoles) return hasSystemRole(tab.systemRoles[0]);
      return hasPermission(tab.permission);
    });
    if (visibleTabs.length > 0) {
      result.push({ ...group, tabs: visibleTabs });
    }
  }
  return result;
}

// 初始化（登录时调用）
export function init({ systemRoles, tenantRoles, orgRoles, groupRoles, permissions }) {
  store.set({
    systemRoles: systemRoles || [],
    tenantRoles: tenantRoles || [],
    orgRoles: orgRoles || [],
    groupRoles: groupRoles || [],
    permissions: new Set(permissions || []),
  });
}
```

#### 3.1.3 Nav.svelte 自动过滤

```svelte
<script>
  import { filterNavRoutes } from '../stores/permission.js';
  import { navRoutes } from '../config/nav-routes.js';
  
  // 一步到位：根据当前权限自动过滤
  $: visibleRoutes = filterNavRoutes(navRoutes);
</script>

{#each visibleRoutes as group}
  <div class="nav-group">
    <span class="nav-group-label">{group.label}</span>
    {#each group.tabs as tab}
      <a href={tab.href} class:active={currentPath === tab.href}>
        {tab.label}
      </a>
    {/each}
  </div>
{/each}
```

#### 3.1.4 App.svelte 简化

```svelte
<script>
  import { hasPermission, hasSystemRole } from '../stores/permission.js';
  
  // 判断布局层级
  $: layout = layoutForUser();
  
  function layoutForUser() {
    if (hasSystemRole('super_admin')) return 'super';
    if (hasSystemRole('marketplace_admin')) return 'marketplace';
    // ... tenant / org / personal
    return 'user';
  }
</script>

{#if $layout === 'user'}
  <UserLayout>
    <Route path="/user">...</Route>
  </UserLayout>
{:else}
  <AdminLayout>
    <!-- 路由仅声明式注册，不写 if -->
    <Route path="/tenants"><Tenants /></Route>
    <!-- Nav 已经过滤了不渲染这些入口，即便直接输 URL 后端也会检查 -->
  </AdminLayout>
{/if}
```

**关键点：前端不强制路由守卫**（后端兜底），前端只负责「不显示不该看的入口」。用户即便手动输入 URL，后端返回 403。

---

### 3.2 页面按钮级权限

#### 3.2.1 配置文件化 (`src/config/actions.js`)

```js
// 每个页面需要权限的操作声明
export const PAGE_ACTIONS = {
  Tenants: {
    create: 'tenant:create',
    delete: 'tenant:delete',
    edit:   'tenant:update',
  },
  Identities: {
    manage: 'system:admin:access',
  },
  Skills: {
    create:      'skill:create',
    edit:        'skill:update',
    delete:      'skill:delete',
    publish:     'marketplace:feature',
    unpublish:   'marketplace:unfeature',
    submitReview:'skill:submit_review',
  },
  Review: {
    approve: 'skill:approve_review',
    reject:  'skill:reject_review',
  },
  Groups: {
    create: 'group:create',
    delete: 'group:delete',
    edit:   'group:update',
    manageMembers: 'group:member_add',
  },
  // ... 其余页面同理
};
```

#### 3.2.2 页面使用方式（极其简单）

```svelte
<script>
  import { hasPermission } from '../stores/permission.js';
  import { PAGE_ACTIONS } from '../config/actions.js';
  const ACT = PAGE_ACTIONS.Tenants;  // 取本页配置
</script>

{#if hasPermission(ACT.create)}
  <button on:click={openCreateDialog}>Create Tenant</button>
{/if}

{#each tenants as t}
  <tr>
    <td>{t.name}</td>
    <td>
      {#if hasPermission(ACT.edit)}
        <button on:click={() => editTenant(t)}>Edit</button>
      {/if}
      {#if hasPermission(ACT.delete)}
        <button on:click={() => deleteTenant(t.id)}>Delete</button>
      {/if}
    </td>
  </tr>
{/each}
```

---

### 3.3 Tenant/Org 上下文过滤

```js
// 列表数据请求时带上身份作用域
function apiQueryParams() {
  const s = get(permissionStore);
  
  // super_admin: 不限制（== 拉全部）
  if (s.systemRoles.includes('super_admin')) return {};
  
  // tenant_admin: 限制 tenant_id
  if (s.tenantRoles.length > 0) {
    return { tenant_ids: s.tenantRoles.map(t => t.tenant_id) };
  }
  
  // org 角色: 限制 org_id
  if (s.orgRoles.length > 0) {
    return { org_ids: s.orgRoles.map(o => o.org_id) };
  }
  
  return {};
}
```

---

## 四、文件改动清单

### 后端（3 个文件）

| # | 文件 | 改动 |
|---|------|------|
| 1 | `src/api/models.rs` | `UserInfoResponse` 增加 `system_roles`/`tenant_roles`；新增 `TenantRoleInfo` |
| 2 | `src/api/handlers.rs` | `user_login_handler` 填充新字段；新增 `get_my_permissions_handler` |
| 3 | `src/api/routes.rs` | 注册 `/users/me/permissions` |

### 前端（9 个文件，含 4 个新建）

| # | 文件 | 改动类型 |
|---|------|----------|
| 4 | `admin/src/stores/permission.js` | **新建** — 权限 store |
| 5 | `admin/src/config/nav-routes.js` | **新建** — 导航路由配置表 |
| 6 | `admin/src/config/actions.js` | **新建** — 页面操作权限配置表 |
| 7 | `admin/src/stores/auth.js` | **修改** — 登录后联动 init permissionStore |
| 8 | `admin/src/components/Nav.svelte` | **修改** — 改用 `filterNavRoutes` |
| 9 | `admin/src/App.svelte` | **修改** — 五层布局自动判断 |
| 10 | `admin/src/routes/Login.svelte` | **修改** — 登录响应写入 permissionStore |
| 11 | `admin/src/routes/Tenants.svelte` | **修改** — 按钮加 `hasPermission()` |
| 12 | `admin/src/routes/Skills.svelte` | **修改** — 按钮加 `hasPermission()` |
| 13 | `admin/src/routes/Review.svelte` | **修改** — 按钮加 `hasPermission()` |
| 14 | `admin/src/routes/OrganizationDetail.svelte` | **修改** — 按钮加 `hasOrgRole()` |
| 15 | `admin/src/routes/Groups.svelte` | **修改** — 按钮加 `hasPermission()` |
| 16 | `admin/src/routes/Identities.svelte` | **修改** — 按钮加 `hasPermission()` |
| 17 | `admin/src/routes/ApiKeys.svelte` | **修改** — 按钮加 `hasPermission()` |
| 18 | `admin/src/routes/Organizations.svelte` | **修改** — 列表过滤 + 按钮权限 |
| 19 | `admin/src/routes/OrgTools.svelte` | **修改** — 按钮加 `hasPermission()` |
| 20 | `admin/src/routes/Sandbox.svelte` | **修改** — 按钮加 `hasPermission()` |

---

## 五、安全边界

| 层面 | 策略 |
|------|------|
| **前端权限** | 仅用于 UX 优化 — 隐藏不该看到的 tab 和按钮 |
| **后端权限** | 唯一安全防线 — 所有 API 端点通过 `PermissionService` 强制校验 |
| **双重保险** | 用户 devtools 绕过前端直接调 API → 后端仍拒绝 |

---

## 六、实现顺序

```
Phase 0: 后端（30 min）
  → models.rs: 加 system_roles/tenant_roles
  → handlers.rs: 登录填充新字段 + /users/me/permissions
  → routes.rs: 注册新端点

Phase 1: 前端配置表（30 min）
  → permission.js store
  → nav-routes.js 配置
  → actions.js 配置

Phase 2: Nav + App 改造（30 min）
  → Nav.svelte: filterNavRoutes()
  → App.svelte: layoutForUser()

Phase 3: 逐页面按钮改造（90 min）
  → 14 个页面加入 hasPermission() 检查

Phase 4: 列表上下文过滤（30 min）
  → Organizations/Groups/Skills 列表按身份范围过滤
```

**总工作量：约 3.5 小时**（比原方案简化后减少 40%）

---

## 七、核心设计决策总结

1. **不写 Guard 组件** — 用 Nav 配置表自动过滤代替路由守卫
2. **不写到处散落的 `{#if}`** — 页面操作统一声明在 `actions.js`
3. **不强制前端路由拦截** — 后端永远是最后防线，前端只管「不显示」
4. **一个 `hasPermission()` 走天下** — 所有地方调用同一个函数
5. **配置即文档** — `nav-routes.js` 和 `actions.js` 本身就是权限矩阵的可视化文档
