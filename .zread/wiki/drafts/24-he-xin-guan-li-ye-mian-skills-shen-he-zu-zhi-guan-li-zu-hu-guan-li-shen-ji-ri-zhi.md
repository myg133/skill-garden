管理后台的核心管理页面由四个独立的功能模块组成——**Skills 审核队列**（Review Queue）、**组织管理**（Organizations）、**租户管理**（Tenants）和**审计日志**（Audit Logs）。这四个页面分别对应 Skill 生命周期审核、组织架构管理、多租户基础设施和操作审计追踪，是管理员日常操作的核心界面。所有页面的路由权限由 `nav-routes.js` 中的 `need` 字段驱动，Layout 引擎自动过滤不可见的标签页按钮，确保 UI 只展示当前用户有权限访问的入口。Sources: [nav-routes.js](admin/src/config/nav-routes.js#L10-L78)

## Skills 审核队列：四队列审批工作流

审核页面是系统中权限语义最丰富的页面之一，它管理 Skill 从草稿到上架的完整审批链路。页面加载时通过 `api.listSkills()` 获取所有 Skill，然后按状态字段分拣到四个独立的队列中：`status === 'pending_review'` 进入内部审核队列；`marketplace_status === 'pending_review'` 进入市场审核队列；`marketplace_status === 'pending_delist'` 进入下架请求队列；`marketplace_status === 'pending_update'` 进入待更新审核队列。Sources: [Review.svelte](admin/src/routes/Review.svelte#L10-L41)

**内部审核队列**使用 `ReviewActions` 组件，该组件提供 Approve 和 Reject 两个操作按钮，并内嵌了 `RejectModal` 用于填写驳回原因。按钮的可见性由 `canApproveReject(skill)` 函数控制——对于个人 Skill，所有者可以自审批；对于组织 Skill，需要组织内有 Reviewer 及以上角色，且不能审核自己提交的 Skill。审核通过后调用 `api.approveSkill(id)`，驳回则调用 `api.rejectSkill(id, reason)`，操作完成后页面自动导航回 `/review` 刷新队列。Sources: [ReviewActions.svelte](admin/src/components/ReviewActions.svelte#L1-L80), [skillPerms.js](admin/src/lib/skillPerms.js#L137-L152)

**市场审核队列**、**下架请求队列**和**待更新审核队列**使用更直接的内联按钮设计，每个 Skill 卡片上直接显示"通过/驳回"或"批准下架/驳回"按钮。这三个队列的审批操作对应不同的后端 API 端点：市场审核调用 `marketplaceReviewApprove/Reject`，下架审批调用 `marketplaceApproveDelist/RejectDelist`，更新审批调用 `marketplaceApproveUpdate/RejectUpdate`。每个队列的操作按钮受 `canReviewMarketplace` 权限控制，该权限来自 `hasPermission('marketplace:review_approve')` 或 `hasPermission('marketplace:review_reject')`。Sources: [Review.svelte](admin/src/routes/Review.svelte#L210-L452), [api.js](admin/src/lib/api.js#L811-L849)

三个市场队列的权限控制由后端 `audit.rs` 中的 `compute_audit_scope` 函数保障——`super_admin` 全平台可见，`tenant_admin` 仅能查看其管理租户下的记录。前端通过 `hasPermission('audit:read_global')` 控制页面入口。Sources: [audit.rs](src/api/handlers/audit.rs#L28-L68)

## 组织管理：列表视图与详情面板

组织管理页面由列表页和详情页两层构成。**列表页**（Organizations.svelte）以卡片网格展示所有组织，每张卡片显示组织名称首字母图徽、创建者角色徽章、创建日期和所属租户名称。列表页的顶部导航区分了三种角色视野：`isSystemAdmin`（super_admin / marketplace_admin）看到"管理所有租户下的组织"，`isTenantAdmin` 看到"管理你租户下的组织"，普通用户看到"你的组织"。Sources: [Organizations.svelte](admin/src/routes/Organizations.svelte#L107-L120)

列表页支持按租户过滤——`canManageTenants` 为 true 时显示租户下拉选择器，过滤参数 `tenant_id` 传递给 `api.listOrganizations()`。创建组织通过 Modal 表单完成，需要 `tenant:org_create` 权限。删除操作需要二次确认。Sources: [Organizations.svelte](admin/src/routes/Organizations.svelte#L22-L104)

**组织详情页**（OrganizationDetail.svelte）是组织管理的核心操作界面。它使用 `OrgOverviewHeader` 组件展示组织名称、ID、Slug、租户信息和四个统计卡片（创建时间、成员数、活跃会话数、注册工具数）。头部下方提供四个标签页：Members、Sessions、Tools、Groups，通过 `activeTab` 状态切换。Sources: [OrganizationDetail.svelte](admin/src/routes/OrganizationDetail.svelte#L315-L345), [OrgOverviewHeader.svelte](admin/src/components/OrgOverviewHeader.svelte#L1-L89)

**Members 标签页**通过 `OrgMembersTab` 组件实现，展示成员表格（ID、用户名、邮箱、角色、加入时间、操作）。角色分配支持 owner/admin/reviewer/developer/member 五级体系，操作为行内编辑模式。成员管理有三级权限控制：`canInviteMember`、`canManageRoles`、`canRemoveMember`，均由 `isOrgAdmin || isTenantOrSuper` 决定。Sources: [OrgMembersTab.svelte](admin/src/components/OrgMembersTab.svelte#L1-L145)

**Groups 标签页**通过 `OrgGroupsTab` 组件管理组织下的子群组，支持创建（Modal 表单）、编辑（行内编辑）、删除（确认后操作）群组。群组类型分为 team/project/department 三种。群组成员管理通过 Modal 弹窗展示，支持添加/移除成员和角色变更。Sources: [OrgGroupsTab.svelte](admin/src/components/OrgGroupsTab.svelte#L1-L307)

**Sessions 标签页**展示组织内所有 MCP 会话的列表，包括身份名称、状态、创建时间、最后活跃时间、结束时间和持续时间。`isOrgAdmin` 或 `isTenantOrSuper` 用户可以结束活跃会话。**Tools 标签页**展示已注册的组织工具列表，并提供注册新工具的入口。Sources: [OrganizationDetail.svelte](admin/src/routes/OrganizationDetail.svelte#L347-L569)

## 租户管理：多租户基础设施

租户管理页面（Tenants.svelte）是系统中最顶层的管理界面，仅 `super_admin` 和 `tenant_admin` 可以访问。页面以卡片网格展示所有租户，每张卡片显示租户名称首字母图徽、Slug 标识符、状态徽章（active/其他）和创建日期。Sources: [Tenants.svelte](admin/src/routes/Tenants.svelte#L63-L133)

创建租户通过 Modal 表单实现，需要填写 Name（公司名称）和 Slug（唯一标识符），提交时调用 `api.createTenant()`。删除租户需要二次确认，调用 `api.deleteTenant()`。这两个操作都需要 `ACTIONS.Tenants.create` 和 `ACTIONS.Tenants.delete` 权限码。Sources: [Tenants.svelte](admin/src/routes/Tenants.svelte#L135-L178), [actions.js](admin/src/config/actions.js#L14-L18)

后端 `list_tenants_handler` 实现了基于角色的可见性过滤——`super_admin` 可以看到所有租户，`tenant_admin` 只能看到其被分配管理的租户列表。创建租户则要求调用者必须是 `super_admin`。Sources: [tenants.rs](src/api/handlers/tenants.rs#L10-L51)

## 审计日志：操作溯源与过滤

审计日志页面（AuditLogs.svelte）提供全平台操作的可追溯视图。页面顶部是一个过滤面板，包含四个过滤器：Action（操作类型下拉，可选 skill_create/skill_approve/skill_reject/skill_update/skill_delete 等）、Agent ID（文本输入框）、From/To（日期范围选择器）。Search 按钮触发 `fetchLogs()`，Reset 按钮清空所有过滤器。Sources: [AuditLogs.svelte](admin/src/routes/AuditLogs.svelte#L1-L129)

日志数据通过 `api.listAuditLogs()` 获取，后端支持按 action、agent_id、resource_type、tenant_id、organization_id、identity_id 等多维度过滤。前端对日期过滤做了客户端二次处理——将 `from_date` 和 `to_date` 转换为时间戳范围，对返回的日志列表进行 `Array.filter()`。Sources: [AuditLogs.svelte](admin/src/routes/AuditLogs.svelte#L19-L48), [api.js](admin/src/lib/api.js#L571-L574)

**AuditTable 组件**将日志渲染为结构化表格，包含六列：Timestamp（时间戳，font-mono 字体显示）、Operator（操作者名称 + 身份类型徽章：user/agent/external_agent/system）、Action（带颜色编码的徽章：create 绿色、approve 蓝色、reject 红色、delete 琥珀色、update 蓝色）、Resource（资源类型:ID 格式）、Details（从详情 JSON 中提取人可读摘要）、IP（IP 地址，font-mono 字体）。操作类型徽章的颜色映射通过 `actionBadge()` 函数实现，身份类型徽章通过 `identityTypeBadge()` 函数实现。Sources: [AuditTable.svelte](admin/src/components/AuditTable.svelte#L1-L84)

后端 `audit.rs` 中的审计日志查询实现了严格的角色级 scope 隔离——`compute_audit_scope` 函数检查当前用户身份，`super_admin` 获得无限制访问，`tenant_admin` 的查询被限制在其管理的租户范围内，并预先反查出该租户下所有 identity 用于历史日志的兜底过滤。任何没有 `super_admin` 或 `tenant_admin` 角色的用户请求审计日志都会返回 403 Forbidden。Sources: [audit.rs](src/api/handlers/audit.rs#L86-L200)

## 权限体系全景

这四个管理页面的权限校验层级清晰，可以总结为以下分层结构：

| 管理页面 | 权限入口控制 | 操作权限码 | 数据可见性范围 |
|---------|------------|-----------|--------------|
| Skills 审核 | `skill:approve_review` (内部) / `marketplace:review_approve` (市场) | `skill:approve_review`, `skill:reject_review`, `marketplace:review_approve`, `marketplace:review_reject` | 当前组织内 pending_review (内部) / 全平台 pending_review (市场) |
| 组织管理 | `org:read` | `tenant:org_create`, `org:update`, `org:delete`, `org:member_invite`, `org:member_role_assign` | 按用户角色：super_admin 全平台 / tenant_admin 所属租户 / 普通用户所属组织 |
| 租户管理 | `tenant:read` | `tenant:create`, `tenant:delete` | super_admin 全平台 / tenant_admin 仅管理租户 |
| 审计日志 | `audit:read_global` | `audit:read_global` | super_admin 全平台 / tenant_admin 仅管理租户 + 该租户下 identity 的操作 |

Sources: [actions.js](admin/src/config/actions.js#L12-L134), [permission.js](admin/src/stores/permission.js#L120-L155)

## 下一步阅读

- 权限体系的完整设计请参阅 [RBAC 权限模型：System/Tenant/Org/Group 四级角色体系](8-rbac-quan-xian-mo-xing-system-tenant-org-group-si-ji-jiao-se-ti-xi)
- 前端权限系统的 Store 驱动机制请参阅 [前端权限系统：Store 驱动的角色判断与 UI 动态渲染](23-qian-duan-quan-xian-xi-tong-store-qu-dong-de-jiao-se-pan-duan-yu-ui-dong-tai-xuan-ran)
- 管理后台的整体布局与认证流程请参阅 [Admin 布局：认证流程、权限初始化与组织上下文切换](22-admin-bu-ju-ren-zheng-liu-cheng-quan-xian-chu-shi-hua-yu-zu-zhi-shang-xia-wen-qie-huan)
- 后端 API 的 Handler 权限校验模式请参阅 [Handler 模式：请求处理、权限校验与错误处理](11-handler-mo-shi-qing-qiu-chu-li-quan-xian-xiao-yan-yu-cuo-wu-chu-li)