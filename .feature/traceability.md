# REQ-003 Phase 1 追溯性矩阵

## 验收标准追溯

| 验收项 | 状态 | 代码位置 | 说明 |
|--------|------|---------|------|
| AC-001: super_admin 登录后看到完整菜单 | ✅ PASS | `admin/src/config/nav-routes.js` - allowedRoles: [ROLE_SUPER_ADMIN] | 概览/租户/用户/组织/内容/系统/基础设施 |
| AC-002: tenant_admin 登录后只看到租户相关菜单 | ✅ PASS | `admin/src/config/nav-routes.js` - allowedRoles: [ROLE_TENANT_ADMIN] | 概览/租户/组织/内容 |
| AC-003: org_admin 登录后只看到组织相关菜单 | ✅ PASS | `admin/src/config/nav-routes.js` - allowedRoles: [ROLE_ORG_ADMIN] | 概览/组织/成员/工具 |
| AC-004: 普通用户登录后跳转到 /user | ✅ PASS | `admin/src/stores/permission.js` - getDefaultRoute() | 默认路由为 /user |
| AC-005: super_admin 默认进入 /stats | ✅ PASS | `admin/src/stores/permission.js` - getDefaultRoute() | 系统角色检查后返回 /stats |
| AC-006: tenant_admin 默认进入租户详情页 | ✅ PASS | `admin/src/stores/permission.js` - getDefaultRoute() | 返回 /tenants/{id} |
| AC-007: org_admin 默认进入组织详情页 | ✅ PASS | `admin/src/stores/permission.js` - getDefaultRoute() | 返回 /organizations/{id} |
| AC-008: 各角色概览页面显示相关的快捷操作 | ✅ PASS | `admin/src/config/nav-routes.js` - quickActionCards | tenant_admin/org_admin/super_admin 快捷操作 |

## 核心实现追溯

### permission.js 新增函数
| 函数 | 位置 | 说明 |
|------|------|------|
| `isSuperAdmin()` | permission.js:L95-98 | 判断是否为超级管理员 |
| `isTenantAdmin()` | permission.js:L100-103 | 判断是否为租户管理员 |
| `isOrgAdmin()` | permission.js:L105-108 | 判断是否为组织管理员 |
| `getDefaultRoute()` | permission.js:L110-133 | 获取角色专属默认路由 |
| `getFirstTenantId()` | permission.js:L141-145 | 获取第一个租户 ID |
| `getFirstOrgId()` | permission.js:L147-151 | 获取第一个组织 ID |

### nav-routes.js 新增配置
| 配置项 | 位置 | 说明 |
|--------|------|------|
| `allowedRoles` | nav-routes.js | 每个菜单组允许的角色 |
| `filterNavRoutesByRole()` | nav-routes.js:L108-121 | 按角色过滤菜单组 |
| `quickActionCards` | nav-routes.js:L136-158 | 角色快捷操作卡片配置 |

### App.svelte 改动
| 功能 | 位置 | 说明 |
|------|------|------|
| 角色判断 | App.svelte:L48-50 | isSA/isTA/isOA reactive variables |
| 默认路由重定向 | App.svelte:L52-69 | 登录后根据角色重定向 |
| 新增路由 | App.svelte | /tenants/:id, /org-members |

### Nav.svelte 改动
| 功能 | 位置 | 说明 |
|------|------|------|
| getUserRole() | Nav.svelte:L63-70 | 获取当前用户角色 |
| 角色过滤 | Nav.svelte:L77-87 | filterNavRoutesByRole(userRole) |

### 新增组件
| 组件 | 说明 |
|------|------|
| TenantDetail.svelte | 租户详情页，用于 tenant_admin 默认着陆 |
| OrgMembers.svelte | 组织成员管理页，用于 org_admin 快捷访问 |
