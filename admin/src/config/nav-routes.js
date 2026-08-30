/**
 * 导航路由配置表
 * 每个路由项声明自己需要的权限码，layout 引擎自动过滤可见 tab
 *
 * need: null 表示无需特殊权限（所有人可见）
 * need: "permission_code" 表示需要该权限码
 * systemRole: "role_name" 表示需要该系统角色
 * labelKey: i18n 翻译 key
 * allowedRoles: ['super_admin', 'tenant_admin', 'org_admin'] - 允许查看此菜单组的角色
 *   - 'super_admin': 超级管理员
 *   - 'tenant_admin': 租户管理员
 *   - 'org_admin': 组织管理员 (org_admin 或 owner)
 */

// 角色常量
export const ROLE_SUPER_ADMIN = 'super_admin';
export const ROLE_TENANT_ADMIN = 'tenant_admin';
export const ROLE_ORG_ADMIN = 'org_admin';

/**
 * 完整导航配置（供 Nav.svelte 使用，按角色过滤）
 */
export const adminNavRoutes = [
  {
    key: 'overview',
    labelKey: 'nav.overview',
    icon: 'overview',
    allowedRoles: [ROLE_SUPER_ADMIN, ROLE_TENANT_ADMIN, ROLE_ORG_ADMIN],
    tabs: [
      { href: '/stats',      labelKey: 'nav.dashboard',    icon: 'dashboard',     need: 'system:admin:access' },
    ]
  },
  {
    key: 'tenants',
    labelKey: 'nav.tenants',
    icon: 'tenants',
    allowedRoles: [ROLE_SUPER_ADMIN, ROLE_TENANT_ADMIN],
    tabs: [
      { href: '/tenants',    labelKey: 'nav.tenants',        icon: 'tenants',       need: 'tenant:read' },
    ]
  },
  {
    key: 'users',
    labelKey: 'nav.users',
    icon: 'users',
    allowedRoles: [ROLE_SUPER_ADMIN],
    tabs: [
      { href: '/identities', labelKey: 'nav.identities',    icon: 'identities',    need: 'system:admin:access' },
      { href: '/api-keys',   labelKey: 'nav.apiKeys',       icon: 'api-keys',      need: 'system:admin:access' },
    ]
  },
  {
    key: 'org',
    labelKey: 'nav.organizations',
    icon: 'organizations',
    allowedRoles: [ROLE_SUPER_ADMIN, ROLE_TENANT_ADMIN, ROLE_ORG_ADMIN],
    tabs: [
      { href: '/',             labelKey: 'nav.organizations', icon: 'organizations', need: 'org:read' },
      { href: '/groups',       labelKey: 'nav.groups',        icon: 'groups',        need: 'tenant:read' },
      { href: '/org-tools',   labelKey: 'nav.orgTools',      icon: 'org-tools',     need: 'org:read' },
    ]
  },
  {
    key: 'members',
    labelKey: 'nav.members',
    icon: 'users',
    allowedRoles: [ROLE_ORG_ADMIN],
    tabs: [
      // 成员管理入口 - 跳转到第一个组织的成员页面
      { href: '/org-members',  labelKey: 'nav.members',       icon: 'users',         need: 'org:member:read' },
    ]
  },
  {
    key: 'skills',
    labelKey: 'nav.content',
    icon: 'skills',
    allowedRoles: [ROLE_SUPER_ADMIN, ROLE_TENANT_ADMIN, ROLE_ORG_ADMIN],
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
    allowedRoles: [ROLE_SUPER_ADMIN, ROLE_TENANT_ADMIN, ROLE_ORG_ADMIN],
    tabs: [
      { href: '/profile',      labelKey: 'nav.myProfile',    icon: 'profile',       need: null },
      { href: '/my-api-keys', labelKey: 'nav.myApiKeys',    icon: 'my-api-keys',   need: null },
    ]
  },
  {
    key: 'system',
    labelKey: 'nav.system',
    icon: 'settings',
    allowedRoles: [ROLE_SUPER_ADMIN],
    tabs: [
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
    allowedRoles: [ROLE_SUPER_ADMIN],
    tabs: [
      { href: '/sandboxes',    labelKey: 'nav.sandboxes',      icon: 'sandbox',       need: 'system:admin:access' },
    ]
  },
];

/**
 * 根据角色过滤导航菜单组
 * @param {string} userRole - 用户角色: 'super_admin' | 'tenant_admin' | 'org_admin'
 * @returns {Array} 过滤后的导航组
 */
export function filterNavRoutesByRole(userRole) {
  if (!userRole) return [];
  
  return adminNavRoutes.filter(group => {
    // 如果没有 allowedRoles，默认允许所有角色
    if (!group.allowedRoles || group.allowedRoles.length === 0) {
      return true;
    }
    return group.allowedRoles.includes(userRole);
  });
}



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

/**
 * 角色相关的快捷操作卡片配置
 */
export const quickActionCards = {
  tenant_admin: [
    { key: 'manage_members', labelKey: 'quickActions.manageMembers', icon: 'users', href: '/tenants' },
    { key: 'view_orgs', labelKey: 'quickActions.viewOrganizations', icon: 'organizations', href: '/' },
    { key: 'invite_members', labelKey: 'quickActions.inviteMembers', icon: 'invite', href: '/tenants' },
  ],
  org_admin: [
    { key: 'add_members', labelKey: 'quickActions.addMembers', icon: 'users', href: '/' },
    { key: 'manage_tools', labelKey: 'quickActions.manageTools', icon: 'org-tools', href: '/org-tools' },
    { key: 'view_skills', labelKey: 'quickActions.viewSkills', icon: 'skills', href: '/skills' },
  ],
  super_admin: [
    { key: 'view_stats', labelKey: 'quickActions.viewStats', icon: 'dashboard', href: '/stats' },
    { key: 'manage_tenants', labelKey: 'quickActions.manageTenants', icon: 'tenants', href: '/tenants' },
    { key: 'view_audit', labelKey: 'quickActions.viewAudit', icon: 'audit-logs', href: '/audit' },
  ],
};

/**
 * 获取角色对应的快捷操作卡片
 * @param {string} role - 用户角色
 * @returns {Array} 快捷操作卡片列表
 */
export function getQuickActionsForRole(role) {
  return quickActionCards[role] || [];
}
