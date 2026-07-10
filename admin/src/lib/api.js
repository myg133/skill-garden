const API_BASE = '/api/v1';

async function request(path, options = {}) {
  const token = localStorage.getItem('admin_token');
  const headers = {
    'Content-Type': 'application/json',
    ...(token && { Authorization: `Bearer ${token}` }),
    ...options.headers
  };

  const res = await fetch(`${API_BASE}${path}`, { ...options, headers });

  // Token expired or unauthorized — clear token and redirect to login
  if (res.status === 401) {
    localStorage.removeItem('admin_token');
    // Use dynamic import to avoid circular dependency
    try {
      const { navigate } = await import('svelte-routing');
      navigate('/login', { replace: true });
    } catch {}
    throw new Error('Session expired. Please log in again.');
  }

  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: 'Request failed' }));
    throw new Error(err.message || `HTTP ${res.status}`);
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
    const err = await res.json().catch(() => ({ message: 'Request failed' }));
    throw new Error(err.message || `HTTP ${res.status}`);
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

  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: 'Upload failed' }));
    throw new Error(err.message || `HTTP ${res.status}`);
  }

  return res.json();
}

export const api = {
  // Admin Auth
  adminLogin(username, password) {
    return requestNoAuth('/admin/login', {
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
    return request(`/admin/skills/${id}/approve`, { method: 'POST' });
  },

  rejectSkill(id, reason) {
    return request(`/admin/skills/${id}/reject`, {
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
