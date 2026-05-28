const API_BASE = '/api/v1';

async function request(path, options = {}) {
  const token = localStorage.getItem('admin_token');
  const headers = {
    'Content-Type': 'application/json',
    ...(token && { Authorization: `Bearer ${token}` }),
    ...options.headers
  };

  const res = await fetch(`${API_BASE}${path}`, { ...options, headers });

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

  // Skills
  listSkills(params = {}) {
    const qs = new URLSearchParams(params).toString();
    return request(`/skills${qs ? `?${qs}` : ''}`);
  },

  getSkill(id) {
    return request(`/skills/${id}`);
  },

  getSkillStats(id) {
    return request(`/skills/${id}/stats`);
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

  listApprovedTools(orgId) {
    return request(`/org-tools/${orgId}?approved_only=true`);
  },

  getAdminStatus() {
    return request('/admin/status');
  }
};
