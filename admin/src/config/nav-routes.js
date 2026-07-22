/**
 * 导航路由配置表
 * 每个路由项声明自己需要的权限码，layout 引擎自动过滤可见 tab
 *
 * need: null 表示无需特殊权限（所有人可见）
 * need: "permission_code" 表示需要该权限码
 * systemRole: "role_name" 表示需要该系统角色
 */

export const adminNavRoutes = [
  {
    key: 'overview',
    label: 'Overview',
    icon: 'overview',
    tabs: [
      { href: '/stats',      label: 'Dashboard',    icon: 'dashboard',     need: 'system:admin:access' },
    ]
  },
  {
    key: 'users',
    label: 'Users',
    icon: 'users',
    tabs: [
      { href: '/identities', label: 'Identities',    icon: 'identities',    need: 'system:admin:access' },
      { href: '/api-keys',   label: 'API Keys',      icon: 'api-keys',      need: 'system:admin:access' },
    ]
  },
  {
    key: 'org',
    label: 'Organizations',
    icon: 'organizations',
    tabs: [
      { href: '/',             label: 'Organizations', icon: 'organizations', need: 'org:read' },
      { href: '/groups',       label: 'Groups',        icon: 'groups',        need: 'group:read' },
      { href: '/org-tools',    label: 'Org Tools',     icon: 'org-tools',     need: 'org:read' },
    ]
  },
  {
    key: 'skills',
    label: 'Content',
    icon: 'skills',
    tabs: [
      { href: '/skills',       label: 'Skills',        icon: 'skills',        need: null },
      { href: '/review',       label: 'Review',        icon: 'review',        need: null },
      { href: '/marketplace-roles', label: 'Marketplace Roles', icon: 'roles',  need: 'marketplace:role_assign' },
    ]
  },
  {
    key: 'account',
    label: 'Account',
    icon: 'profile',
    tabs: [
      { href: '/profile',    label: 'My Profile',    icon: 'profile',       need: null },
      { href: '/my-api-keys',label: 'My API Keys',   icon: 'my-api-keys',   need: null },
    ]
  },
  {
    key: 'system',
    label: 'System',
    icon: 'settings',
    tabs: [
      { href: '/tenants',      label: 'Tenants',        icon: 'tenants',       need: 'system:admin:access' },
      { href: '/system-roles', label: 'System Roles',   icon: 'roles',          need: 'system:admin:access' },
      { href: '/sessions',     label: 'Sessions',        icon: 'sessions',      need: 'system:admin:access' },
      { href: '/audit',        label: 'Audit Logs',     icon: 'audit-logs',    need: 'system:admin:access' },
      { href: '/settings',     label: 'Settings',       icon: 'settings',      need: 'system:admin:access' },
    ]
  },
  {
    key: 'infra',
    label: 'Infrastructure',
    icon: 'infrastructure',
    tabs: [
      { href: '/sandboxes',    label: 'Sandboxes',      icon: 'sandbox',       need: 'system:admin:access' },
    ]
  },
];

/** 用户侧导航（非管理员布局） */
export const userNavRoutes = [
  {
    key: 'dashboard',
    label: 'Dashboard',
    icon: 'dashboard',
    tabs: [
      { href: '/user',              label: 'Dashboard',    icon: 'dashboard',    need: null },
      { href: '/user/marketplace',  label: 'Marketplace',  icon: 'marketplace',  need: null },
      { href: '/user/skills',       label: 'My Skills',    icon: 'skills',       need: null },
      { href: '/user/submissions',  label: 'Submissions',  icon: 'submissions',  need: null },
      { href: '/profile',           label: 'My Profile',   icon: 'profile',      need: null },
      { href: '/my-api-keys',       label: 'My API Keys',  icon: 'my-api-keys',  need: null },
    ]
  },
];
