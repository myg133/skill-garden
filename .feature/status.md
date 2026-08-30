# REQ-003 Phase 2 Stage 1 状态

## 基本信息

| 属性 | 值 |
|------|-----|
| **REQ 编号** | REQ-003 |
| **Phase** | Phase 2 Stage 1 |
| **标题** | SaaS 自动创建租户 |
| **状态** | 已完成开发 |
| **开始日期** | 2024-08-30 |
| **负责人** | anspire-open-developer |

## 功能范围

### 2.1 运营模式开关
- [x] 环境变量配置：`AION_HIVE_TENANT_MODE`
- [x] Config 结构体扩展（lib.rs）

### 2.2 注册时自动创建租户（SaaS 模式）
- [x] UserRegisterBody 添加 tenant_name 字段
- [x] user_register_handler 扩展创建租户逻辑
- [x] 自动分配 tenant_admin 角色

### 2.3 SaaS 模式配置
- [x] AION_HIVE_SELF_SERVICE_TENANT
- [x] AION_HIVE_MAX_TENANTS_PER_USER
- [x] AION_HIVE_TENANT_APPROVAL_REQUIRED

## 验收标准

| 编号 | 描述 | 状态 |
|------|------|------|
| AC-201 | 用户注册时自动创建个人租户 | 待 QA 验证 |
| AC-202 | 注册用户自动成为该租户的 tenant_admin | 待 QA 验证 |
| AC-220 | 租户管理员只能看到自己租户的数据 | 待后续阶段 |
| AC-222 | 不同 TENANT_MODE 配置下，UI 展示正确的创建入口 | 待后续阶段 |

## 改动文件

- `src/lib.rs` - 读取 TENANT_MODE 环境变量，添加 TenantMode、TenantConfig
- `src/api/models.rs` - UserRegisterBody 添加 tenant_name
- `src/api/handlers/users.rs` - 注册逻辑扩展
- `src/api/http_state.rs` - 状态中添加 tenant_config
- `src/utils/mod.rs` - 添加 slugify 工具函数
- `admin/src/routes/Register.svelte` - 注册表单增加租户名称字段
- `admin/src/lib/api.js` - userRegister 支持 tenant_name
- `.env.example` - 新增配置项
- `CHANGELOG.md` - 更新变更日志

## 提交历史

- 2024-08-30: Phase 2 Stage 1 完成 - SaaS 自动创建租户
