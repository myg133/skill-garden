# 权限模型审计与细化设计 (v3)

> 状态: **待确认**  
> 日期: 2026-07-20  
> 目的: 梳理当前所有角色类型、权限码、前后端映射关系，识别不一致与缺失项，输出统一设计后再实施
>
> **v3 核心变更**: 引入 Skill 双轨状态模型（内部轨 `status` + 市场轨 `marketplace_status`），将「组织内部使用」和「市场公开发布」彻底解耦。详见第 10 节。

---

## 1. 角色全景总览

AionHive 采用 **四层 RBAC** 架构:

```
System → Tenant → Organization → [Group - 暂不处理]
  ↑          ↑          ↑
  2种角色    1种角色     5种角色
```

另有 **personal** 级别（个人用户）表示无任何管理角色的纯个人用户。

### 1.1 角色速查表

| 层级 | 角色名 | 数据库来源 | 管理范围 |
|------|--------|----------|------|
| **System** | `super_admin` | `system_role_assignments` | 全平台、**仅**设 tenant_admin / marketplace_admin（顶级角色，不设子角色） |
| **System** | `marketplace_admin` | `system_role_assignments` | 市场 Skill 审核/上下架/精选推荐，**可设 marketplace_reviewer**，不可见组织 |
| **System** | `marketplace_reviewer` | `system_role_assignments` | ← **新增**。Marketplace Skill 审核/下架，**不可**做精选推荐，不可见组织。**由 marketplace_admin 任命，非 super_admin** |
| **Tenant** | `tenant_admin` | `tenant_role_assignments` | 租户内管理，可设 org admin/owner |
| **Organization** | `owner` | `org_memberships` | 组织完全控制（含上架 Skill、转让/删除组织） |
| **Organization** | `admin` | `org_memberships` | 组织日常管理（不可删除/转让），拥有组织内全部权限 |
| **Organization** | `reviewer` | `org_memberships` | 审核员，可审核/驳回 Skill |
| **Organization** | `developer` | `org_memberships` | 开发者，可创建/编辑/删除自己的 Skill |
| **Organization** | `member` | `org_memberships` | 普通成员，仅读取和安装 |
| **Personal** | `user` | — (默认，所有已认证用户) | 个人 Skill 空间 |

> **Group 角色（lead/member）暂不处理，在本次审计中忽略。**

### 1.2 遗留字段

- `identities.is_system_admin`: 旧系统管理员标记（`bool`），已被 `system_role_assignments` 取代
- `roles` 表 (迁移 014): 旧角色表，已被 `role_permissions` 表取代
- `permissions` 表 (迁移 014): 旧权限码 (`skill:approve`, `skill:publish` 等)，部分与新版权限码不兼容

---

## 2. 角色管理层级（核心设计）

```
super_admin
  ├── 可设置: marketplace_admin（给定用户邮箱）
  └── 可设置: tenant_admin（给定用户邮箱）
       └── 可设置: organization 的 owner / admin（不可跨层设 marketplace_admin/reviewer）
            └── 可设置: organization 内部角色（reviewer / developer / member）

marketplace_admin
  └── 可设置: marketplace_reviewer（给定用户邮箱）← **新增委托**
       （marketplace_reviewer 不可设其他角色）

marketplace_admin:   市场完全管理（审核/下架/精选推荐），可设 reviewer，不可见组织，不可更改自身角色
marketplace_reviewer: 市场内容审核和下架，不可做精选推荐，不可更改自身角色  ← 新增；由 marketplace_admin 任命
org owner/admin:     拥有组织内所有权限，可上架 Skill 到 marketplace
```

**原则**：
- **不跨层**：super_admin 只设顶级角色（marketplace_admin / tenant_admin），不直接设子角色
- **逐级委托**：marketplace_admin 自己管理 reviewer 团队；tenant_admin 自己管理组织层级
- **邮箱即用**：设置顶级角色时仅需用户邮箱，无需复杂表单。最常用的两种角色是 **marketplace_admin** 和 **org admin**

---

## 3. 完整权限码清单

> **重要**: Skill 生命周期采用**双轨状态模型**——内部轨（`status`）控制组织/个人内的可用性，市场轨（`marketplace_status`）控制市场上下架。两者独立流转。详见第 10 节。

### 3.1 Tenant 权限

| 权限码 | 说明 | 拥有者 |
|--------|------|--------|
| `tenant:create` | 创建租户 | super_admin |
| `tenant:read` | 查看租户 | super_admin, tenant_admin |
| `tenant:update` | 更新租户 | super_admin, tenant_admin |
| `tenant:delete` | 删除租户 | super_admin |
| `tenant:sso_config` | 配置 SSO | super_admin, tenant_admin |
| `tenant:settings_read` | 读取租户设置 | tenant_admin |
| `tenant:settings_write` | 修改租户设置 | tenant_admin |
| `tenant:member_read` | 查看租户成员 | tenant_admin |
| `tenant:member_invite` | 邀请租户成员 | tenant_admin |
| `tenant:member_remove` | 移除租户成员 | tenant_admin |
| `tenant:org_create` | 创建组织 | tenant_admin |
| `tenant:org_delete` | 删除组织 | tenant_admin |
| `tenant:billing_read` | 读计费 | tenant_admin |
| `tenant:billing_update` | 写计费 | tenant_admin |

### 3.2 Organization 权限

| 权限码 | 说明 | 拥有者 |
|--------|------|--------|
| `org:read` | 读组织 | super_admin, tenant_admin, org(owner,admin,reviewer,developer,member) |
| `org:update` | 更新组织 | org(owner,admin) |
| `org:delete` | 删除组织 | org(owner) |
| `org:transfer` | 转让组织 | org(owner) |
| `org:settings_read` | 读设置 | org(owner,admin) |
| `org:settings_write` | 写设置 | org(owner,admin) |
| `org:member_read` | 查成员 | super_admin/tenant_admin: all; org(*) own |
| `org:member_invite` | 邀请成员 | org(owner,admin) |
| `org:member_remove` | 移除成员 | org(owner,admin) |
| `org:member_role_assign` | 分配角色 | org(owner,admin) |
| `org:member_suspend` | 暂停成员 | org(owner,admin) |
| `org:skill_transfer` | 转移 Skill | org(owner,admin) |

### 3.3 Skill 权限

#### 3.3.1 基础 Skill 操作

| 权限码 | 说明 | 拥有者 |
|--------|------|--------|
| `skill:create` | 创建 Skill | org(owner,admin,developer), personal(user) |
| `skill:read` | 读 Skill 元数据 | 所有角色（按 scope 限制） |
| `skill:read_content` | 读 Skill 内容 | 所有角色（按 scope 限制） |
| `skill:update` | 更新 Skill | org(owner,admin,developer), personal(user) ← **developer 新增** |
| `skill:delete` | 删除 Skill | org(owner,admin,developer), personal(user) ← **developer 新增（仅限自己创建的）** |
| `skill:install` | 安装 Skill | 所有角色 |
| `skill:version_create` | 创建版本 | org(owner,admin,developer), personal(user) ← **developer 新增** |
| `skill:version_rollback` | 回滚版本 | org(owner,admin), personal(user) |
| `skill:change_visibility` | 改可见性 | org(owner,admin), personal(user) |
| `skill:fork` | Fork Skill | 所有角色 |

> **移除**: `skill:associate_group` / `skill:dissociate_group`（group 暂不处理）

#### 3.3.2 内部审核权限（轨道 1：内部生命周期）

| 权限码 | 说明 | 拥有者 | 备注 |
|--------|------|--------|------|
| `skill:submit_review` | 提交**内部**审核 | org(owner,admin,reviewer,developer), personal(user) | status: draft → pending_review。个人用户可自我审核 |
| `skill:approve_review` | 内部审核通过 | super_admin, org(owner,admin,reviewer) | status: pending_review → approved。**marketplace_admin/reviewer 不参与内部审核** |
| `skill:reject_review` | 内部审核驳回 | super_admin, org(owner,admin,reviewer) | status: pending_review → rejected |
| `skill:publish` | 发布 Skill（内部使用） | org(owner,admin,developer), personal(user) | status: approved → published。visibility 设为 OrgVisible 或 Private。**不涉及市场** |

> **关键变更**: `skill:publish` 不再自动上架市场。市场操作走独立的权限和状态轨道。

#### 3.3.3 市场操作权限（轨道 2：市场生命周期）

| 权限码 | 说明 | 拥有者 | 备注 |
|--------|------|--------|------|
| `skill:publish_to_marketplace` | 提交 Skill 到市场审核 | org(owner,admin), personal(user) ← **新增，替代 org:skill_publish** | 前置条件: status=published 且 marketplace_status 为 NULL/rejected/delisted。marketplace_status → pending_review |
| `marketplace:review_approve` | 市场审核通过 | marketplace_admin, marketplace_reviewer ← **新增** | marketplace_status: pending_review → listed。自动设 visibility=Marketplace |
| `marketplace:review_reject` | 市场审核驳回 | marketplace_admin, marketplace_reviewer ← **新增** | marketplace_status: pending_review → rejected |

> **已移除**: `org:skill_publish` — 被 `skill:publish_to_marketplace` 替代。旧权限码将在迁移中清理。

### 3.4 API Key 权限

| 权限码 | 说明 | 拥有者 |
|--------|------|--------|
| `apikey:create` | 创建 Key | org(*), personal(user) |
| `apikey:read` | 读 Key | org(*), personal(user) |
| `apikey:revoke` | 吊销 Key | org(*), personal(user) |
| `apikey:scope_set` | 设置权限范围 | org(owner,admin) |
| `apikey:rate_limit_set` | 设置速率限制 | org(owner,admin) |

### 3.5 Profile 权限

| 权限码 | 说明 | 拥有者 | 约束 |
|--------|------|--------|------|
| `profile:read` | 读 Profile | 所有认证用户 | — |
| `profile:update` | 更新 Profile | 所有认证用户 | **仅可更新自己的 Profile** |
| `profile:delete` | 删除 Profile | 所有认证用户 | **仅可删除自己的 Profile，需二次确认** |

> **关键变更**: `profile:delete` 改为仅限自己删除自己。组织 admin 不可删除成员 profile。删除操作需前端二次确认。

### 3.6 Marketplace 权限

| 权限码 | 说明 | 拥有者 |
|--------|------|--------|
| `marketplace:feature` | 精选推荐 | super_admin, marketplace_admin |
| `marketplace:unfeature` | 取消精选 | super_admin, marketplace_admin |
| `marketplace:delist` | 从市场下架 | super_admin, marketplace_admin, marketplace_reviewer ← **reviewer 新增** |
| `marketplace:relist` | 重新上架已下架的 Skill | marketplace_admin ← **新增** |
| `marketplace:manage` | 市场管理页面入口 | super_admin, marketplace_admin, marketplace_reviewer ← **reviewer 新增** |
| `marketplace:role_assign` | 任命/移除 marketplace_reviewer | marketplace_admin ← **新增；仅 marketplace_admin 拥有** |

> **注**: 市场审核的通过/驳回权限见 3.3.3（`marketplace:review_approve` / `marketplace:review_reject`），为独立的审核权限码。`marketplace:delist` 是下架操作（已上架 Skill → delisted），不可与审核驳回混淆。

> **marketplace_admin vs marketplace_reviewer 区别**: reviewer 可审核/下架违规内容，但**不能**做精选推荐（feature/unfeature），适合委托日常审核工作而保留精选决策权在 admin 手中。

> **委托原则**: super_admin **不直接**设 marketplace_reviewer。marketplace_admin 通过 `marketplace:role_assign` 自行管理 reviewer 团队。详见 4.3。

> **关键变更**: org owner/admin 可通过 `skill:publish_to_marketplace`（见 3.3.3）将已发布的 Skill 提交到市场审核。

### 3.7 System 权限

| 权限码 | 说明 | 拥有者 |
|--------|------|--------|
| `system:admin:access` | 管理后台入口 | super_admin |
| `system:role_assign` | 分配系统级顶级角色（marketplace_admin / tenant_admin） | super_admin ← **仅设顶级角色，不设子角色** |
| `tenant:role_assign` | 分配租户级角色（org owner/admin） | tenant_admin ← **新增** |
| `marketplace:role_assign` | 任命/移除 marketplace_reviewer | marketplace_admin ← **新增；见 3.6** |
| `audit:read_global` | 全局审计 | super_admin |
| `audit:read_org` | 组织审计 | org(owner,admin) |

### 3.8 旧权限码（待清理）

| 权限码 | 状态 |
|--------|------|
| `skill:approve`, `skill:publish`, `tool:*`, `org:manage`, `org:configure`, `org:view`, `member:manage`, `member:view`, `tenant:manage`, `group:manage`, `group:view`, `audit:read`, `api_key:manage`, `skill:associate_group`, `skill:dissociate_group` | 从 014 迁移遗留，需清理 |

### 3.9 已发布到市场的 Skill 删除规则 ← **更新为双轨模型**

当 Skill 的 `marketplace_status = 'listed'`（已上架市场）后，**所有者仍可删除，但需遵守以下规则**：

| 场景 | 操作者 | 行为 | 前端 | 后端 |
|------|--------|------|------|------|
| 组织 Skill（已上架市场） | org owner / admin / developer(own) | ✅ 允许删除 | **二次确认弹窗**："此 Skill 已发布到市场（当前状态: 已上架），删除后将从市场自动下架，可能影响已安装该 Skill 的用户。确认删除？" | 1. 将 `marketplace_status` 置为 `NULL` 2. 从市场索引中移除 3. 删除 Skill 记录 |
| 个人 Skill（已上架市场） | personal(user) | ✅ 允许删除 | 同上二次确认 | 同上 |
| 市场 Skill | marketplace_admin | 不可直接删除，通过 `marketplace:delist` 下架（不删除源 Skill） | 下架确认弹窗 | 仅将 `marketplace_status` 置为 `delisted`，`visibility` 回退到上架前的值，不删除 Skill |
| 任意 Skill（超管） | super_admin | ✅ 允许强制删除 | 二次确认（含"强制删除"警告） | 同上 cascade，无需检查归属 |

**关键原则**：
- **所有者可删**：Skill 的归属方（组织 admin/owner/developer 或 个人用户）始终拥有删除权，无论 marketplace_status 为何
- **自动下架**：删除操作自动将 `marketplace_status` 置为 NULL，无需先手动 `marketplace:delist`
- **前端阻断**：二次确认弹窗是唯一的安全网，防止误删已发布的 Skill。弹窗文案按 marketplace_status 值动态变化（listed → "已上架" / pending_review → "审核中"）
- **marketplace_admin 不可删源 Skill**：只能通过 `marketplace:delist` 下架，无权删除不属于自己的 Skill

> **与 `marketplace:delist` 的区别**：`marketplace:delist` 是市场管理操作（下架但保留 Skill，`marketplace_status = delisted`），而所有者 `skill:delete` 是彻底删除（含市场状态清除）。

---

## 4. 前端布局策略

### 4.1 统一 Layout 设计

**放弃三级布局方案，所有管理角色共用 Admin Layout，纯用户走 User Layout。**

```
               ┌─────────────┐
               │  已登录？    │
               └──────┬──────┘
                      │
          ┌───────────┼───────────┐
          │           │           │
    super_admin    非纯 user     纯 user
          │    (marketplace_admin │
          │     tenant_admin      │
          │     org owner/admin   │
          │     reviewer/developer│
          │     /member)          │
          │           │           │
      Admin Layout  Admin Layout  User Layout
      (含 System   (纯管理角色    (普通用户页
       tab 组)     按权限过滤)
```

**核心逻辑**:

- 有**任意管理角色**（system 角色 / tenant 角色 / org 角色 / `is_system_admin`）→ 进入 **Admin Layout**
- 无任何管理角色 → 进入 **User Layout**
- Admin Layout 内按 `hasPermission()` 过滤导航组和 tab
- **marketplace_admin / marketplace_reviewer** 在 Admin Layout 中仅看到 Content 相关 tab（Marketplace、Skills、Review），不显示 Organizations、System 等组

### 4.1.1 组织切换器（Admin Layout 核心交互）← **新增**

对于拥有 org 角色的用户（tenant_admin / org owner / admin / reviewer / developer / member），Admin Layout 的顶部导航栏增加一个 **组织切换器**，作为全局上下文控制器：

```
┌──────────────────────────────────────────────────────────────────┐
│  🏢 前端工程组 ▾    [Skills] [Review] [Members]     [🔔] [👤]    │
└──────────────────────────────────────────────────────────────────┘
```

点击展开：
```
[ 🏢 前端工程组 ▾ ]
  ├── 🏢 前端工程组       (当前角色: admin)
  ├── 🏢 后端服务组       (当前角色: developer)
  ├── 🏢 DevOps 组        (当前角色: member)
  ├── ──────────────
  └── 👤 个人空间
```

**核心规则**：

| 规则 | 说明 |
|------|------|
| **一对一展示** | 选中一个组织（或个人）后，整个页面内容（Skills、Review、Members 等）**仅展示该组织的数据**，不混合多个组织 |
| **默认选中** | 首次登录默认选中用户角色最高的组织；无组织角色但有 tenant_admin 时显示"租户全局"；仅 marketplace_admin/reviewer 时不显示切换器 |
| **记忆状态** | 切换的组织选择记录到 `localStorage`，下次登录自动恢复 |
| **角色标签** | 下拉中每个组织旁边显示用户在该组织中的角色（admin / developer / member） |
| **权限跟随** | 选中不同组织后，页面内按钮的可用状态按用户**在当前组织的角色**动态变化 |
| **全局 Tab 不变** | 切换组织不影响 Marketplace、Profile、My API Keys 等个人/全局页面，仅影响组织相关 Tab |

**无切换器场景**：
- super_admin → 无切换器，视图为全局跨租户
- marketplace_admin / marketplace_reviewer → 无切换器，视图为市场管理
- 纯 user → User Layout，无切换器

### 4.2 身份标识

在 Admin Layout 的 UserNav 中显示当前用户的管理身份标签：

| 角色 | 标签 |
|------|------|
| super_admin | `🔧 超管` |
| marketplace_admin | `🏪 市场管理` |
| marketplace_reviewer | `🛡️ 市场审核` ← **新增** |
| tenant_admin | `🏢 租户管理` |
| org owner | `👑 组织所有者` |
| org admin | `⚙️ 组织管理` |

同一用户可拥有多个标签（如同时是 org admin + marketplace_admin）。

### 4.3 超管角色分配页（`/system-roles`）← **新增**

super_admin 通过此页面任命顶级管理员，**仅需用户邮箱**即可完成。

#### 4.3.1 页面设计

```
┌──────────────────────────────────────────────────────────────────┐
│  🔧 系统角色分配                                                  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [+ 添加管理员]                                                   │
│  ┌──────────────────────────────────────────────────────────────┐│
│  │  用户邮箱  │ 角色               │ 分配时间    │ 操作         ││
│  │──────────────────────────────────────────────────────────────││
│  │  alice@ex  │ 🏪 市场管理        │ 07-15 09:30 │ [移除]       ││
│  │  bob@ex    │ 🏢 租户管理(租户A)  │ 07-14 14:20 │ [移除]       ││
│  │  carol@ex  │ 🏪 市场管理        │ 07-12 10:00 │ [移除]       ││
│  │  dave@ex   │ 🏢 租户管理(租户B)  │ 07-10 08:45 │ [移除]       ││
│  └──────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────┘
```

点击 `[+ 添加管理员]` 弹出 modal：

```
┌───────────────────────────────────────────┐
│  添加管理员                          [✕]   │
│                                           │
│  用户邮箱 *                               │
│  ┌───────────────────────────────────────┐│
│  │ user@example.com                      ││
│  └───────────────────────────────────────┘│
│                                           │
│  角色 *                                   │
│  ┌───────────────────────────────────────┐│
│  │ 🏪 市场管理 (marketplace_admin)  ▾    ││
│  └───────────────────────────────────────┘│
│                                           │
│  租户（仅角色为"租户管理"时需要）           │
│  ┌───────────────────────────────────────┐│
│  │ 租户A                           ▾     ││
│  └───────────────────────────────────────┘│
│                                           │
│                          [取消]  [确认添加] │
└───────────────────────────────────────────┘
```

#### 4.3.2 核心规则

| 规则 | 说明 |
|------|------|
| **邮箱即身份** | 输入用户邮箱 → 后端自动查找 identity → 写入 `system_role_assignments` |
| **两种角色** | 下拉仅两个选项：`marketplace_admin` / `tenant_admin`。**没有 marketplace_reviewer** |
| **租户必选** | 选 `tenant_admin` 时必须指定租户（一个用户可以同时是多个租户的 admin） |
| **不可设自己** | 不能通过此页面修改自己的角色（UI 层面禁用 + 后端校验） |
| **不可撤自己** | 不能移除自己的管理员角色（防止把自己踢出去后平台无人管理） |
| **邮箱不存在** | 输入不存在的邮箱 → 后端返回 "用户未注册" 提示 |
| **已存在提示** | 用户已有该角色 → 提示 "该用户已是 XXX" |

#### 4.3.3 为什么 super_admin 不设 marketplace_reviewer？

| 原因 | 说明 |
|------|------|
| **委托原则** | marketplace_admin 最清楚团队中谁适合做审核员 |
| **避免越权** | super_admin 不参与市场运营决策，只做最高层的人事任命 |
| **简化超管 UI** | 超管页面只做两件事：任命市场负责人 + 任命租户负责人 |
| **职责清晰** | marketplace_admin = 市场需求方（feature + reviewer 管理）；tenant_admin = 组织供给方（org 层级管理） |

> marketplace_reviewer 的设置页面由 marketplace_admin 在 `/marketplace-roles`（见 4.4）中管理。

### 4.4 市场管理员 reviewer 分配页（`/marketplace-roles`）← **新增**

marketplace_admin 通过此页面管理自己的 reviewer 团队。设计原则同上（邮箱即用），但**仅可选 `marketplace_reviewer` 一种角色**。

```
┌──────────────────────────────────────────────────────────────────┐
│  🏪 市场审核员管理                                                │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [+ 添加审核员]                                                   │
│  ┌──────────────────────────────────────────────────────────────┐│
│  │  用户邮箱    │ 角色              │ 分配时间    │ 操作        ││
│  │──────────────────────────────────────────────────────────────││
│  │  eve@ex     │ 🛡️ 市场审核       │ 07-16 11:00 │ [移除]      ││
│  │  frank@ex   │ 🛡️ 市场审核       │ 07-18 15:30 │ [移除]      ││
│  └──────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────┘
```

**规则**：
- 仅 `marketplace_admin` 可访问，`marketplace_reviewer` 不可见此页面
- 添加 modal 只需邮箱，角色固定为 `marketplace_reviewer`（无下拉）
- 不可设 marketplace_admin（不能给自己加同级）
- 不能修改自身角色

---

## 5. 页面级权限矩阵（目标态）

| 页面/路由 | super_admin | marketplace_admin | marketplace_reviewer | tenant_admin | org:owner | org:admin | org:reviewer | org:developer | org:member | 纯 user |
|-----------|:-----------:|:-----------------:|:--------------------:|:------------:|:---------:|:---------:|:------------:|:-------------:|:----------:|:-------:|
| `/stats` | ✅ | ✅(市场统计) | ✅(市场统计) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `/tenants` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `/identities` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `/api-keys` (admin) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `/sessions` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `/audit` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `/settings` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `/sandboxes` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `/roles` (仅 system 角色分配页) | ✅(仅设 market_admin/tenant_admin) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `/roles` (租户角色分配页) | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `/roles` (市场 reviewer 分配页) | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `/marketplace` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `/skills` (列表) | ✅ | ✅(市场+个人) | ✅(市场+个人) | ✅(租户范围) | ✅(当前org) | ✅(当前org) | ✅(当前org) | ✅(当前org) | ✅(当前org) | ✅(仅个人) |
| `/skills/:id` 读 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅(own) |
| `/skills/:id` 编辑 | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ | ✅(own) | ❌ | ✅(own) |
| `/skills/:id` 删除 | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ | ✅(own) | ❌ | ✅(own) |
| `/skills/:id` 审核 | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅(own) |
| `/skills/:id` 市场操作(1) | ✅ | ✅(全部) | ✅(仅下架) | ❌ | ✅(上架) | ✅(上架) | ❌ | ❌ | ❌ | ❌ |
| `/review` | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| `/` (organizations) | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| `/organizations/:id` 管理 | ✅ | ❌ | ❌ | ✅ | ✅(full) | ✅(no del/transfer) | ❌ | ❌ | ❌ | ❌ |
| `/organizations/:id` 查看 | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| `/org-tools` | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| `/profile` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `/my-api-keys` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `/user` | — | — | — | — | — | — | — | — | — | ✅ |
| `/user/marketplace` | — | — | — | — | — | — | — | — | — | ✅ |
| `/user/skills` | — | — | — | — | — | — | — | — | — | ✅ |
| `/user/submissions` | — | — | — | — | — | — | — | — | — | ✅ |

> `—` = 不进入该 Layout；`❌` = 不可见/403；`✅` = 可见；`✅(own)` = 仅限自己的资源
> 
> (1) `/skills/:id` 市场操作：对 org owner/admin 是"上架到市场"（`skill:publish_to_marketplace`）；对 marketplace_admin 是"精选推荐/取消精选/下架"（`feature/unfeature/delist`）；对 marketplace_reviewer 是"下架"（`delist`）

---

## 6. Skill 列表页视图设计（按角色区分）

Skill 列表是使用频率最高的页面，不同角色看到的数据范围和交互方式有本质差异。

### 6.1 组织成员视图（org:owner / admin / reviewer / developer / member）

拥有 org 角色的用户，进入 `/skills` 页面时，**页面内容由顶部组织切换器控制**（见 4.1.1），不再出现多组织混合的"全部"视图。

#### 6.1.1 核心原则：一对一，不混合

```
用户同时是「前端工程组」的 developer 和「后端服务组」的 member
→ 页面只展示当前选中组织的数据，不混合两个组织
→ 切换器选中「前端工程组」→ 看前端组的 Skills
→ 切换器选中「后端服务组」→ 看后端组的 Skills
→ 切换器选中「👤 个人」→ 看自己的个人 Skills
```

#### 6.1.2 选中某个组织时

切换器选中「🏢 前端工程组」：

```
┌──────────────────────────────────────────────────────────────┐
│  🏢 前端工程组 ▾    [Skills] [Review] [Members]               │
├──────────────────────────────────────────────────────────────┤
│  Skills              [ 状态 ▾ ]              [🔍 搜索...]     │
│                                                              │
│  名称         │ 版本  │ 状态      │ 创建者   │ 更新时间  │操作│
│───────────────┼───────┼───────────┼──────────┼───────────┼────│
│  code-review  │ 2.1.0 │ ✅ 已发布  │ 张三     │ 07-18    │ .. │
│  lint-check   │ 1.2.0 │ 📝 草稿   │ 李四     │ 07-15    │ .. │
│  deploy-bot   │ 0.5.0 │ 🔍 审核中  │ 自己     │ 07-10    │ .. │
```

- **数据来源**: 仅当前选中组织的 Skill（`scope=org`, `org_id=选中组织ID`）
- **不显示「来源」列**（当前上下文已确定是哪个组织）
- **显示「创建者」列**（组织内协作所需）
- **操作列**: 按钮按当前用户**在此组织内的最高角色**动态计算（如在上面的开发组是 developer → 可编辑自己创建的；在后端组是 member → 仅查看）
- **状态筛选**: Draft / Pending Review / Published / Rejected / Archived

#### 6.1.3 选中「👤 个人」时

切换器选中「👤 个人空间」：

```
┌──────────────────────────────────────────────────────────────┐
│  👤 个人空间 ▾    [Skills]                                   │
├──────────────────────────────────────────────────────────────┤
│  My Skills           [ 状态 ▾ ]              [🔍 搜索...]    │
│                                                              │
│  名称         │ 版本  │ 状态      │ 更新时间  │ 操作          │
│───────────────┼───────┼───────────┼───────────┼───────────────│
│  my-utils     │ 1.0.0 │ ✅ 已发布  │ 07-19    │ 查看 编辑 ... │
│  formatter    │ 0.1.0 │ 📝 草稿   │ 07-16    │ 查看 编辑 ... │
```

- **数据来源**: 仅个人 Skill（`scope=personal`）
- **不显示「来源」列**、**不显示「创建者」列**
- **操作权限最大化**: 自己的 Skill 可编辑/删除/提交审核
- **导航简化**: 个人空间下不显示 Review、Members 等组织专属 Tab

#### 6.1.4 切换器交互细节

| 细节 | 说明 |
|------|------|
| **切换响应** | 切换组织/个人后，当前页面数据立即刷新，不跳转路由 |
| **记忆选中** | 选中的组织/个人存入 `localStorage`，刷新页面不丢失 |
| **Review Tab 跟随** | 切到某组织后，`/review` 也展示该组织的待审核列表 |
| **Members Tab 跟随** | 切到某组织后，Members 显示该组织成员（需要 `org:member_read`） |

---

### 6.2 市场管理角色视图（marketplace_admin / marketplace_reviewer）

这两种角色进入 Admin Layout 后**无组织切换器**，Skill 列表页看到的是 **已发布到市场的 Skill**，自身没有组织归属。

#### 6.2.1 页面结构（Tab 切换）

```
┌──────────────────────────────────────────────────────────────┐
│  Skills        [ 📊 市场统计 ] [ 📋 市场 Skill ] [ 👤 个人 ]  │
│                                                              │
│  (当前 Tab 内容区域)                                          │
└──────────────────────────────────────────────────────────────┘
```

三个 Tab:

| Tab | 内容 | marketplace_admin | marketplace_reviewer |
|-----|------|:-----------------:|:--------------------:|
| **📊 市场统计** | 上架数/新增/待审核/下载量 + 图表 | ✅ | ✅ |
| **📋 市场 Skill** | 所有 `marketplace_status='listed'` 的 Skill | ✅（可精选※） | ✅（不可精选） |
| **👤 个人** | 自己创建的 Skill | ✅ | ✅ |

> **※ 精选推荐（feature/unfeature）**：仅 `marketplace_admin` 拥有，`marketplace_reviewer` 不可操作。

#### 6.2.2 市场统计 Tab

```
┌───────────────────────────────────────────────────────────────┐
│  📊 市场统计                                                  │
│                                                               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐         │
│  │ 已上架    │ │ 本月新增  │ │ 待审核    │ │ 本周下载  │         │
│  │   128    │ │   +12    │ │    8     │ │  1.2k   │         │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘         │
│                                                               │
│  [最近上架列表...]          [热门 Skill 排行...]               │
└───────────────────────────────────────────────────────────────┘
```

#### 6.2.3 市场 Skill Tab

所有已发布到 marketplace 的 Skill：

```
┌───────────────────────────────────────────────────────────────┐
│  📋 市场 Skill                    [ 分类 ▾ ]   [🔍 搜索...]    │
│                                                               │
│  名称         │ 版本  │ 来源组织/个人 │ ⭐ │ 下载  │ 操作     │
│───────────────┼───────┼───────────────┼───┼───────┼──────────│
│  code-review  │ 2.1.0 │ 🏢 前端组     │4.8│ 3.2k  │ 下架 ... │
│  deploy-bot   │ 0.5.0 │ 👤 alice      │4.5│ 1.8k  │ 下架 ... │
│  sql-helper   │ 1.3.0 │ 🏢 数据组     │4.2│  980  │ 下架 ... │
```

- **数据来源**: 所有 `marketplace_status = 'listed'` 的 Skill（已上架市场）
- **来源列**: 显示该 Skill 归属的组织或「个人」+ 用户名
- **操作差异**:
  - `marketplace_admin`: 可精选推荐(⭐)/取消精选(⭐)/下架 + 审核 + 查看详情
  - `marketplace_reviewer`: 仅可下架 + 审核 + 查看详情（**无精选操作按钮**）
- **可见性**: 看不到未上架到市场的 Skill

#### 6.2.4 个人 Tab

与 6.1.3 的「👤 个人空间」视图相同，管理自己创建的 Skill。

---

### 6.3 租户管理员视图（tenant_admin）

tenant_admin 使用与组织成员相同的 **组织切换器**（见 4.1.1），但切换器列表包含租户下**所有组织**（而非仅自己加入的组织），外加「👤 个人空间」。

```
组织切换器下拉:
  ├── 🏢 前端工程组
  ├── 🏢 后端服务组
  ├── 🏢 DevOps 组
  ├── 🏢 数据组          ← tenant_admin 可看到租户下所有组织
  ├── ──────────────
  └── 👤 个人空间
```

选中「🏢 前端工程组」后：

```
┌───────────────────────────────────────────────────────────────┐
│  🏢 前端工程组 ▾    [Skills] [Members] [Settings]              │
├───────────────────────────────────────────────────────────────┤
│  Skills                 [ 状态 ▾ ]            [🔍 搜索...]     │
│                                                               │
│  名称         │ 版本  │ 状态      │ 创建者   │ 更新时间  │操作  │
│───────────────┼───────┼───────────┼──────────┼───────────┼──────│
│  code-review  │ 2.1.0 │ ✅ 已发布  │ 张三     │ 07-18    │ 查看  │
│  lint-check   │ 1.2.0 │ 📝 草稿   │ 李四     │ 07-15    │ 查看  │
│  deploy-bot   │ 0.5.0 │ 🔍 审核中  │ 王五     │ 07-10    │ 查看  │
```

- **数据范围**: 当前选中组织的所有 Skill（含所有状态，不限发布到市场的）
- **组织切换器**: 展示租户下所有组织，不需要自己在该组织中有角色
- **导航**: 含 Skills、Members、Settings 等组织管理 Tab
- **操作**: 只读查看，无编辑/删除/审核/上架权限（与 org 成员视图的核心区别）
- **「👤 个人空间」**: 切换到个人视图，管理自己的 Skill

---

### 6.4 超级管理员视图（super_admin）

```
┌───────────────────────────────────────────────────────────────┐
│  Skills     [ 全部 ]    [ 租户 ▾ ] [ 组织 ▾ ] [ 状态 ▾ ]      │
│                                                               │
│  名称         │ 版本  │ 租户 │ 组织      │ 状态      │ 操作    │
│───────────────┼───────┼──────┼───────────┼───────────┼─────────│
│  code-review  │ 2.1.0 │ 腾讯 │ 🏢 前端组  │ ✅ 已发布  │ 查看.. │
│  deploy-bot   │ 0.5.0 │ 阿里 │ 🏢 后端组  │ 📝 草稿   │ 查看.. │
│  my-utils     │ 1.0.0 │  -   │ 👤 个人    │ ✅ 已发布  │ 查看.. │
```

- **数据范围**: 全平台所有 Skill（跨租户）
- **多级过滤**: 租户 → 组织 → 个人，逐级下钻
- **操作**: 可查看所有 Skill 详情，有全局管理操作（强制下架、删除等）
- **个人 Skill**: 通过切换到「👤 个人」管理自己的 Skill

---

### 6.5 纯用户视图（无组织角色的 user）

见 User Layout 的 `/user/skills`。仅显示自己的个人 Skill，无过滤菜单，只有状态筛选：

```
┌──────────────────────────────────────────────────────────────┐
│  My Skills                      [ 状态 ▾ ]   [🔍 搜索...]     │
│                                                              │
│  名称         │ 版本  │ 状态      │ 更新时间  │ 操作          │
│───────────────┼───────┼───────────┼───────────┼───────────────│
│  my-utils     │ 1.0.0 │ ✅ 已发布  │ 07-19    │ 查看 编辑 ... │
│  formatter    │ 0.1.0 │ 📝 草稿   │ 07-16    │ 查看 编辑 ... │
```

- 无组织归属，无来源列
- 操作权限最大（全部是自己的 Skill）

---

### 6.6 各角色 Skill 列表数据范围总结

| 角色 | 默认数据范围 | 视图控制方式 | 跨组织混合？ | 个人空间 |
|------|------------|------------|:---:|:---:|
| **super_admin** | 全平台所有 Skill | 多级过滤器（租户→组织→个人） | ✅（可跨） | ✅（切换器: 👤个人） |
| **marketplace_admin** | 市场已发布 Skill | Tab 切换（统计/市场/个人） | ✅（市场范围） | ✅（👤 Tab） |
| **marketplace_reviewer** | 市场已发布 Skill | Tab 切换（统计/市场/个人） | ✅（市场范围） | ✅（👤 Tab） |
| **tenant_admin** | 租户下当前选中组织 | 组织切换器（租户所有组织+个人） | ❌（一对一） | ✅（切换器: 👤个人空间） |
| **org owner/admin** | 当前选中组织 | 组织切换器（仅自己加入的组织+个人） | ❌（一对一） | ✅（切换器: 👤个人空间） |
| **org reviewer/developer/member** | 当前选中组织 | 组织切换器（仅自己加入的组织+个人） | ❌（一对一） | ✅（切换器: 👤个人空间） |
| **纯 user** | 仅自己个人 Skill | 无（User Layout） | ❌ | ✅（默认） |

---

### 6.7 相关 API 参数设计（新增/修改）

为支持上述视图（含组织切换器 + 双轨状态），Skill 列表 API 需要扩展查询参数：

| 参数 | 类型 | 说明 |
|------|------|------|
| `scope` | enum | `org` / `personal` / `marketplace`（移除 `all`，不再有多组织混合查询） |
| `org_id` | uuid? | scope=org 时**必传**当前选中组织ID（切换器已选定） |
| `tenant_id` | uuid? | super_admin 按租户过滤 |
| `marketplace_status` | enum? | marketplace_admin/reviewer 查询时传 `listed` / `pending_review` / `delisted` 等；替代旧 `published_to_marketplace` 布尔 |
| `status` | enum? | `draft` / `pending_review` / `published` / `rejected` / `approved` |
| `search` | string? | 名称/描述搜索 |
| `page` / `per_page` | int | 分页 |

**后端行为**:

- `scope=marketplace` → 仅返回 `marketplace_status = 'listed'` 的 Skill（市场浏览页面）
- `scope=marketplace` + `marketplace_status=pending_review` → marketplace_admin/reviewer 审核队列
- `scope=org` + `org_id` → 查询指定组织的 Skill（org 切换器选中后调用；tenant_admin 可传租户下任意 org_id）
- `scope=personal` → 仅返回用户的个人 Skill（切换器选中「👤 个人空间」后调用）
- super_admin 可额外传 `tenant_id` 缩小范围
- tenant_admin 查询限制在当前租户内
- **已移除 `scope=all`**：组织切换器模式下，组件永远明确知道自己需要哪个 scope
- **已移除 `published_to_marketplace` 布尔参数**：由 `marketplace_status` 枚举替代

**返回数据新增字段**:

```json
{
  "skills": [
    {
      "id": "...",
      "name": "code-review",
      "version": "2.1.0",
      "status": "published",
      "marketplace_status": "listed",      // ← 新增: NULL | pending_review | listed | rejected | delisted
      "source_type": "org",                // ← 新增: "org" | "personal"
      "source_name": "前端工程组",           // ← 新增: 组织名 或 "个人"
      "org_id": "...",                     // ← 新增: 组织ID (personal 时为 null)
      "tenant_name": "腾讯",                // ← 新增: super_admin 视图使用
      "creator_name": "张三",
      "visibility": "marketplace",
      "updated_at": "2026-07-18T10:00:00Z",
      ...
    }
  ],
  "stats": {
    "total": 85,
    "by_source": {
      "orgs": [
        {"org_id": "...", "org_name": "前端工程组", "count": 12},
        {"org_id": "...", "org_name": "后端服务组", "count": 8}
      ],
      "personal": 5
    }
  }
}
```

> `stats.by_source` 用于构建前端下拉菜单中的组织列表和计数。

---

## 7. 已识别问题与解决方案

### 7.1 P0/P1（阻塞上线）

| # | 问题 | 解决方案 |
|---|------|----------|
| **P1** | `require_admin()` 只认 super_admin，marketplace_admin/reviewer API 全部 401 | 拆分为 `require_super_admin()` / `require_marketplace_or_super()`; marketplace 相关路由用后者（同时覆盖 admin 和 reviewer） |
| **P2** | marketplace_admin/reviewer 进入 Admin Layout 后看到大量无权限导航组 | marketplace 角色在 Admin Layout 中只显示 Content 组（Marketplace、Skills、Review），其余组通过 `hasPermission` 隐藏整组 |
| **P3** | marketplace_admin/reviewer 不可见组织信息 | marketplace 角色不授予 `org:read` 等 org 相关权限，导航自动隐藏 |

### 7.2 P2（功能完善）

| # | 问题 | 解决方案 |
|---|------|----------|
| **M1** | `Skills.publish` 混用 `marketplace:feature`，org admin 发布按钮隐藏 | ~~新增 `org:skill_publish` → org admin/owner 可见上架按钮~~ **已过时**。已在 v3 中由 M12 双轨模型彻底解决：`skill:publish` 仅做内部发布（不涉及市场），`skill:publish_to_marketplace` 独立提交市场审核 |
| **M2** | developer 缺少 `skill:update` / `skill:version_create` | 补充 developer 权限码；database migration + 后端 permission 表 |
| **M3** | `profile:delete` 授予所有 org 角色不安全 | 改为仅限删除自己的 profile，后端按 identity_id 校验，前端二次确认弹窗 |
| **M4** | 缺少角色分配 API 和权限控制 | 新增 `system:role_assign` → super_admin 设 tenant_admin / marketplace_admin（**仅顶级角色**）；`tenant:role_assign` → tenant_admin 设 org admin/owner；`marketplace:role_assign` → marketplace_admin 设 reviewer（**子角色委托给对应 admin**） |
| **M5** | `/roles` 路由名指向不明，与旧 `roles` 表语义冲突 | 拆分为 `/system-roles`（super_admin 设 marketplace_admin / tenant_admin，**邮箱即用，不设 reviewer**）、`/marketplace-roles`（marketplace_admin 设 reviewer）、`/tenant-roles`（tenant_admin 设 org role）。或仍用 `/roles`，内部按角色分流 |
| **M6** | developer 可编辑自己创建的 Skill，但无 `skill:delete`→ 可编辑不可删 | ✅ **方案 B**：给 developer 增加 `skill:delete`，仅限自己创建的 Skill。权限表、矩阵已更新。**注意**：若 developer 的 Skill 已发布到市场，见第 3.8 节的市场 Skill 删除规则 |
| **M7** | marketplace_admin 是否有「👤 个人」Tab？能否创建个人 Skill？ | ✅ **需要**：保留个人 Tab。marketplace_admin 作为已认证用户，按 `skill:create` 的 `personal(user)` 覆盖，可创建/管理个人 Skill |
| **M8** | tenant_admin 在 Skill 操作列仅有「查看」，需明确 | 矩阵中 tenant_admin 对 `/skills/:id` 编辑/删除/审核/上架均为 ❌，与第 6.3 节一致 → tenant_admin 对租户 Skill 为**只读查看** |
| **M9** | 缺少 marketplace_reviewer 角色 ← **新增** | 新增系统级 `marketplace_reviewer` 角色：可审核/下架市场 Skill，不可精选推荐（feature/unfeature）。**由 marketplace_admin 通过 `/marketplace-roles` 任命，非 super_admin 直接管理**。权限码见 3.6，矩阵见第 5 节 |
| **M10** | 组织切换器：多组织混在"全部"视图，权限计算复杂且用户体验差 ← **新增** | 改为 Admin Layout 顶部组织切换器（4.1.1），选中一个组织后全页面仅展示该组织数据。移除 `/skills` 的「全部」混合视图。`scope=all` API 参数移除 |
| **M11** | super_admin 设 marketplace_reviewer 属于越权，应逐级委托 ← **新增** | super_admin 只设顶级角色（marketplace_admin / tenant_admin），marketplace_reviewer 改由 marketplace_admin 通过 `marketplace:role_assign` 自行管理。UI 仅需邮箱即可完成设置。详见 4.3、4.4 |
| **M12** | Skill 单轨状态模型无法区分「内部可用」和「市场发布」← **新增** | **最高优先级**。实施双轨状态模型（见第 10 节）：新增 `marketplace_status` 字段 + `pre_marketplace_visibility` 字段 + 废弃 `admin_unpublished`。拆分权限码为内部审核（3.3.2）和市场操作（3.3.3）。`skill:publish` 不再触发市场操作

### 7.3 P3（清理）

| # | 问题 | 解决方案 |
|---|------|----------|
| **L1** | 旧权限码残留 | migration 清理 `permissions` 表中未使用的行 |
| **L2** | `roles` 表未使用 | 保留但标记 deprecated |
| **L3** | `is_system_admin` 与 system_role_assignments 并存 | 兼容读取但不写入 |

### 7.4 多角色叠加场景

一个用户可能同时拥有多个角色（如：既是 marketplace_admin，又属于某组织的 developer）。需要明确叠加时的行为规则：

| 场景示例 | 进入 Layout | Skill 列表行为 | 关键冲突点 |
|---------|------------|---------------|-----------|
| marketplace_admin + org developer | Admin Layout（双身份标签） | 按 6.2 marketplace_admin 为主视图（市场统计/市场/个人 Tab），因为是 admin 级角色；同时顶部显示 org 入口可切到 6.1 组织视图 | 两个视图之间的切换体验；Skill 操作权限取并集（developer edit 权限可对 org skill 生效） |
| tenant_admin + org member | Admin Layout | 默认 tenant 视图（租户范围所有 Skills），可下钻到具体组织 | tenant_admin 对 Skill 只读，但作为 org member 也无编辑权 → 无冲突 |
| org admin(组织A) + developer(组织B) | Admin Layout | 「全部」视图含两个组织的 Skill + 个人 Skill | 来源列和操作列按每行 Skill 的归属和当前用户在该组织的角色动态计算，**不能**统一用一个角色判断 |

**叠加原则**：
- **Layout 入口**：有任一管理角色 → Admin Layout
- **权限取并集**：用户拥有的所有角色的权限取并集。如 marketplace_admin + org developer → 同时拥有 marketplace 管理权限 和 org developer 的 edit 权限
- **Skill 操作列动态计算**：对列表中的每行 Skill，按其归属范围（org scope / personal scope / marketplace scope）和用户在当前 scope 下的最高角色，实时计算可用操作按钮
- **身份标签**：UserNav 中展示所有管理角色标签（可多个）

> **前端实现要点**：后端返回 Skill 列表时需带上每行的 `scope` + `org_id` 等字段，前端根据用户所有角色做并集权限判断。

---

## 8. 实施计划

### Phase 0: 双轨状态模型迁移 ← **新增，最高优先级**

> **必须先于其他所有 Phase 执行。** 状态模型是后续所有 API 和前端工作的基础。

1. **数据库**:
   - 新增 `marketplace_status` 列（VARCHAR(20), DEFAULT NULL），含 CHECK 约束
   - 新增 `pre_marketplace_visibility` 列（VARCHAR(20), DEFAULT NULL），记录上架前的 visibility
   - 添加 `marketplace_status` 索引
   - 数据迁移：现有 `published + marketplace + !admin_unpublished` → `marketplace_status='listed'`；`published + marketplace + admin_unpublished` → `marketplace_status='delisted'`
   - 迁移后标记 `admin_unpublished` 为 deprecated（暂不删除列，Phase 2 再清理）
2. **Model 层**: `Skill` 结构体新增 `marketplace_status: Option<String>` 和 `pre_marketplace_visibility: Option<String>` 字段
3. **权限表**: 新增 `skill:publish_to_marketplace`、`marketplace:review_approve`、`marketplace:review_reject`、`marketplace:relist`；**移除 `org:skill_publish`**
4. **状态机**: `src/db/repositories/skill.rs` 新增 `update_marketplace_status()` 方法，含状态转换合法性校验
5. **API 适配**: `publish_skill_handler` 不再自动设 visibility=marketplace；新增 `submit_to_marketplace_handler`、`marketplace_approve_handler`、`marketplace_reject_handler`、`marketplace_relist_handler`；修改 `unpublish_skill_handler` → marketplace delist
6. **旧 API 废弃**: `admin_publish_skill_handler`（替换为 marketplace:relist）；相关 endpoint 保留兼容但内部逻辑切换到双轨

### Phase 1: 后端权限守卫修正

1. `require_admin()` → 拆分为:
   - `require_super_admin()` — System/Users/Tenants/Sessions/Audit/Settings/Sandboxes/Identities
   - `require_marketplace_or_super()` — Marketplace 相关操作
2. 新增角色分配 API:
   - `POST /api/admin/system/role-assign` → super_admin 设 marketplace_admin / tenant_admin（**仅顶级角色，不含 reviewer**）
   - `POST /api/admin/marketplace/role-assign` → marketplace_admin 设 marketplace_reviewer ← **新增，委托给 market admin**
   - `POST /api/admin/tenant/role-assign` → tenant_admin 设 org owner/admin
   - 不可跨层、不可修改自身角色
3. `profile:delete` handler 限制为仅删除自己的 profile

### Phase 2: 数据库迁移

1. 清理旧权限码（014 迁移遗留）
2. 新增通用管理权限码: `system:role_assign`, `tenant:role_assign`, `marketplace:role_assign`
3. 补充 developer 的 `skill:update` / `skill:version_create` / `skill:delete`
4. 权限分配 migration:
   - marketplace_admin 移除 `org:read`, `org:member_read` 等 org 权限
   - marketplace_admin 新增 `marketplace:role_assign`, `marketplace:review_approve`, `marketplace:review_reject`, `marketplace:relist`
   - marketplace_reviewer 新增 `marketplace:review_approve`, `marketplace:review_reject`
   - org owner/admin 新增 `skill:publish_to_marketplace`
5. 新增 `marketplace_reviewer` 角色 + 权限分配
6. 标记 `admin_unpublished` 列为 deprecated（Phase 4 后删除）

### Phase 3: 前端 Layout 优化

1. 导航组过滤: marketplace_admin/reviewer 仅显示 Content（Marketplace / Skills / Review）组
2. tenant_admin 显示 Organizations 组（组织列表、详情）+ Content
3. 身份标签在 UserNav 中展示（含 marketplace_reviewer `🛡️ 市场审核`）
4. `is_admin` 判断统一为"有任意管理角色"
5. **组织切换器**: Admin Layout 顶部增加 `OrgSwitcher` 组件，替代 Skills 页面内下拉过滤器

### Phase 4: 前端 Actions 与权限细化

1. `ACTIONS.Skills` 拆分为 `publishInternal` / `submitToMarketplace` / `withdrawFromMarketplace`
2. `profile:delete` 按钮仅对"自己的 profile"可见 + 点击二次确认
3. developer 角色按钮增强（update / version_create / delete 可见）
4. 角色分配页面（super_admin → `/system-roles` 仅设 market_admin/tenant_admin，邮箱即用；marketplace_admin → `/marketplace-roles` 设 reviewer；tenant_admin → `/tenant-roles` 设 org role）
5. marketplace_admin/reviewer 自身角色不可更改的 UI 限制
6. marketplace_reviewer 不显示「精选/取消精选」按钮
7. **市场审核按钮**: 新增「通过」「驳回」按钮（marketplace_admin/reviewer 审核待上架 Skill）
8. **重新上架按钮**: marketplace_admin 对 `delisted` Skill 显示「重新上架」
9. **双状态展示**: Skill 详情页同时显示内部状态和市场状态（见 10.9.1）

### Phase 5: Skill 列表视图重构

1. **组织切换器**: 开发 `OrgSwitcher.svelte` 置于 Admin Layout 顶部，选中后全页面 scope 跟随
2. **移除「全部」混合视图**: Skills 页不再需要 OrgFilterDropdown，改为读取 OrgSwitcher 上下文
3. **Skill 表格列**: 组织视图显示「创建者」列，个人视图隐藏
4. **marketplace_admin/reviewer Tab 页**: 市场统计 / 市场 Skill 列表 / 个人 Skill 三 Tab 结构；reviewer 无精选操作按钮
5. **市场统计 Dashboard**: 上架数、新增数、待审核数、下载量卡片 + 图表
6. **API 适配**: 前端请求参数 `scope`/`org_id`/`marketplace_status` 按角色和切换器选中状态变化
7. **tenant_admin / super_admin 视图**: tenant_admin 复用 OrgSwitcher（租户所有组织）；super_admin 使用独立多级过滤器
8. **纯 user 视图**: 简化为仅个人 Skill 列表 + 状态筛选
9. **localStorage 记忆**: 切换器选中状态持久化

### Phase 6: Skill 列表 API 改造

1. `GET /api/skills` 新增查询参数: `scope`（移除 `all`）, `org_id`, `tenant_id`, `marketplace_status`
2. 返回数据新增字段: `source_type`, `source_name`, `org_id`, `tenant_name`, `marketplace_status`
3. 后端按角色自动裁剪 scope 范围（marketplace 角色忽略 `org_id` 参数等）
4. marketplace_reviewer 路由用与 marketplace_admin 相同的 `require_marketplace_or_super()` 守卫
5. **市场审核 API**: 新增 `POST /api/skills/:id/submit-to-marketplace`、`POST /api/skills/:id/marketplace-approve`、`POST /api/skills/:id/marketplace-reject`、`POST /api/skills/:id/marketplace-relist`

---

> **下一步**: 请审阅 v2 文档，确认后按 Phase 1-4 依次实施。

---

## 9. 前端页面设计详案 ← **新增**

> 本节按角色逐一设计完整页面结构，含导航配置、数据流、操作按钮、组件树，作为前端开发的直接蓝图。

### 9.1 路由总览

```
                       ┌──────────────────────┐
                       │    /login            │  所有角色
                       └──────────┬───────────┘
                                  │
                  ┌───────────────┼───────────────┐
                  │                               │
            Admin Layout                     User Layout
         (有任意管理角色)                     (纯 user)
                  │                               │
  ┌───────────────┼───────────────┐       ┌───────┼────────┐
  │               │               │       │       │        │
super_admin  marketplace_*   org/tenant  /user  /user/   /user/
                                  │             marketplace skills submissions
                        ┌─────────┼─────────┐
                        │         │         │
                   tenant_admin  org_*    多角色叠加
```

**完整路由表**：

| 路由 | Layout | 可见角色 | 关键组件 |
|------|--------|---------|---------|
| `/login` | 无 | 未认证 | `Login.svelte` |
| `/stats` | Admin | super_admin, marketplace_* | `Dashboard.svelte` |
| `/tenants` | Admin | super_admin | `TenantList.svelte` → 含新建/编辑/删除 |
| `/tenants/:id` | Admin | super_admin | `TenantDetail.svelte` → SSO/Settings/Members |
| `/identities` | Admin | super_admin | `IdentityList.svelte` |
| `/api-keys` | Admin | super_admin | `AdminApiKeys.svelte` |
| `/sessions` | Admin | super_admin | `SessionList.svelte` |
| `/audit` | Admin | super_admin | `AuditLog.svelte` |
| `/settings` | Admin | super_admin | `SystemSettings.svelte` |
| `/sandboxes` | Admin | super_admin | `SandboxList.svelte` |
| `/system-roles` | Admin | super_admin | `SystemRoleAssign.svelte` → 邮箱+角色下拉 |
| `/marketplace` | Admin | super_admin, marketplace_*, tenant_admin, org_* | `Marketplace.svelte` |
| `/marketplace-roles` | Admin | marketplace_admin | `MarketplaceRoleAssign.svelte` → 邮箱输入 |
| `/skills` | Admin | super_admin | `SkillsSuperAdmin.svelte` → 多级过滤器 |
| `/skills` | Admin | marketplace_* | `SkillsMarketplace.svelte` → 三Tab |
| `/skills` | Admin | tenant_admin, org_* | `SkillsOrg.svelte` → OrgSwitcher 控制 |
| `/skills/:id` | Admin | (按角色) | `SkillDetail.svelte` → 动态操作按钮 |
| `/review` | Admin | super_admin, marketplace_*, org(owner,admin,reviewer) | `ReviewQueue.svelte` → 按角色自动切换：marketplace_* → 市场审核队列（marketplace_status=pending_review）；org_* → 内部审核队列（status=pending_review） |
| `/` | Admin | super_admin, tenant_admin, org_* | `Organizations.svelte` → 列表 |
| `/organizations/:id` | Admin | super_admin, tenant_admin, org_* | `OrganizationDetail.svelte` |
| `/organizations/:id/members` | Admin | org(owner,admin) | `OrgMembersTab.svelte` → 角色分配 |
| `/organizations/:id/groups` | Admin | org(owner,admin) | `OrgGroupsTab.svelte` |
| `/organizations/:id/settings` | Admin | org(owner,admin) | `OrgSettings.svelte` |
| `/org-tools` | Admin | super_admin, org(owner,admin) | `OrgTools.svelte` |
| `/tenant-roles` | Admin | tenant_admin | `TenantRoleAssign.svelte` → 设 org owner/admin |
| `/profile` | Admin | 所有管理角色 | `Profile.svelte` |
| `/my-api-keys` | Admin | 所有管理角色 | `MyApiKeys.svelte` |
| `/user` | User | 纯 user | `UserHome.svelte` |
| `/user/marketplace` | User | 纯 user | `UserMarketplace.svelte` |
| `/user/skills` | User | 纯 user | `UserSkills.svelte` |
| `/user/submissions` | User | 纯 user | `UserSubmissions.svelte` |

---

### 9.2 导航设计：按角色的 Nav 配置

侧边导航使用配置驱动，按角色动态渲染。每个导航项定义 `permission` 守卫 + 组织切换器依赖。

#### 9.2.1 导航配置表

```js
// nav-config.js — 导航组定义
const NAV_GROUPS = [
  {
    id: 'system',
    label: '系统管理',
    icon: 'settings',
    roles: ['super_admin'],
    items: [
      { route: '/stats',        label: '数据总览',   icon: 'bar-chart',  perm: 'system:admin:access' },
      { route: '/tenants',      label: '租户管理',   icon: 'building',   perm: 'tenant:read' },
      { route: '/identities',   label: '用户管理',   icon: 'users',      perm: 'system:admin:access' },
      { route: '/system-roles', label: '系统角色',   icon: 'shield',     perm: 'system:role_assign' },
      { route: '/api-keys',     label: 'API 密钥',   icon: 'key',        perm: 'system:admin:access' },
      { route: '/sessions',     label: '会话管理',   icon: 'activity',   perm: 'system:admin:access' },
      { route: '/audit',        label: '审计日志',   icon: 'file-text',  perm: 'audit:read_global' },
      { route: '/settings',     label: '系统设置',   icon: 'sliders',    perm: 'system:admin:access' },
      { route: '/sandboxes',    label: '沙箱管理',   icon: 'box',        perm: 'system:admin:access' },
    ]
  },
  {
    id: 'marketplace',
    label: '市场运营',
    icon: 'shopping-bag',
    roles: ['marketplace_admin', 'marketplace_reviewer'],
    items: [
      { route: '/stats',              label: '市场统计',     icon: 'bar-chart',  perm: 'marketplace:manage' },
      { route: '/marketplace',        label: '市场浏览',     icon: 'globe',      perm: 'skill:read' },
      { route: '/skills',             label: 'Skill 管理',   icon: 'code',       perm: 'skill:read' },
      { route: '/review',             label: '审核队列',     icon: 'check-circle', perm: 'marketplace:review_approve' },
      { route: '/marketplace-roles',  label: '审核员管理',   icon: 'user-check', perm: 'marketplace:role_assign', adminOnly: true },
    ]
  },
  {
    id: 'content',
    label: 'Skill 工作台',
    icon: 'code',
    roles: ['tenant_admin', 'org_owner', 'org_admin', 'org_reviewer', 'org_developer', 'org_member'],
    orgScoped: true,  // ← 内容由组织切换器控制
    items: [
      { route: '/skills',   label: 'Skills',     icon: 'file-code', perm: 'skill:read' },
      { route: '/review',   label: '审核',        icon: 'check-circle', perm: 'skill:approve_review', roles: ['org_owner', 'org_admin', 'org_reviewer'] },
    ]
  },
  {
    id: 'organization',
    label: '组织管理',
    icon: 'users',
    roles: ['tenant_admin', 'org_owner', 'org_admin'],
    orgScoped: true,
    items: [
      { route: '/',                         label: '组织列表',     icon: 'grid',         perm: 'org:read' },
      { route: '/organizations/:id/members',label: '成员管理',     icon: 'user-plus',    perm: 'org:member_invite' },
      { route: '/organizations/:id/groups', label: '团队管理',     icon: 'layers',       perm: 'org:read' },
      { route: '/organizations/:id/settings', label: '组织设置',   icon: 'settings',     perm: 'org:settings_read' },
      { route: '/org-tools',                label: '组织工具',     icon: 'tool',         perm: 'org:settings_read' },
    ]
  },
  {
    id: 'tenant',
    label: '租户管理',
    icon: 'briefcase',
    roles: ['tenant_admin'],
    items: [
      { route: '/tenant-roles',  label: '角色分配',   icon: 'shield',     perm: 'tenant:role_assign' },
      { route: '/tenants/:id',   label: '租户设置',   icon: 'settings',   perm: 'tenant:settings_read' },
    ]
  },
  {
    id: 'personal',
    label: '个人',
    icon: 'user',
    roles: ['all'],  // 所有 Admin Layout 角色
    items: [
      { route: '/profile',     label: '个人资料',   icon: 'user',       perm: 'profile:read' },
      { route: '/my-api-keys', label: '我的 API',   icon: 'key',        perm: 'apikey:read' },
    ]
  }
];
```

#### 9.2.2 导航渲染规则

```
Nav.render(currentUser):
  for each group in NAV_GROUPS:
    1. 用户角色 ∩ group.roles = ∅ → 跳过整个组
    2. group.orgScoped=true 且 OrgSwitcher 选中 '👤 个人' → 跳过
    3. group 内逐项过滤:
       - hasPermission(item.perm) = false → 跳过
       - item.adminOnly=true 且用户非 marketplace_admin → 跳过
       - item.roles 存在且用户角色 ∩ item.roles = ∅ → 跳过
    4. group.items 全部被过滤 → 跳过整个组
    5. 渲染 group，剩余项为可见项
```

#### 9.2.3 各角色最终导航效果

| 角色 | 可见导航组 |
|------|----------|
| **super_admin** | 系统管理 (9项) + 个人 (2项) = **11项** |
| **marketplace_admin** | 市场运营 (5项,含审核员管理) + 个人 (2项) = **7项** |
| **marketplace_reviewer** | 市场运营 (4项,无审核员管理) + 个人 (2项) = **6项** |
| **tenant_admin** | Skill 工作台 (2项) + 组织管理 (5项) + 租户管理 (2项) + 个人 (2项) |
| **org owner** | Skill 工作台 (2项) + 组织管理 (5项,含 org-tools) + 个人 (2项) |
| **org admin** | Skill 工作台 (2项) + 组织管理 (4项) + 个人 (2项) |
| **org reviewer** | Skill 工作台 (2项,审核可见) + 个人 (2项) |
| **org developer** | Skill 工作台 (1项,仅 Skills) + 个人 (2项) |
| **org member** | Skill 工作台 (1项,仅 Skills) + 个人 (2项) |
| **纯 user** | User Layout: 首页/市场/Skills/提交记录 (4项) |

---

### 9.3 按角色分页完整设计

#### 9.3.1 super_admin 页面全集

**Layout 结构**：
```
┌──────────────────────────────────────────────────────────────────────┐
│  [🔧 SkillGarden]  ← logo                               [🔔] [👤 🔧超管 ▾] │
├────────────┬─────────────────────────────────────────────────────────┤
│            │                                                         │
│  📊 数据总览 │  当前路由对应的页面内容                                   │
│  🏢 租户管理 │                                                         │
│  👥 用户管理 │                                                         │
│  🛡 系统角色 │                                                         │
│  🔑 API 密钥 │                                                         │
│  📡 会话管理 │                                                         │
│  📋 审计日志 │                                                         │
│  ⚙ 系统设置 │                                                         │
│  📦 沙箱管理 │                                                         │
│  ─────────  │                                                         │
│  👤 个人资料 │                                                         │
│  🔑 我的 API │                                                         │
│            │                                                         │
└────────────┴─────────────────────────────────────────────────────────┘
```

**关键页面**：

**(a) `/stats` — 平台总览**

```
┌──────────────────────────────────────────────────────────────┐
│  平台总览                                                     │
│                                                              │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐     │
│  │ 租户   │ │ 组织   │ │ 用户   │ │ Skill  │ │ 今日   │     │
│  │   5    │ │  23    │ │  1.2k  │ │  486   │ │ 下载   │     │
│  └────────┘ └────────┘ └────────┘ └────────┘ │ 3.2k  │     │
│                                              └────────┘     │
│  ┌─────────────────────┐ ┌─────────────────────┐            │
│  │ 租户 Skill 分布     │ │ 日活趋势 (30d)      │            │
│  │  [柱状图]           │ │  [折线图]           │            │
│  └─────────────────────┘ └─────────────────────┘            │
└──────────────────────────────────────────────────────────────┘
```

**(b) `/skills` — 全平台 Skill 视图**

```
┌──────────────────────────────────────────────────────────────┐
│  Skills                                                      │
│  过滤: [ 全部 ] [ 租户 ▾ ] [ 组织 ▾ ] [ 状态 ▾ ] [🔍 搜索]   │
│                                                              │
│  名称         │版本 │租户 │组织       │状态    │市场│操作      │
│  ─────────────┼─────┼─────┼───────────┼────────┼───┼─────────│
│  code-review  │2.1.0│腾讯 │🏢 前端组   │✅已发布 │ ✅│查 删 下  │
│  deploy-bot   │0.5.0│阿里 │🏢 后端组   │📝草稿   │ ❌│查 删     │
│  sql-helper   │1.3.0│字节 │🏢 数据组   │🔍审核中 │ —│查 审 下  │
│  my-utils     │1.0.0│ —   │👤 个人    │✅已发布 │ ✅│查 删 下  │
│                      │ 1-4 / 486 │  <  1  2 ... 50  >      │
└──────────────────────────────────────────────────────────────┘
```

**(c) `/system-roles`** — 见 4.3 节（邮箱+角色下拉）

**(d) `/tenants`** — 租户管理列表（含新建/编辑/删除）

---

#### 9.3.2 marketplace_admin 页面全集

**Layout 结构**：
```
┌──────────────────────────────────────────────────────────────────────┐
│  [🔧 SkillGarden]                                    [🔔] [👤 🏪市场管理 ▾] │
├────────────┬─────────────────────────────────────────────────────────┤
│            │                                                         │
│  📊 市场统计 │  当前路由对应的页面内容                                   │
│  🌐 市场浏览 │                                                         │
│  💻 Skill管理│                                                         │
│  ✅ 审核队列 │                                                         │
│  👥 审核员管理│                                                         │
│  ─────────  │                                                         │
│  👤 个人资料 │                                                         │
│  🔑 我的 API │                                                         │
│            │                                                         │
└────────────┴─────────────────────────────────────────────────────────┘
```

**(a) `/stats` — 市场统计**

```
┌──────────────────────────────────────────────────────────────┐
│  📊 市场统计                                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │ 已上架    │ │ 本月新增  │ │ 待审核    │ │ 本周下载  │        │
│  │   128    │ │   +12    │ │    8     │ │  1.2k   │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
│  ┌────────────────────┐ ┌────────────────────┐               │
│  │ 上架趋势 (30d)     │ │ 分类分布 (饼图)    │               │
│  └────────────────────┘ └────────────────────┘               │
│  ┌────────────────────────────────────────────┐              │
│  │ 🏆 热门 Skill Top 10                        │              │
│  │  1. code-review  ⭐4.8  下载 12.3k          │              │
│  └────────────────────────────────────────────┘              │
└──────────────────────────────────────────────────────────────┘
```

**(b) `/skills` — 市场管理 Skill 视图（三 Tab）**

```
┌──────────────────────────────────────────────────────────────┐
│  Skill 管理      [ 📊 市场统计 ] [ 📋 市场 Skill ] [ 👤 我的 ] │
├──────────────────────────────────────────────────────────────┤
│  过滤: [ 分类 ▾ ] [ 排序: 最新 ▾ ]          [🔍 搜索...]      │
│                                                              │
│  名称         │版本  │来源           │⭐  │下载  │操作         │
│  ─────────────┼──────┼───────────────┼───┼──────┼────────────│
│  code-review  │2.1.0 │🏢 前端组       │4.8│12.3k │⭐下 审 查   │
│  deploy-bot   │0.5.0 │👤 alice       │4.5│10.8k │⭐下 审 查   │
│                      │ 1-4 / 128 │  <  1  2 ... 13  >       │
│                                                              │
│  操作: ⭐=精选/取消 下=下架 审=审核 查=查看详情                  │
│  批量: [全选] [批量下架] [批量精选]                             │
└──────────────────────────────────────────────────────────────┘
```

**(c) `/review` — 市场审核队列（marketplace_status 维度）**

> **注意区分两种审核队列**：
> - **内部审核**（`/review`，org owner/admin/reviewer）：审核 `status=pending_review` 的组织 Skill，决定是否可在组织内使用
> - **市场审核**（`/review`，marketplace_admin/reviewer）：审核 `marketplace_status=pending_review` 的 Skill，决定是否可上架市场
> - 两个队列使用同一个 `ReviewQueue.svelte` 组件，根据当前用户角色 + 所选 context 自动切换数据源

```
┌──────────────────────────────────────────────────────────────┐
│  审核队列    待审核: 8    今天已处理: 23                       │
│  ┌────────────────────────────────────────────┐              │
│  │ 🔍 code-review  v2.1.0                     │              │
│  │ 🏢 前端工程组 · 提交: 张三 · 07-19          │              │
│  │ 变更说明: 新增 Python 支持                  │              │
│  │ [查看 Diff]  [查看 Skill 详情]              │              │
│  │ 审核意见: [___________________]             │              │
│  │ [✅ 通过]  [❌ 驳回]                        │              │
│  └────────────────────────────────────────────┘              │
│                        [← 上一项] [下一项 →]                  │
```

**(d) `/marketplace-roles`** — 见 4.4 节

---

#### 9.3.3 marketplace_reviewer 页面全集

与 marketplace_admin **结构相同**，差异仅在于：

| 页面 | marketplace_admin | marketplace_reviewer |
|------|:--:|:--:|
| 导航项 | 5 项（含审核员管理） | 4 项（无审核员管理） |
| `/skills` 市场 Tab 操作列 | `⭐下 审 查` | `下 审 查` |
| 批量操作 | 含「批量精选」 | 仅「批量下架」 |
| 身份标签 | `🏪 市场管理` | `🛡️ 市场审核` |
| `/marketplace-roles` | ✅ 可见 | ❌ 不可见 |

---

#### 9.3.4 tenant_admin 页面全集

**Layout 结构**（含 OrgSwitcher）：
```
┌──────────────────────────────────────────────────────────────────────┐
│  [🔧 SkillGarden]  🏢 前端工程组 ▾    [🔔] [👤 🏢租户管理 ▾]          │
├────────────┬─────────────────────────────────────────────────────────┤
│            │                                                         │
│  💻 Skills  │  当前路由 / 选中组织 对应的页面内容                        │
│  ✅ 审核    │                                                         │
│  ────────  │                                                         │
│  📋 组织列表 │                                                         │
│  👥 成员管理 │                                                         │
│  📦 团队管理 │                                                         │
│  ⚙ 组织设置 │                                                         │
│  🔧 组织工具 │                                                         │
│  ────────  │                                                         │
│  🛡 角色分配 │                                                         │
│  ⚙ 租户设置 │                                                         │
│  ────────  │                                                         │
│  👤 个人资料 │                                                         │
│  🔑 我的 API │                                                         │
│            │                                                         │
└────────────┴─────────────────────────────────────────────────────────┘
```

**关键页面**：

**(a) `/skills`** — 选中某组织后仅查看（操作列只有「查看」，见 6.3）

**(b) `/tenant-roles`** — 租户角色分配

```
┌──────────────────────────────────────────────────────────────┐
│  租户角色分配                              [+ 分配组织管理员]  │
│  用户邮箱      │ 组织      │ 角色        │ 分配时间  │ 操作   │
│  zhang@ex     │ 🏢 前端组  │ 👑 所有者   │ 07-15    │ [✕]   │
│  li@ex        │ 🏢 前端组  │ ⚙ 管理员   │ 07-16    │ [✕]   │
└──────────────────────────────────────────────────────────────┘
```

Modal：
```
┌───────────────────────────────────────┐
│  分配组织管理员                  [✕]   │
│  用户邮箱 *                           │
│  [              ]                     │
│  目标组织 *                           │
│  [ 🏢 前端工程组            ▾ ]       │
│  角色 *                               │
│  [ 👑 组织所有者 (owner)    ▾ ]       │
│    ├── 👑 组织所有者                  │
│    └── ⚙ 管理员                      │
│                      [取消] [确认]     │
└───────────────────────────────────────┘
```

---

#### 9.3.5 org owner / admin 页面全集

**Layout 结构**（含 OrgSwitcher，仅显示自己加入的组织）：
```
┌──────────────────────────────────────────────────────────────────────┐
│  [🔧 SkillGarden]  🏢 前端工程组 ▾    [🔔] [👤 👑组织所有者 ▾]        │
├────────────┬─────────────────────────────────────────────────────────┤
│            │                                                         │
│  💻 Skills  │  当前组织的数据                                          │
│  ✅ 审核    │                                                         │
│  ────────  │                                                         │
│  📋 组织列表 │                                                         │
│  👥 成员管理 │                                                         │
│  📦 团队管理 │                                                         │
│  ⚙ 组织设置 │                                                         │
│  🔧 组织工具 │                                                         │
│  ────────  │                                                         │
│  👤 个人资料 │                                                         │
│  🔑 我的 API │                                                         │
│            │                                                         │
└────────────┴─────────────────────────────────────────────────────────┘
```

**(a) `/skills`** — 与 6.1.2 一致

组织 Skill 操作列（owner/admin 同权）：
```
  自己的 Skill: [查看] [编辑] [删除] [创建版本] [提交审核] [上架到市场]
  他人的 Skill: [查看] [编辑] [删除] [创建版本] [上架到市场]
```

**(b) `/organizations/:id/members`** — 成员管理

```
┌──────────────────────────────────────────────────────────────┐
│  🏢 前端工程组 · 成员管理                        [+ 邀请成员]  │
│  成员       │ 邮箱          │ 角色      │ 加入时间  │ 操作     │
│  张三(自己) │ zhang@ex     │ 👑 owner  │ 01-15    │ —        │
│  李四       │ li@ex        │ ⚙ admin  │ 02-20    │ 改角色.. │
│  王五       │ wang@ex      │ 🔍 reviewer│ 03-10   │ 改角色.. │
│  赵六       │ zhao@ex      │ 💻 developer│ 04-05  │ 改角色.. │
│  孙七       │ sun@ex       │ 👤 member  │ 05-12    │ [移除]   │
└──────────────────────────────────────────────────────────────┘
```

角色修改下拉：
```
[ 💻 developer ▾ ]
  ├── ⚙ admin
  ├── 🔍 reviewer
  ├── 💻 developer
  └── 👤 member
```

> 不可将自己降级；owner 不可被移除。

**(c) `/organizations/:id/settings`** — 组织设置

```
  [组织名称]  [组织描述]
  [保存更改]

  ⚠️ 危险区域
  [转让组织所有权]  ← 仅 owner 可见
  [删除此组织]      ← 仅 owner 可见
```

---

#### 9.3.6 org reviewer / developer / member 页面

**Layout 精简版**（仅 Skill 工作台 + 个人）：
```
┌──────────────────────────────────────────────────────────────┐
│  [🔧 SkillGarden]  🏢 前端工程组 ▾  [🔔] [👤 🔍审核员 ▾]     │
├────────────┬─────────────────────────────────────────────────┤
│  💻 Skills  │  (内容区)                                        │
│  ✅ 审核    │  ← 仅 reviewer 可见                              │
│  ────────  │                                                 │
│  👤 个人资料 │                                                 │
│  🔑 我的 API │                                                 │
└────────────┴─────────────────────────────────────────────────┘
```

**操作列差异**：

| 操作 \ 角色 | reviewer | developer (own) | developer (他人) | member |
|-------------|:--------:|:---------------:|:----------------:|:------:|
| 查看 | ✅ | ✅ | ✅ | ✅ |
| 编辑 | ❌ | ✅ | ❌ | ❌ |
| 删除 | ❌ | ✅(二次确认※) | ❌ | ❌ |
| 创建版本 | ❌ | ✅ | ❌ | ❌ |
| 提交审核 | ❌ | ✅ | ❌ | ❌ |
| 审核操作 | ✅ | ❌ | ❌ | ❌ |

> ※ developer(own) 删除已上架市场的 Skill 时触发二次确认（见 3.9）

---

#### 9.3.7 纯 user 页面（User Layout）

```
┌──────────────────────────────────────────────────────────────────────┐
│  [🌟 SkillGarden]  [首页] [市场] [我的Skills] [提交记录]    [👤 ▾]    │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  /user            → 欢迎页 + 统计卡片 + 最近活动                       │
│  /user/marketplace → 市场 Skill 卡片列表（安装/详情）                   │
│  /user/skills     → 个人 Skill 表格（编辑/删除/提交 全权限）             │
│  /user/submissions → 提交审核记录列表                                  │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

---

### 9.4 组件树

```
App.svelte
├── Router
│   ├── Login.svelte
│   │
│   ├── AdminLayout.svelte
│   │   ├── AdminSidebar.svelte          ← 按 NAV_CONFIG + 角色渲染
│   │   │   ├── NavGroup.svelte
│   │   │   └── NavItem.svelte           ← 权限守卫
│   │   ├── AdminTopbar.svelte
│   │   │   ├── OrgSwitcher.svelte       ← 条件渲染
│   │   │   ├── NotificationBell.svelte
│   │   │   └── UserNav.svelte           ← 身份标签
│   │   └── <slot />
│   │       ├── Dashboard.svelte
│   │       ├── SystemRoleAssign.svelte
│   │       ├── MarketplaceRoleAssign.svelte
│   │       ├── TenantRoleAssign.svelte
│   │       ├── SkillsSuperAdmin.svelte
│   │       ├── SkillsMarketplace.svelte
│   │       ├── SkillsOrg.svelte         ← OrgSwitcher 驱动
│   │       │   ├── SkillTable.svelte    ← 通用表格，动态操作列
│   │       │   └── SkillDetail.svelte
│   │       ├── ReviewQueue.svelte
│   │       ├── OrganizationDetail.svelte
│   │       │   ├── OrgOverviewHeader.svelte
│   │       │   ├── OrgMembersTab.svelte ← 含角色分配
│   │       │   ├── OrgGroupsTab.svelte
│   │       │   └── OrgSettings.svelte
│   │       ├── Profile.svelte
│   │       └── MyApiKeys.svelte
│   │
│   └── UserLayout.svelte
│       ├── UserTopbar.svelte
│       └── <slot />
│           ├── UserHome.svelte
│           ├── UserMarketplace.svelte
│           ├── UserSkills.svelte
│           └── UserSubmissions.svelte
│
├── Modal (全局)
│   ├── ConfirmModal.svelte             ← 通用二次确认
│   ├── RoleAssignModal.svelte          ← 邮箱+角色分配
│   └── SkillDeleteConfirm.svelte       ← 含市场下架警告
│
└── ToastContainer.svelte
```

---

### 9.5 状态管理

#### 9.5.1 核心 Store

```js
// stores/auth.js
authStore = {
  user: { id, email, name, avatar },
  roles: ['org_admin', 'developer'],      // 所有管理角色
  permissions: ['skill:create', ...],     // 权限码并集
  orgMemberships: [                        // 加入的组织
    { org_id, org_name: '前端工程组', role: 'admin' },
    { org_id, org_name: '后端服务组', role: 'developer' },
  ],
  systemRoles: [],
  tenantRoles: [],
};

// stores/orgSwitcher.js
orgSwitcherStore = {
  selectedType: 'org' | 'personal',
  selectedOrgId: 'uuid' | null,
  selectedOrgName: '前端工程组' | null,
  currentRole: 'admin',                    // 在当前选中组织中的角色
  availableOrgs: [...],                    // 切换器列表
};

// stores/permissions.js
hasPermission(code)      → bool
hasOrgPermission(code)   → bool            // 在当前选中组织中的权限
isOrgRole(role)          → bool            // 在当前选中组织中的角色
isSystemRole(role)       → bool
```

#### 9.5.2 OrgSwitcher 驱动流程

```
登录 → authStore 加载
  ├── super_admin → AdminLayout (无 OrgSwitcher)
  ├── marketplace_* → AdminLayout (无 OrgSwitcher)
  ├── 有 orgMemberships → AdminLayout (含 OrgSwitcher)
  └── 纯 user → UserLayout

AdminLayout mount:
  ├── 初始化 OrgSwitcher:
  │   ├── 读 localStorage('orgSwitcher.selectedOrgId')
  │   ├── 有 → 恢复; 无 → 默认选角色最高的组织
  │   └── tenant_admin → 额外获取租户下所有组织
  └── 选中变化 → 刷新页面数据 + localStorage 写入
```

#### 9.5.3 操作按钮可见性公式

```js
// === 内部生命周期按钮 ===

canEdit(skill):
  return hasPermission('skill:update')
      && (isOrgRole('owner') || isOrgRole('admin')
          || (isOrgRole('developer') && skill.creator_id === user.id)
          || (skill.source_type === 'personal' && skill.creator_id === user.id))

canDelete(skill):
  return hasPermission('skill:delete')
      && (isOrgRole('owner') || isOrgRole('admin')
          || (isOrgRole('developer') && skill.creator_id === user.id)
          || (skill.source_type === 'personal' && skill.creator_id === user.id))

canSubmitReview(skill):
  return hasPermission('skill:submit_review')
      && (skill.status === 'draft' || skill.status === 'rejected')
      // 个人用户可自我审核；组织用户提交给 org reviewer

canApproveReview(skill):
  return hasPermission('skill:approve_review')
      && skill.status === 'pending_review'
      // 仅 org owner/admin/reviewer（同一组织内）

canRejectReview(skill):
  return hasPermission('skill:reject_review')
      && skill.status === 'pending_review'

canPublish(skill):
  return hasPermission('skill:publish')
      && (skill.status === 'approved'
          || (skill.source_type === 'personal' && skill.status === 'draft'))
      // 个人用户可跳过审核直接从 draft 发布

// === 市场生命周期按钮 ===

canSubmitToMarketplace(skill):
  return hasPermission('skill:publish_to_marketplace')
      && skill.status === 'published'
      && (skill.marketplace_status === null
          || skill.marketplace_status === 'rejected'
          || skill.marketplace_status === 'delisted')
      && ((isOrgRole('owner') || isOrgRole('admin'))
          || (skill.source_type === 'personal' && skill.creator_id === user.id))

canWithdrawFromMarketplace(skill):
  return hasPermission('skill:publish_to_marketplace')
      && skill.marketplace_status === 'pending_review'
      && skill.creator_id === user.id

canMarketplaceApprove(skill):
  return hasPermission('marketplace:review_approve')
      && skill.marketplace_status === 'pending_review'

canMarketplaceReject(skill):
  return hasPermission('marketplace:review_reject')
      && skill.marketplace_status === 'pending_review'

canDelist(skill):
  return hasPermission('marketplace:delist')
      && skill.marketplace_status === 'listed'

canRelist(skill):
  return hasPermission('marketplace:relist')
      && skill.marketplace_status === 'delisted'

canFeature(skill):
  return hasPermission('marketplace:feature')
      && skill.marketplace_status === 'listed' && !skill.is_featured

canUnfeature(skill):
  return hasPermission('marketplace:unfeature')
      && skill.is_featured
```

---

### 9.6 完整操作按钮矩阵

#### 9.6.1 组织 Skill 行（当前选中该组织）— 内部生命周期

| 操作 \ 角色 | owner | admin | dev(own) | dev(他人) | reviewer | member | tenant_admin |
|------------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| 查看详情 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 编辑 | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| 删除 | ✅ | ✅ | ✅※ | ❌ | ❌ | ❌ | ❌ |
| 创建版本 | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| 回滚版本 | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 提交内部审核 | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| 撤回审核 | (提交者) | (提交者) | (提交者) | ❌ | ❌ | ❌ | ❌ |
| 内部审核通过/驳回 | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ |
| 发布（内部使用） | ✅ | ✅ | ✅(own) | ❌ | ❌ | ❌ | ❌ |

> ※ = developer(own) 删除已上架 Skill 时二次确认（见 3.9）

#### 9.6.2 组织 Skill 行 — 市场生命周期操作

| 操作 \ 角色 | owner | admin | dev(own) | reviewer | member |
|------------|:---:|:---:|:---:|:---:|:---:|
| 提交到市场 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 撤回市场提交 | ✅(提交者) | ✅(提交者) | ❌ | ❌ | ❌ |

> 市场操作的审核/通过/驳回/下架/重新上架见 9.6.3 市场审核视图。

#### 9.6.3 市场审核视图（marketplace_admin/reviewer）

| 操作 \ 角色 | marketplace_admin | marketplace_reviewer | super_admin |
|------------|:---:|:---:|:---:|
| 查看详情 | ✅ | ✅ | ✅ |
| 市场审核通过 | ✅ | ✅ | ✅ |
| 市场审核驳回 | ✅ | ✅ | ✅ |
| 精选/取消精选 | ✅ | ❌ | ✅ |
| 下架 | ✅ | ✅ | ✅ |
| 重新上架 | ✅ | ❌ | ✅ |
| 强制删除 | ❌ | ❌ | ✅ |

---

### 9.7 响应式与空状态

| 场景 | 处理 |
|------|------|
| 首次登录无组织 | OrgSwitcher 默认「👤 个人空间」 |
| 组织中无 Skill | 空状态插图 + [创建第一个 Skill] 按钮 |
| 租户下无组织 | OrgSwitcher 空 + "暂无组织，请先创建" + 跳转链接 |
| 市场无上架 Skill | "暂无 Skill 上架到市场" |
| 审核队列为空 | "🎉 暂无待审核项" |
| 权限不足 | 403 页面 + 返回按钮 |
| 数据加载中 | Skeleton loader |
| 网络错误 | Toast 通知 + 重试按钮 |

---

### 9.8 前端文件结构（目标态）

```
admin/src/
├── main.js
├── App.svelte
├── app.css
│
├── layouts/
│   ├── AdminLayout.svelte
│   └── UserLayout.svelte
│
├── components/
│   ├── nav/
│   │   ├── AdminSidebar.svelte
│   │   ├── NavGroup.svelte
│   │   ├── NavItem.svelte
│   │   └── OrgSwitcher.svelte
│   ├── layout/
│   │   ├── AdminTopbar.svelte
│   │   ├── UserTopbar.svelte
│   │   ├── UserNav.svelte
│   │   └── NotificationBell.svelte
│   ├── skill/
│   │   ├── SkillTable.svelte
│   │   ├── SkillDetail.svelte
│   │   ├── SkillEdit.svelte
│   │   ├── SkillVersionHistory.svelte
│   │   └── SkillReviewActions.svelte
│   ├── org/
│   │   ├── OrgOverviewHeader.svelte
│   │   ├── OrgMembersTab.svelte
│   │   ├── OrgGroupsTab.svelte
│   │   └── OrgSettings.svelte
│   ├── modals/
│   │   ├── ConfirmModal.svelte
│   │   ├── RoleAssignModal.svelte
│   │   └── SkillDeleteConfirm.svelte
│   ├── shared/
│   │   ├── EmptyState.svelte
│   │   ├── SkeletonLoader.svelte
│   │   ├── Forbidden.svelte
│   │   ├── Pagination.svelte
│   │   └── StatusBadge.svelte
│   └── Icon.svelte
│
├── stores/
│   ├── auth.js
│   ├── orgSwitcher.js
│   └── permissions.js
│
├── lib/
│   ├── api.js
│   ├── navConfig.js
│   └── constants.js
│
├── routes/
│   ├── Login.svelte
│   ├── admin/
│   │   ├── Dashboard.svelte
│   │   ├── TenantList.svelte
│   │   ├── TenantDetail.svelte
│   │   ├── IdentityList.svelte
│   │   ├── SystemRoleAssign.svelte
│   │   ├── MarketplaceRoleAssign.svelte
│   │   ├── TenantRoleAssign.svelte
│   │   ├── SkillsSuperAdmin.svelte
│   │   ├── SkillsMarketplace.svelte
│   │   ├── SkillsOrg.svelte
│   │   ├── SkillDetail.svelte
│   │   ├── ReviewQueue.svelte
│   │   ├── Marketplace.svelte
│   │   ├── Organizations.svelte
│   │   ├── OrganizationDetail.svelte
│   │   ├── AdminApiKeys.svelte
│   │   ├── SessionList.svelte
│   │   ├── AuditLog.svelte
│   │   ├── SystemSettings.svelte
│   │   ├── SandboxList.svelte
│   │   ├── OrgTools.svelte
│   │   ├── Profile.svelte
│   │   └── MyApiKeys.svelte
│   └── user/
│       ├── UserHome.svelte
│       ├── UserMarketplace.svelte
│       ├── UserSkills.svelte
│       └── UserSubmissions.svelte
│
└── i18n/
    ├── zh-CN.js
    └── en.js
```

---

> **第 9 节总结**：前端开发时按此节的分页设计逐一实现。每角色的页面布局、操作按钮、数据流均已明确。实施顺序建议：Auth + Layout + NavConfig → OrgSwitcher → SkillTable(通用) → 各角色 Skills 页面 → 审核队列 → 角色分配页面。

---

## 10. Skill 双轨状态模型设计 ← **新增**

> 本节是本次设计最核心的架构变更。将当前单轨 `status` 拆分为**内部轨**和**市场轨**两条独立生命周期。

### 10.1 问题诊断：当前模型的三个致命缺陷

**当前代码**（`src/models/skill.rs` + `src/db/repositories/skill.rs`）：

```
draft → pending_review → approved → published (= 自动上市场, visibility=marketplace)
              ↓
           rejected
```

| # | 缺陷 | 具体表现 |
|---|------|---------|
| 1 | **内部审核 = 市场审核** | `skill:approve_review` 通过后，Skill 的 `status = approved`，但此时它既可以仅内部使用，也可以发布到市场。当前代码中 `publish` 操作直接设置 `visibility = marketplace`，**没有市场独立审核环节** |
| 2 | **published = 已上架市场** | 一个组织审核通过的 Skill 可能只想在组织内使用（`visibility = OrgVisible`），但当前 `publish` handler 强制设为 `Marketplace` |
| 3 | **`admin_unpublished` 布尔字段是 hack** | 迁移 028 新增的 `admin_unpublished BOOLEAN` 只能表达"被下架"一个额外状态，无法表达"审核中""已驳回""重新上架"等市场生命周期状态 |

**问题本质**：Skill 在当前模型中只有**一个维度**的状态，但它实际面临**两个维度的生命周期**：

- **维度 1**：「这个 Skill 在我组织/个人空间内的可用性如何？」— 内部生命周期
- **维度 2**：「这个 Skill 在市场上的状态如何？」— 市场生命周期

### 10.2 双轨模型：两条独立的状态线

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Skill 状态模型 (v3)                           │
│                                                                     │
│  轨道 1: 内部生命周期 (status)                                       │
│  ─────────────────────────────                                      │
│  draft ──→ pending_review ──→ approved ──→ published                │
│               │                  │            │                      │
│               └──→ rejected      │            │                      │
│                                  │            │                      │
│  轨道 2: 市场生命周期 (marketplace_status)                           │
│  ──────────────────────────────────────                              │
│  NULL ──→ pending_review ──→ listed ──→ delisted                    │
│               │                              │                       │
│               └──→ rejected                  └──→ pending_review     │
│                                                   (重新提交)         │
│                                                                     │
│  两条轨道独立流转，通过权限码控制交叉点                                │
└─────────────────────────────────────────────────────────────────────┘
```

### 10.3 轨道 1：内部生命周期 (`status`)

这是 Skill 在其所属范围（组织或个人）内的可用性状态。

```
                 ┌──────────┐
                 │  draft   │  创作中，仅创建者可见
                 └────┬─────┘
                      │ skill:submit_review
                      ▼
              ┌───────────────┐
              │ pending_review│  等待内部审核
              └───┬───────┬───┘
                  │       │
  skill:approve_  │       │  skill:reject_review
  review          │       │
                  ▼       ▼
          ┌──────────┐  ┌──────────┐
          │ approved │  │ rejected │  审核驳回
          └────┬─────┘  └──────────┘
               │ skill:publish
               ▼
          ┌───────────┐
          │ published │  已发布，内部可用
          └───────────┘  visibility = OrgVisible 或 Private
```

| 状态 | 含义 | 可见性 | 谁可见 | 可执行的操作 |
|------|------|--------|--------|------------|
| `draft` | 创作中，未提交审核 | Private | 仅创建者 | 编辑、删除、提交审核 |
| `pending_review` | 已提交内部审核 | Private | 创建者 + 审核者 | 审核通过/驳回、撤回 |
| `approved` | 内部审核通过 | Private | 创建者 + 审核者 | 发布（内部）、编辑 |
| `published` | 已发布，内部可用 | **OrgVisible**（组织）或 **Private**（个人） | 按 visibility | 编辑、创建版本、提交到市场 |
| `rejected` | 内部审核驳回 | Private | 创建者 + 审核者 | 重新编辑后再次提交审核 |

**关键规则**：

| 规则 | 说明 |
|------|------|
| **个人用户可自我审核** | personal(user) 可自行将 `pending_review` 转为 `approved`（无需他人审核）。也可以选择直接 `draft → published` 跳过审核 |
| **组织必须审核** | 组织内的 Skill 必须由 org owner/admin/reviewer 审核通过后方可发布 |
| **published ≠ 上市场** | `published` 仅表示内部可用。要上架市场需走轨道 2 的市场审核流程 |
| **published 是上市场的必要条件** | 只有 `status = published` 的 Skill 才能提交到市场（`skill:publish_to_marketplace`） |
| **编辑已发布的 Skill** | 编辑 `published` 状态的 Skill → 创建新版本 → 新版本独立走审核流程 |

### 10.4 轨道 2：市场生命周期 (`marketplace_status`)

这是 Skill 在 marketplace 上的状态。`marketplace_status` 是一个新增字段，默认 `NULL`。

```
                 ┌──────┐
                 │ NULL │  未提交到市场（默认值）
                 └──┬───┘
                    │ skill:publish_to_marketplace
                    ▼
            ┌───────────────┐
            │ pending_review│  等待市场管理员/审核员审核
            └───┬───────┬───┘
                │       │
marketplace:    │       │  marketplace:
review_approve  │       │  review_reject
                ▼       ▼
        ┌──────────┐  ┌──────────┐
        │  listed  │  │ rejected │  市场审核驳回
        └────┬─────┘  └────┬─────┘
             │             │
             │             │ 可重新提交 → pending_review
             │             │
    marketplace:delist      │
             │             │
             ▼             │
        ┌──────────┐       │
        │ delisted │       │
        └────┬─────┘       │
             │             │
             │ marketplace:relist
             │             │
             └──→ listed ←─┘
```

| 状态 | 含义 | visibility 变化 | 谁可见 |
|------|------|----------------|--------|
| `NULL` | 未提交到市场（默认） | 不变（OrgVisible 或 Private） | 按 visibility |
| `pending_review` | 等待市场审核 | 不变（OrgVisible 或 Private） | 按 visibility + marketplace_admin/reviewer 可查看和审核 |
| `listed` | 已上架市场 | **强制设为 `Marketplace`** | 全平台所有用户 |
| `rejected` | 市场审核驳回 | 不变 | 按 visibility（提交者可见驳回意见） |
| `delisted` | 已下架 | **回退为上架前的值**（OrgVisible 或 Private） | 按 visibility |

**关键规则**：

| 规则 | 说明 |
|------|------|
| **前置条件** | 只有 `status = published` 的 Skill 才能 `skill:publish_to_marketplace` |
| **重新提交** | `marketplace_status = rejected` 或 `delisted` 的 Skill 可重新提交（回到 `pending_review`） |
| **审核通过 → 自动上架** | `marketplace:review_approve` 直接将 `marketplace_status` 设为 `listed`，visibility 强制 `Marketplace` |
| **下架记录原 visibility** | `marketplace:delist` 将 `marketplace_status` 设为 `delisted`，`visibility` 回退到上架前保存的值 |
| **重新上架** | `marketplace:relist` 将 `delisted` 的 Skill 重新设为 `listed`，visibility 恢复为 `Marketplace` |
| **删除自动下架** | 所有者删除已上架的 Skill → 自动将 `marketplace_status` 置为 NULL，从市场索引移除 |
| **`admin_unpublished` 废弃** | 迁移 028 的 `admin_unpublished` 布尔字段被 `marketplace_status = delisted` 替代，旧字段保留但不再写入 |

### 10.5 完整状态流转决策表

以下表格覆盖所有合法的状态转换（内部轨 × 市场轨的组合变化）：

| # | 操作 | 权限 | 前置条件 | 后置状态 | 说明 |
|---|------|------|---------|---------|------|
| **内部轨道操作** |
| 1 | 创建 Skill | `skill:create` | — | `status=draft`, `marketplace_status=NULL`, `visibility=Private` | 初始状态 |
| 2 | 提交内部审核 | `skill:submit_review` | `status∈{draft,rejected}` | `status=pending_review` | 市场轨不受影响 |
| 3 | 撤回审核 | `skill:submit_review` (自己) | `status=pending_review` | `status=draft` | 仅提交者可撤回 |
| 4 | 内部审核通过 | `skill:approve_review` | `status=pending_review`, 审核者在同一组织 | `status=approved` | 市场轨不受影响 |
| 5 | 内部审核驳回 | `skill:reject_review` | `status=pending_review` | `status=rejected` | 需填写驳回理由 |
| 6 | 发布（内部使用） | `skill:publish` | `status=approved`（或个人用户 `draft`） | `status=published`, `visibility=OrgVisible` (组织) / `Private` (个人) | **不触发市场操作** |
| **市场轨道操作** |
| 7 | 提交到市场 | `skill:publish_to_marketplace` | `status=published`, `marketplace_status∈{NULL,rejected,delisted}` | `marketplace_status=pending_review` | Skill 进入市场审核队列 |
| 8 | 撤回市场提交 | `skill:publish_to_marketplace` (自己) | `marketplace_status=pending_review` | `marketplace_status=NULL` | 仅提交者可撤回 |
| 9 | 市场审核通过 | `marketplace:review_approve` | `marketplace_status=pending_review` | `marketplace_status=listed`, `visibility=Marketplace` | 自动上架，保存原 visibility 供后续回退 |
| 10 | 市场审核驳回 | `marketplace:review_reject` | `marketplace_status=pending_review` | `marketplace_status=rejected` | 需填写驳回理由 |
| 11 | 下架 | `marketplace:delist` | `marketplace_status=listed` | `marketplace_status=delisted`, `visibility=回退到上架前值` | 违规内容/主动下架 |
| 12 | 重新上架 | `marketplace:relist` | `marketplace_status=delisted` | `marketplace_status=listed`, `visibility=Marketplace` | 仅 marketplace_admin |
| **删除操作** |
| 13 | 所有者删除 | `skill:delete` | 任意状态 | Skill 删除，市场索引清理 | 见 3.9 节，已上架市场的需二次确认 |
| 14 | 超管强制删除 | `skill:delete` (super_admin) | 任意状态 | Skill 强制删除 | 跨租户，需二次确认 |

### 10.6 数据库 Schema 变更

```sql
-- 新增 marketplace_status 列
ALTER TABLE skills 
ADD COLUMN marketplace_status VARCHAR(20) DEFAULT NULL
CHECK (marketplace_status IS NULL OR marketplace_status IN (
    'pending_review', 'listed', 'rejected', 'delisted'
));

-- 新增上架前 visibility 记录列（下架时回退用）
ALTER TABLE skills
ADD COLUMN pre_marketplace_visibility VARCHAR(20) DEFAULT NULL;

-- 为 marketplace_status 添加索引（市场列表查询高频）
CREATE INDEX idx_skills_marketplace_status ON skills(marketplace_status) 
WHERE marketplace_status IS NOT NULL;
```

### 10.7 数据迁移策略

现有数据需要从单轨迁移到双轨：

| 当前状态 | 迁移后的 status | 迁移后的 marketplace_status | 迁移后的 visibility |
|---------|:---:|:---:|:---:|
| `status='published'`, `visibility='marketplace'`, `admin_unpublished=false` | `published` | `listed` | `Marketplace` |
| `status='published'`, `visibility='marketplace'`, `admin_unpublished=true` | `published` | `delisted` | `OrgVisible` (fallback) |
| `status='published'` (非 marketplace visibility) | `published` | `NULL` | 保持原值 |
| `status='approved'` | `approved` | `NULL` | 保持原值 |
| `status='pending_review'` | `pending_review` | `NULL` | 保持原值 |
| `status='draft'` | `draft` | `NULL` | 保持原值 |
| `status='rejected'` | `rejected` | `NULL` | 保持原值 |

迁移 SQL（伪代码）：
```sql
-- Step 1: 添加新列（见 10.6）
-- Step 2: 数据迁移
UPDATE skills SET marketplace_status = 'listed', pre_marketplace_visibility = visibility
WHERE status = 'published' AND visibility = 'marketplace' AND admin_unpublished = false;

UPDATE skills SET marketplace_status = 'delisted', pre_marketplace_visibility = 'org_visible', visibility = 'org_visible'
WHERE status = 'published' AND visibility = 'marketplace' AND admin_unpublished = true;

-- Step 3: 所有非市场 Skill 的 marketplace_status 保持 NULL（默认）
-- Step 4: 删除 admin_unpublished 列（可选，Phase 2 执行）
-- ALTER TABLE skills DROP COLUMN admin_unpublished;
```

### 10.8 对现有 API 的影响

| API | 当前行为 | 变更后行为 |
|-----|---------|-----------|
| `POST /api/skills/:id/submit-review` | status → pending_review | **不变**。内部审核提交 |
| `POST /api/skills/:id/approve` | status → approved | **不变**。内部审核通过。不再接受 marketplace_admin 调用此端点（市场审核走新端点） |
| `POST /api/skills/:id/reject` | status → rejected | **不变**。内部审核驳回 |
| `POST /api/skills/:id/publish` | status → published, visibility → marketplace | **关键变更**。status → published, visibility → OrgVisible（org）/ Private（personal）。**不再触发市场操作** |
| `POST /api/skills/:id/submit-to-marketplace` | — | **新增**。marketplace_status → pending_review。前置条件: status=published |
| `POST /api/skills/:id/marketplace-approve` | — | **新增**。marketplace_status: pending_review → listed。visibility → Marketplace |
| `POST /api/skills/:id/marketplace-reject` | — | **新增**。marketplace_status: pending_review → rejected |
| `POST /api/skills/:id/unpublish` (admin) | status → approved, visibility → private, admin_unpublished=true | **关键变更**。marketplace_status: listed → delisted。visibility 回退。不再修改 status |
| `POST /api/skills/:id/admin-publish` | status → published, visibility → marketplace, admin_unpublished=false | **废弃**。替换为 marketplace:relist。marketplace_status: delisted → listed |

### 10.9 前端 UI 影响

#### 10.9.1 状态展示

Skill 列表/详情中需要同时展示两个维度的状态：

```
┌──────────────────────────────────────────────────────────────┐
│  code-review  v2.1.0                                         │
│  内部状态: ✅ 已发布    市场状态: 🏪 已上架                     │
│  可见性: 市场公开                                              │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│  lint-check  v1.2.0                                          │
│  内部状态: ✅ 已发布    市场状态: 🔍 审核中                     │
│  可见性: 组织内可见                                            │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│  my-utils  v1.0.0                                            │
│  内部状态: ✅ 已发布    市场状态: —（未提交）                     │
│  可见性: 仅自己                                                │
└──────────────────────────────────────────────────────────────┘
```

#### 10.9.2 操作按钮更新

在 9.5.3 的按钮可见性公式基础上，关键命名变更：

```js
// === 命名变更 ===
// 旧 canPublishToMarket(skill) → 新 canSubmitToMarketplace(skill)（见 9.5.3）
// 旧 canPublish(skill) 含义不变（仅内部发布），公式见 9.5.3
```

> 完整的市场相关按钮公式（`canSubmitToMarketplace`、`canMarketplaceApprove`、`canMarketplaceReject`、`canDelist`、`canRelist`、`canFeature`、`canUnfeature`）已在 9.5.3 中统一定义，此处不再重复。

### 10.10 与设计文档其他章节的一致性

| 章节 | 更新内容 |
|------|---------|
| §3.3 Skill 权限 | 拆分为 3.3.1（基础）+ 3.3.2（内部审核）+ 3.3.3（市场操作）；新增 `skill:publish_to_marketplace`、`marketplace:review_approve`、`marketplace:review_reject` |
| §3.6 Marketplace 权限 | 新增 `marketplace:relist`；审核权限引用 3.3.3 |
| §3.9 删除规则 | `published_to_marketplace` → `marketplace_status='listed'` |
| §6.2 市场管理视图 | 增加 marketplace_status 筛选维度 |
| §6.7 API 参数 | `published_to_marketplace` → `marketplace_status` |
| §7 问题列表 | 新增 M12（双轨状态迁移） |
| §8 实施计划 | 新增 Phase 0（状态模型迁移，最高优先级） |
| §9.5.3 按钮公式 | 新增市场审核/上架/下架/重新上架按钮公式 |

---

> **第 10 节总结**：双轨状态模型是本设计最核心的架构变更。它将 Skill 的内部可用性（status）与市场分发（marketplace_status）完全解耦，使「组织内部使用」和「市场公开发布」成为两个独立决策。实施时必须先完成数据库迁移和状态机改造，再调整 API 和前端。
