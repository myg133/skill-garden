const API_BASE = '/api/v1';

// ---------------------------------------------------------------------------
// ApiError — 结构化错误，前端组件可根据 code/status 做差异化展示
// ---------------------------------------------------------------------------
export class ApiError extends Error {
  /**
   * @param {number} status    HTTP 状态码
   * @param {string} code      后端返回的错误码（可选）
   * @param {string} message   面向用户的友好消息
   */
  constructor(status, code, message) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code || '';
  }
}

// ---------------------------------------------------------------------------
// 错误消息映射 — 将后端技术消息翻译为中文用户友好消息
// ---------------------------------------------------------------------------
const ERROR_MESSAGES = {
  // 通用
  unauthorized: '会话已过期，请重新登录',
  forbidden: '您没有执行此操作的权限',
  not_found: '请求的资源不存在',
  validation_error: '输入参数不合法',
  // Skill 权限
  skill_not_found: 'Skill 不存在或已被删除',
  permission_denied: '权限不足，请联系管理员',
  // 身份相关
  identity_not_found: '身份信息不存在',
  identity_id_missing: '身份信息缺失，请使用新版 API Key 重新认证',
  // API Key
  api_key_not_found: 'API Key 不存在',
  api_key_already_revoked: 'API Key 已被撤销',
  // 组织
  org_not_found: '组织不存在',
  not_org_member: '您不是该组织的成员',
  // Token
  token_expired: '凭证已过期，请重新登录',
  token_invalid: '凭证无效',
};

/**
 * 尝试匹配错误消息中的关键词，返回中文友好翻译
 */
function humanize(status, rawMessage) {
  if (!rawMessage) {
    // 根据 HTTP 状态码返回通用描述
    const defaults = {
      400: '请求参数有误',
      403: '您没有执行此操作的权限',
      404: '请求的资源不存在',
      409: '操作冲突，请检查后重试',
      429: '请求太频繁，请稍后再试',
      500: '服务器内部错误，请稍后重试',
      502: '网关错误，请稍后重试',
      503: '服务暂时不可用，请稍后重试',
    };
    return defaults[status] || `请求失败 (HTTP ${status})`;
  }

  // 按关键词匹配
  const lower = rawMessage.toLowerCase();
  for (const [key, msg] of Object.entries(ERROR_MESSAGES)) {
    if (lower.includes(key.replace(/_/g, ' ')) || lower.includes(key)) {
      return msg;
    }
  }

  // 没有匹配到，返回原始消息（可能是后端已经返回的合理中文消息）
  return rawMessage;
}

// ---------------------------------------------------------------------------
// 请求核心
// ---------------------------------------------------------------------------
async function request(path, options = {}) {
  const token = localStorage.getItem('admin_token');
  const headers = {
    'Content-Type': 'application/json',
    ...(token && { Authorization: `Bearer ${token}` }),
    ...options.headers
  };

  const res = await fetch(`${API_BASE}${path}`, { ...options, headers });

  // Token expired or unauthorized — save current path, clear token, redirect to login
  if (res.status === 401) {
    localStorage.removeItem('admin_token');
    // 保存当前页面路径，登录成功后回跳
    const currentPath = window.location.pathname + window.location.search;
    if (currentPath !== '/login') {
      localStorage.setItem('login_redirect', currentPath);
    }
    try {
      const { navigate } = await import('svelte-routing');
      navigate('/login', { replace: true });
    } catch {}
    throw new ApiError(401, 'unauthorized', '会话已过期，请重新登录');
  }

  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new ApiError(
      res.status,
      err.code || '',
      humanize(res.status, err.message)
    );
  }

  return res.json();
}

async function requestNoAuth(path, options = {}) {
  const headers = {
    'Content-Type': 'application/json',
    ...options.headers
  };

  const res = await fetch(`${API_BASE}${path}`, { ...options, headers });

  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new ApiError(
      res.status,
      err.code || '',
      humanize(res.status, err.message)
    );
  }

  return res.json();
}

async function requestUpload(path, formData) {
  const token = localStorage.getItem('admin_token');
  const headers = {};
  if (token) headers.Authorization = `Bearer ${token}`;

  const res = await fetch(`${API_BASE}${path}`, {
    method: 'POST',
    headers,
    body: formData,
  });

  // Token expired during upload — same redirect logic as request()
  if (res.status === 401) {
    localStorage.removeItem('admin_token');
    const currentPath = window.location.pathname + window.location.search;
    if (currentPath !== '/login') {
      localStorage.setItem('login_redirect', currentPath);
    }
    try {
      const { navigate } = await import('svelte-routing');
      navigate('/login', { replace: true });
    } catch {}
    throw new ApiError(401, 'unauthorized', '会话已过期，请重新登录');
  }

  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new ApiError(
      res.status,
      err.code || '',
      humanize(res.status, err.message)
    );
  }

  return res.json();
}

export const api = {
  // Auth (统一登录 — 含 admin/user 角色)
  adminLogin(username, password) {
    return requestNoAuth('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password })
    });
  },

  // Agent Auth (for agent clients, not admin UI)
  getToken(agentId, agentSecret) {
    return requestNoAuth('/auth/agent/token', {
      method: 'POST',
      body: JSON.stringify({ agent_id: agentId, agent_secret: agentSecret })
    });
  },

  // User Auth (self-service)
  userLogin(username, password) {
    return requestNoAuth('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password })
    });
  },

  userRegister(username, password, displayName, email) {
    return requestNoAuth('/auth/register', {
      method: 'POST',
      body: JSON.stringify({ 
        username, 
        password, 
        display_name: displayName || undefined, 
        email: email || undefined 
      })
    });
  },

  getMe() {
    return request('/users/me');
  },

  getMyPermissions() {
    return request('/users/me/permissions');
  },

  updateMe(data) {
    return request('/users/me', {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  },

  getUserOrgs() {
    return request('/users/me/orgs');
  },

  getUserByUsername(username) {
    return request(`/users/${username}`);
  },

  // Self-service API Keys (user)
  listMyApiKeys(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/api-keys${qs ? `?${qs}` : ''}`);
  },

  createMyApiKey(data) {
    return request('/api-keys', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  revokeMyApiKey(id) {
    return request(`/api-keys/${id}`, { method: 'DELETE' });
  },

  disableMyApiKey(id) {
    return request(`/api-keys/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ status: 'disabled' })
    });
  },

  enableMyApiKey(id) {
    return request(`/api-keys/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ status: 'active' })
    });
  },

  // Tenants
  listTenants(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/admin/tenants${qs ? `?${qs}` : ''}`);
  },

  createTenant(data) {
    return request('/admin/tenants', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  getTenant(id) {
    return request(`/admin/tenants/${id}`);
  },

  updateTenant(id, data) {
    return request(`/admin/tenants/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  },

  deleteTenant(id) {
    return request(`/admin/tenants/${id}`, { method: 'DELETE' });
  },

  // Identities
  listIdentities(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/admin/identities${qs ? `?${qs}` : ''}`);
  },

  createIdentity(data) {
    return request('/admin/identities', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  getIdentity(id) {
    return request(`/admin/identities/${id}`);
  },

  updateIdentity(id, data) {
    return request(`/admin/identities/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  },

  deleteIdentity(id) {
    return request(`/admin/identities/${id}`, { method: 'DELETE' });
  },

  // System Role Assignments
  assignSystemRole(identity_id, role_name) {
    return request('/admin/system-role-assignments', {
      method: 'POST',
      body: JSON.stringify({ identity_id, role_name })
    });
  },

  revokeSystemRole(identity_id, role_name) {
    return request('/admin/system-role-assignments', {
      method: 'DELETE',
      body: JSON.stringify({ identity_id, role_name })
    });
  },

  listSystemRoleAssignments(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/admin/system-role-assignments${qs ? `?${qs}` : ''}`);
  },

  getIdentitySystemRoles(id) {
    return request(`/admin/identities/${id}/system-roles`);
  },

  // Marketplace Reviewer Assignments (marketplace_admin manages reviewers)
  assignMarketplaceReviewer(identity_id) {
    return request('/admin/marketplace-reviewers', {
      method: 'POST',
      body: JSON.stringify({ identity_id })
    });
  },

  revokeMarketplaceReviewer(identity_id) {
    return request('/admin/marketplace-reviewers', {
      method: 'DELETE',
      body: JSON.stringify({ identity_id })
    });
  },

  listMarketplaceReviewers() {
    return request('/admin/marketplace-reviewers');
  },

  // Tenant Role Assignments (tenant_admin manages org admins)
  assignTenantRole(identity_id, tenant_id, role_name) {
    return request('/admin/tenant-role-assignments', {
      method: 'POST',
      body: JSON.stringify({ identity_id, tenant_id, role_name })
    });
  },

  revokeTenantRole(identity_id, tenant_id, role_name) {
    return request('/admin/tenant-role-assignments', {
      method: 'DELETE',
      body: JSON.stringify({ identity_id, tenant_id, role_name })
    });
  },

  listTenantRoleAssignments(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/admin/tenant-role-assignments${qs ? `?${qs}` : ''}`);
  },

  // Groups
  listGroups(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/admin/groups${qs ? `?${qs}` : ''}`);
  },

  createGroup(data) {
    return request('/admin/groups', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  getGroup(id) {
    return request(`/admin/groups/${id}`);
  },

  updateGroup(id, data) {
    return request(`/admin/groups/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  },

  deleteGroup(id) {
    return request(`/admin/groups/${id}`, { method: 'DELETE' });
  },

  // Roles
  listRoles() {
    return request('/admin/roles');
  },

  getRole(id) {
    return request(`/admin/roles/${id}`);
  },

  // API Keys
  listApiKeys(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/admin/api-keys${qs ? `?${qs}` : ''}`);
  },

  createApiKey(data) {
    return request('/admin/api-keys', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  deleteApiKey(id) {
    return request(`/admin/api-keys/${id}`, { method: 'DELETE' });
  },

  disableApiKey(id) {
    return request(`/admin/api-keys/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ status: 'disabled' })
    });
  },

  enableApiKey(id) {
    return request(`/admin/api-keys/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ status: 'active' })
    });
  },

  // Audit Entries
  listAuditEntries(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/admin/audit-entries${qs ? `?${qs}` : ''}`);
  },

  // Skills
  listSkills(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/skills${qs ? `?${qs}` : ''}`);
  },

  // My Skills (user-facing — only current user's skills)
  listMySkills() {
    return request('/my-skills');
  },

  // Submit skill for review
  submitSkillForReview(id, comment) {
    return request(`/skills/${id}/submit-review`, {
      method: 'POST',
      body: JSON.stringify({ comment: comment || null })
    });
  },

  // Publish approved skill
  publishSkill(id) {
    return request(`/skills/${id}/publish`, { method: 'POST' });
  },

  // Admin: unpublish a published skill (下架)
  adminUnpublishSkill(id) {
    return request(`/admin/skills/${id}/unpublish`, { method: 'POST' });
  },

  // Admin: republish a skill to marketplace (上架)
  adminPublishSkill(id) {
    return request(`/admin/skills/${id}/publish`, { method: 'POST' });
  },

  getSkill(id) {
    return request(`/skills/${id}`);
  },

  getSkillFiles(id) {
    return request(`/skills/${id}/files`);
  },

  getSkillFile(id, filePath) {
    return request(`/skills/${id}/files/${encodeURIComponent(filePath)}`);
  },

  getSkillStats(id) {
    return request(`/skills/${id}/stats`);
  },

  updateSkill(id, data) {
    return request(`/skills/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  },

  deleteSkill(id) {
    return request(`/skills/${id}`, { method: 'DELETE' });
  },

  uploadSkill(formData) {
    return requestUpload('/skills/upload', formData);
  },

  // Skill Upload Preview & Confirm
  previewSkillUpload(formData) {
    return requestUpload('/skills/upload/preview', formData);
  },

  getPreviewFile(previewId, filePath) {
    return request(`/skills/upload/preview/${previewId}/files/${encodeURIComponent(filePath)}`);
  },

  confirmSkillUpload(previewId, data = {}) {
    return request(`/skills/upload/preview/${previewId}/confirm`, {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  listAuditLogs(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/admin/audit-logs${qs ? `?${qs}` : ''}`);
  },

  approveSkill(id) {
    return request(`/skills/${id}/approve`, { method: 'POST' });
  },

  rejectSkill(id, reason) {
    return request(`/skills/${id}/reject`, {
      method: 'POST',
      body: JSON.stringify({ reason })
    });
  },

  // Organizations
  listOrganizations(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/organizations${qs ? `?${qs}` : ''}`);
  },

  getOrganization(id) {
    return request(`/organizations/${id}`);
  },

  createOrganization(data) {
    return request('/organizations', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  updateOrganization(id, data) {
    return request(`/organizations/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  },

  deleteOrganization(id) {
    return request(`/organizations/${id}`, { method: 'DELETE' });
  },

  // Sessions
  listSessions(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/sessions${qs ? `?${qs}` : ''}`);
  },

  getSession(id) {
    return request(`/sessions/${id}`);
  },

  endSession(id) {
    return request(`/sessions/${id}/end`, { method: 'POST' });
  },

  // Org Tools
  listOrgTools(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/org-tools${qs ? `?${qs}` : ''}`);
  },

  getOrgTool(id) {
    return request(`/org-tools/${id}`);
  },

  registerOrgTool(data) {
    return request('/org-tools', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  deleteOrgTool(id) {
    return request(`/org-tools/${id}`, { method: 'DELETE' });
  },

  approveOrgTool(id) {
    return request(`/org-tools/${id}/approve`, { method: 'POST' });
  },

  rejectOrgTool(id) {
    return request(`/org-tools/${id}/reject`, { method: 'POST' });
  },

  listApprovedTools(orgId) {
    return request(`/org-tools/${orgId}?approved_only=true`);
  },

  getAdminStatus() {
    return request('/admin/status');
  },

  // Organization Members
  listOrgMembers(slug) {
    return request(`/orgs/${slug}/members`);
  },

  listOrgMembersById(orgId) {
    return request(`/orgs/id/${orgId}/members`);
  },

  inviteOrgMember(slug, data) {
    return request(`/orgs/${slug}/members`, {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  inviteOrgMemberById(orgId, data) {
    return request(`/orgs/id/${orgId}/members/invite`, {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  updateOrgMember(slug, username, data) {
    return request(`/orgs/${slug}/members/${username}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  },

  updateOrgMemberById(orgId, username, data) {
    return request(`/orgs/id/${orgId}/members/${username}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  },

  removeOrgMember(slug, username) {
    return request(`/orgs/${slug}/members/${username}`, { method: 'DELETE' });
  },

  removeOrgMemberById(orgId, username) {
    return request(`/orgs/id/${orgId}/members/${username}`, { method: 'DELETE' });
  },

  // Organization Groups (by org slug)
  listOrgGroups(slug) {
    return request(`/orgs/${slug}/groups`);
  },

  createOrgGroup(slug, data) {
    return request(`/orgs/${slug}/groups`, {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  getOrgGroup(slug, groupId) {
    return request(`/orgs/${slug}/groups/${groupId}`);
  },

  updateOrgGroup(slug, groupId, data) {
    return request(`/orgs/${slug}/groups/${groupId}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  },

  deleteOrgGroup(slug, groupId) {
    return request(`/orgs/${slug}/groups/${groupId}`, { method: 'DELETE' });
  },

  // Organization Group Members (by org slug + group id)
  listOrgGroupMembers(slug, groupId) {
    return request(`/orgs/${slug}/groups/${groupId}/members`);
  },

  updateOrgGroupMember(slug, groupId, username, data) {
    return request(`/orgs/${slug}/groups/${groupId}/members/${username}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  },

  removeOrgGroupMember(slug, groupId, username) {
    return request(`/orgs/${slug}/groups/${groupId}/members/${username}`, { method: 'DELETE' });
  },

  // Group Members (by group id)
  listGroupMembers(groupId) {
    return request(`/groups/${groupId}/members`);
  },

  addGroupMember(groupId, data) {
    return request(`/groups/${groupId}/members`, {
      method: 'POST',
      body: JSON.stringify(data)
    });
  },

  updateGroupMember(groupId, agentId, data) {
    return request(`/groups/${groupId}/members/${agentId}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  },

  removeGroupMember(groupId, agentId) {
    return request(`/groups/${groupId}/members/${agentId}`, { method: 'DELETE' });
  },

  // Group Permissions
  listGroupDefaultPermissions() {
    return request(`/groups/default-permissions`);
  },

  listGroupPermissions(groupId) {
    return request(`/groups/${groupId}/permissions`);
  },

  updateGroupPermission(groupId, data) {
    return request(`/groups/${groupId}/permissions`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  },

  deleteGroupPermission(groupId, permissionCode, data) {
    return request(`/groups/${groupId}/permissions/${permissionCode}`, {
      method: 'DELETE',
      body: JSON.stringify(data),
    });
  },

  // Marketplace
  listMarketplaceSkills(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/marketplace${qs ? `?${qs}` : ''}`);
  },

  // Marketplace review & lifecycle
  submitToMarketplace(skillId) {
    return request(`/skills/${skillId}/submit-to-marketplace`, { method: 'POST' });
  },

  marketplaceReviewApprove(skillId) {
    return request(`/admin/marketplace/${skillId}/approve`, { method: 'POST' });
  },

  marketplaceReviewReject(skillId) {
    return request(`/admin/marketplace/${skillId}/reject`, { method: 'POST' });
  },

  marketplaceRelist(skillId) {
    return request(`/admin/marketplace/${skillId}/relist`, { method: 'POST' });
  },

  marketplaceDelist(skillId) {
    return request(`/admin/marketplace/${skillId}/delist`, { method: 'POST' });
  },

  // Marketplace delist request workflow
  requestMarketplaceDelist(skillId, reason) {
    return request(`/skills/${skillId}/request-delist`, {
      method: 'POST',
      body: JSON.stringify({ reason }),
    });
  },

  marketplaceApproveDelist(skillId) {
    return request(`/admin/marketplace/${skillId}/approve-delist`, { method: 'POST' });
  },

  marketplaceRejectDelist(skillId) {
    return request(`/admin/marketplace/${skillId}/reject-delist`, { method: 'POST' });
  },

  // Sandbox
  listSandboxes() {
    return request('/admin/sandboxes');
  },

  getSandboxHealth() {
    return request('/admin/sandboxes/health');
  },

  removeSandbox(key) {
    return request(`/admin/sandboxes/${encodeURIComponent(key)}`, { method: 'DELETE' });
  },
};
