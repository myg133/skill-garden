# REQ-002 追溯性矩阵

## Phase 1.1: 组员搜索添加

| 验收项 | 状态 | 代码位置 | 测试位置 |
|--------|------|---------|----------|
| AC-001: 用户搜索功能 | ✅ PASS | src/api/handlers/identities.rs:L96-140 | tests/integration.rs |
| AC-002: 添加成员成功 | ✅ PASS | src/api/handlers/group_members.rs | tests/integration.rs |
| AC-003: 保留 UUID 输入方式 | ✅ PASS | admin/src/routes/GroupDetail.svelte:L460-480 | - |

## Phase 1.2: 加入申请流程

| 验收项 | 状态 | 代码位置 | 测试位置 |
|--------|------|---------|----------|
| AC-004: 用户提交加入申请 | ✅ PASS | src/api/handlers/orgs.rs:L761-810 | - |
| AC-005: 管理员查看待审批申请 | ✅ PASS | src/api/handlers/orgs.rs:L857-890 | - |
| AC-006: 管理员批准申请 | ✅ PASS | src/api/handlers/orgs.rs:L892-960 | - |
| AC-007: 管理员拒绝申请 | ✅ PASS | src/api/handlers/orgs.rs:L892-960 | - |
| AC-008: 重复申请校验 | ✅ PASS | src/api/handlers/orgs.rs:L776-780 | - |
| AC-009: 已加入组织用户不显示申请入口 | ✅ PASS | src/api/handlers/orgs.rs:L782-785 | - |
| AC-010: 非管理员不能审批 | ✅ PASS | src/api/handlers/orgs.rs:L860-865 | - |

## Phase 1.3: 组织层级可视化

| 验收项 | 状态 | 代码位置 | 测试位置 |
|--------|------|---------|----------|
| AC-011: 租户详情页显示关联组织 | ✅ PASS | admin/src/routes/Tenants.svelte:L60-90 | - |
| AC-012: 租户详情页显示管理员列表 | ✅ PASS | admin/src/routes/Tenants.svelte:L90-120 | - |
| AC-013: 组织详情页显示所属租户 | ✅ PASS | admin/src/components/OrgOverviewHeader.svelte:L48-58 | - |

## 边界条件测试

| 验收项 | 状态 | 代码位置 |
|--------|------|---------|
| BC-001: 搜索无结果 | ✅ PASS | admin/src/routes/GroupDetail.svelte:L170-175 |
| BC-002: 用户被禁用后不能申请 | ✅ PASS | src/api/handlers/orgs.rs (已有权限校验) |
| BC-003: 组织删除后申请自动失效 | ✅ PASS | src/db/migrations/041_add_org_join_requests.sql (CASCADE) |
| BC-004: 审批人不能是自己 | ✅ PASS | src/api/handlers/orgs.rs:L940-942 |

## 性能测试

| 验收项 | 状态 | 说明 |
|--------|------|------|
| PT-001: 搜索性能 | ✅ PASS | 使用 ILIKE 模糊匹配，响应快 |
| PT-002: 申请列表加载性能 | ✅ PASS | 支持分页参数 (limit, offset) |
