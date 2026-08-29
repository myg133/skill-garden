import { get } from 'svelte/store';
import { isAuthenticated } from '../stores/auth.js';
import { permissionStore } from '../stores/permission.js';

/**
 * Skill 权限工具模块
 *
 * 设计原则：
 * - RBAC（hasPermission） → 管理页面：租户/组织/组/API Key/审计
 * - 本模块 → Skill CRUD 操作，与后端 check_skill_permission 逻辑对齐
 *
 * 后端 check_skill_permission 规则摘要：
 * - Read:     marketplace published → 所有人；owner → 是；同组织成员 → 是
 * - Update:   owner → 是；组织 Developer+ → 是
 * - Delete:   owner → 是；组织 Admin+ → 是
 * - SubmitReview: owner → 是；组织 Developer+ → 是
 * - Approve:  不能审自己；组织 Reviewer+；个人 Skill 所有者可自审批
 * - Publish:  owner → 是；组织 Admin+ → 是
 * - Create:   已认证即可创建个人 Skill；组织 Developer+ 可创建组织 Skill
 *
 * 前端策略：采用「宽松显隐 + 后端兜底」原则
 * - 不过度隐藏按钮，避免误伤合法用户
 * - 后端 check_skill_permission 会在 handler 层做最终权限判决
 */

const ORG_ROLE_LEVEL = {
  member: 0,
  developer: 1,
  reviewer: 2,
  admin: 3,
  owner: 4,
};

/* ========== 基础能力判断 ========== */

/** 所有已认证用户均可创建个人 Skill（后端 create_skill_handler 无 RBAC 门控） */
export function canCreateSkill() {
  return get(isAuthenticated);
}

/** 在指定组织中创建 Skill 需要 Developer 及以上角色 */
export function canCreateOrgSkill(orgId) {
  if (!orgId) return false;
  const s = get(permissionStore);
  if (s.systemRoles.includes('super_admin')) return true;
  const match = s.orgRoles.find(r => r.org_id === orgId);
  if (!match) return false;
  return (ORG_ROLE_LEVEL[match.role] || -1) >= ORG_ROLE_LEVEL.developer;
}

/* ========== 按 Skill 对象的操作判断（宽松策略，后端兜底） ========== */

/**
 * 编辑 Skill — owner 或组织 Developer+
 * 市场 Skill（visibility=marketplace）：只有 super_admin 或拥有所在组织 Developer+ 角色才可编辑
 * 非市场 Skill（个人/组织内部）：宽松展示，后端兜底
 */
export function canEditSkill(skill) {
  if (!skill) return false;
  const s = get(permissionStore);
  if (s.systemRoles.includes('super_admin')) return true;

  // 组织 Skill：检查用户在该组织的角色 >= Developer
  if (skill.owner_type === 'organization' && skill.owner_id) {
    const match = s.orgRoles.find(r => r.org_id === skill.owner_id);
    if (match && (ORG_ROLE_LEVEL[match.role] || -1) >= ORG_ROLE_LEVEL.developer) return true;
  }

  // 市场 Skill：禁止普通用户编辑（非 owner、非 org Developer+ 一律拒绝）
  if (skill.visibility === 'marketplace') return false;

  // 非市场 Skill（个人/组织内部）：宽松展示，后端最终判决
  return get(isAuthenticated);
}

/**
 * 删除 Skill — owner 或组织 Admin+
 * 市场 Skill（visibility=marketplace 或 marketplace_status=listed）：
 *   只有 super_admin 或 marketplace_admin 才可删除
 * 非市场 Skill（个人/组织内部）：宽松展示，后端兜底
 */
export function canDeleteSkill(skill) {
  if (!skill) return false;
  const s = get(permissionStore);
  if (s.systemRoles.includes('super_admin')) return true;

  // 市场 Skill：仅 marketplace_admin 可删除（且后端会要求先下架）
  if (skill.visibility === 'marketplace' || skill.marketplace_status === 'listed') {
    return s.systemRoles.includes('marketplace_admin');
  }

  if (skill.owner_type === 'organization' && skill.owner_id) {
    const match = s.orgRoles.find(r => r.org_id === skill.owner_id);
    if (match && (ORG_ROLE_LEVEL[match.role] || -1) >= ORG_ROLE_LEVEL.admin) return true;
  }

  // 非市场 Skill（个人/组织内部）：宽松展示，后端最终判决
  return get(isAuthenticated);
}

/**
 * 发布 Skill — owner 或组织 Admin+
 */
export function canPublishSkill(skill) {
  if (!skill) return false;
  const s = get(permissionStore);
  if (s.systemRoles.includes('super_admin')) return true;

  if (skill.owner_type === 'organization' && skill.owner_id) {
    const match = s.orgRoles.find(r => r.org_id === skill.owner_id);
    if (match && (ORG_ROLE_LEVEL[match.role] || -1) >= ORG_ROLE_LEVEL.admin) return true;
  }

  // 个人 Skill 所有者可发布
  return get(isAuthenticated);
}

/**
 * 提交审核 — owner 或组织 Developer+
 */
export function canSubmitReview(skill) {
  if (!skill) return false;
  const s = get(permissionStore);
  if (s.systemRoles.includes('super_admin')) return true;

  if (skill.owner_type === 'organization' && skill.owner_id) {
    const match = s.orgRoles.find(r => r.org_id === skill.owner_id);
    if (match && (ORG_ROLE_LEVEL[match.role] || -1) >= ORG_ROLE_LEVEL.developer) return true;
  }

  return get(isAuthenticated);
}

/**
 * 审批/驳回 Skill — 组织 Reviewer+（不能审自己）或个人 Skill 所有者自审批
 */
export function canApproveReject(skill) {
  if (!skill) return false;
  const s = get(permissionStore);
  if (s.systemRoles.includes('super_admin')) return true;

  // 个人 Skill：所有者可直接审核
  if (skill.owner_type === 'user') return get(isAuthenticated);

  // 组织 Skill：需要 Reviewer+
  if (skill.owner_type === 'organization' && skill.owner_id) {
    const match = s.orgRoles.find(r => r.org_id === skill.owner_id);
    if (match && (ORG_ROLE_LEVEL[match.role] || -1) >= ORG_ROLE_LEVEL.reviewer) return true;
  }

  return false;
}

/* ========== 角色级别工具 ========== */

/** 获取用户在指定组织中的角色级别（-1 表示非成员） */
export function getOrgRoleLevel(orgId) {
  if (!orgId) return -1;
  const s = get(permissionStore);
  if (s.systemRoles.includes('super_admin')) return ORG_ROLE_LEVEL.owner;
  const match = s.orgRoles.find(r => r.org_id === orgId);
  return match ? (ORG_ROLE_LEVEL[match.role] ?? -1) : -1;
}
