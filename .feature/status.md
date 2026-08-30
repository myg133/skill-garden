# REQ-003 Phase 1 状态

## 基本信息

| 属性 | 值 |
|------|-----|
| **REQ 编号** | REQ-003 |
| **阶段** | Phase 1 |
| **标题** | 菜单/导航差异化 |
| **状态** | 待验证 |
| **开始日期** | 2024-08-30 |
| **负责人** | anspire-open-developer |

## 功能要求

### 1.1 角色专属导航菜单
| 角色 | 可见菜单组 |
|------|-----------|
| `super_admin` | 概览 / 租户 / 用户 / 组织 / 内容 / 系统 / 基础设施 |
| `tenant_admin` | 概览 / 租户 / 组织 / 内容 |
| `org_admin` | 概览 / 组织 / 成员 / 工具 |

### 1.2 角色专属默认着陆页
| 角色 | 默认页面 |
|------|---------|
| `super_admin` | `/stats` |
| `tenant_admin` | 第一个租户详情页 |
| `org_admin` | 第一个组织详情页 |
| 其他 | `/user` |

### 1.3 快捷操作卡片
- tenant_admin: "管理成员" / "查看组织" / "邀请成员"
- org_admin: "添加成员" / "管理工具" / "查看 Skills"

## 改动文件
- `admin/src/stores/permission.js` - 新增角色判断函数
- `admin/src/App.svelte` - 角色专属着陆页逻辑
- `admin/src/components/Nav.svelte` - 角色化菜单渲染
- `admin/src/config/nav-routes.js` - 角色专属导航配置
- `admin/src/routes/TenantDetail.svelte` - 租户详情页（新）
- `admin/src/routes/OrgMembers.svelte` - 组织成员页（新）
- `admin/src/i18n/en.json` - 新增翻译键
- `admin/src/i18n/zh.json` - 新增翻译键

## 验收标准
- [x] AC-001: super_admin 登录后看到完整菜单
- [x] AC-002: tenant_admin 登录后只看到租户相关菜单
- [x] AC-003: org_admin 登录后只看到组织相关菜单
- [x] AC-004: 普通用户登录后跳转到 /user
- [x] AC-005: super_admin 默认进入 /stats
- [x] AC-006: tenant_admin 默认进入租户详情页
- [x] AC-007: org_admin 默认进入组织详情页
- [x] AC-008: 各角色概览页面显示相关的快捷操作

## 开发进度

### Step 1: 接收任务
- [x] 确认工作区
- [x] 读 manifest.json
- [x] 读需求文档
- [x] 更新 status.md

### Step 2: 开发实现
- [x] permission.js - 新增 isTenantAdmin, isOrgAdmin, getDefaultRoute
- [x] nav-routes.js - 扩展角色专属路由配置
- [x] Nav.svelte - 角色化菜单渲染
- [x] App.svelte - 角色专属着陆页逻辑
- [x] TenantDetail.svelte - 租户详情页组件
- [x] OrgMembers.svelte - 组织成员页组件
- [x] i18n - 新增翻译键

### Step 3: 自验证
- [x] 追溯性检查
- [x] 构建验证 (npm run build 成功)
- [x] 质量检查

### Step 4: 提交验证
- [x] 更新状态为"待验证"
- [ ] 通知 BA Agent

## 提交历史

- 2024-08-30: 完成 REQ-003 Phase 1 开发
  - 角色专属导航菜单
  - 角色专属默认着陆页
  - 角色专属快捷操作卡片配置
  - TenantDetail.svelte 和 OrgMembers.svelte 组件
