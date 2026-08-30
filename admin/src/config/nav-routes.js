/**
 * 导航路由配置表
 * 每个路由项声明自己需要的权限码，layout 引擎自动过滤可见 tab
 *
 * need: null 表示无需特殊权限（所有人可见）
 * need: "permission_code" 表示需要该权限码
 * systemRole: "role_name" 表示需要该系统角色
 * labelKey: i18n 翻译 key
 */

export const adminNavRoutes = [
  {
    key: 'overview',
    labelKey: 'nav.overview',
    icon: 'overview',
    tabs: [
      { href: '/stats',      labelKey: 'nav.dashboard',    icon: 'dashboard',     need: 'system:admin:access' },
    ]
  },
  {
    key: 'users',
    labelKey: 'nav.users',
    icon: 'users',
    tabs: [
      { href: '/identities', labelKey: 'nav.identities',    icon: 'identities',    need: 'system:admin:access' },
      { href: '/api-keys',   labelKey: 'nav.apiKeys',       icon: 'api-keys',      need: 'system:admin:access' },
    ]
  },
  {
    key: 'org',
    labelKey: 'nav.organizations',
    icon: 'organizations',
    tabs: [
      { href: '/',             labelKey: 'nav.organizations', icon: 'organizations', need: 'org:read' },
      { href: '/groups',       labelKey: 'nav.groups',        icon: 'groups',        need: 'tenant:read' },
      { href: '/org-tools',    labelKey: 'nav.orgTools',      icon: 'org-tools',     need: 'org:read' },
    ]
  },
  {
    key: 'skills',
    labelKey: 'nav.content',
    icon: 'skills',
    tabs: [
      { href: '/marketplace',  labelKey: 'nav.marketplace',   icon: 'marketplace',   need: null },
      { href: '/skills',       labelKey: 'nav.skills',        icon: 'skills',        need: null },
      { href: '/review',       labelKey: 'nav.review',        icon: 'review',        need: 'skill:approve_review' },
      { href: '/marketplace-roles', labelKey: 'nav.marketplaceRoles', icon: 'roles',  need: 'marketplace:role_assign' },
    ]
  },
  {
    key: 'account',
    labelKey: 'nav.account',
    icon: 'profile',
    tabs: [
      { href: '/profile',    labelKey: 'nav.myProfile',    icon: 'profile',       need: null },
      { href: '/my-api-keys',labelKey: 'nav.myApiKeys',  icon: 'my-api-keys',   need: null },
    ]
  },
  {
    key: 'system',
    labelKey: 'nav.system',
    icon: 'settings',
    tabs: [
      { href: '/tenants',      labelKey: 'nav.tenants',        icon: 'tenants',       need: 'tenant:read' },
      { href: '/system-roles', labelKey: 'nav.systemRoles',   icon: 'roles',          need: 'system:admin:access' },
      { href: '/sessions',     labelKey: 'nav.sessions',       icon: 'sessions',      need: 'system:admin:access' },
      { href: '/audit',        labelKey: 'nav.auditLogs',      icon: 'audit-logs',    need: 'system:admin:access' },
      { href: '/settings',     labelKey: 'nav.settings',       icon: 'settings',      need: 'system:admin:access' },
    ]
  },
  {
    key: 'infra',
    labelKey: 'nav.infrastructure',
    icon: 'infrastructure',
    tabs: [
      { href: '/sandboxes',    labelKey: 'nav.sandboxes',      icon: 'sandbox',       need: 'system:admin:access' },
    ]
  },
];

/** 用户侧导航（非管理员布局） */
export const userNavRoutes = [
  {
    key: 'dashboard',
    labelKey: 'nav.dashboard',
    icon: 'dashboard',
    tabs: [
      { href: '/user',              labelKey: 'nav.dashboard',    icon: 'dashboard',    need: null },
      { href: '/user/marketplace',  labelKey: 'nav.marketplace',  icon: 'marketplace',  need: null },
      { href: '/user/skills',       labelKey: 'nav.mySkills',     icon: 'skills',       need: null },
      { href: '/user/submissions',  labelKey: 'nav.submissions',  icon: 'submissions',  need: null },
      { href: '/profile',           labelKey: 'nav.myProfile',    icon: 'profile',      need: null },
      { href: '/my-api-keys',       labelKey: 'nav.myApiKeys',   icon: 'my-api-keys',  need: null },
    ]
  },
];
