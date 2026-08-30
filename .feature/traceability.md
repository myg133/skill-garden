# 追溯性矩阵 - REQ-003 Phase 2 Stage 3

## 验收标准追踪

| 验收项 | 状态 | 代码位置 | 测试位置 |
|--------|------|---------|----------|
| AC-203: `SELF_SERVICE=true` + `APPROVAL=true` 时，用户可申请创建租户 | PASS | `src/api/handlers/tenants.rs:create_tenant_request_handler` | 集成测试需 DB |
| AC-204: 申请需 super_admin 审批 | PASS | `src/api/handlers/tenants.rs:list_tenant_requests_handler`, `review_tenant_request_handler` | 集成测试需 DB |
| 审批通过后自动创建租户并分配 admin | PASS | `src/api/handlers/tenants.rs:review_tenant_request_handler` | 集成测试需 DB |
| 配额检查: MAX_TENANTS_PER_USER | PASS | `src/api/handlers/tenants.rs:create_tenant_request_handler` | 集成测试需 DB |

## 功能点追踪

| 功能点 | 实现位置 | 说明 |
|--------|---------|------|
| 申请创建租户 API | `src/api/handlers/tenants.rs:create_tenant_request_handler` | POST /admin/tenants/requests |
| 列出申请列表 | `src/api/handlers/tenants.rs:list_tenant_requests_handler` | GET /admin/tenants/requests |
| 审批申请 | `src/api/handlers/tenants.rs:review_tenant_request_handler` | PUT /admin/tenants/requests/:id |
| 自动创建租户 | `src/api/handlers/tenants.rs:review_tenant_request_handler` | 审批通过时创建 |
| 分配 tenant_admin | `src/api/handlers/tenants.rs:review_tenant_request_handler` | 分配角色给申请人 |
| 前端申请表单 | `admin/src/routes/Tenants.svelte` | 申请按钮和表单弹窗 |
| 前端审批列表 | `admin/src/routes/Tenants.svelte` | super_admin 查看和审批 |

## 质量检查

| 检查项 | 状态 |
|--------|------|
| 代码编译通过 | PASS |
| 集成测试通过 | PASS (6/6) |
| 代码格式化 | PASS (cargo fmt) |
| Lint 检查 | PASS (仅有预先存在的 warnings) |
| 文档已更新 | PASS (CHANGELOG.md) |
