/**
 * 页面操作权限配置表
 *
 * 每个页面定义其操作对应的权限码，页面组件显式引用：
 *   import { ACTIONS } from '../config/actions.js';
 *   const ACT = ACTIONS.Tenants;
 *   {#if hasPermission(ACT.create)}
 *     <button>Create</button>
 *   {/if}
 */

export const ACTIONS = {
  // ===== Tenants 页面 =====
  Tenants: {
    create: 'tenant:create',
    edit:   'tenant:update',
    delete: 'tenant:delete',
    view:   'tenant:read',
  },

  // ===== Identities 页面 =====
  Identities: {
    create: 'system:admin:access',
    edit:   'system:admin:access',
    delete: 'system:admin:access',
    view:   'system:admin:access',
    roles:  'system:admin:access',
  },

  // ===== Organizations 页面 =====
  Organizations: {
    create: 'tenant:org_create',
    view:   'org:read',
  },

  // ===== Organization Detail 页面 =====
  OrganizationDetail: {
    editSettings:  'org:update',
    deleteOrg:     'org:delete',
    transferOrg:   'org:transfer',
    inviteMember:  'org:member_invite',
    removeMember:  'org:member_remove',
    manageRoles:   'org:member_role_assign',
    skillTransfer: 'org:skill_transfer',
  },

  // ===== Marketplace 页面 =====
  Marketplace: {
    feature:   'marketplace:feature',
    unfeature: 'marketplace:unfeature',
    delist:    'marketplace:delist',
    relist:    'marketplace:relist',
  },

  // ===== Skills 页面 =====
  Skills: {
    create:               'skill:create',
    edit:                 'skill:update',
    delete:               'skill:delete',
    submitReview:         'skill:submit_review',
    // 内部发布
    publishInternal:      'skill:publish',
    // 市场操作
    submitToMarketplace:  'skill:publish_to_marketplace',
    marketFeature:        'marketplace:feature',
    marketUnfeature:      'marketplace:unfeature',
    marketDelist:         'marketplace:delist',
    marketRelist:         'marketplace:relist',
    marketApprove:        'marketplace:review_approve',
    marketReject:         'marketplace:review_reject',
  },

  // ===== Review 页面 =====
  Review: {
    approve:            'skill:approve_review',
    reject:             'skill:reject_review',
    marketApprove:      'marketplace:review_approve',
    marketReject:       'marketplace:review_reject',
  },

  // ===== Groups 页面 =====
  Groups: {
    create: 'group:create',
    edit:   'group:update',
    delete: 'group:delete',
    view:   'group:read',
  },

  // ===== Group Detail 页面 =====
  GroupDetail: {
    addMember:    'group:member_add',
    removeMember: 'group:member_remove',
    manageRoles:  'group:member_role_assign',
    editSkill:    'skill:update',
    deleteSkill:  'skill:delete',
    approveReview:'skill:approve_review',
  },

  // ===== API Keys 页面 (admin) =====
  ApiKeys: {
    create: 'apikey:create',
    revoke: 'apikey:revoke',
    view:   'apikey:read',
    scopeSet: 'apikey:scope_set',
  },

  // ===== Sandbox 页面 =====
  Sandbox: {
    manage: 'system:admin:access',
  },

  // ===== Audit 页面 =====
  Audit: {
    read: 'audit:read_global',
  },

  // ===== Settings 页面 =====
  Settings: {
    manage: 'system:admin:access',
  },

  // ===== Sessions 页面 =====
  Sessions: {
    end: 'system:admin:access',
  },

  // ===== Org Tools 页面 =====
  OrgTools: {
    create:  'org:update',
    delete:  'org:update',
    approve: 'org:update',
    reject:  'org:update',
  },
};
