# REQ-002 状态

## 基本信息

| 属性 | 值 |
|------|------|
| **REQ 编号** | REQ-002 |
| **标题** | Tenant-Org-User 加入流程优化 |
| **状态** | 开发中 |
| **开始日期** | 2024-08-30 |
| **负责人** | anspire-open-developer |

## 优先级

| Phase | 标题 | 优先级 | 估计工时 | 状态 |
|-------|------|--------|----------|------|
| 1.1 | 组员搜索添加 | P0 | 4h | 进行中 |
| 1.2 | 加入申请流程 | P1 | 12h | 待开始 |
| 1.3 | 组织层级可视化 | P1 | 8h | 待开始 |

## 进度

### Phase 1.1: 组员搜索添加 (P0)
- [ ] 后端搜索 API (`GET /identities/search`)
- [ ] 前端搜索组件 (GroupDetail.svelte)
- [ ] 保留 UUID 输入方式

### Phase 1.2: 加入申请流程 (P1)
- [ ] 数据模型迁移 (`org_join_requests` 表)
- [ ] 申请 API (POST /orgs/{id}/join-request)
- [ ] 审批 API (GET/PUT /orgs/{id}/join-requests)
- [ ] 前端申请/审批 UI

### Phase 1.3: 组织层级可视化 (P1)
- [ ] 租户详情页增强
- [ ] 组织详情页增强

## 技术决策

| 决策 | 值 |
|------|-----|
| join_policy 默认值 | approval_required |
| 申请留言 | 可选 |
| 多审批人 | 是 |
| Group 名称唯一性 | 同一 Org 内唯一 |

## 提交历史

- 2024-08-30: 初始分配，开始 Phase 1.1 开发
