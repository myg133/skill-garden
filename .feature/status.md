# REQ-002 状态

## 基本信息

| 属性 | 值 |
|------|------|
| **REQ 编号** | REQ-002 |
| **标题** | Tenant-Org-User 加入流程优化 |
| **状态** | 已完成 |
| **开始日期** | 2024-08-30 |
| **负责人** | anspire-open-developer |

## 优先级

| Phase | 标题 | 优先级 | 估计工时 | 状态 |
|-------|------|--------|----------|------|
| 1.1 | 组员搜索添加 | P0 | 4h | ✅ 已完成 |
| 1.2 | 加入申请流程 | P1 | 12h | ✅ 已完成 |
| 1.3 | 组织层级可视化 | P1 | 8h | ✅ 已完成 |

## 已完成功能

### Phase 1.1: 组员搜索添加
- [x] 后端搜索 API (`GET /api/v1/identities/search`)
- [x] IdentityService 和 IdentityRepository 新增 search 方法
- [x] 前端搜索组件 (GroupDetail.svelte)
- [x] 保留 UUID 输入方式作为备选

### Phase 1.2: 加入申请流程
- [x] 数据模型迁移 (`org_join_requests` 表)
- [x] `join_policy` 字段添加到 organizations 表
- [x] OrgJoinRequestRepository 和 OrgJoinRequestService
- [x] API 端点:
  - POST /orgs/:id/join-request
  - DELETE /orgs/:id/join-request
  - GET /orgs/:id/my-join-request
  - GET /orgs/:id/join-requests
  - PUT /orgs/:id/join-requests/:request_id
- [x] 审批通过后自动创建 org_memberships 记录

### Phase 1.3: 组织层级可视化
- [x] Tenants.svelte: 显示关联组织和租户管理员
- [x] OrganizationDetail.svelte: 显示所属租户
- [x] OrgOverviewHeader.svelte: 新增 tenantName 属性

## 技术决策

| 决策 | 值 |
|------|-----|
| join_policy 默认值 | approval_required |
| 申请留言 | 可选 |
| 多审批人 | 是 |
| Group 名称唯一性 | 同一 Org 内唯一 |

## 提交历史

- 2024-08-30: Phase 1.1 完成 - 组员搜索功能
- 2024-08-30: Phase 1.2 完成 - 组织加入申请流程
- 2024-08-30: Phase 1.3 完成 - 组织层级可视化
