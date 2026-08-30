# REQ-001 状态

## 基本信息

| 属性 | 值 |
|------|-----|
| **REQ 编号** | REQ-001 |
| **标题** | Admin UI 多语言支持 |
| **状态** | 已退回 (第三轮 QA) |
| **开始日期** | 2024-08-30 |
| **负责人** | anspire-open-developer |
| **QA 轮次** | 第三轮 |

## 第三轮 QA 验证结果

### Build 测试: ✅ PASSED (2.38s)

### ✅ 已修复

| 文件 | 修复项 | 状态 |
|------|--------|------|
| GroupDetail.svelte | permissionLabels 翻译 | ✅ |
| GroupDetail.svelte | placeholder/label 翻译 | ✅ |
| MarketplaceRoles.svelte | ROLE_LABEL 翻译 | ✅ |
| MarketplaceRoles.svelte | 表头翻译 | ✅ |
| ApiKeys.svelte | 大部分 label/placeholder 翻译 | ✅ |
| LanguageSwitcher.svelte | role="menuitem" | ✅ |

### ❌ 仍需修复 (BLOCKER)

**ApiKeys.svelte 残留硬编码:**

| 行号 | 代码 | 修复建议 |
|------|------|---------|
| 46 | `addToast('身份列表加载失败', 'warning');` | `addToast($_('apiKeys.identityListLoadFailed'), 'warning');` |
| 55 | `addToast('组织列表加载失败', 'warning');` | `addToast($_('apiKeys.organizationListLoadFailed'), 'warning');` |

### 修复步骤

1. 在 `en.json` 添加:
```json
"apiKeys": {
  "identityListLoadFailed": "Failed to load identity list",
  "organizationListLoadFailed": "Failed to load organization list"
}
```

2. 在 `zh.json` 添加:
```json
"apiKeys": {
  "identityListLoadFailed": "身份列表加载失败",
  "organizationListLoadFailed": "组织列表加载失败"
}
```

3. 修改 `ApiKeys.svelte`:
```javascript
// Line 46
addToast($_('apiKeys.identityListLoadFailed'), 'warning');
// Line 55
addToast($_('apiKeys.organizationListLoadFailed'), 'warning');
```

## 验收标准检查

| ID | 验收项 | 状态 | 备注 |
|----|--------|------|------|
| AC-001 | 页面加载时自动检测浏览器语言 | ✅ | getInitialLocale() 正确实现 |
| AC-002 | 页面右上角有语言切换器 | ✅ | LanguageSwitcher.svelte 存在 |
| AC-003 | 点击语言切换，中英文即时切换 | ✅ | setLocale() + $_() 响应式 |
| AC-004 | 语言偏好保存在 localStorage | ✅ | aionhive_locale key |
| AC-005 | URL 参数 ?lang=zh/?lang=en 强制指定语言 | ✅ | URLSearchParams 检查 |
| AC-006 | 所有页面文字已翻译 | ⚠️ | ApiKeys.svelte 残留 2 处硬编码 |

## 备注

- 2024-08-30 第三轮 QA: 完成度约 95%，仅剩 ApiKeys.svelte 2 处硬编码
- Build 通过
- 所有第一轮、第二轮问题已修复
- 需要修复 ApiKeys.svelte 后重新提交
