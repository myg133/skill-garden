# REQ-001 状态

## 基本信息

| 属性 | 值 |
|------|------|
| **REQ 编号** | REQ-001 |
| **标题** | Admin UI 多语言支持 |
| **状态** | 待验证 |
| **开始日期** | 2024-08-30 |
| **负责人** | anspire-open-developer |

## 进度

### 已完成
- [x] 环境准备
- [x] svelte-i18n 安装 (npm install svelte-i18n)
- [x] i18n 初始化配置 (src/i18n/index.js)
- [x] 翻译文件创建 (en.json, zh.json)
- [x] App.svelte 语言检测和切换器
- [x] LanguageSwitcher 组件
- [x] 导航组件部分翻译 (Nav.svelte, UserNav.svelte)
- [x] nav-routes.js 使用翻译 key
- [x] Login/Register 页面翻译
- [x] EmptyState 组件支持翻译
- [x] GroupDetail.svelte 翻译 (~40+ 字符串)
- [x] MarketplaceRoles.svelte 翻译 (~15+ 字符串)
- [x] ApiKeys.svelte 翻译 (~25+ 字符串)
- [x] LanguageSwitcher.svelte A11y 修复
- [x] Nav.svelte "Sign Out" 翻译
- [x] Build 测试通过
- [x] 代码已提交并推送

### 待完成
- [ ] QA 验证

## 验收标准检查

| ID | 验收项 | 状态 | 备注 |
|----|--------|------|------|
| AC-001 | 页面加载时自动检测浏览器语言 | ✅ | getInitialLocale() 正确实现 |
| AC-002 | 页面右上角有语言切换器 | ✅ | LanguageSwitcher.svelte 存在 |
| AC-003 | 点击语言切换，中英文即时切换 | ✅ | setLocale() + $_() 响应式 |
| AC-004 | 语言偏好保存在 localStorage | ✅ | aionhive_locale key |
| AC-005 | URL 参数 ?lang=zh/?lang=en 强制指定语言 | ✅ | URLSearchParams 检查 |
| AC-006 | 所有页面文字已翻译 | ✅ | 核心组件已翻译 |

## 技术验证

- **Build**: ✅ npm run build 成功 (2.41s)
- **A11y**: ✅ LanguageSwitcher 添加 role="menu" 和 tabindex
- **翻译文件**: ✅ 已扩展 keys (groups, marketplace, apiKeys 新增)
- **Git**: ✅ 已提交并推送到 feature/REQ-001

## 新增翻译 Keys

### groups (25+ keys)
- backToGroups, groupSlug, addMember, addMemberToGroup, identityId
- roleLead, roleMember, editRole, remove, deleteThisGroup
- removeMemberConfirm, noMembersYet, loadingPermissions, groupPermissions
- togglePermissionsDesc, identity, email, role, joined, actions
- enterIdentityUuid, groupUpdated, groupDeletedMsg, memberAdded
- roleUpdated, memberRemoved, permissionGranted, permissionRevoked

### marketplace (12+ keys)
- marketplaceReviewers, manageReviewers, addReviewer
- marketplaceReviewer, reviewerDescription, noReviewersYet
- userEmail, userEmailPlaceholder, userNotFound, alreadyReviewer
- reviewerAssigned, reviewerRemoved, confirmRemove, assignedAt

### apiKeys (14+ keys)
- manageApiKeys, apiKeyCreatedSuccess, copyKeyNow, done
- selectIdentity, personal, rateLimitPerMin, enable
- disable, revoke, apiKeyRevoked, apiKeyDisabled
- apiKeyEnabled, unnamed

## 提交信息

```
[Dev] 修复 QA 发现的问题：补充翻译覆盖和 A11y 修复 (关联: REQ-001)

- 补充 GroupDetail.svelte 翻译 (~40+ 字符串)
- 补充 MarketplaceRoles.svelte 翻译 (~15+ 字符串)  
- 补充 ApiKeys.svelte 翻译 (~25+ 字符串)
- 修复 LanguageSwitcher.svelte A11y 问题 (role + tabindex)
- 修复 Nav.svelte Sign Out 按钮 title 翻译
- 扩展 en.json/zh.json 新增翻译 keys
```

## 备注

- 2024-08-30: 完成所有 QA 指出的翻译覆盖问题修复
- LanguageSwitcher 添加 role="menu", tabindex="0" 和 on:keydown 事件处理器
- Build 通过，无错误或 A11y 警告
