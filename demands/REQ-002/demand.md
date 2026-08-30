# 需求：Tenant-Org-User 加入流程优化

- **需求编号**: REQ-002
- **优先级**: P1
- **状态**: 已评审
- **评审日期**: 2024-08-30
- **创建日期**: 2024-08-30
- **关联问题**: UI 逻辑混乱，租户/组织/用户/组概念模糊

## 需求描述

### 背景

当前系统存在以下问题：
1. 用户注册后无法选择租户，不知道自己属于哪个租户
2. 无法主动申请加入组织，只能等管理员邀请
3. 添加组成员需要手动输入 UUID，体验很差
4. 组织层级关系不直观（租户 → 组织 → 组 → 用户）

### 目标

1. **提升用户体验**：用户可以主动申请加入组织，管理员审批流程清晰
2. **降低使用门槛**：添加组成员支持搜索，无需记忆 UUID
3. **增强可视化**：组织层级关系清晰，支持租户 → 组织 → 组的导航

### 业务场景支持

| 场景 | 特点 | 加入方式 |
|------|------|---------|
| **SaaS 运营** | 多租户，面向外部客户 | 用户主动申请 + 管理员审批 |
| **内部交付** | 单租户，面向企业内部 | 仅管理员邀请 |

### 决策点确认

| # | 决策 | 值 |
|---|------|-----|
| A | 组织加入政策默认值 | `approval_required`（需审批） |
| B | 申请加入是否需要留言 | 可选 |
| C | 是否支持多人审批 | 是（所有 org admin 都可以审批） |
| D | Group 名称唯一性 | 同一 Org 内唯一 |

---

## 用户故事

### US-001: 用户搜索加入组
**作为** 组织成员（admin 角色）  
**我想要** 通过搜索用户名/邮箱来添加组成员  
**以便于** 不用记忆 UUID，降低操作门槛

### US-002: 用户申请加入组织
**作为** 注册用户  
**我想要** 主动申请加入某个组织  
**以便于** 快速获得组织访问权限（适用于 SaaS 场景）

### US-003: 管理员审批申请
**作为** 组织管理员  
**我想要** 审批用户的加入申请  
**以便于** 控制组织成员准入

### US-004: 租户管理员查看下属组织
**作为** 租户管理员  
**我想要** 在租户详情页看到所有下属组织  
**以便于** 快速导航和管理

---

## 功能要求

### Phase 1: 核心体验优化

#### 1.1 组员搜索添加（P0）
- [ ] 后端新增 `/identities/search?q={query}` API，支持按名字/邮箱/username 模糊搜索
- [ ] GroupDetail.svelte 改造：移除 UUID 输入框，改为搜索下拉组件
- [ ] 保留 UUID 输入方式作为备选（高级用户）

#### 1.2 加入申请流程（P1）
- [ ] 新增 `org_join_requests` 表
- [ ] 新增 `/orgs/{id}/join-request` API（POST 创建申请）
- [ ] 新增 `/orgs/{id}/join-requests` API（GET 列表、PUT 审批）
- [ ] Organizations.svelte 增加「申请加入」按钮
- [ ] OrganizationDetail.svelte 增加「管理申请」Tab（管理员可见）
- [ ] 申请状态变更通知申请人

#### 1.3 组织层级可视化（P1）
- [ ] Tenants.svelte 增强：显示关联组织列表，支持跳转
- [ ] Tenants.svelte 增强：显示租户管理员列表
- [ ] OrganizationDetail.svelte 侧边栏显示所属租户信息

### Phase 2: 权限体系完善（后续迭代）
- [ ] 统一 Group 权限模型定义
- [ ] Group 角色（leader/member）实现
- [ ] 权限叠加逻辑

### Phase 3: 运营增强（后续迭代）
- [ ] 邀请链接生成
- [ ] 注册时租户选择
- [ ] 组织公开/私有配置

---

## 非功能要求

| 维度 | 要求 |
|------|------|
| **性能** | 用户搜索接口响应 < 200ms |
| **安全** | 申请审批需权限校验，仅 org admin 可操作 |
| **兼容性** | 前端改动保持现有 UI 风格一致 |
| **可扩展** | 未来可扩展为邀请链接方式 |

---

## 数据模型变更

### 新增表: org_join_requests

```sql
CREATE TABLE org_join_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    identity_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' 
        CHECK (status IN ('pending', 'approved', 'rejected')),
    message TEXT,
    reviewed_by UUID REFERENCES identities(id),
    reviewed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_pending_request UNIQUE (organization_id, identity_id)
);

CREATE INDEX idx_org_join_requests_org ON org_join_requests(organization_id);
CREATE INDEX idx_org_join_requests_identity ON org_join_requests(identity_id);
CREATE INDEX idx_org_join_requests_status ON org_join_requests(status);
```

### 新增表: identities_search（全文搜索视图）

```sql
-- 创建搜索视图用于模糊查询
CREATE OR REPLACE VIEW identities_search AS
SELECT 
    id,
    name,
    username,
    email,
    identity_type,
    status,
    CONCAT_WS(' ', name, username, email) as search_text
FROM identities
WHERE status = 'active';

CREATE INDEX idx_identities_search ON identities USING gin(to_tsvector('simple', search_text));
```

### 新增字段: organizations.join_policy

```sql
ALTER TABLE organizations 
ADD COLUMN join_policy VARCHAR(20) 
NOT NULL DEFAULT 'approval_required'
CHECK (join_policy IN ('invite_only', 'approval_required', 'open'));

COMMENT ON COLUMN organizations.join_policy IS 
'加入政策: invite_only=仅邀请, approval_required=需审批, open=开放加入';
```

---

## API 设计

### 1. 用户搜索
```
GET /identities/search?q={query}&limit={limit}&org_id={org_id}

Request:
  - q: 搜索关键词（名字/邮箱/username）
  - limit: 返回数量，默认 10，最大 50
  - org_id: 可选，限制为特定组织的成员

Response 200:
{
  "data": [
    {
      "id": "uuid",
      "name": "张三",
      "username": "zhangsan",
      "email": "zhangsan@example.com",
      "identity_type": "user",
      "avatar_url": "..."
    }
  ]
}
```

### 2. 加入申请
```
POST /orgs/{org_id}/join-request
Authorization: Bearer {token}

Request:
{
  "message": "我想加入团队一起学习 AI 开发"  // 可选
}

Response 201:
{
  "id": "uuid",
  "organization_id": "uuid",
  "status": "pending",
  "message": "我想加入团队...",
  "created_at": "2024-08-30T12:00:00Z"
}

Response 400:
- 已存在待处理的申请
- 组织加入政策不允许申请（invite_only/open 场景）

DELETE /orgs/{org_id}/join-request
// 用户取消自己的申请
```

### 3. 申请管理（管理员）
```
GET /orgs/{org_id}/join-requests?status=pending
Authorization: Bearer {token} (需 org admin 权限)

Response 200:
{
  "data": [
    {
      "id": "uuid",
      "identity": {
        "id": "uuid",
        "name": "张三",
        "email": "zhangsan@example.com"
      },
      "status": "pending",
      "message": "我想加入...",
      "created_at": "2024-08-30T12:00:00Z"
    }
  ]
}

PUT /orgs/{org_id}/join-requests/{request_id}
Authorization: Bearer {token} (需 org admin 权限)

Request:
{
  "action": "approve" | "reject",
  "message": "审批备注"  // 可选
}

Response 200:
{
  "id": "uuid",
  "status": "approved" | "rejected",
  "reviewed_by": "admin_uuid",
  "reviewed_at": "2024-08-30T12:30:00Z"
}

// 审批通过时自动创建 org_memberships 记录
```

---

## 前端改动

### 1. GroupDetail.svelte - 搜索添加成员

```
改动前:
┌────────────────────────────────┐
│ 添加组成员                      │
│ Identity ID: [____________]    │
│                    [添加]      │
└────────────────────────────────┘

改动后:
┌────────────────────────────────────────────────────┐
│ 添加组成员                                          │
│ 🔍 搜索用户: [________________▼]                    │
│         ┌─────────────────────────────┐            │
│         │ 🔍 张三  zhangsan@...  [+] │            │
│         │ 🔍 李四  lisi@...     [+] │            │
│         └─────────────────────────────┘            │
│                                                      │
│ 或输入 Identity ID: [____________] [添加]          │
└────────────────────────────────────────────────────┘
```

### 2. OrganizationDetail.svelte - 申请/管理加入

```
位置: Members Tab 旁边新增 "申请" 或 "管理" Tab

普通用户视角:
┌─────────────────────────────────────────────────────┐
│  [Members] [申请加入]                               │
│  ───────────────────────────────────────────────   │
│  🚀 Engineering                                    │
│  ──────────────────────────────────────────────    │
│  加入方式: 需要审批                                 │
│  ┌──────────────────────────────────────────────┐ │
│  │ 留言（可选）:                                 │ │
│  │ [我想加入团队一起学习 AI 开发...]            │ │
│  └──────────────────────────────────────────────┘ │
│                                    [提交申请]      │
│                                                      │
│  我的申请状态: pending                              │
│  提交时间: 2024-08-30 12:00                         │
└─────────────────────────────────────────────────────┘

管理员视角:
┌─────────────────────────────────────────────────────┐
│  [Members] [管理申请]                               │
│  ───────────────────────────────────────────────   │
│  待审批申请 (3)                                    │
│  ┌────────────────────────────────────────────────┐│
│  │ 👤 张三 zhangsan@...   申请: 2小时前          ││
│  │    留言: "我想加入团队..."                    ││
│  │    [拒绝] [批准]                              ││
│  ├────────────────────────────────────────────────┤│
│  │ 👤 李四 lisi@...      申请: 1天前             ││
│  │    留言: "对 AI 开发感兴趣"                  ││
│  │    [拒绝] [批准]                              ││
│  └────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────┘
```

### 3. Tenants.svelte - 租户详情增强

```
改动: 点击租户卡片进入详情页（原有列表模式 + 新增详情模式）

┌─────────────────────────────────────────────────────────┐
│  🏢 Acme Corporation                     [编辑] [删除] │
│  ─────────────────────────────────────────────────────  │
│  Plan: Enterprise                                         │
│  成员数: 50 | 组织数: 3 | 创建: 2024-01-15            │
├─────────────────────────────────────────────────────────┤
│  📋 关联组织                                              │
│  ┌────────────────────────────────────────────────────┐ │
│  │ 🏢 Engineering         15 成员   [管理] [查看]     │ │
│  │ 🏢 Sales               20 成员   [管理] [查看]     │ │
│  │ 🏢 Marketing           15 成员   [管理] [查看]     │ │
│  └────────────────────────────────────────────────────┘ │
│  [+ 创建组织]                                            │
├─────────────────────────────────────────────────────────┤
│  👥 租户管理员                                            │
│  ┌────────────────────────────────────────────────────┐ │
│  │ 👤 admin@acme.com                    tenant_admin  │ │
│  │ 👤 co-admin@acme.com                tenant_admin  │ │
│  └────────────────────────────────────────────────────┘ │
│  [+ 添加管理员]                                          │
└─────────────────────────────────────────────────────────┘
```

---

## 技术实现要点

### 后端 (Rust/Axum)

1. **搜索优化**: 使用 PostgreSQL 全文搜索或 ILIKE 模糊匹配
2. **权限校验**: 
   - 申请加入: 需登录用户
   - 审批: 需 org admin 或更高权限
3. **事务处理**: 审批通过时原子创建 membership 记录

### 前端 (Svelte)

1. **防抖搜索**: 300ms 防抖避免频繁请求
2. **状态管理**: 申请状态实时更新
3. **权限控制**: 根据用户角色显示/隐藏按钮

---

## 影响范围

| 文件 | 改动类型 | 说明 |
|------|---------|------|
| `src/db/migrations/` | 新增 | 迁移脚本 |
| `src/db/repositories/` | 新增 | join_request 仓库 |
| `src/api/handlers/identities.rs` | 新增 | 搜索 API |
| `src/api/handlers/orgs.rs` | 新增 | 申请/审批 API |
| `src/api/routes.rs` | 修改 | 路由注册 |
| `admin/src/routes/GroupDetail.svelte` | 修改 | 搜索组件 |
| `admin/src/routes/OrganizationDetail.svelte` | 修改 | 申请管理 Tab |
| `admin/src/routes/Tenants.svelte` | 修改 | 详情页增强 |
| `admin/src/lib/api.js` | 新增 | API 封装 |

---

## 里程碑

- [ ] 需求评审通过
- [ ] 数据模型迁移完成
- [ ] 后端 API 开发完成
- [ ] 前端 UI 开发完成
- [ ] QA 验证通过
- [ ] 合并上线

---

## 优先级说明

**P1 原因**：
- Phase 1.1 是核心体验问题，当前 UUID 输入方式几乎不可用
- Phase 1.2-1.3 是用户反馈强烈的问题
- 属于「重要功能缺失」级别
