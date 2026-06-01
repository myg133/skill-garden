# Multi-Tenant Admin & RBAC System Design

> **设计参考**: GitHub 的用户/组织模型 — 用户可以独立存在（个人账户），也可以加入多个组织。

---

## 1. 设计目标

构建企业级多租户、多组织的 Skill 管理平台，支持：

- **用户独立与组织协作并存** — 参考 GitHub 模型，用户可以是个人账户，也可以同时属于多个组织
- **Skill 所有权模型** — Skill 归属于用户（个人 Skill）或组织（团队 Skill），严格控制编辑权限
- **组织级 Skill 审核** — 审核在组织内部完成，系统级仅负责全局治理
- **跨组织 Skill 共享** — 公开 Skill（marketplace）跨组织可发现、安装、使用，但不能修改
- **Agent 自助注册** — Agent 用户可自行注册、获取 API Key，挂靠组织或独立工作
- **企业级能力** — 租户隔离、SSO、配额、审计、分析

---

## 2. 核心实体模型

### 2.1 实体关系图

```
┌─────────────────────────────────────────────────────────────────┐
│                         Tenant (租户/企业)                        │
│  企业级容器：计费、SSO、全局策略、多组织管理                          │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                  Organization (组织/团队)                    │  │
│  │  类似 GitHub Organization，Skill 的协作和所有权单元             │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │  │
│  │  │   Group A    │  │   Group B    │  │   Group C    │        │  │
│  │  │  (子团队)     │  │  (子团队)     │  │  (子团队)     │        │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘        │  │
│  │                                                            │  │
│  │  Skills: [skill-a, skill-b]    Members: [alice, bob]       │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌─────────────────┐   ┌─────────────────┐                       │
│  │  User (alice)   │   │  User (bob)     │  独立个人用户也可以    │
│  │  member of Org1 │   │  independent    │  不属于任何组织        │
│  │  + Org2          │   │  personal skills│                       │
│  └─────────────────┘   └─────────────────┘                       │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 核心设计原则

| 原则 | 说明 |
|------|------|
| **User 是一等公民** | 用户可以不归属任何组织（个人账户），也可以加入多个组织 |
| **Skill 归属明确** | 每个 Skill 归属一个 Owner（User 或 Organization）。个人 Skill 只有 owner 可编辑；组织 Skill 由 Org 角色和 Group 关系共同决定编辑权 |
| **审核在组织内** | Skill 提交到 marketplace 前，由所属组织的 Reviewer/Admin 审核 |
| **公开只读共享** | marketplace Skill 任何组织可安装使用，但不能修改源代码 |
| **Agent 即 User** | Agent 就是 `type=agent` 的 User，注册、API Key 流程一致 |

---

## 3. 数据库模型

### 3.1 Tenant (租户)

企业级隔离容器，承载组织、计费、SSO 配置。

```sql
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    status VARCHAR(50) DEFAULT 'active',        -- active, suspended, deleted
    billing_plan VARCHAR(50) DEFAULT 'free',     -- free, pro, enterprise
    sso_config JSONB DEFAULT NULL,               -- SSO/OIDC 配置
    settings JSONB DEFAULT '{}',                 -- 租户级全局配置
    created_by UUID REFERENCES users(id),       -- 创建者（super_admin）
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### 3.2 User (用户)

**统一用户模型** — 所有人（人类管理员、Agent、外部调用者）都是 User。参考 GitHub 用户模型。

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(255) UNIQUE NOT NULL,
    display_name VARCHAR(255),
    email VARCHAR(255) UNIQUE,
    avatar_url VARCHAR(500),

    -- 用户类型：human(人类管理员), agent(AI Agent), service(服务账号)
    user_type VARCHAR(50) NOT NULL DEFAULT 'human',

    -- 认证凭据（human: bcrypt, agent: secret hash, service: api key only）
    password_hash VARCHAR(255),

    status VARCHAR(50) DEFAULT 'active',         -- active, suspended, deleted
    metadata JSONB DEFAULT '{}',                 -- 扩展元数据

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_user_type ON users(user_type);
```

**User 类型的含义**：

| user_type | 认证方式 | 典型场景 |
|-----------|---------|----------|
| `human` | 用户名+密码 / SSO | 平台管理员、组织管理员、开发者 |
| `agent` | agent_id + secret / API Key | Claude、GPT 等 AI Agent 调用平台 |
| `service` | API Key only | CI/CD、Webhook、自动化脚本 |

#### 系统级角色分配

系统级角色（super_admin、marketplace_admin）不属于任何组织，通过独立表分配：

```sql
CREATE TABLE system_role_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_name VARCHAR(50) NOT NULL,              -- 'super_admin' | 'marketplace_admin'
    assigned_by UUID REFERENCES users(id),
    assigned_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(user_id, role_name)
);
```

### 3.3 Organization (组织)

类似 GitHub Organization — Skill 的协作单元和所有权边界。

```sql
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE SET NULL,  -- 可选，独立组织不需要
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) NOT NULL,
    display_name VARCHAR(255),
    description TEXT,
    avatar_url VARCHAR(500),

    -- 可见性：public(公开组织), private(私有组织)
    visibility VARCHAR(50) DEFAULT 'public',

    status VARCHAR(50) DEFAULT 'active',         -- active, suspended, deleted

    -- 组织级策略配置
    settings JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(tenant_id, slug)
);

CREATE INDEX idx_organizations_tenant ON organizations(tenant_id);
CREATE INDEX idx_organizations_slug ON organizations(slug);
```

### 3.4 Organization Membership (组织成员)

用户与组织的多对多关系，带角色。**一个用户可以属于多个组织**。

```sql
CREATE TABLE org_memberships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    -- 组织内角色
    role VARCHAR(50) NOT NULL DEFAULT 'member',
    -- owner: 组织所有者（最高权限）
    -- admin: 组织管理员（管理成员、Skill、设置）
    -- reviewer: Skill 审核员（审核组织内提交的 Skill）
    -- developer: 开发者（创建和编辑 Skill）
    -- member: 普通成员（查看和使用 Skill）

    joined_at TIMESTAMPTZ DEFAULT NOW(),
    invited_by UUID REFERENCES users(id),

    UNIQUE(user_id, organization_id)
);

CREATE INDEX idx_org_memberships_user ON org_memberships(user_id);
CREATE INDEX idx_org_memberships_org ON org_memberships(organization_id);
```

### 3.5 Group (组 — 可选)

组织内的子团队划分，用于精细化管理。**参考 GitHub Teams 模型**。

#### Group-User-Organization 三者关系

```
Organization (组织)
  │
  │  用户必须先加入组织，才能加入组织内的 Group
  │  org_memberships.role 定义用户在组织内的基本角色
  │
  ├── User alice ── org_memberships(role=developer)
  │     │
  │     ├── Group "frontend" ── group_memberships(role=member)
  │     └── Group "platform"  ── group_memberships(role=lead)
  │
  ├── User bob ──── org_memberships(role=admin)
  │     │
  │     └── Group "frontend" ── group_memberships(role=member)
  │
  └── User carol ── org_memberships(role=member)
        （只加入组织，不加入任何 Group — 完全允许）
```

**核心约束**：

| 规则 | 说明 |
|------|------|
| **Group 必须属于 Org** | `groups.organization_id` FK，Group 不能独立存在 |
| **先 Org 后 Group** | 用户必须先有 `org_memberships` 记录，才能被加入该 Org 下的 Group |
| **Org 可选 Group** | 用户可以是 Org 成员但不加入任何 Group（默认拥有 Org 级别权限即可） |
| **Group 角色叠加** | Group 内的 `lead` 角色在 Group 范围内获得额外权限，不影响 Org 级别角色 |
| **多 Group 归属** | 一个用户在同一 Org 内可以属于多个 Group |

#### 权限叠加规则

```
用户的最终权限 = Org 角色权限 ∪ Group 角色附加权限

示例：
  alice: org_memberships(role=developer) + group_memberships(frontend, role=lead)
  
  → alice 在整个 Org 内拥有 developer 权限（创建 Skill、编辑自己的 Skill）
  → alice 在 frontend Group 内额外拥有 lead 权限（管理 Group 成员、审批 Group 内 Skill）
  → alice 对 platform Group 的 Skill 没有特殊权限（只是 org developer 基本权限）
```

#### 表定义

```sql
CREATE TABLE groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(organization_id, slug)
);

CREATE TABLE group_memberships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Group 内角色：lead(组长) / member(组员)
    role VARCHAR(50) NOT NULL DEFAULT 'member',

    joined_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(group_id, user_id)
);

CREATE INDEX idx_group_memberships_group ON group_memberships(group_id);
CREATE INDEX idx_group_memberships_user ON group_memberships(user_id);
```

#### 应用层约束（数据库不强制，代码保证）

```sql
-- 加入 Group 前必须验证：
-- 用户已是该 Group 所属 Organization 的成员
-- 
-- 伪代码：
-- INSERT INTO group_memberships (group_id, user_id, role) VALUES (...)
-- WHERE EXISTS (
--     SELECT 1 FROM org_memberships om
--     JOIN groups g ON g.organization_id = om.organization_id
--     WHERE g.id = :group_id AND om.user_id = :user_id
-- )
```

#### Group 的典型使用场景

| 场景 | Group 示例 | 作用 |
|------|-----------|------|
| **Skill 分工协作** | frontend / backend / ml-team | 不同团队维护各自的 Skill 集合 |
| **审核分流** | review-panel-a / review-panel-b | 将大量审核请求分流到不同审核小组 |
| **项目隔离** | project-alpha / project-beta | 项目级别的 Skill 可见性和权限隔离 |
| **跨职能管理** | security-review / compliance | 特定职能的 Group 拥有跨 Group 的审核权 |

### 3.6 API Key

每个 User 可创建多个 API Key，用于外部 Agent 或自动化服务调用。

```sql
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- API Key 可限定作用范围（组织级别）
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,

    key_hash VARCHAR(255) NOT NULL,
    key_prefix VARCHAR(12) NOT NULL,             -- 前 12 位明文，用于 UI 识别

    name VARCHAR(255),                           -- Key 的描述名称
    scopes JSONB DEFAULT '[]',                   -- 权限范围: ["skill:read", "skill:install"]
    rate_limit INTEGER DEFAULT 1000,             -- 每分钟请求数限制
    status VARCHAR(50) DEFAULT 'active',
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_api_keys_user ON api_keys(user_id);
CREATE INDEX idx_api_keys_org ON api_keys(organization_id);
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);
```

### 3.7 Skill (核心)

Skill 所有权模型：**每个 Skill 必须有一个 Owner**。

```sql
CREATE TABLE skills (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    version VARCHAR(50) NOT NULL,
    content TEXT NOT NULL DEFAULT '',

    -- 创建者
    author_id UUID NOT NULL REFERENCES users(id),

    -- 所有权模型：'user' 个人 Skill / 'organization' 组织 Skill
    owner_type VARCHAR(50) NOT NULL DEFAULT 'user',
    owner_id UUID NOT NULL,

    -- 可见性
    visibility VARCHAR(50) NOT NULL DEFAULT 'private',
    -- private: 仅 Owner 可见可用
    -- org_visible: Owner + 同组织成员可见可用（owner_type=organization 时有效）
    -- marketplace: 公开市场，所有人可发现、安装、使用（但不可修改）

    -- 审核状态（组织级审核）
    review_status VARCHAR(50) DEFAULT 'draft',
    -- draft: 草稿（Owner 内部可见）
    -- pending_review: 等待审核（提交到组织审核队列）
    -- approved: 审核通过（可发布到 marketplace）
    -- rejected: 审核驳回（附带驳回原因）

    reviewed_by UUID REFERENCES users(id),
    reviewed_at TIMESTAMPTZ,
    review_comment TEXT,

    -- Git 仓库
    git_url VARCHAR(500),

    compatibility VARCHAR(100) DEFAULT '>=1.0.0',
    install_count INTEGER NOT NULL DEFAULT 0,

    -- 引用的工具列表
    skill_tools JSONB DEFAULT '[]',

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(name, version, owner_type, owner_id)
);

CREATE INDEX idx_skills_owner ON skills(owner_type, owner_id);
CREATE INDEX idx_skills_author ON skills(author_id);
CREATE INDEX idx_skills_visibility ON skills(visibility);
CREATE INDEX idx_skills_review ON skills(review_status);
CREATE INDEX idx_skills_name ON skills(name);
CREATE INDEX idx_skills_created ON skills(created_at DESC);
```

**Skill 所有权与可见性矩阵**：

| owner_type | visibility | 谁能看到 | 谁能使用 | 谁能编辑 | 谁能审核 |
|-----------|-----------|---------|---------|---------|---------|
| user | private | Owner 本人 | Owner 本人 | Owner 本人 | N/A |
| user | marketplace | 所有人 | 所有人 | Owner 本人 | 平台审核员 |
| organization | private | Org 成员 + 关联 Group 成员 | Org 成员 | 见下方「编辑权限判断」 | N/A |
| organization | org_visible | Org 成员 | Org 成员 | 见下方「编辑权限判断」 | N/A |
| organization | marketplace | 所有人 | 所有人 | 见下方「编辑权限判断」 | Org Reviewer |

**所有权本质**：

```
┌──────────────────────────────────────────────────────────────┐
│  owner_type = user                                           │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Skill 属于个人                                        │   │
│  │  owner_id = alice                                     │   │
│  │  只有 alice 能编辑，alice 离开平台也不影响              │   │
│  │  类似 GitHub 个人仓库                                   │   │
│  └──────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│  owner_type = organization                                   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Skill 属于组织，不属于个人                              │   │
│  │  owner_id = org_acme                                   │   │
│  │  author_id 仅记录"谁创建的"（审计用途，不授予编辑权）     │   │
│  │  编辑权来自于：Org 角色（admin/owner）或 Group 成员关系   │   │
│  │  离开组织 = 失去所有编辑权                               │   │
│  │  类似 GitHub Enterprise 组织仓库                         │   │
│  └──────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

---

### 3.7.1 Skill 编辑权限判断（核心）

编辑权限分两种情况 — **个人 Skill 和组织 Skill 的判断逻辑完全不同**。

#### 个人 Skill（owner_type = user）

```
判断个人 Skill 的编辑权限：

  ┌─ 是 owner_id（所有者）？ ── YES → ✅ 允许编辑
  └─ 其他人 ──────────────────────────→ ❌ 只读
```

#### 组织 Skill（owner_type = organization）

```
判断组织 Skill 的编辑权限（author_id 不参与判断）：

  ┌─ 1. 是 Org owner/admin？ ──────────────── YES → ✅ 允许编辑
  │     （组织管理员可以编辑组织内所有 Skill）
  │
  ├─ 2. Skill 关联了 Group 且用户在 Group 中？
  │     │
  │     ├─ 在 Group 中是 lead？ ──────────── YES → ✅ 允许编辑
  │     └─ 在 Group 中是 member？ ────────── YES → ✅ 允许编辑（协作模式）
  │
  └─ 3. 以上都不满足 ────────────────────────────→ ❌ 只读
       （包括 author 本人，如果已不在 Org 中或不在关联 Group 中）
```

**关键区别**：组织 Skill 的 `author_id` 仅用于审计记录（"谁创建的"），**不授予编辑权**。编辑权完全来自 Org 成员关系或 Group 成员关系。

#### 典型场景演练（组织 Skill）

| # | 场景 | author | Org角色 | Group角色 | Skill关联Group | 能否编辑 | 原因 |
|---|------|--------|---------|----------|:---:|:---:|------|
| A | 创建者在 Org 内，无 Group | alice | developer | 无 | 无 | ❌ | 不在 Org admin 也不在关联 Group |
| B | 创建者在 Org 内，关联到自己的 Group | alice | developer | frontend: lead | frontend | ✅ | Group lead |
| C | Org 管理员管理全局 | bob | admin | 无 | 无 | ✅ | Org admin |
| D | Group member 协作编辑 | carol | developer | frontend: member | frontend | ✅ | Group member |
| E | 非 Group 成员尝试编辑 | dave | developer | backend: member | frontend | ❌ | 不在 frontend Group |
| F | **创建者离开组织** | alice | 已离开 | 已离开 | frontend | ❌ | Skill 属于组织，离开=失去一切编辑权 |

> **场景 A vs 场景 F 的设计意图**：组织 Skill 的所有权属于组织。author 本人如果要编辑，也必须通过 Org 角色或 Group 关系。这避免了"人走了 Skill 的编辑权还在那人手里"的问题。Org admin 始终可以接管任何组织 Skill。

#### 如果创建者离开后 Skill 没人管了？

```
Skill "deploy-tool" — alice 创建并关联到 frontend Group
alice 是 frontend 唯一的 lead，也是唯一的成员

alice 离开组织 →
  frontend Group 变成空组
  deploy-tool 没有活跃的编辑者了
  
  Org admin (bob) 介入：
    1. 将自己或其他人加入 frontend Group（或新 Group）
    2. 或者将 Skill 重新关联到有人的 Group
    3. Org admin 本身也可以直接编辑
```

---

### 3.7.2 Skill 创建权限判断

```
判断用户能否创建 Skill：

  用户需先选择 Skill 归属类型（owner_type）：

  ┌─ 1. 创建组织 Skill（owner_type=organization）
  │     │
  │     ├─ 用户对该组织有 developer/admin/owner 角色？ ── YES → ✅ 允许创建
  │     └─ 否则 ─────────────────────────────────────────────→ ❌ 无权限
  │
  └─ 2. 创建个人 Skill（owner_type=user）
        └─ 任何注册用户均可创建 ─────────────────────────────→ ✅ 允许创建

  Group 不是创建 Skill 的前置条件。
  即使用户不在任何 Group 中，只要有 Org developer 角色，就可以创建组织 Skill。
```

**设计原则**：

| 原则 | 说明 |
|------|------|
| **创建时明确归属** | 用户在创建 Skill 时需选择归属（个人/组织）。在组织空间下默认归属该组织，在个人空间下默认归属个人，但均可切换 |
| **创建门槛在 Org 层** | 组织 Skill 创建由 Org 角色控制（developer 及以上），不由 Group 控制 |
| **Group 只控制编辑** | Group 的作用是"分享编辑权"，不是"授予创建权" |
| **个人用户也能创建** | 无论是否属于组织，任何注册用户都可以创建 `owner_type=user` 的个人 Skill |
| **个人 Skill 创建者可编辑** | 对于 `owner_type=user` 的个人 Skill，author_id 即 owner，始终可编辑。组织 Skill 的编辑权由 Org 角色和 Group 关系决定，author_id 仅作审计用途 |

#### 创建上下文与归属选择

Skill 创建有两种入口上下文，影响 `owner_type` 的默认值，但用户始终可以显式切换：

```
┌─────────────────────────────────────────────────────────────┐
│  入口 1: 个人空间 — POST /api/v1/skills                      │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  默认: owner_type = "user"                           │    │
│  │  用户可切换为 organization，需指定 target_org_slug    │    │
│  │  前提: 用户在该组织有 developer 及以上角色             │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  入口 2: 组织空间 — POST /api/v1/orgs/{slug}/skills          │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  默认: owner_type = "organization"，owner_id = 该组织 │    │
│  │  用户可切换为 user（个人 Skill）                       │    │
│  │  前提: 用户在该组织有 developer 及以上角色             │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

**归属约束**：

| 约束 | 说明 |
|------|------|
| 组织 Skill 必须指定目标组织 | `owner_type=organization` 时，`owner_id` 必须是一个用户拥有 developer+ 角色的组织 |
| 个人 Skill 无组织依赖 | `owner_type=user` 时，任何注册用户均可创建，不要求属于任何组织 |
| 创建后归属不可变更 | Skill 创建后 `owner_type` 和 `owner_id` 不可修改（如需迁移，走 `org:skill_transfer` 转移流程） |
| 跨组织创建不可行 | 用户不能将 Skill 归属到一个自己没有 developer 角色的组织 |

#### Skill 创建场景

| # | 用户 | Org角色 | 创建入口 | 选择 owner_type | 能否创建 | 结果 |
|---|------|---------|---------|:---:|:---:|------|
| H | alice | developer (Org-A) | 组织空间 Org-A | organization | ✅ | 归属 Org-A |
| I | bob | developer (Org-A) | 个人空间 | 切换为 organization (Org-A) | ✅ | 归属 Org-A |
| J | carol | member (Org-A) | 组织空间 Org-A | organization | ❌ | member 无创建权限 |
| K | carol | member (Org-A) | 个人空间 | user | ✅ | 归属个人（member 也能创建个人 Skill） |
| L | dave | 无组织 | 个人空间 | user | ✅ | 归属个人 |
| M | eve | developer (Org-A, Org-B) | 组织空间 Org-B | organization | ✅ | 归属 Org-B（eve 在两个组织都是 developer） |
| N | frank | developer (Org-A) | 组织空间 Org-B | organization | ❌ | frank 在 Org-B 无角色 |
| O | frank | developer (Org-A) | 组织空间 Org-B | user | ✅ | 归属个人（切换为个人 Skill） |

> **关键理解**：
> - 场景 I 和 K：同一个 bob，在个人空间下仍可创建组织 Skill（只要显式指定目标组织且有权限）
> - 场景 J 和 K：carol 虽然是 member（不能创建组织 Skill），但仍可以创建个人 Skill
> - 场景 M 和 N：多组织用户需要选择正确的目标组织；没有该组织角色则不能创建
> - **创建权由 Org 角色 + 用户选择的 owner_type 共同决定，Group 只影响后续编辑权**

---

### 3.7.3 Group-Skill 关联管理

```sql
-- 在已有 group_skills 表基础上，增加管理审计字段
CREATE TABLE group_skills (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    skill_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    added_by UUID REFERENCES users(id),
    added_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(group_id, skill_id)
);
```

**关联操作权限**：

| 操作 | 谁能执行 | 条件 |
|------|---------|------|
| 将 Skill 关联到 Group | Skill author 或 Org admin | Skill 的 owner_type=organization 且属同一 Org |
| 从 Group 移除 Skill | Skill author 或 Org admin 或 Group lead | — |
| 查看 Group-Skill 关联 | Group 成员或 Org 成员 | — |

**流程示例**：

```
alice 创建 Skill "deploy-tool" (owner_type=organization)
  │
  │  此时 alice 想编辑 deploy-tool，她需要：
  │  - 把 Skill 关联到一个她所在的 Group（比如 frontend）
  │  - 或者她是 Org admin
  │
  │  POST /api/v1/skills/deploy-tool/groups  { group_id: "frontend" }
  │
  ├── 结果：frontend 所有成员现在可以编辑 deploy-tool
  │
  │  alice (frontend: lead)  可以编辑 deploy-tool ✓
  │  bob   (frontend: member) 可以编辑 deploy-tool ✓（协作编辑）
  │  carol (backend: member)  不能编辑 deploy-tool ✗（不在 frontend）
  │  dave  (无Group)          不能编辑 deploy-tool ✗
  │
  │  后来 alice 离开组织
  │  
  └── alice 不能再编辑 deploy-tool ✗
      （Skill 所有权属于组织，离开组织即失去一切权限）
      
      Org admin (bob) 接管：
      1. bob 本身就是 Org admin → 可以编辑 ✓
      2. bob 确保 frontend Group 还有活跃成员
      3. deploy-tool 继续正常维护
```

### 3.8 Skill 标签 & 依赖

```sql
CREATE TABLE skill_tags (
    skill_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    tag VARCHAR(100) NOT NULL,
    PRIMARY KEY (skill_id, tag)
);

CREATE TABLE skill_dependencies (
    skill_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    dependency_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    PRIMARY KEY (skill_id, dependency_id)
);
```

### 3.9 审计日志

```sql
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id),
    organization_id UUID REFERENCES organizations(id),
    user_id UUID REFERENCES users(id),
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id UUID,
    details JSONB DEFAULT '{}',
    ip_address VARCHAR(45),
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_audit_user ON audit_logs(user_id);
CREATE INDEX idx_audit_org ON audit_logs(organization_id);
CREATE INDEX idx_audit_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_created ON audit_logs(created_at DESC);
```

---

## 4. 角色与权限

### 4.1 角色层次

```
System Level (系统级 — 全局治理)
├── super_admin         超级管理员：租户管理、平台配置、全局策略
└── marketplace_admin   市场管理员：marketplace 内容治理、精选/下架

Organization Level (组织级 — 核心业务)
├── owner               组织所有者：完全控制组织内一切资源
├── admin               组织管理员：成员管理、Skill 管理、设置
├── reviewer            审核员：审核组织内提交的 Skill，批准/驳回
├── developer           开发者：创建和编辑 Skill
└── member              成员：查看、安装、使用组织内的 Skill

Group Level (组级 — 组织内子团队，角色绑定到具体 Group)
├── lead                组负责人：管理组成员、审批组内 Skill 审核
└── member              组员：在组范围内协作编辑 Skill

Personal Level (个人级 — 独立用户)
└── user                默认角色：管理自己的 Skill、API Key、加入组织
```

### 4.2 细粒度权限点定义

#### 4.2.1 数据库表

```sql
CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(100) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    action VARCHAR(50) NOT NULL,
    scope VARCHAR(50) DEFAULT 'global',
    -- scope 取值：
    --   global    全局（系统级权限）
    --   tenant    租户级
    --   org       组织级
    --   group     组级
    --   own       仅限自己的资源
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_permissions_resource ON permissions(resource_type, action);

CREATE TABLE role_permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    role_level VARCHAR(50) NOT NULL,          -- 'system' | 'organization' | 'group' | 'personal'
    role_name VARCHAR(50) NOT NULL,           -- 'super_admin' | 'admin' | 'developer' | 'lead' ...
    permission_code VARCHAR(100) NOT NULL REFERENCES permissions(code),
    scope_restriction VARCHAR(50) DEFAULT 'none',
    -- scope_restriction: 权限进一步限定
    --   none      无额外限制（角色允许的任何范围）
    --   own       仅自己的资源
    --   org       仅组织内资源
    --   group     仅 Group 关联的资源

    UNIQUE(role_level, role_name, permission_code)
);

CREATE INDEX idx_role_perms_role ON role_permissions(role_level, role_name);

CREATE TABLE group_permission_overrides (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    role_name VARCHAR(50) NOT NULL,              -- 'lead' or 'member'
    permission_code VARCHAR(100) NOT NULL REFERENCES permissions(code),
    granted BOOLEAN NOT NULL DEFAULT TRUE,       -- TRUE=授予, FALSE=撤销

    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(group_id, role_name, permission_code)
);
```

#### 4.2.2 权限点清单（按资源分组，总计 55 项）

##### Skill 权限（核心，粒度最细）

| code | 名称 | 作用域 | 说明 |
|------|------|:---:|------|
| `skill:create` | 创建 Skill | org/own | 在组织内创建 Skill，或个人创建 |
| `skill:read` | 查看 Skill 基础信息 | org/own | 查看 Skill 名称、描述、版本等 |
| `skill:read_content` | 查看 Skill 内容 | org/own/group | 查看 SKILL.md 完整内容 |
| `skill:update` | 编辑 Skill | org/group | 修改 Skill 内容、配置（需编辑权限判断） |
| `skill:delete` | 删除 Skill | org/group | 删除 Skill（需编辑权限判断） |
| `skill:install` | 安装/使用 Skill | org/own | 安装 Skill 到本地环境 |
| `skill:version_create` | 创建新版本 | org/group | 发布 Skill 新版本 |
| `skill:version_rollback` | 回滚版本 | org/group | 回滚到历史版本 |
| `skill:submit_review` | 提交审核 | own | 将草稿 Skill 提交到审核队列 |
| `skill:approve_review` | 批准审核 | org | 批准 Skill 通过审核 |
| `skill:reject_review` | 驳回审核 | org | 驳回 Skill 审核（附评论） |
| `skill:change_visibility` | 修改可见性 | org/group | 切换 private/org_visible/marketplace |
| `skill:associate_group` | 关联到 Group | org/group | 将 Skill 加入 Group 作用域 |
| `skill:dissociate_group` | 解除 Group 关联 | org/group | 将 Skill 移出 Group 作用域 |
| `skill:fork` | Fork Skill | global | 基于已有 Skill 创建副本（跨组织也允许） |

##### Organization 权限

| code | 名称 | 作用域 | 说明 |
|------|------|:---:|------|
| `org:read` | 查看组织基础信息 | org | 查看组织名称、描述、成员数等 |
| `org:update` | 修改组织信息 | org | 修改组织名称、描述、头像 |
| `org:delete` | 删除组织 | org | 删除组织及其所有资源 |
| `org:transfer` | 转移组织所有权 | org | 将 owner 身份转移给其他成员 |
| `org:settings_read` | 查看组织设置 | org | 查看组织策略、Webhook、SSO 配置 |
| `org:settings_write` | 修改组织设置 | org | 修改组织策略、配额等配置 |
| `org:member_read` | 查看成员列表 | org | 查看组织内所有成员及角色 |
| `org:member_invite` | 邀请成员 | org | 邀请用户加入组织 |
| `org:member_remove` | 移除成员 | org | 将成员从组织中移除 |
| `org:member_role_assign` | 分配成员角色 | org | 修改成员在组织内的角色 |
| `org:member_suspend` | 暂停成员 | org | 临时暂停成员的访问权限 |
| `org:skill_transfer` | 转移 Skill 所有权 | org | 将组织 Skill 转移到另一个组织 |

##### Group 权限

| code | 名称 | 作用域 | 说明 |
|------|------|:---:|------|
| `group:create` | 创建 Group | org | 在组织内创建子团队 |
| `group:read` | 查看 Group 信息 | org/group | 查看 Group 名称、描述、成员 |
| `group:update` | 修改 Group | org/group | 修改 Group 名称/描述 |
| `group:delete` | 删除 Group | org/group | 删除 Group（不影响 Skill 和成员） |
| `group:member_read` | 查看组成员 | org/group | 查看 Group 内成员列表 |
| `group:member_add` | 添加组成员 | org/group | 将组织成员加入 Group |
| `group:member_remove` | 移除组成员 | org/group | 将成员从 Group 中移除 |
| `group:member_role_assign` | 设置组内角色 | org/group | 设置组员为 lead 或 member |
| `group:permission_override` | 自定义组权限 | org | 对 Group 的默认权限进行覆盖 |

##### API Key 权限

| code | 名称 | 作用域 | 说明 |
|------|------|:---:|------|
| `apikey:create` | 创建 API Key | own | 为当前用户创建 API Key |
| `apikey:read` | 查看 API Key | own | 查看自己的 API Key 列表 |
| `apikey:revoke` | 撤销 API Key | own | 撤销/删除自己的 API Key |
| `apikey:scope_set` | 设置 Key 作用域 | org | 限定 Key 的组织范围 |
| `apikey:rate_limit_set` | 设置速率限制 | org | 设置 Key 的每分钟请求上限 |

##### Tenant 权限（系统级）

| code | 名称 | 作用域 | 说明 |
|------|------|:---:|------|
| `tenant:create` | 创建租户 | global | 创建新租户 |
| `tenant:read` | 查看租户 | global | 查看租户信息 |
| `tenant:update` | 修改租户 | global | 修改租户配置、计费计划 |
| `tenant:delete` | 删除租户 | global | 删除租户及其所有数据 |
| `tenant:sso_config` | 配置 SSO | global | 配置租户级 SSO/OIDC |

##### 个人 Profile 权限

| code | 名称 | 作用域 | 说明 |
|------|------|:---:|------|
| `profile:read` | 查看个人信息 | own | 查看自己的用户名、邮箱、头像 |
| `profile:update` | 修改个人信息 | own | 修改自己的显示名、头像等 |
| `profile:delete` | 注销账户 | own | 注销自己的账户 |

##### 审计日志权限

| code | 名称 | 作用域 | 说明 |
|------|------|:---:|------|
| `audit:read_org` | 查看组织审计日志 | org | 查看所属组织的审计日志 |
| `audit:read_tenant` | 查看租户审计日志 | tenant | 查看租户级别审计日志 |
| `audit:read_global` | 查看全局审计日志 | global | 查看全平台审计日志 |

##### Marketplace 权限

| code | 名称 | 作用域 | 说明 |
|------|------|:---:|------|
| `marketplace:feature` | 精选 Skill | global | 将 Skill 加入精选列表 |
| `marketplace:unfeature` | 取消精选 | global | 将 Skill 从精选列表移除 |
| `marketplace:delist` | 下架 Skill | global | 从 marketplace 下架违规内容 |

---

### 4.3 角色 → 权限绑定矩阵

#### 4.3.1 系统级角色绑定

| 权限点 | super_admin | marketplace_admin | 说明 |
|--------|:---:|:---:|------|
| `tenant:create` | ✓ | — | |
| `tenant:read` | ✓ | ✓ | |
| `tenant:update` | ✓ | — | |
| `tenant:delete` | ✓ | — | |
| `tenant:sso_config` | ✓ | — | |
| `marketplace:feature` | ✓ | ✓ | |
| `marketplace:unfeature` | ✓ | ✓ | |
| `marketplace:delist` | ✓ | ✓ | |
| `audit:read_global` | ✓ | — | |
| `skill:fork` | ✓ | ✓ | |
| `skill:read` | ✓ | ✓ | |
| `skill:read_content` | ✓ | ✓ | |
| `skill:install` | ✓ | ✓ | |
| `skill:approve_review` | ✓ | ✓ | 审核个人用户提交的 marketplace Skill |
| `skill:reject_review` | ✓ | ✓ | 驳回个人用户提交的 marketplace Skill |

#### 4.3.2 组织级角色绑定矩阵（核心）

| 权限点 | owner | admin | reviewer | developer | member |
|--------|:---:|:---:|:---:|:---:|:---:|
| `org:read` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `org:update` | ✓ | ✓ | — | — | — |
| `org:delete` | ✓ | — | — | — | — |
| `org:transfer` | ✓ | — | — | — | — |
| `org:settings_read` | ✓ | ✓ | — | — | — |
| `org:settings_write` | ✓ | ✓ | — | — | — |
| `org:member_read` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `org:member_invite` | ✓ | ✓ | — | — | — |
| `org:member_remove` | ✓ | ✓ | — | — | — |
| `org:member_role_assign` | ✓ | ✓ | — | — | — |
| `org:member_suspend` | ✓ | ✓ | — | — | — |
| `org:skill_transfer` | ✓ | ✓ | — | — | — |
| `group:create` | ✓ | ✓ | — | — | — |
| `group:read` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `group:update` | ✓ | ✓ | — | — | — |
| `group:delete` | ✓ | ✓ | — | — | — |
| `group:member_read` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `group:member_add` | ✓ | ✓ | — | — | — |
| `group:member_remove` | ✓ | ✓ | — | — | — |
| `group:member_role_assign` | ✓ | ✓ | — | — | — |
| `group:permission_override` | ✓ | ✓ | — | — | — |
| `skill:create` | ✓ | ✓ | — | ✓ | — |
| `skill:read` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `skill:read_content` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `skill:update` | ✓ | ✓ | — | — | — |
| `skill:delete` | ✓ | ✓ | — | — | — |
| `skill:install` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `skill:version_create` | ✓ | ✓ | — | — | — |
| `skill:version_rollback` | ✓ | ✓ | — | — | — |
| `skill:submit_review` | ✓ | ✓ | ✓ | ✓ | — |
| `skill:approve_review` | ✓ | ✓ | ✓ | — | — |
| `skill:reject_review` | ✓ | ✓ | ✓ | — | — |
| `skill:change_visibility` | ✓ | ✓ | — | — | — |
| `skill:associate_group` | ✓ | ✓ | — | own | — |
| `skill:dissociate_group` | ✓ | ✓ | — | own | — |
| `skill:fork` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `apikey:create` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `apikey:read` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `apikey:revoke` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `apikey:scope_set` | ✓ | ✓ | — | — | — |
| `apikey:rate_limit_set` | ✓ | ✓ | — | — | — |
| `profile:read` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `profile:update` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `profile:delete` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `audit:read_org` | ✓ | ✓ | — | — | — |

#### 4.3.3 个人级角色绑定

| 权限点 | user |
|--------|:---:|
| `skill:create` | ✓ (个人 Skill) |
| `skill:read` | ✓ (自己的 + marketplace) |
| `skill:read_content` | ✓ (自己的 + marketplace) |
| `skill:update` | ✓ (自己的个人 Skill) |
| `skill:delete` | ✓ (自己的个人 Skill) |
| `skill:install` | ✓ |
| `skill:version_create` | ✓ (自己的个人 Skill) |
| `skill:version_rollback` | ✓ (自己的个人 Skill) |
| `skill:submit_review` | ✓ (自己的个人 Skill) |
| `skill:change_visibility` | ✓ (自己的个人 Skill) |
| `skill:fork` | ✓ |
| `apikey:create` | ✓ |
| `apikey:read` | ✓ |
| `apikey:revoke` | ✓ |
| `profile:read` | ✓ |
| `profile:update` | ✓ |
| `profile:delete` | ✓ |

---

### 4.4 Group 角色 → 权限绑定

Group 角色（lead / member）的权限通过 `role_permissions` 定义，但 **Group 权限只在 Group 作用域内生效**——即仅对通过 `group_skills` 关联到该 Group 的 Skill 有效。

#### 4.4.1 默认 Group 权限绑定

| 权限点 | lead | member | 说明 |
|--------|:---:|:---:|------|
| `group:read` | ✓ | ✓ | 查看 Group 信息 |
| `group:update` | ✓ | — | 修改 Group 名称/描述 |
| `group:delete` | ✓ | — | 删除 Group |
| `group:member_read` | ✓ | ✓ | 查看成员列表 |
| `group:member_add` | ✓ | — | 邀请 Org 成员加入 Group |
| `group:member_remove` | ✓ | — | 移除 Group 成员 |
| `group:member_role_assign` | ✓ | — | 设置 lead/member |
| `skill:read` | ✓ | ✓ | 查看 Group 关联 Skill 信息 |
| `skill:read_content` | ✓ | ✓ | 查看 Group 关联 Skill 完整内容 |
| `skill:update` | ✓ | ✓ | 编辑 Group 关联的 Skill |
| `skill:delete` | ✓ | — | 删除 Group 关联的 Skill |
| `skill:version_create` | ✓ | — | 发布新版本 |
| `skill:version_rollback` | ✓ | — | 回滚版本 |
| `skill:submit_review` | ✓ | ✓ | 提交 Group 内 Skill 审核 |
| `skill:approve_review` | ✓ | — | 批准 Group 内 Skill 审核 |
| `skill:reject_review` | ✓ | — | 驳回 Group 内 Skill 审核 |
| `skill:change_visibility` | ✓ | — | 修改可见性 |
| `skill:associate_group` | ✓ | — | 将更多 Skill 关联进 Group |
| `skill:dissociate_group` | ✓ | — | 移除 Skill 关联 |

#### 4.4.2 Group 权限的叠加规则

```
用户的最终权限 = Org 角色权限 ∪ Σ(各 Group 角色权限)

规则：
1. Org 角色权限是基础层，始终生效
2. Group 权限只对 group_skills 关联的 Skill 生效
3. Group lead 不能突破 Org role 的上限（Org member + Group lead ≠ Org admin）
4. 用户属于多个 Group 时，取各 Group 权限的并集
5. 多源权限冲突时，"允许"优先（乐观策略），但 `group_permission_overrides` 的显式 DENY 覆盖此规则（见 4.4.3 优先级）
6. `group_permission_overrides` 中显式 GRANT 可突破 Group 角色默认权限上限
```

#### 4.4.3 Group 权限自定义覆盖

允许 Org admin 对**特定 Group** 调整 `lead` 或 `member` 的权限，实现灵活的权限控制。通过 `group_permission_overrides` 表实现。

**使用场景**：

| 场景 | 配置 | 效果 |
|------|------|------|
| 宽松协作 | `group: frontend, role: member, perm: skill:delete → TRUE` | frontend 组员可以删除组内 Skill |
| 收紧权限 | `group: intern, role: member, perm: skill:update → FALSE` | intern 组员只能查看，不能编辑 |
| 高级审核 | `group: review-panel, role: member, perm: skill:approve_review → TRUE` | 审核组内 member 也能批准 Skill |
| 限制暴露 | `group: extern, role: member, perm: skill:read_content → FALSE` | 外部协作者不能看 Skill 正文 |

**覆盖优先级**：

```
1. super_admin → 直接放行（最高优先级）
2. group_permission_overrides 中显式 DENY (granted=FALSE) → 拒绝
3. group_permission_overrides 中显式 GRANT (granted=TRUE) → 允许
4. role_permissions 中默认绑定 → 正常判断
5. 无匹配 → 拒绝（默认拒绝策略）
```

---

### 4.5 权限判断引擎

#### 4.5.1 核心判断函数

```
HAS_PERMISSION(user, perm_code, resource):

  // 1. super_admin 拥有所有权限
  IF user HAS ROLE 'super_admin' → return TRUE

  // 2. 汇总用户的所有角色：[(level, name, scope_id)]
  roles = [
    -- 系统级角色
    SELECT 'system', role_name, NULL FROM system_role_assignments WHERE user_id = user.id
    UNION
    -- 组织级角色
    SELECT 'organization', role, organization_id FROM org_memberships WHERE user_id = user.id
    UNION
    -- Group 级角色
    SELECT 'group', gm.role, gm.group_id FROM group_memberships gm WHERE gm.user_id = user.id
  ]

  // 3. 遍历角色，检查角色绑定的权限点
  FOR each (level, name, scope_id) in roles:
    perms = SELECT * FROM role_permissions
            WHERE role_level = level AND role_name = name
              AND permission_code = perm_code

    FOR each perm in perms:

      // 4. 应用 scope_restriction
      IF perm.scope_restriction == 'own':
        IF resource.author_id != user.id → CONTINUE

      IF perm.scope_restriction == 'org':
        IF resource.owner_id != scope_id → CONTINUE

      IF perm.scope_restriction == 'group':
        IF resource NOT IN (SELECT skill_id FROM group_skills WHERE group_id = scope_id)
          → CONTINUE

      // 5. 检查 Group 级别的权限覆盖
      FOR each group of user:
        override = SELECT * FROM group_permission_overrides
                   WHERE group_id = group.id
                     AND role_name = group.role
                     AND permission_code = perm_code

        IF override EXISTS AND override.granted == FALSE → CONTINUE
        IF override EXISTS AND override.granted == TRUE  → return TRUE

      return TRUE

  return FALSE
```

#### 4.5.2 Skill 编辑权限判断

```
CAN_EDIT_SKILL(user, skill):

  // 1. 个人 Skill：只有 owner 能编辑
  IF skill.owner_type == 'user':
    RETURN (skill.owner_id == user.id)

  // 2. super_admin 可编辑所有
  IF user HAS ROLE 'super_admin':
    RETURN TRUE

  // 3. 组织 Skill：完全通过权限点系统判断
  IF skill.owner_type == 'organization':

    // 3a. 检查 org 级 skill:update 权限
    IF HAS_PERMISSION(user, 'skill:update', skill, scope='org') → return TRUE

    // 3b. 检查 group 级 skill:update 权限（关联到 Group 的 Skill）
    FOR each group of user IN skill.owner_org:
      IF skill IS IN group_skills FOR this group:
        IF HAS_PERMISSION(user, 'skill:update', skill, scope='group') → return TRUE

    RETURN FALSE

  RETURN FALSE
```

#### 4.5.3 权限验证场景示例

| # | 用户 | Org角色 | Group角色 | Skill归属 | 操作 | 结果 | 原因 |
|---|------|---------|----------|-----------|------|:---:|------|
| 1 | alice | developer | frontend: lead | Org Skill, 关联 frontend | 编辑 Skill | ✅ | `role_permissions`(group.lead → skill:update) + scope匹配 |
| 2 | bob | developer | frontend: member | Org Skill, 关联 frontend | 编辑 Skill | ✅ | `role_permissions`(group.member → skill:update) + scope匹配 |
| 3 | carol | developer | backend: lead | Org Skill, 关联 frontend | 编辑 Skill | ❌ | Group scope 不匹配（不在 frontend 组） |
| 4 | dave | member | 无 | Org Skill, 关联 frontend | 编辑 Skill | ❌ | Org 无 skill:update 且不在任何 Group |
| 5 | eve | admin | 无 | Org Skill, 无 Group | 编辑 Skill | ✅ | `role_permissions`(org.admin → skill:update) |
| 6 | frank | developer | intern: member | Org Skill, 关联 intern | 编辑 Skill | ❌ | `group_permission_overrides`(intern.member → skill:update=FALSE) |
| 7 | grace | reviewer | 无 | Org Skill, pending_review | 批准审核 | ✅ | `role_permissions`(org.reviewer → skill:approve_review) |
| 8 | henry | 个人用户 | 无 | 个人 Skill | 编辑 Skill | ✅ | `owner_type=user` + `owner_id` 匹配 |
| 9 | ivan | 个人用户 | 无 | 其他个人 Skill | 编辑 Skill | ❌ | 不是 owner |
| 10 | julia | developer | 无 | Org Skill, 无 Group | 创建 Skill | ✅ | `role_permissions`(org.developer → skill:create) |
| 11 | kate | member | 无 | Org Skill | 创建 Skill | ❌ | `org.member` 没有 `skill:create` |
| 12 | luke | 已离开 | 已离开 | Org Skill (曾是 author) | 编辑 Skill | ❌ | author_id 不授予编辑权，离开组织失去一切权限 |

---

## 5. 业务流程

### 5.1 Skill 生命周期

```
┌──────────────────────────────────────────────────────────────────────┐
│                        Skill 生命周期（组织级）                        │
│                                                                      │
│   ┌─────────┐    提交审核     ┌──────────────┐    审核通过    ┌─────────┐│
│   │  draft  │ ───────────────→ │ pending_review│ ────────────→ │approved ││
│   │ (草稿)  │                 │   (待审核)    │               │ (已通过) ││
│   └─────────┘                 └──────┬───────┘               └────┬────┘│
│        ↑                             │ 驳回                       │     │
│        │                             ▼                            │     │
│        │                       ┌──────────┐                      │     │
│        └─────────────────────── │ rejected │   发布 marketplace    │     │
│              重新编辑           │ (已驳回)  │                      │     │
│                                 └──────────┘              ┌───────▼───┐ │
│                                                           │ marketplace│ │
│                                                           │ (已上架)   │ │
│                                                           └───────────┘ │
└──────────────────────────────────────────────────────────────────────┘
```

**审核状态与可见性的关系**：

`review_status` 和 `visibility` 是两个独立维度，但存在耦合约束：

| review_status | 允许的 visibility | 说明 |
|:---:|------|------|
| `draft` | `private` | 草稿只允许私有，不可设为 org_visible 或 marketplace |
| `pending_review` | `private` | 审核中的 Skill 暂不对外暴露 |
| `approved` | `private` / `org_visible` / `marketplace` | 审核通过后可自由设置可见性 |
| `rejected` | `private` | 驳回的 Skill 回到私有状态，修改后可重新提交 |

> **关键理解**：`approved` 不等于 `marketplace`。审核通过只是一个质量门槛，通过后 Skill 可以保持 `org_visible`（仅组织内发布）或推向 `marketplace`（全平台发布）。`marketplace` 要求 `review_status=approved`。

**审核规则**：

| 审核方 | 范围 | 权限 |
|--------|------|------|
| 组织 Reviewer/Admin | 本组织内提交的 Skill | 批准、驳回（附评论） |
| 平台 marketplace_admin | 所有 approved + marketplace Skill | 精选、下架违规内容 |

**禁止自审**：提交审核的人不能批准或驳回自己提交的 Skill。`skill:approve_review` / `skill:reject_review` 在执行时需额外校验 — `skill.author_id != current_user.id`。

### 5.2 用户注册与组织加入

```
独立用户注册流程：
  1. User 注册（username + password / agent_id + secret）
  2. 自动获得个人账户、可创建个人 Skill
  3. 可创建 API Key，独立使用平台服务

加入组织流程：
  1. 组织 Owner/Admin 邀请 User（通过 username/email）
  2. User 接受邀请
  3. 获得对应组织角色（member/developer/admin/reviewer/owner）
  4. 可为该组织创建 Skill（owner_type='organization'）
  5. 一个 User 可同时属于多个组织

Agent 自助注册流程：
  1. Agent 调用 POST /api/v1/agents/register 提交注册信息
  2. 系统创建 User（user_type='agent'），返回 agent_id + secret
  3. Agent 可自行创建 API Key（POST /api/v1/api-keys）
  4. Agent 可被邀请加入组织，参与组织级协作

注销限制：
  - 用户是任何组织的唯一 owner 时，不允许注销账户（需先转移所有权或解散组织）
  - 注销后，用户创建的个人 Skill（owner_type=user）将被标记为 deleted
  - 注销后，用户的 API Key 全部失效
```

### 5.3 跨组织 Skill 共享

```
场景：Org-A 开发了 Skill "data-analyzer" 并发布到 marketplace

  Org-A (Owner)           Org-B (Consumer)        独立用户 Charlie
  ┌──────────────┐       ┌──────────────┐       ┌──────────────┐
  │ 可编辑 ✓     │       │ 可安装/使用 ✓ │       │ 可安装/使用 ✓ │
  │ 可删除 ✓     │       │ 不可编辑 ✗    │       │ 不可编辑 ✗    │
  │ 可更新版本 ✓ │       │ 不可删除 ✗    │       │ 不可删除 ✗    │
  │ 审核管理 ✓   │       │ 只读安装 ✗    │       │ 只读安装 ✗    │
  └──────────────┘       └──────────────┘       └──────────────┘
```

### 5.4 API Key 使用场景

```
外部 Agent 调用流程：
  1. Agent 注册为 User（user_type='agent'），获取 agent_id + secret
  2. 使用 agent_id + secret 获取 JWT token
  3. 使用 JWT token 创建 API Key（可限定 org 范围）
  4. 使用 API Key 调用平台 Skill（Bearer token）

  ┌─────────────┐    API Key     ┌─────────────┐    install skill   ┌──────────┐
  │  External   │ ──────────────→│  AionHive   │ ──────────────→   │  Skill   │
  │  Agent      │   Bearer token │  Gateway    │                   │  Runtime │
  └─────────────┘                └─────────────┘                   └──────────┘
                                       │
                                       ├── 验证 API Key 有效性
                                       ├── 检查速率限制
                                       ├── 检查 Skill 可见性
                                       └── 记录审计日志
```

---

## 6. API 设计

### 6.1 认证端点

```
POST   /api/v1/auth/login              - User 登录（human: username+password, agent: agent_id+secret）
POST   /api/v1/auth/register           - User 注册
POST   /api/v1/agents/register         - Agent 自助注册
```

### 6.2 用户管理

```
GET    /api/v1/users/me                 - 获取当前用户信息
PUT    /api/v1/users/me                 - 更新当前用户信息
GET    /api/v1/users/me/orgs            - 获取我所属的组织列表
GET    /api/v1/users/{username}         - 获取用户公开信息
```

### 6.3 组织管理

```
POST   /api/v1/orgs                     - 创建组织
GET    /api/v1/orgs                     - 列出组织（支持 tenant_id 过滤）
GET    /api/v1/orgs/{slug}              - 获取组织详情
PUT    /api/v1/orgs/{slug}              - 更新组织（需 Owner/Admin）
DELETE /api/v1/orgs/{slug}              - 删除组织（需 Owner）

GET    /api/v1/orgs/{slug}/members      - 组织成员列表
POST   /api/v1/orgs/{slug}/members      - 邀请成员（需 Owner/Admin）
PUT    /api/v1/orgs/{slug}/members/{username} - 更新成员角色
DELETE /api/v1/orgs/{slug}/members/{username} - 移除成员

GET    /api/v1/orgs/{slug}/skills       - 组织下的 Skill 列表
GET    /api/v1/orgs/{slug}/reviews      - 组织审核队列
```

### 6.4 Skill 管理

```
POST   /api/v1/skills                   - 创建 Skill（默认 owner_type=user，可指定 organization + target_org）
POST   /api/v1/orgs/{slug}/skills       - 在组织下创建 Skill（默认 owner_type=organization，可切换为 user）
GET    /api/v1/skills                   - 搜索/列出 Skill（支持 visibility、owner、tag 过滤）
GET    /api/v1/skills/{id}              - 获取 Skill 详情
PUT    /api/v1/skills/{id}              - 编辑 Skill（需编辑权限）
DELETE /api/v1/skills/{id}              - 删除 Skill（需编辑权限）

POST   /api/v1/skills/{id}/submit-review  - 提交审核（draft → pending_review）
POST   /api/v1/skills/{id}/approve     - 批准（pending_review → approved，需 Reviewer）
POST   /api/v1/skills/{id}/reject      - 驳回（pending_review → rejected，需 Reviewer）
POST   /api/v1/skills/{id}/publish     - 发布到 marketplace（approved → marketplace）

POST   /api/v1/skills/{id}/install     - 安装 Skill（记录 install_count）
GET    /api/v1/skills/{id}/groups       - 查看 Skill 关联的 Group 列表
POST   /api/v1/skills/{id}/groups       - 将 Skill 关联到指定 Group
DELETE /api/v1/skills/{id}/groups/{group_id} - 解除 Skill-Group 关联
GET    /api/v1/marketplace              - 浏览 marketplace
```

### 6.5 API Key 管理

```
POST   /api/v1/api-keys                 - 创建 API Key
GET    /api/v1/api-keys                 - 列出我的 API Keys
DELETE /api/v1/api-keys/{id}            - 撤销 API Key
```

### 6.6 组管理（可选，组织内子团队）

```
POST   /api/v1/orgs/{slug}/groups              - 创建组
GET    /api/v1/orgs/{slug}/groups              - 列出组
GET    /api/v1/orgs/{slug}/groups/{id}         - 获取组详情
PUT    /api/v1/orgs/{slug}/groups/{id}         - 更新组
DELETE /api/v1/orgs/{slug}/groups/{id}         - 删除组

GET    /api/v1/orgs/{slug}/groups/{id}/members           - 组成员列表
POST   /api/v1/orgs/{slug}/groups/{id}/members           - 添加组成员（需已是 Org 成员）
PUT    /api/v1/orgs/{slug}/groups/{id}/members/{username} - 更新组内角色
DELETE /api/v1/orgs/{slug}/groups/{id}/members/{username} - 移除组成员

GET    /api/v1/orgs/{slug}/groups/{id}/skills             - 组关联的 Skill 列表
POST   /api/v1/orgs/{slug}/groups/{id}/skills             - 将 Skill 关联到组
DELETE /api/v1/orgs/{slug}/groups/{id}/skills/{skill_id}  - 移除组-Skill 关联
```

### 6.7 租户管理（系统级）

```
POST   /api/v1/admin/tenants            - 创建租户
GET    /api/v1/admin/tenants            - 列出租户
PUT    /api/v1/admin/tenants/{id}       - 更新租户
DELETE /api/v1/admin/tenants/{id}       - 删除租户
```

### 6.8 审计日志

```
GET    /api/v1/admin/audit-logs         - 查询审计日志（支持 org, user, action 过滤）
```

---

## 7. 权限检查中间件流程

```
1. 从请求提取身份：
   - Authorization: Bearer <JWT>     → 解析 JWT 获取 user_id
   - X-API-Key: <key>                → 查询 api_keys 表获取 user_id + org scope

2. 加载 User 角色：
   - 系统级角色（super_admin / marketplace_admin）
   - 组织成员角色（通过 org_memberships 查询）
   - 合并所有角色和权限点

3. 检查资源所有权 / 可见性：
   - Skill: 验证 owner_type + owner_id + visibility
   - Organization: 验证 membership + role

4. 验证操作权限：
   - 权限点匹配（如 skill:update, org:manage）
   - 组织级权限还检查 scope（只能操作所属组织的资源）

5. 记录审计日志（异步，不阻塞请求）
```

---

## 8. 企业级扩展能力

### 8.1 SSO / OIDC 集成

```
Tenant 级别配置 SSO:
  - OIDC Provider URL
  - Client ID / Secret
  - 自动将 SSO 用户映射到组织角色
  - 支持 Just-In-Time (JIT) 用户创建
```

### 8.2 配额与速率限制

```
分层限流模型:
  Tenant 级: 全局 API 调用总量（企业版）
  Organization 级: 组织内 Skill 安装/调用配额
  User 级: 个人 API Key 每分钟调用量
```

### 8.3 Skill 版本化

```
- 语义化版本控制 (semver)
- 每个版本独立存储 Skill 内容
- 支持版本回滚和发布说明 (release notes)
- 消费方可锁定特定版本
```

### 8.4 Webhook 事件通知

```
事件类型:
  - skill.created, skill.updated, skill.deleted
  - skill.review_submitted, skill.review_approved, skill.review_rejected
  - skill.published (上架 marketplace)
  - member.invited, member.joined, member.removed
  - apikey.created, apikey.revoked

每组织可配置多个 Webhook URL + 签名密钥
```

### 8.5 使用分析

```
统计维度:
  - 按 Skill: 安装数、调用量、评分、审核周期
  - 按 Organization: 成员数、Skill 数、活跃度
  - 按 Tenant: 组织数、总量、计费数据
  - 全局 marketplace: 热门 Skill、趋势
```

### 8.6 数据隔离

```
多租户隔离策略:
  - 数据库级: 每个 Tenant 独立 Schema（企业版）
  - 行级安全 (RLS): PostgreSQL Row-Level Security
  - 应用层: 所有查询强制带 tenant_id / org_id 过滤
```

---

## 9. 迁移路径（从当前系统演进）

### 当前状态
```
agents 表 ───┐
             ├──→ skills.author_agent_id (FK 已移除)
admin_users ─┘
organizations (已有)
```

### 目标状态
```
Phase 1: 建立 User 模型
  - 迁移 agents → users (user_type='agent')
  - 迁移 admin_users → users (user_type='human')
  - skills.author_agent_id → skills.author_id (FK → users.id)

Phase 2: 组织成员模型
  - 创建 org_memberships 表
  - 将现有 org-agent 关系迁移到 org_memberships

Phase 3: Skill 审核流程
  - 添加 review_status 字段
  - 实现组织级审核 API

Phase 4: 跨组织共享 + Marketplace
  - 完善 visibility 模型
  - 实现 marketplace
```

---

## 10. 实现优先级

| 优先级 | 阶段 | 内容 |
|--------|------|------|
| P0 | User 模型 | users 表 + 迁移 agents/admin_users → users |
| P0 | 组织成员 | org_memberships 表 + 邀请/加入流程 |
| P1 | Skill 所有权 | owner_type + owner_id，编辑权限控制 |
| P1 | 组织级审核 | 审核状态机 + Reviewer 角色 |
| P2 | API Key 增强 | 组织范围限定 + 速率限制 |
| P2 | Marketplace | 公开 Skill 发现 + 跨组织安装 |
| P3 | SSO | OIDC 集成 |
| P3 | 分析 | 使用统计面板 |
| P4 | 多租户隔离 | RLS + Schema 隔离 |