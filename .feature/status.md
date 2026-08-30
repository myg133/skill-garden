# REQ-001 状态

## 基本信息

| 属性 | 值 |
|------|------|
| **REQ 编号** | REQ-001 |
| **标题** | Admin UI 多语言支持 |
| **状态** | 已退回 (第二轮 QA) |
| **开始日期** | 2024-08-30 |
| **负责人** | anspire-open-developer |
| **QA 轮次** | 第二轮 |

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
- [x] GroupDetail.svelte 部分翻译
- [x] MarketplaceRoles.svelte 部分翻译
- [x] ApiKeys.svelte 部分翻译
- [x] LanguageSwitcher.svelte A11y 部分修复
- [x] Nav.svelte "Sign Out" 翻译
- [x] Build 测试通过
- [x] 代码已提交并推送

### 待完成（第二轮 QA 退回原因）
- [ ] 补充剩余硬编码英文翻译（见下方详情）
- [ ] 完善 A11y（role="menuitem"）

## 第二轮 QA 验证结果

### ❌ 不通过 - 需修改后重新验证

### Build 测试: ✅ PASSED (2.37s)

### 发现的问题

#### BLOCKER - 硬编码英文（翻译覆盖不全）

**GroupDetail.svelte:**
- 第 30-43 行: `permissionLabels` 对象硬编码英文（'Read Skills', 'Write Skills', 等）
- 第 144 行: `placeholder="UUID of the identity"` 硬编码
- 第 148 行: `<label>Role</label>` 硬编码

**MarketplaceRoles.svelte:**
- 第 10 行: `const ROLE_LABEL = 'Marketplace Reviewer'` 硬编码常量
- 第 105-107 行: 表头 "User", "Email", "Role" 硬编码

**ApiKeys.svelte:**
- 第 78 行: `addToast('API Key revoked', 'success')` 硬编码英文
- 第 228 行: `<label>Identity</label>` 硬编码
- 第 234 行: `<label>Organization</label>` 硬编码
- 第 304 行: `placeholder="My API Key"` 硬编码
- 第 46, 55 行: 混合语言 addToast（中文/英文）

#### WARNING - A11y 不完整

**LanguageSwitcher.svelte:**
- 已有: role="menu", tabindex="0", on:keydown 处理器 ✅
- 缺失: role="menuitem" on menu buttons
- 缺失: aria-labelledby attribute

## 修复建议

1. **GroupDetail.svelte permissionLabels**: 
   - 改为翻译 keys，如 `$_('groups.readSkills')`
   - 在 en.json/zh.json 添加对应 keys

2. **MarketplaceRoles.svelte ROLE_LABEL**:
   - 改为 `$_('marketplace.marketplaceReviewer')`

3. **表头文本**:
   - 使用现有翻译 keys：`groups.identity`, `groups.email`, `groups.role`

4. **ApiKeys labels**:
   - Identity → `$_('groups.identity')`
   - Organization → `$_('organizations.title')`

5. **placeholders**:
   - "My API Key" → `$_('apiKeys.namePlaceholder')` 或直接使用英文 placeholder
   - "UUID of the identity" → 使用 `$_('groups.enterIdentityUuid')`

6. **addToast 消息**:
   - 统一使用翻译 keys

7. **LanguageSwitcher A11y**:
   - 给 menu 内的 button 添加 `role="menuitem"`

## 验收标准检查

| ID | 验收项 | 状态 | 备注 |
|----|--------|------|------|
| AC-001 | 页面加载时自动检测浏览器语言 | ✅ | getInitialLocale() 正确实现 |
| AC-002 | 页面右上角有语言切换器 | ✅ | LanguageSwitcher.svelte 存在 |
| AC-003 | 点击语言切换，中英文即时切换 | ✅ | setLocale() + $_() 响应式 |
| AC-004 | 语言偏好保存在 localStorage | ✅ | aionhive_locale key |
| AC-005 | URL 参数 ?lang=zh/?lang=en 强制指定语言 | ✅ | URLSearchParams 检查 |
| AC-006 | 所有页面文字已翻译 | ⚠️ | 部分仍有硬编码英文 |

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

- 2024-08-30 第二轮 QA: 完成度约 70%，仍有硬编码英文需要修复
- Build 通过，但翻译覆盖不全
- LanguageSwitcher 已添加部分 A11y 属性，需补充 menuitem role
