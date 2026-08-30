# REQ-003 Phase 2 Stage 3 状态

## 基本信息

| 属性 | 值 |
|------|-----|
| **REQ 编号** | REQ-003 |
| **Phase** | Phase 2 Stage 3 |
| **标题** | SaaS 自助申请 + 审批流程 |
| **状态** | 开发中 |
| **开始日期** | 2024-08-30 |
| **负责人** | anspire-open-developer |

## 功能范围

### 3.1 申请创建租户 API
- [x] `POST /admin/tenants/requests` - 用户提交创建租户申请
- [x] `GET /admin/tenants/requests` - 列出所有申请（super_admin）
- [x] `PUT /admin/tenants/requests/{id}` - 审批（approve/reject）

### 3.2 审批通过后自动创建
- [x] 审批通过时自动创建租户
- [x] 自动将申请人设为 `tenant_admin`

### 3.3 前端：申请流程
- [x] `Tenants.svelte` - SaaS 模式显示"申请创建租户"按钮
- [x] 申请表单：租户名称 + 可选留言
- [x] 申请列表（super_admin）：显示所有待审批申请
- [x] 审批操作：批准/拒绝

## 验收标准

| 编号 | 描述 | 状态 |
|------|------|------|
| AC-203 | `SELF_SERVICE=true` + `APPROVAL=true` 时，用户可申请创建租户 | 待实现 |
| AC-204 | 申请需 super_admin 审批 | 待实现 |

## 改动文件

### 后端
- `src/db/migrations/042_add_tenant_creation_requests.sql` - 新建表
- `src/models/tenant.rs` - 添加 TenantCreationRequest 模型
- `src/db/repositories/tenant.rs` - 添加请求仓储
- `src/api/models.rs` - 添加 API 请求/响应模型
- `src/api/handlers/tenants.rs` - 添加申请/审批 API
- `src/api/routes.rs` - 注册新路由

### 前端
- `admin/src/lib/api.js` - 添加申请 API 方法
- `admin/src/routes/Tenants.svelte` - 申请按钮、表单、审批列表
- `admin/src/i18n/zh.json` - 添加国际化文本
- `admin/src/i18n/en.json` - 添加国际化文本

### 文档
- `CHANGELOG.md` - 更新变更日志
