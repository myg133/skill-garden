import { writable, get } from 'svelte/store';

function createPermissionStore() {
  const { subscribe, set, update } = writable({
    systemRoles: [],
    tenantRoles: [],
    orgRoles: [],
    groupRoles: [],
    permissions: new Set(),
    loaded: false,
  });

  return {
    subscribe,

    /** 登录时调用：根据登录响应的 user 字段初始化 */
    initFromLogin(user) {
      const systemRoles = user.system_roles || [];
      const tenantRoles = (user.tenant_roles || []).map(t => ({
        tenant_id: t.tenant_id,
        tenant_name: t.tenant_name,
        role: t.role_name,
      }));
      const orgRoles = (user.organizations || []).map(o => ({
        org_id: o.id,
        org_name: o.name,
        role: o.role,
      }));
      set({
        systemRoles,
        tenantRoles,
        orgRoles,
        groupRoles: [],
        permissions: new Set(systemRoles), // 初始化阶段仅凭角色名判断
        loaded: true,
      });
      // 校验持久化的组织上下文：若当前用户不属于该组织，清除
      validateSelectedOrg(orgRoles);
    },

    /** 刷新权限时调用（GET /users/me/permissions） */
    initFromPermissions(data) {
      const systemRoles = data.system_roles || [];
      const tenantRoles = (data.tenant_roles || []).map(t => ({
        tenant_id: t.tenant_id,
        tenant_name: t.tenant_name,
        role: t.role_name,
      }));
      const orgRoles = (data.org_roles || []).map(o => ({
        org_id: o.org_id,
        org_name: o.org_name,
        role: o.role_name,
      }));
      const groupRoles = (data.group_roles || []).map(g => ({
        group_id: g.group_id,
        group_name: g.group_name,
        role: g.role_name,
      }));
      const permissions = new Set(data.permissions || []);
      set({
        systemRoles,
        tenantRoles,
        orgRoles,
        groupRoles,
        permissions,
        loaded: true,
      });
      // 页面刷新后校验组织上下文
      validateSelectedOrg(orgRoles);
    },

    /** 刷新权限列表（页面刷新后调用） */
    async refresh() {
      try {
        const { api } = await import('../lib/api.js');
        const data = await api.getMyPermissions();
        this.initFromPermissions(data);
      } catch {
        // 静默失败，使用登录时的缓存数据
      }
    },

    reset() {
      set({
        systemRoles: [],
        tenantRoles: [],
        orgRoles: [],
        groupRoles: [],
        permissions: new Set(),
        loaded: false,
      });
    },
  };
}

export const permissionStore = createPermissionStore();

/**
 * 校验 localStorage 中持久化的 selected_org 是否在当前用户的组织列表中。
 * 若不在，清除旧的组织上下文，避免新登录用户看到上一个用户遗留的组织信息。
 */
function validateSelectedOrg(orgRoles) {
  try {
    const saved = localStorage.getItem('selected_org');
    if (!saved) return;
    const org = JSON.parse(saved);
    // __personal__ 是个人空间，始终有效
    if (!org || org.id === '__personal__') return;
    const isMember = orgRoles.some(r => r.org_id === org.id);
    if (!isMember) {
      localStorage.removeItem('selected_org');
    }
  } catch {
    localStorage.removeItem('selected_org');
  }
}

// ========== 纯函数入口（不依赖 Svelte reactivity，可任意位置调用） ==========

/** 检查用户是否有指定的 permission_code */
export function hasPermission(code) {
  if (!code) return true;
  const s = get(permissionStore);
  // super_admin 或通配符拥有所有权限
  if (s.systemRoles.includes('super_admin') || s.permissions.has('*')) return true;
  return s.permissions.has(code);
}

/** 检查用户是否有指定的系统角色 */
export function hasSystemRole(role) {
  if (!role) return true;
  const s = get(permissionStore);
  if (s.systemRoles.includes('super_admin')) return true;
  return s.systemRoles.includes(role);
}

/** 检查用户在指定组织中是否有某个角色（或多个角色之一） */
export function hasOrgRole(orgId, ...roles) {
  if (!orgId || roles.length === 0) return true;
  const s = get(permissionStore);
  if (s.systemRoles.includes('super_admin')) return true;
  return s.orgRoles.some(r => r.org_id === orgId && roles.includes(r.role));
}

/** 获取用户为 tenant_admin 的租户 ID 列表 */
export function getTenantIdsWithRole(roleName) {
  const s = get(permissionStore);
  return s.tenantRoles.filter(t => t.role === roleName).map(t => t.tenant_id);
}

/** 获取用户在某角色下的组织 ID 列表 */
export function getOrgIdsWithRole(...roleNames) {
  const s = get(permissionStore);
  return s.orgRoles.filter(o => roleNames.includes(o.role)).map(o => o.org_id);
}

const ADMIN_ROLES = ['super_admin', 'tenant_admin', 'marketplace_admin', 'marketplace_reviewer'];

/** 判断当前用户是否为任意级别的管理员（系统角色或租户管理员角色） */
export function isAnyAdmin() {
  const s = get(permissionStore);
  return (
    s.systemRoles.some(r => ADMIN_ROLES.includes(r)) ||
    s.tenantRoles.some(t => ADMIN_ROLES.includes(t.role))
  );
}

/** 判断当前用户是否为纯个人用户（无任何管理角色） */
export function isPureUser() {
  return !isAnyAdmin();
}

/** 判断当前用户是否为超级管理员 */
export function isSuperAdmin() {
  const s = get(permissionStore);
  return s.systemRoles.includes('super_admin');
}

/** 判断当前用户是否为租户管理员（在任何租户中） */
export function isTenantAdmin() {
  const s = get(permissionStore);
  return s.tenantRoles.some(t => t.role === 'tenant_admin');
}

/** 判断当前用户是否为组织管理员（在任何组织中） */
export function isOrgAdmin() {
  const s = get(permissionStore);
  return s.orgRoles.some(r => r.role === 'org_admin' || r.role === 'owner');
}

/**
 * 获取角色专属默认路由
 * @returns {string} 默认路由路径
 */
export function getDefaultRoute() {
  const s = get(permissionStore);

  // super_admin → /stats
  if (s.systemRoles.includes('super_admin')) {
    return '/stats';
  }

  // tenant_admin → 第一个租户详情页
  if (isTenantAdmin()) {
    const tenantId = s.tenantRoles.find(t => t.role === 'tenant_admin')?.tenant_id;
    if (tenantId) {
      return `/tenants/${tenantId}`;
    }
    return '/tenants';
  }

  // org_admin → 第一个组织详情页
  if (isOrgAdmin()) {
    const orgId = s.orgRoles.find(r => r.role === 'org_admin' || r.role === 'owner')?.org_id;
    if (orgId) {
      return `/organizations/${orgId}`;
    }
    return '/';
  }

  // 其他用户 → /user
  return '/user';
}

/**
 * 获取用户所属的第一个租户 ID（用于 tenant_admin）
 * @returns {string|null}
 */
export function getFirstTenantId() {
  const s = get(permissionStore);
  return s.tenantRoles.find(t => t.role === 'tenant_admin')?.tenant_id || null;
}

/**
 * 获取用户所属的第一个组织 ID（用于 org_admin）
 * @returns {string|null}
 */
export function getFirstOrgId() {
  const s = get(permissionStore);
  const adminRole = s.orgRoles.find(r => r.role === 'org_admin' || r.role === 'owner');
  return adminRole?.org_id || null;
}
