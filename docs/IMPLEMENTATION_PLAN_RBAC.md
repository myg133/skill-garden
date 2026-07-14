# 多租户 RBAC 权限体系实现方案

> **版本**: v1.0  
> **日期**: 2026-07-13  
> **状态**: 待审核  
> **关联**: PR #1 (feature/admin-ui-enhancement) 后续增强

---

## 目录

1. [概述](#1-概述)
2. [当前架构分析](#2-当前架构分析)
3. [目标架构](#3-目标架构)
4. [后端改造](#4-后端改造)
5. [前端改造](#5-前端改造)
6. [数据库变更](#6-数据库变更)
7. [API 变更清单](#7-api-变更清单)
8. [测试计划](#8-测试计划)
9. [实施步骤](#9-实施步骤)
10. [风险评估](#10-风险评估)

---

## 1. 概述

### 1.1 目标

实现基于角色的前端页面差异化展示和后端 Skill/API Key 权限校验：

| 用户类型 | 前端可见页面 | 后端权限 |
|---------|------------|---------|
| **管理员** (is_system_admin=true) | 租户/组织、Skill 管理、审计日志、Tool 管理、所有管理页面 | 全部权限（CRUD + 审核） |
| **普通用户** (is_system_admin=false) | 我的 Skill、市场中 Skill、个人信息、我的 API Key | 个人 Skill CRUD、市场浏览、仅能操作自己的资源 |

### 1.2 核心需求

1. **前端路由差异化**：管理员和普通用户看到不同的导航菜单和页面
2. **Skill 权限校验**：
   - 个人用户创建的 Skill (`owner_type = "user"`) → 只有创建者可以管理
   - 组织用户创建的 Skill (`owner_type = "organization"`) → 同组织成员可访问，高权限角色 (Owner/Admin/Reviewer) 可审核
3. **API Key 创建优化**：
   - 个人用户创建 API Key 时 `organization_id` 可以为空
   - 如果选择组织，列表仅显示用户所在的组织
4. **市场 Skill**：所有已发布的 Skill 对所有人可见

---

## 2. 当前架构分析

### 2.1 已有能力（可复用）

| 组件 | 状态 | 说明 |
|------|------|------|
| `Identity.is_system_admin` | ✅ 已有 | 区分管理员和普通用户 |
| `Skill.owner_type` / `Skill.owner_id` | ✅ 已有 | 支持 `"user"` / `"organization"` 所有权 |
| `Skill.visibility` | ✅ 已有 | `Public` / `OrgVisible` / `Private` |
| `Skill.review_status` | ✅ 已有 | `draft` / `pending_review` / `approved` / `rejected` / `published` |
| `OrgMembership` | ✅ 已有 | 身份与组织的角色关联 (Owner/Admin/Reviewer/Developer/Member) |
| `JWT auth_source` | ✅ 已有 | `AdminLogin` / `UserLogin` 区分 |
| `AgentContext` | ✅ 已有 | 请求级上下文（含 identity_id, roles, org_id） |
| 权限系统 (RBAC) | ✅ 已有 | 48+ 权限点，Role/Permission/IdentityRole 完整 |
| `PermissionService` | ✅ 已有 | `check_permission()` 方法 |
| 前端 auth store | ✅ 已有 | `isAuthenticated`, `currentUser` |

### 2.2 当前缺陷

| 问题 | 严重程度 | 说明 |
|------|---------|------|
| 前端不区分角色 | 🔴 高 | 所有登录用户看到相同的完整管理后台 |
| `api_keys.organization_id` NOT NULL | 🔴 高 | 个人用户无法创建无组织的 API Key |
| Skill 创建/更新无权限校验 | 🟡 中 | 任意认证用户可以修改任意 Skill |
| 审核权限未校验角色 | 🟡 中 | 审核接口 (`approve` / `reject`) 仅检查是否登录 |
| 前端 API Key 创建不限制组织列表 | 🟡 中 | 显示所有组织而非用户所属组织 |
| Nav 组件无角色感知 | 🟡 中 | 所有导航项对所有人可见 |

---

## 3. 目标架构

### 3.1 前后端权限模型

```
┌─────────────────────────────────────────────────────────────┐
│                        前端 (Svelte)                         │
├──────────────────────┬──────────────────────────────────────┤
│    管理员视图          │         用户视图                      │
│                      │                                      │
│  📊 仪表盘 (Stats)    │  📊 我的 Skill (MySkills)            │
│  🏢 租户 (Tenants)    │  🛒 技能市场 (Marketplace)            │
│  🏛️ 组织 (Orgs)       │  👤 个人信息 (Profile)               │
│  👥 身份 (Identities)  │  🔑 我的 API Key (MyApiKeys)         │
│  📦 Skill 管理        │  📋 我的审核任务 (如果属于某组织)      │
│  📋 审核队列          │                                      │
│  🔍 审计日志          │                                      │
│  🔧 组织工具          │                                      │
│  ⚙️ 系统设置          │                                      │
│  👤 个人信息          │                                      │
│  🔑 API Key 管理      │                                      │
│  等等...              │                                      │
└──────────────────────┴──────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      后端权限中间件                            │
│                                                             │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │ require_admin│  │ require_owner │  │ require_org_role  │  │
│  │ (系统管理员)  │  │ (资源所有者)   │  │ (组织角色>=X)      │  │
│  └─────────────┘  └──────────────┘  └───────────────────┘  │
│                                                             │
│  Skill 权限决策树:                                           │
│  ┌─ is_system_admin? ──── YES → 允许所有操作                 │
│  ├─ owner_type=user + owner_id=self? ── YES → 允许 CRUD     │
│  ├─ owner_type=organization + in_same_org?                   │
│  │   ├─ role >= Reviewer → 允许审核                          │
│  │   ├─ role >= Developer → 允许 CRUD                        │
│  │   └─ role = Member → 只读                                 │
│  └─ visibility=Public + review_status=published → 只读       │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Skill 权限矩阵

| 操作 | 系统管理员 | 所有者(个人) | 同组织 Owner/Admin | 同组织 Reviewer | 同组织 Developer | 同组织 Member | 其他用户(公开) |
|------|:--------:|:--------:|:----------------:|:-------------:|:--------------:|:-----------:|:----------:|
| 创建 Skill | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| 查看自己 Skill | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | - |
| 查看组织 Skill | ✅ | - | ✅ | ✅ | ✅ | ✅ | - |
| 查看市场 Skill | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 更新 Skill | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| 删除 Skill | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| 提交审核 | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| 审核通过/拒绝 | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ |
| 发布 Skill | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| 查看 Skill 内容 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅(公开) |

---

## 4. 后端改造

### 4.1 数据库迁移

#### 4.1.1 `api_keys` 表 — organization_id 改为可空

```sql
-- 迁移文件: src/db/migrations/022_make_api_key_org_optional.sql

-- 将 organization_id 改为可空
ALTER TABLE api_keys ALTER COLUMN organization_id DROP NOT NULL;

-- 添加注释
COMMENT ON COLUMN api_keys.organization_id IS '组织 ID，个人用户创建的 API Key 可为空';
```

#### 4.1.2 新增 Skill 所有权索引

```sql
-- 迁移文件: src/db/migrations/023_add_skill_owner_indexes.sql

-- 优化按所有者查询 Skill 的性能
CREATE INDEX IF NOT EXISTS idx_skills_owner ON skills(owner_type, owner_id);
CREATE INDEX IF NOT EXISTS idx_skills_author_identity ON skills(author_identity_id);
```

### 4.2 模型层改造

#### 4.2.1 `ApiKey` 模型 — organization_id 改为 Option

**文件**: `src/models/api_key.rs`

```rust
// 修改前
pub struct ApiKey {
    pub organization_id: Uuid,   // 必填
}

pub struct CreateApiKeyRequest {
    pub organization_id: Uuid,   // 必填
}

// 修改后
pub struct ApiKey {
    pub organization_id: Option<Uuid>,   // 可为空
}

pub struct CreateApiKeyRequest {
    pub organization_id: Option<Uuid>,   // 可为空
}
```

#### 4.2.2 新增 `UserCreateApiKeyRequest`（用户自服务用）

```rust
/// 用户自服务创建 API Key 的请求（区别于管理员创建）
#[derive(Debug, Clone, Deserialize)]
pub struct UserCreateApiKeyRequest {
    /// 组织 ID，可为空（个人用户不选组织时）
    pub organization_id: Option<Uuid>,
    pub name: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default = "default_rate_limit")]
    pub rate_limit: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}
```

#### 4.2.3 `ApiKeyResponse` — organization_id 改为 Option

```rust
pub struct ApiKeyResponse {
    pub organization_id: Option<Uuid>,
    // ... 其他字段不变
}
```

### 4.3 权限服务增强

**文件**: `src/services/permission.rs`

#### 4.3.1 新增 Skill 权限校验方法

```rust
impl PermissionService {
    /// 校验当前用户是否可以对指定 Skill 执行操作
    pub async fn check_skill_permission(
        &self,
        ctx: &AgentContext,
        skill: &Skill,
        action: SkillAction,
    ) -> Result<(), ApiError> {
        // 1. 系统管理员拥有所有权限
        if self.is_system_admin(ctx).await? {
            return Ok(());
        }

        let identity_id = ctx.require_identity()?;

        match action {
            // 读操作：公开已发布的 Skill 所有人可读
            SkillAction::Read => {
                if skill.review_status == "published" && skill.visibility == Visibility::Public {
                    return Ok(());
                }
                // 否则需要是所有者或同组织成员
                self.require_skill_access(ctx, skill, identity_id).await
            }
            // 写操作需要更高权限
            SkillAction::Update | SkillAction::SubmitReview => {
                self.require_skill_write(ctx, skill, identity_id).await
            }
            // 删除只有系统管理员和所有者可以做
            SkillAction::Delete => {
                self.require_skill_owner_or_admin(ctx, skill, identity_id).await
            }
            // 审核需要 Reviewer 及以上角色
            SkillAction::Approve | SkillAction::Reject => {
                self.require_skill_reviewer(ctx, skill, identity_id).await
            }
            SkillAction::Publish => {
                self.require_skill_owner_or_admin(ctx, skill, identity_id).await
            }
        }
    }

    /// 检查用户是否是 Skill 的所有者或同组织成员（至少读权限）
    async fn require_skill_access(
        &self,
        ctx: &AgentContext,
        skill: &Skill,
        identity_id: Uuid,
    ) -> Result<(), ApiError> {
        match skill.owner_type.as_str() {
            "user" => {
                if skill.owner_id == Some(identity_id) {
                    return Ok(());
                }
                // 也要检查 author_identity_id
                if skill.author_identity_id == Some(identity_id) {
                    return Ok(());
                }
            }
            "organization" => {
                if let Some(org_id) = skill.owner_id {
                    if self.is_org_member(identity_id, org_id).await? {
                        return Ok(());
                    }
                }
            }
            _ => {}
        }
        Err(ApiError::Forbidden("无权访问此 Skill".to_string()))
    }

    /// 检查写权限（用户自己是所有者，或组织 Developer 及以上）
    async fn require_skill_write(
        &self,
        ctx: &AgentContext,
        skill: &Skill,
        identity_id: Uuid,
    ) -> Result<(), ApiError> {
        match skill.owner_type.as_str() {
            "user" => {
                if skill.owner_id == Some(identity_id)
                    || skill.author_identity_id == Some(identity_id)
                {
                    return Ok(());
                }
            }
            "organization" => {
                if let Some(org_id) = skill.owner_id {
                    let role = self.get_org_role(identity_id, org_id).await?;
                    if role >= OrgRole::Developer {
                        return Ok(());
                    }
                }
            }
            _ => {}
        }
        Err(ApiError::Forbidden("无权修改此 Skill".to_string()))
    }

    /// 检查所有者或管理员权限
    async fn require_skill_owner_or_admin(
        &self,
        ctx: &AgentContext,
        skill: &Skill,
        identity_id: Uuid,
    ) -> Result<(), ApiError> {
        match skill.owner_type.as_str() {
            "user" => {
                if skill.owner_id == Some(identity_id)
                    || skill.author_identity_id == Some(identity_id)
                {
                    return Ok(());
                }
            }
            "organization" => {
                if let Some(org_id) = skill.owner_id {
                    let role = self.get_org_role(identity_id, org_id).await?;
                    if role >= OrgRole::Admin {
                        return Ok(());
                    }
                }
            }
            _ => {}
        }
        Err(ApiError::Forbidden("无权执行此操作".to_string()))
    }

    /// 检查审核权限（Reviewer 及以上）
    async fn require_skill_reviewer(
        &self,
        ctx: &AgentContext,
        skill: &Skill,
        identity_id: Uuid,
    ) -> Result<(), ApiError> {
        // 组织 Skill 的审核需要 Reviewer 及以上
        if skill.owner_type == "organization" {
            if let Some(org_id) = skill.owner_id {
                let role = self.get_org_role(identity_id, org_id).await?;
                if role >= OrgRole::Reviewer {
                    return Ok(());
                }
            }
        }
        Err(ApiError::Forbidden("无权审核此 Skill".to_string()))
    }

    /// 判断是否是系统管理员
    async fn is_system_admin(&self, ctx: &AgentContext) -> Result<bool, ApiError> {
        if let Some(id) = ctx.identity_id {
            // 从数据库查询 is_system_admin
            return self.identity_repo.is_system_admin(id).await
                .map_err(|e| ApiError::InternalError(e.to_string()));
        }
        Ok(false)
    }

    /// 判断是否是指定组织的成员
    async fn is_org_member(&self, identity_id: Uuid, org_id: Uuid) -> Result<bool, ApiError> {
        self.org_membership_repo
            .is_member(identity_id, org_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))
    }

    /// 获取用户在组织中的角色
    async fn get_org_role(&self, identity_id: Uuid, org_id: Uuid) -> Result<OrgRole, ApiError> {
        self.org_membership_repo
            .get_role(identity_id, org_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))
    }
}

/// Skill 操作类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkillAction {
    Read,
    Update,
    Delete,
    SubmitReview,
    Approve,
    Reject,
    Publish,
}
```

### 4.4 API Handler 改造

**文件**: `src/api/handlers.rs`

#### 4.4.1 Skill 相关 Handler 注入权限校验

需要在以下 handler 中添加权限校验：

```rust
// create_skill_handler: 校验用户是否有创建权限
// - 个人 Skill: owner_type="user", owner_id=identity_id 即可
// - 组织 Skill: 需要是组织成员且角色 >= Developer

// update_skill_handler: 校验写权限
pub async fn update_skill_handler(
    state: State<ApiState>,
    ctx: AgentContext,
    Path(id): Path<String>,
    Json(update): Json<SkillUpdate>,
) -> ApiResult<Json<Skill>> {
    // --- 新增权限校验 ---
    let skill = state.registry.get_skill(&id).await?;
    state.permission_service
        .check_skill_permission(&ctx, &skill, SkillAction::Update)
        .await?;
    // --- 原有逻辑 ---
    // ...
}

// delete_skill_handler: 校验所有者/管理员权限
pub async fn delete_skill_handler(
    state: State<ApiState>,
    ctx: AgentContext,
    Path(id): Path<String>,
) -> ApiResult<Json<()>> {
    let skill = state.registry.get_skill(&id).await?;
    state.permission_service
        .check_skill_permission(&ctx, &skill, SkillAction::Delete)
        .await?;
    // ...
}

// submit_review_skill_handler: 校验写权限
// approve_org_skill_handler / reject_org_skill_handler: 校验审核权限
// publish_skill_handler: 校验所有者权限
// get_skill_handler: 校验读权限（公开已发布的除外）
```

#### 4.4.2 API Key 自服务 Handler 改造

```rust
/// 用户自服务创建 API Key
pub async fn create_my_api_key_handler(
    state: State<ApiState>,
    ctx: AgentContext,
    Json(req): Json<UserCreateApiKeyRequest>,
) -> ApiResult<Json<ApiKeyResponse>> {
    let identity_id = ctx.require_identity()?;

    // 验证：如果提供了 organization_id，必须是用户所属组织
    if let Some(org_id) = req.organization_id {
        let is_member = state.permission_service
            .is_org_member(identity_id, org_id)
            .await?;
        if !is_member {
            return Err(ApiError::Forbidden(
                "不能为不属于的组织创建 API Key".to_string()
            ));
        }
    }

    // 创建 API Key（organization_id 可为 None）
    let api_key = state.api_key_service
        .create_user_api_key(identity_id, req)
        .await?;

    Ok(Json(api_key))
}

/// 返回用户所在的组织列表（用于 API Key 创建时的下拉选择）
pub async fn get_user_orgs_handler(
    state: State<ApiState>,
    ctx: AgentContext,
) -> ApiResult<Json<Vec<Organization>>> {
    let identity_id = ctx.require_identity()?;
    let orgs = state.org_service
        .get_user_organizations(identity_id)
        .await?;
    Ok(Json(orgs))
}
```

#### 4.4.3 Skill 列表接口按角色过滤

```rust
/// 获取 Skill 列表（按角色过滤）
pub async fn list_skills_handler(
    state: State<ApiState>,
    ctx: AgentContext,
    Query(params): Query<SkillListQuery>,
) -> ApiResult<Json<Vec<SkillMetadata>>> {
    let identity_id = ctx.require_identity()?;
    let is_admin = state.permission_service.is_system_admin(&ctx).await?;

    if is_admin {
        // 管理员可以看到所有 Skill
        return Ok(Json(state.registry.list_all_skills(&params).await?));
    }

    // 普通用户：返回自己的 Skill + 所在组织的 Skill + 市场中公开的 Skill
    let orgs = state.org_service.get_user_organizations(identity_id).await?;
    let org_ids: Vec<Uuid> = orgs.iter().map(|o| o.id).collect();

    let skills = state.skill_repo
        .list_user_accessible_skills(identity_id, &org_ids, &params)
        .await?;

    Ok(Json(skills))
}
```

#### 4.4.4 新增用户专用的 Skill 列表接口

```rust
// 路由: GET /api/v1/my-skills
pub async fn list_my_skills_handler(
    state: State<ApiState>,
    ctx: AgentContext,
) -> ApiResult<Json<Vec<SkillMetadata>>> {
    let identity_id = ctx.require_identity()?;
    let skills = state.skill_repo
        .list_skills_by_owner(identity_id)
        .await?;
    Ok(Json(skills))
}

// 路由: GET /api/v1/marketplace
// 已有 marketplace_handler，确保只返回 review_status=published 的 Skill
```

### 4.5 Skill Repository 新增查询方法

**文件**: `src/db/repositories/skill_repository.rs`

```rust
/// 查询用户可访问的 Skill（个人 + 所在组织 + 市场公开）
pub async fn list_user_accessible_skills(
    &self,
    identity_id: Uuid,
    org_ids: &[Uuid],
    params: &SkillListQuery,
) -> Result<Vec<SkillMetadata>, AppError> {
    // SQL 逻辑:
    // WHERE (
    //   -- 用户自己创建的
    //   (owner_type = 'user' AND owner_id = $identity_id)
    //   OR
    //   -- 用户所在组织的 Skill
    //   (owner_type = 'organization' AND owner_id = ANY($org_ids))
    //   OR
    //   -- 市场中已发布的公开 Skill
    //   (review_status = 'published' AND visibility = 'public')
    // )
}

/// 按所有者查询 Skill
pub async fn list_skills_by_owner(
    &self,
    identity_id: Uuid,
) -> Result<Vec<SkillMetadata>, AppError> {
    // WHERE owner_type = 'user' AND (owner_id = $identity_id OR author_identity_id = $identity_id)
}
```

### 4.6 API Key Service 改造

**文件**: `src/services/admin/api_key_service.rs`

```rust
/// 用户自服务创建 API Key（区别于管理员创建）
pub async fn create_user_api_key(
    &self,
    identity_id: Uuid,
    req: UserCreateApiKeyRequest,
) -> Result<ApiKeyResponse, AppError> {
    let raw_key = generate_api_key(); // sk_ 前缀 + 随机字符串
    let key_hash = hash_token(&raw_key);
    let key_prefix = &raw_key[..12]; // sk_xxxx + 前8位

    let api_key = ApiKey {
        id: Uuid::new_v4(),
        identity_id,
        organization_id: req.organization_id, // 可为 None
        key_hash,
        key_prefix: key_prefix.to_string(),
        name: req.name,
        scopes: req.scopes,
        rate_limit: req.rate_limit,
        status: ApiKeyStatus::Active,
        expires_at: req.expires_at,
        created_at: Utc::now(),
        last_used_at: None,
    };

    self.repo.insert(&api_key).await?;
    Ok(ApiKeyResponse::from_api_key(api_key, raw_key))
}
```

### 4.7 路由注册变更

**文件**: `src/api/routes.rs`

```rust
// 新增用户专用路由（非 admin 前缀）
.route("/api/v1/my-skills", get(list_my_skills_handler))
.route("/api/v1/my-orgs", get(get_my_orgs_handler))  // 获取用户所在组织列表

// 保持现有路由不变，handler 内部添加权限校验
```

### 4.8 登录响应增强

**文件**: `src/api/handlers.rs` — login handlers

```rust
/// 用户登录响应增加 is_admin 字段
#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub is_admin: bool,           // is_system_admin
    pub organizations: Vec<OrgInfo>,  // 用户所属组织
}

#[derive(Serialize)]
pub struct OrgInfo {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub role: String,  // 用户在该组织中的角色
}
```

---

## 5. 前端改造

### 5.1 Auth Store 增强

**文件**: `admin/src/lib/stores/auth.js`

```javascript
// 当前只有 isAuthenticated, currentUser
// 需要新增:

export const isAdmin = writable(false);
export const userOrganizations = writable([]);

// 登录成功后:
export function setAuth(token, userInfo) {
    localStorage.setItem('token', token);
    currentUser.set(userInfo);
    isAdmin.set(userInfo.is_admin === true);
    userOrganizations.set(userInfo.organizations || []);
    isAuthenticated.set(true);
}

export function isUserAdmin() {
    let value;
    isAdmin.subscribe(v => value = v)();
    return value;
}

export function getUserOrgs() {
    let value;
    userOrganizations.subscribe(v => value = v)();
    return value;
}
```

### 5.2 App.svelte 路由重构

**文件**: `admin/src/App.svelte`

核心变更：根据 `isAdmin` 注册不同的路由组。

```svelte
<script>
  import { isAuthenticated, isAdmin } from './lib/stores/auth.js';

  // 管理员和用户共享页面
  import Profile from './routes/Profile.svelte';
  import MyApiKeys from './routes/MyApiKeys.svelte';

  // 管理员专有页面
  import Stats from './routes/Stats.svelte';
  import Tenants from './routes/Tenants.svelte';
  import Organizations from './routes/Organizations.svelte';
  import OrganizationDetail from './routes/OrganizationDetail.svelte';
  import Identities from './routes/Identities.svelte';
  import Groups from './routes/Groups.svelte';
  import GroupDetail from './routes/GroupDetail.svelte';
  import Roles from './routes/Roles.svelte';
  import Skills from './routes/Skills.svelte';        // 管理员 Skill 管理
  import SkillDetail from './routes/SkillDetail.svelte';
  import Review from './routes/Review.svelte';
  import AuditLogs from './routes/AuditLogs.svelte';
  import AuditEntries from './routes/AuditEntries.svelte';
  import OrgTools from './routes/OrgTools.svelte';
  import ApiKeys from './routes/ApiKeys.svelte';
  import Sessions from './routes/Sessions.svelte';
  import Settings from './routes/Settings.svelte';
  import Sandbox from './routes/Sandbox.svelte';

  // 用户专有页面
  import MySkills from './routes/MySkills.svelte';     // 新增
  import Marketplace from './routes/Marketplace.svelte'; // 新增
</script>

{#if $isAdmin}
  <!-- 管理员路由 -->
  <Route path="/" component={Stats} />
  <Route path="/stats" component={Stats} />
  <Route path="/tenants" component={Tenants} />
  <Route path="/organizations" component={Organizations} />
  <Route path="/organizations/:id" component={OrganizationDetail} />
  <Route path="/identities" component={Identities} />
  <Route path="/groups" component={Groups} />
  <Route path="/groups/:id" component={GroupDetail} />
  <Route path="/roles" component={Roles} />
  <Route path="/skills" component={Skills} />
  <Route path="/skills/:id" component={SkillDetail} />
  <Route path="/review" component={Review} />
  <Route path="/audit" component={AuditLogs} />
  <Route path="/audit-entries" component={AuditEntries} />
  <Route path="/org-tools" component={OrgTools} />
  <Route path="/api-keys" component={ApiKeys} />
  <Route path="/sessions" component={Sessions} />
  <Route path="/settings" component={Settings} />
  <Route path="/sandboxes" component={Sandbox} />
  <!-- 管理员也有个人信息页 -->
  <Route path="/profile" component={Profile} />
  <Route path="/my-api-keys" component={MyApiKeys} />
{:else}
  <!-- 普通用户路由 -->
  <Route path="/" component={MySkills} />
  <Route path="/my-skills" component={MySkills} />
  <Route path="/marketplace" component={Marketplace} />
  <Route path="/profile" component={Profile} />
  <Route path="/my-api-keys" component={MyApiKeys} />
  <!-- 如果用户属于某组织，可访问有限的审核页面 -->
  {#if $userOrganizations && $userOrganizations.length > 0}
    <Route path="/my-review" component={MyReview.svelte} />
  {/if}
{/if}
```

### 5.3 Nav.svelte 导航重构

**文件**: `admin/src/components/Nav.svelte`

```svelte
<script>
  import { isAdmin, userOrganizations } from '../lib/stores/auth.js';
  import { link } from 'svelte-routing';
</script>

<nav class="sidebar">
  {#if $isAdmin}
    <!-- ========== 管理员导航 ========== -->
    <div class="nav-section">
      <div class="nav-section-title">概览</div>
      <a href="/stats" use:link>📊 仪表盘</a>
    </div>

    <div class="nav-section">
      <div class="nav-section-title">多租户管理</div>
      <a href="/tenants" use:link>🏢 租户</a>
      <a href="/organizations" use:link>🏛️ 组织</a>
      <a href="/identities" use:link>👥 身份管理</a>
      <a href="/groups" use:link>📁 组管理</a>
      <a href="/roles" use:link>🔐 角色权限</a>
    </div>

    <div class="nav-section">
      <div class="nav-section-title">Skill 管理</div>
      <a href="/skills" use:link>📦 全部 Skill</a>
      <a href="/review" use:link>📋 审核队列</a>
      <a href="/org-tools" use:link>🔧 组织工具</a>
    </div>

    <div class="nav-section">
      <div class="nav-section-title">运维管理</div>
      <a href="/audit" use:link>🔍 审计日志</a>
      <a href="/api-keys" use:link>🔑 API Key 管理</a>
      <a href="/sessions" use:link>📡 会话管理</a>
      <a href="/sandboxes" use:link>📦 沙箱管理</a>
      <a href="/settings" use:link>⚙️ 系统设置</a>
    </div>

    <div class="nav-section">
      <div class="nav-section-title">个人</div>
      <a href="/profile" use:link>👤 个人信息</a>
      <a href="/my-api-keys" use:link>🔑 我的 API Key</a>
    </div>
  {:else}
    <!-- ========== 普通用户导航 ========== -->
    <div class="nav-section">
      <div class="nav-section-title">Skill</div>
      <a href="/my-skills" use:link>📦 我的 Skill</a>
      <a href="/marketplace" use:link>🛒 技能市场</a>

      {#if $userOrganizations?.length > 0}
        <!-- 如果有 Reviewer 及以上角色，显示审核入口 -->
        <a href="/my-review" use:link>📋 待审核</a>
      {/if}
    </div>

    <div class="nav-section">
      <div class="nav-section-title">个人</div>
      <a href="/profile" use:link>👤 个人信息</a>
      <a href="/my-api-keys" use:link>🔑 我的 API Key</a>
    </div>
  {/if}
</nav>
```

### 5.4 新增页面

#### 5.4.1 MySkills.svelte（用户 Skill 管理）

```svelte
<!-- 我的 Skill 列表页面 -->
<!-- 显示当前用户创建的 Skill，支持 CRUD -->
<!-- Tab: 全部 | 草稿 | 审核中 | 已发布 | 已拒绝 -->
<!-- 每个 Skill 卡片：名称、状态、版本、创建时间、操作按钮 -->
```

#### 5.4.2 Marketplace.svelte（技能市场）

```svelte
<!-- 技能市场页面 -->
<!-- 显示所有已发布(published)的公开(public) Skill -->
<!-- 搜索、标签过滤、排序 -->
<!-- 可以查看详情、安装 -->
```

### 5.5 MyApiKeys.svelte 改造

**文件**: `admin/src/routes/MyApiKeys.svelte`

核心变更：创建 API Key 时的组织选择器改为只显示用户所在组织，且允许不选。

```svelte
<script>
  import { userOrganizations } from '../lib/stores/auth.js';

  // 创建 API Key 表单
  let createForm = {
    organization_id: null,  // 默认为空
    name: '',
    scopes: [],
    expires_at: null,
  };
</script>

<!-- 组织选择下拉 -->
<div class="form-group">
  <label>所属组织（可选）</label>
  <select bind:value={createForm.organization_id}>
    <option value={null}>-- 个人（不关联组织）--</option>
    {#each $userOrganizations as org}
      <option value={org.id}>{org.name}</option>
    {/each}
  </select>
</div>
```

---

## 6. 数据库变更

### 6.1 迁移文件清单

| 迁移编号 | 文件名 | 说明 |
|---------|--------|------|
| 022 | `make_api_key_org_optional.sql` | api_keys.organization_id 改为可空 |
| 023 | `add_skill_owner_indexes.sql` | 添加 Skill 所有权查询索引 |
| 024 | `seed_user_role.sql` | 种子数据：默认普通用户角色 |

### 6.2 022: api_keys.organization_id 可空

```sql
-- 先检查是否已有 NOT NULL 约束，如有则移除
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'api_keys'
          AND column_name = 'organization_id'
          AND is_nullable = 'NO'
    ) THEN
        ALTER TABLE api_keys ALTER COLUMN organization_id DROP NOT NULL;
    END IF;
END $$;
```

### 6.3 024: 默认用户角色种子

```sql
-- 为非管理员的注册用户自动赋予 'skill_developer' 角色
-- 此迁移确保角色表中存在默认用户角色

INSERT INTO roles (id, name, role_type, scope_level, permissions, description, created_at)
VALUES (
    gen_random_uuid(),
    'skill_user',
    'system',
    'global',
    '["skill:create", "skill:read", "skill:update", "skill:submit_review", "skill:install"]',
    '普通 Skill 用户，可创建和管理自己的 Skill',
    NOW()
)
ON CONFLICT (name) DO NOTHING;
```

---

## 7. API 变更清单

### 7.1 新增接口

| 方法 | 路径 | 说明 | 权限 |
|------|------|------|------|
| GET | `/api/v1/my-skills` | 获取当前用户的 Skill 列表 | 登录用户 |
| GET | `/api/v1/my-orgs` | 获取当前用户所在的组织列表 | 登录用户 |
| GET | `/api/v1/my-review` | 获取用户的待审核列表（组织内） | Reviewer+ |
| GET | `/api/v1/marketplace` | 技能市场（已有，确认过滤逻辑） | 所有人 |

### 7.2 修改接口

| 方法 | 路径 | 变更说明 |
|------|------|---------|
| POST | `/api/v1/skills` | 添加 owner_type/owner_id 自动设置 |
| GET | `/api/v1/skills` | 普通用户仅返回可访问的 Skill |
| PUT | `/api/v1/skills/:id` | 添加权限校验 |
| DELETE | `/api/v1/skills/:id` | 添加所有者权限校验 |
| POST | `/api/v1/skills/:id/submit-review` | 添加写权限校验 |
| POST | `/api/v1/skills/:id/approve` | 添加审核权限校验 |
| POST | `/api/v1/skills/:id/reject` | 添加审核权限校验 |
| POST | `/api/v1/skills/:id/publish` | 添加所有者权限校验 |
| GET | `/api/v1/skills/:id` | 添加读权限校验（公开除外） |
| POST | `/api/v1/auth/login` | 响应增加 `is_admin` + `organizations` |
| POST | `/api/v1/auth/register` | 注册成功赋予默认 `skill_user` 角色 |
| POST | `/api/v1/api-keys` | `organization_id` 改为可选，校验组织归属 |
| GET | `/api/v1/api-keys` | 返回格式 `organization_id` 可为 null |

### 7.3 响应格式变更

**登录响应**:
```json
{
  "token": "eyJ...",
  "user": {
    "id": "uuid",
    "username": "alice",
    "display_name": "Alice",
    "email": "alice@example.com",
    "is_admin": false,
    "organizations": [
      {
        "id": "uuid",
        "name": "工程团队",
        "slug": "engineering",
        "role": "developer"
      }
    ]
  }
}
```

**API Key 响应**:
```json
{
  "id": "uuid",
  "identity_id": "uuid",
  "organization_id": null,        // 可为 null
  "key": "sk_xxxx...",
  "key_prefix": "sk_abcd1234",
  "name": "My Personal Key",
  "scopes": [],
  "rate_limit": 1000,
  "status": "active",
  "expires_at": null,
  "created_at": "2026-07-13T00:00:00Z"
}
```

---

## 8. 测试计划

### 8.1 后端单元测试

| 测试用例 | 说明 |
|---------|------|
| `test_skill_permission_owner_crud` | Skill 所有者可以 CRUD 自己的 Skill |
| `test_skill_permission_other_user_readonly` | 非所有者无法修改他人 Skill |
| `test_skill_permission_admin_all` | 管理员可以操作所有 Skill |
| `test_skill_permission_org_member_write` | 同组织 Developer 可以修改组织 Skill |
| `test_skill_permission_org_reviewer_approve` | Reviewer 可以审核组织 Skill |
| `test_skill_permission_org_member_no_delete` | 普通成员不能删除组织 Skill |
| `test_skill_permission_marketplace_read` | 任何人都可以读取已发布的公开 Skill |
| `test_api_key_org_optional` | 个人用户可以创建无组织的 API Key |
| `test_api_key_org_validation` | 不能为不属于的组织创建 API Key |
| `test_api_key_admin_create` | 管理员创建 API Key 不受组织限制 |

### 8.2 后端集成测试

| 测试用例 | 说明 |
|---------|------|
| `test_admin_login_returns_admin_true` | 管理员登录返回 is_admin=true |
| `test_user_login_returns_admin_false` | 普通用户登录返回 is_admin=false |
| `test_user_register_gets_default_role` | 注册用户自动获得 skill_user 角色 |
| `test_list_skills_as_user` | 普通用户只能看到自己的 Skill |
| `test_list_skills_as_admin` | 管理员可以看到所有 Skill |

### 8.3 前端测试（手动验证清单）

| 验证项 | 管理员 | 普通用户 |
|--------|:-----:|:------:|
| 登录后重定向到正确页面 | `/stats` | `/my-skills` |
| 导航菜单仅显示对应项目 | ✅ | ✅ |
| API Key 创建时组织下拉正确 | 全部组织 | 仅用户组织+可为空 |
| Skill 列表正确过滤 | 全部 | 自己的+市场 |
| 无法通过 URL 直接访问无权限页面 | ✅ | ✅ |
| 修改他人 Skill 返回 403 | N/A | ✅ |

---

## 9. 实施步骤

### 阶段一：数据库 + 模型层（预计 2h）

| 步骤 | 文件 | 说明 |
|------|------|------|
| 1.1 | `src/db/migrations/022_*.sql` | api_keys.organization_id 改为可空 |
| 1.2 | `src/db/migrations/023_*.sql` | 添加 Skill 所有者索引 |
| 1.3 | `src/db/migrations/024_*.sql` | 默认普通用户角色种子 |
| 1.4 | `src/models/api_key.rs` | organization_id 改为 Option\<Uuid\> |
| 1.5 | `src/models/api_key.rs` | 新增 UserCreateApiKeyRequest |
| 1.6 | `src/db/repositories/api_key_repository.rs` | 适配 Option 字段 |
| 1.7 | `src/db/repositories/org_membership_repository.rs` | 新增 is_member / get_role 方法 |
| 1.8 | `cargo check` | 确保编译通过 |

### 阶段二：权限服务（预计 3h）

| 步骤 | 文件 | 说明 |
|------|------|------|
| 2.1 | `src/services/permission.rs` | 新增 SkillAction 枚举 |
| 2.2 | `src/services/permission.rs` | 新增 check_skill_permission |
| 2.3 | `src/services/permission.rs` | 新增 is_system_admin / is_org_member / get_org_role |
| 2.4 | `src/services/permission.rs` | 编写单元测试 |
| 2.5 | `cargo test` | 确保测试通过 |

### 阶段三：API Handler 改造（预计 4h）

| 步骤 | 文件 | 说明 |
|------|------|------|
| 3.1 | `src/api/handlers.rs` | 所有 Skill handler 添加权限校验 |
| 3.2 | `src/api/handlers.rs` | create_my_api_key_handler 改造 |
| 3.3 | `src/api/handlers.rs` | 新增 list_my_skills_handler |
| 3.4 | `src/api/handlers.rs` | 新增 get_my_orgs_handler |
| 3.5 | `src/api/handlers.rs` | 登录响应增强 (is_admin + organizations) |
| 3.6 | `src/api/handlers.rs` | 注册时赋予默认角色 |
| 3.7 | `src/api/routes.rs` | 注册新路由 |
| 3.8 | `src/services/admin/api_key_service.rs` | 新增 create_user_api_key |
| 3.9 | `src/db/repositories/skill_repository.rs` | 新增 list_user_accessible_skills, list_skills_by_owner |
| 3.10 | `cargo build && cargo test` | 确保编译和测试通过 |

### 阶段四：前端改造（预计 5h）

| 步骤 | 文件 | 说明 |
|------|------|------|
| 4.1 | `admin/src/lib/stores/auth.js` | 新增 isAdmin, userOrganizations |
| 4.2 | `admin/src/lib/api.js` | 适配新的登录响应格式 |
| 4.3 | `admin/src/App.svelte` | 按角色注册路由 |
| 4.4 | `admin/src/components/Nav.svelte` | 按角色显示导航 |
| 4.5 | `admin/src/routes/MySkills.svelte` | 新增用户 Skill 管理页面 |
| 4.6 | `admin/src/routes/Marketplace.svelte` | 新增技能市场页面 |
| 4.7 | `admin/src/routes/MyApiKeys.svelte` | 改造组织选择器 |
| 4.8 | `admin/src/routes/Login.svelte` | 登录后按角色跳转 |
| 4.9 | 前端构建测试 | `cd admin && npm run build` |

### 阶段五：集成测试 + 文档（预计 2h）

| 步骤 | 说明 |
|------|------|
| 5.1 | 端到端手动测试（管理员 + 普通用户两条路径） |
| 5.2 | 更新 README / CHANGELOG |
| 5.3 | 运行 `cargo test` 确保全部通过 |
| 5.4 | 运行 `cargo clippy` 修复新增 warning |

**预计总工时**: 16 小时

---

## 10. 风险评估

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| 现有 API Key 数据 organization_id 为 NOT NULL | 已有数据迁移无影响，但需确保所有读取代码适配 Option | 低 | 迁移前备份，灰度发布 |
| 前端路由重构引入回归 bug | 用户体验受损 | 中 | 保留旧路由别名，逐步废弃 |
| 权限校验过严导致现有集成方无法使用 | Agent API Key 访问受阻 | 中 | 保持 Agent API Key 认证的不变，仅对 Web 登录用户添加校验 |
| 用户注册后无默认角色 | 无法创建 Skill | 中 | 迁移中种子默认角色，注册 handler 中自动赋予 |
| 数据库迁移在已有环境执行失败 | 服务不可用 | 低 | 使用 IF EXISTS 保护性 SQL |

---

## 附录 A: 文件变更总览

```
文件变更统计:
├── 新增文件 (5)
│   ├── src/db/migrations/022_make_api_key_org_optional.sql
│   ├── src/db/migrations/023_add_skill_owner_indexes.sql
│   ├── src/db/migrations/024_seed_default_user_role.sql
│   ├── admin/src/routes/MySkills.svelte
│   └── admin/src/routes/Marketplace.svelte
│
├── 修改文件 (15+)
│   ├── src/models/api_key.rs                    # organization_id -> Option
│   ├── src/services/permission.rs               # +Skill 权限校验
│   ├── src/services/admin/api_key_service.rs    # +create_user_api_key
│   ├── src/api/handlers.rs                      # +权限校验, 新 handler, 登录增强
│   ├── src/api/routes.rs                        # +新路由
│   ├── src/db/repositories/skill_repository.rs  # +新查询方法
│   ├── src/db/repositories/api_key_repository.rs # 适配 Option
│   ├── src/db/repositories/org_membership_repository.rs # +is_member, get_role
│   ├── admin/src/App.svelte                     # 路由按角色拆分
│   ├── admin/src/components/Nav.svelte          # 导航按角色拆分
│   ├── admin/src/lib/stores/auth.js             # +isAdmin, userOrganizations
│   ├── admin/src/lib/api.js                     # +新 API 调用
│   ├── admin/src/routes/Login.svelte            # 登录后按角色跳转
│   └── admin/src/routes/MyApiKeys.svelte        # 组织选择器改造
```

## 附录 B: 关键设计决策记录

1. **为什么不在 Router 层做权限中间件？** — Axum 的 `FromRequestParts` 已经在 `AgentContext` 中实现了 JWT 解析。Skill 级别的权限校验放在 Handler 层更灵活，因为需要先查询 Skill 的所有权信息才能做判断。

2. **为什么 admin 和 user 不拆成两个独立前端应用？** — 共享组件多（Skill 详情、个人信息、API Key 管理），维护成本更低。通过路由分组 + Nav 条件渲染即可达到差异化效果。

3. **API Key 的 organization_id 为什么改为 Option 而不是拆表？** — 改动最小，向后兼容。如果未来有更复杂的 API Key 策略，可以再考虑独立模型。

4. **为什么不在 JWT claims 中直接存 is_admin？** — 角色可能变化（管理员被降级），JWT 是无状态的，存数据库查询更实时。可以通过 Redis 缓存优化性能。

---

> **下一步**: 请审核此方案，确认后按阶段逐步实施。
