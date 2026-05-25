# Admin Dashboard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Svelte + Vite + Tailwind admin dashboard for reviewing Skills and viewing audit logs

**Architecture:** SPA with client-side routing. Svelte handles components and state. API calls to existing Rust backend on port 8080.

**Tech Stack:** Svelte 4, Vite 5, Tailwind CSS (CDN), svelte-routing

---

## File Structure

```
admin/
├── index.html
├── package.json
├── vite.config.js
├── src/
│   ├── main.js
│   ├── App.svelte
│   ├── app.css              # Tailwind directives
│   ├── lib/
│   │   └── api.js           # API client functions
│   ├── stores/
│   │   └── app.js           # Svelte stores
│   ├── components/
│   │   ├── Badge.svelte
│   │   ├── Nav.svelte
│   │   ├── SkillRow.svelte
│   │   ├── ReviewActions.svelte
│   │   ├── RejectModal.svelte
│   │   ├── AuditTable.svelte
│   │   ├── StatCard.svelte
│   │   ├── EmptyState.svelte
│   │   ├── LoadingSpinner.svelte
│   │   └── Toast.svelte
│   └── routes/
│       ├── Home.svelte      # Redirects to /review
│       ├── Review.svelte
│       ├── SkillDetail.svelte
│       ├── AuditLogs.svelte
│       └── Stats.svelte
```

---

## Task 1: Project Setup

**Files:**
- Create: `admin/package.json`
- Create: `admin/vite.config.js`
- Create: `admin/index.html`
- Create: `admin/src/main.js`
- Create: `admin/src/app.css`

- [ ] **Step 1: Create admin/package.json**

```json
{
  "name": "aion-hive-admin",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^3.1.0",
    "svelte": "^4.2.12",
    "vite": "^5.2.0"
  },
  "dependencies": {
    "svelte-routing": "^2.13.0"
  }
}
```

- [ ] **Step 2: Create admin/vite.config.js**

```javascript
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true
      }
    }
  }
});
```

- [ ] **Step 3: Create admin/index.html**

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>AionHive Admin</title>
    <script src="https://cdn.tailwindcss.com"></script>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.js"></script>
  </body>
</html>
```

- [ ] **Step 4: Create admin/src/main.js**

```javascript
import App from './App.svelte';

const app = new App({
  target: document.getElementById('app')
});

export default app;
```

- [ ] **Step 5: Create admin/src/app.css**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

- [ ] **Step 6: Commit**

```bash
git add admin/
git commit -m "feat(admin): scaffold Svelte + Vite project"
```

---

## Task 2: API Client

**Files:**
- Create: `admin/src/lib/api.js`

- [ ] **Step 1: Create admin/src/lib/api.js**

```javascript
const API_BASE = '/api';

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

export const api = {
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
  }
};
```

- [ ] **Step 2: Commit**

```bash
git add admin/src/lib/api.js
git commit -m "feat(admin): add API client"
```

---

## Task 3: Svelte Stores

**Files:**
- Create: `admin/src/stores/app.js`

- [ ] **Step 1: Create admin/src/stores/app.js**

```javascript
import { writable } from 'svelte/store';

export const toasts = writable([]);

export function addToast(message, type = 'error') {
  const id = Date.now();
  toasts.update(t => [...t, { id, message, type }]);
  setTimeout(() => {
    toasts.update(t => t.filter(toast => toast.id !== id));
  }, 4000);
}
```

- [ ] **Step 2: Commit**

```bash
git add admin/src/stores/app.js
git commit -m "feat(admin): add Svelte stores"
```

---

## Task 4: Base Components

**Files:**
- Create: `admin/src/components/Badge.svelte`
- Create: `admin/src/components/Nav.svelte`
- Create: `admin/src/components/EmptyState.svelte`
- Create: `admin/src/components/LoadingSpinner.svelte`
- Create: `admin/src/components/Toast.svelte`

- [ ] **Step 1: Create admin/src/components/Badge.svelte**

```svelte
<script>
  export let status = 'draft';

  const colors = {
    pending_review: 'bg-yellow-100 text-yellow-800',
    published: 'bg-green-100 text-green-800',
    rejected: 'bg-red-100 text-red-800',
    draft: 'bg-gray-100 text-gray-800'
  };

  const labels = {
    pending_review: 'Pending',
    published: 'Published',
    rejected: 'Rejected',
    draft: 'Draft'
  };
</script>

<span class="px-2 py-1 text-xs font-medium rounded-full {colors[status] || colors.draft}">
  {labels[status] || status}
</span>
```

- [ ] **Step 2: Create admin/src/components/Nav.svelte**

```svelte
<script>
  import { Link } from 'svelte-routing';
</script>

<nav class="bg-white border-b border-gray-200 px-6 py-3">
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-8">
      <Link to="/" class="text-lg font-semibold text-gray-900">AionHive Admin</Link>
      <div class="flex gap-6">
        <Link to="/review" class="text-gray-600 hover:text-gray-900">Review Queue</Link>
        <Link to="/audit" class="text-gray-600 hover:text-gray-900">Audit Logs</Link>
        <Link to="/stats" class="text-gray-600 hover:text-gray-900">Stats</Link>
      </div>
    </div>
  </div>
</nav>
```

- [ ] **Step 3: Create admin/src/components/EmptyState.svelte**

```svelte
<script>
  export let message = 'No data';
</script>

<div class="flex flex-col items-center justify-center py-12 text-gray-500">
  <svg class="w-12 h-12 mb-4 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
      d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
  </svg>
  <p>{message}</p>
</div>
```

- [ ] **Step 4: Create admin/src/components/LoadingSpinner.svelte**

```svelte
<div class="flex justify-center py-8">
  <div class="w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
</div>
```

- [ ] **Step 5: Create admin/src/components/Toast.svelte**

```svelte
<script>
  import { toasts } from '../stores/app.js';
</script>

<div class="fixed top-4 right-4 z-50 flex flex-col gap-2">
  {#each $toasts as toast (toast.id)}
    <div class="px-4 py-3 rounded-lg shadow-lg text-white {
      toast.type === 'success' ? 'bg-green-500' : 'bg-red-500'
    }">
      {toast.message}
    </div>
  {/each}
</div>
```

- [ ] **Step 6: Commit**

```bash
git add admin/src/components/
git commit -m "feat(admin): add base components (Badge, Nav, EmptyState, LoadingSpinner, Toast)"
```

---

## Task 5: Review Queue Page

**Files:**
- Create: `admin/src/routes/Review.svelte`
- Create: `admin/src/components/SkillRow.svelte`
- Create: `admin/src/components/ReviewActions.svelte`
- Create: `admin/src/components/RejectModal.svelte`

- [ ] **Step 1: Create admin/src/components/SkillRow.svelte**

```svelte
<script>
  import Badge from './Badge.svelte';
  import ReviewActions from './ReviewActions.svelte';
  import { Link } from 'svelte-routing';

  export let skill;
</script>

<tr class="border-b border-gray-100 hover:bg-gray-50">
  <td class="px-4 py-3">
    <Link to="/skills/{skill.id}" class="text-blue-600 hover:text-blue-800 font-medium">
      {skill.name}
    </Link>
  </td>
  <td class="px-4 py-3 text-gray-600 text-sm">{skill.agent_id}</td>
  <td class="px-4 py-3">
    <div class="flex gap-1 flex-wrap">
      {#each (skill.tags || []).slice(0, 3) as tag}
        <span class="px-2 py-0.5 bg-gray-100 text-gray-600 text-xs rounded">{tag}</span>
      {/each}
    </div>
  </td>
  <td class="px-4 py-3 text-gray-600 text-sm">{new Date(skill.created_at).toLocaleDateString()}</td>
  <td class="px-4 py-3">
    <ReviewActions {skill} />
  </td>
</tr>
```

- [ ] **Step 2: Create admin/src/components/ReviewActions.svelte**

```svelte
<script>
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { navigate } from 'svelte-routing';

  export let skill;

  let loading = false;

  async function handleApprove() {
    loading = true;
    try {
      await api.approveSkill(skill.id);
      addToast(`${skill.name} approved`, 'success');
      navigate('/review', { replace: true });
    } catch (e) {
      addToast(e.message);
    } finally {
      loading = false;
    }
  }

  async function handleReject(reason) {
    loading = true;
    try {
      await api.rejectSkill(skill.id, reason);
      addToast(`${skill.name} rejected`, 'success');
      navigate('/review', { replace: true });
    } catch (e) {
      addToast(e.message);
    } finally {
      loading = false;
    }
  }
</script>

<div class="flex gap-2">
  <button
    on:click={handleApprove}
    disabled={loading}
    class="px-3 py-1 text-sm font-medium text-white bg-green-600 rounded hover:bg-green-700 disabled:opacity-50">
    Approve
  </button>
  <button
    on:click={() => handleReject('Not approved')}
    disabled={loading}
    class="px-3 py-1 text-sm font-medium text-red-600 border border-red-600 rounded hover:bg-red-50 disabled:opacity-50">
    Reject
  </button>
</div>
```

- [ ] **Step 3: Create admin/src/components/RejectModal.svelte**

```svelte
<script>
  import { createEventDispatcher } from 'svelte';

  export let show = false;
  export let skillName = '';

  const dispatch = createEventDispatcher();

  let reason = '';
  let error = '';

  function handleSubmit() {
    if (reason.length < 10) {
      error = 'Reason must be at least 10 characters';
      return;
    }
    dispatch('submit', reason);
  }
</script>

{#if show}
<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
  <div class="bg-white rounded-lg p-6 w-full max-w-md">
    <h3 class="text-lg font-semibold mb-4">Reject "{skillName}"</h3>
    <textarea
      bind:value={reason}
      placeholder="Rejection reason (min 10 characters)"
      rows="4"
      class="w-full px-3 py-2 border border-gray-300 rounded mb-2"
    ></textarea>
    {#if error}
      <p class="text-red-500 text-sm mb-2">{error}</p>
    {/if}
    <div class="flex justify-end gap-2">
      <button
        on:click={() => dispatch('cancel')}
        class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded">
        Cancel
      </button>
      <button
        on:click={handleSubmit}
        class="px-4 py-2 text-white bg-red-600 rounded hover:bg-red-700">
        Reject
      </button>
    </div>
  </div>
</div>
{/if}
```

- [ ] **Step 4: Create admin/src/routes/Review.svelte**

```svelte
<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import SkillRow from '../components/SkillRow.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import Badge from '../components/Badge.svelte';

  let skills = [];
  let loading = true;
  let error = '';

  onMount(async () => {
    try {
      const res = await api.listSkills({ status: 'pending_review', limit: 50 });
      skills = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-6">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-2xl font-semibold">Review Queue</h1>
    <Badge status="pending_review" />
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="text-red-500">{error}</div>
  {:else if skills.length === 0}
    <EmptyState message="No pending skills to review" />
  {:else}
    <div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
      <table class="w-full">
        <thead class="bg-gray-50">
          <tr>
            <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Name</th>
            <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Agent</th>
            <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Tags</th>
            <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Created</th>
            <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each skills as skill (skill.id)}
            <SkillRow {skill} />
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
```

- [ ] **Step 5: Commit**

```bash
git add admin/src/routes/Review.svelte admin/src/components/SkillRow.svelte admin/src/components/ReviewActions.svelte admin/src/components/RejectModal.svelte
git commit -m "feat(admin): add Review Queue page"
```

---

## Task 6: Skill Detail Page

**Files:**
- Create: `admin/src/routes/SkillDetail.svelte`
- Create: `admin/src/components/StatCard.svelte`

- [ ] **Step 1: Create admin/src/components/StatCard.svelte**

```svelte
<script>
  export let title = '';
  export let value = 0;
  export let subtitle = '';
</script>

<div class="bg-white rounded-lg border border-gray-200 p-4">
  <p class="text-sm text-gray-500 mb-1">{title}</p>
  <p class="text-2xl font-semibold text-gray-900">{value}</p>
  {#if subtitle}
    <p class="text-xs text-gray-400 mt-1">{subtitle}</p>
  {/if}
</div>
```

- [ ] **Step 2: Create admin/src/routes/SkillDetail.svelte**

```svelte
<script>
  import { onMount } from 'svelte';
  import { params } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import Badge from '../components/Badge.svelte';
  import StatCard from '../components/StatCard.svelte';
  import ReviewActions from '../components/ReviewActions.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  let skill = null;
  let stats = null;
  let loading = true;
  let error = '';

  onMount(async () => {
    try {
      const [skillRes, statsRes] = await Promise.all([
        api.getSkill($params.id),
        api.getSkillStats($params.id)
      ]);
      skill = skillRes.data;
      stats = statsRes.data;
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-6">
  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="text-red-500">{error}</div>
  {:else if skill}
    <div class="flex items-center justify-between mb-6">
      <div class="flex items-center gap-4">
        <h1 class="text-2xl font-semibold">{skill.name}</h1>
        <Badge status={skill.status} />
      </div>
      <ReviewActions {skill} />
    </div>

    <div class="grid grid-cols-4 gap-4 mb-6">
      <StatCard title="Installs" value={stats?.install_count || 0} />
      <StatCard title="Evaluations" value={stats?.evaluation_count || 0} />
      <StatCard title="Success Rate" value="{((stats?.success_rate || 0) * 100).toFixed(1)}%" />
      <StatCard title="Confidence" value={(stats?.confidence || 0).toFixed(2)} />
    </div>

    <div class="bg-white rounded-lg border border-gray-200 p-6 mb-6">
      <h2 class="text-lg font-medium mb-4">Description</h2>
      <p class="text-gray-700">{skill.description}</p>
    </div>

    <div class="bg-white rounded-lg border border-gray-200 p-6 mb-6">
      <h2 class="text-lg font-medium mb-4">Tags</h2>
      <div class="flex gap-2 flex-wrap">
        {#each skill.tags || [] as tag}
          <span class="px-3 py-1 bg-gray-100 text-gray-700 rounded-full text-sm">{tag}</span>
        {/each}
      </div>
    </div>

    <div class="bg-white rounded-lg border border-gray-200 p-6">
      <h2 class="text-lg font-medium mb-4">Content Preview</h2>
      <pre class="whitespace-pre-wrap text-sm text-gray-600 bg-gray-50 p-4 rounded overflow-auto max-h-64">{skill.content?.slice(0, 1000)}{skill.content?.length > 1000 ? '...' : ''}</pre>
    </div>
  {/if}
</div>
```

- [ ] **Step 3: Commit**

```bash
git add admin/src/routes/SkillDetail.svelte admin/src/components/StatCard.svelte
git commit -m "feat(admin): add Skill Detail page"
```

---

## Task 7: Audit Logs Page

**Files:**
- Create: `admin/src/routes/AuditLogs.svelte`
- Create: `admin/src/components/AuditTable.svelte`

- [ ] **Step 1: Create admin/src/components/AuditTable.svelte**

```svelte
<script>
  export let logs = [];
</script>

<table class="w-full">
  <thead class="bg-gray-50">
    <tr>
      <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Timestamp</th>
      <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Agent</th>
      <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Action</th>
      <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Resource</th>
    </tr>
  </thead>
  <tbody>
    {#each logs as log}
      <tr class="border-b border-gray-100 hover:bg-gray-50">
        <td class="px-4 py-3 text-sm text-gray-600">{new Date(log.created_at).toLocaleString()}</td>
        <td class="px-4 py-3 text-sm text-gray-600">{log.agent_id || '-'}</td>
        <td class="px-4 py-3 text-sm">
          <span class="px-2 py-1 bg-blue-100 text-blue-700 rounded text-xs">{log.action}</span>
        </td>
        <td class="px-4 py-3 text-sm text-gray-600">{log.resource_type}: {log.resource_id}</td>
      </tr>
    {/each}
  </tbody>
</table>
```

- [ ] **Step 2: Create admin/src/routes/AuditLogs.svelte**

```svelte
<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import AuditTable from '../components/AuditTable.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  let logs = [];
  let loading = true;
  let error = '';

  let filters = {
    action: '',
    agent_id: '',
    from_date: '',
    to_date: ''
  };

  async function fetchLogs() {
    loading = true;
    try {
      const params = {};
      if (filters.action) params.action = filters.action;
      if (filters.agent_id) params.agent_id = filters.agent_id;
      if (filters.from_date) params.from = filters.from_date;
      if (filters.to_date) params.to = filters.to_date;
      params.limit = 50;

      const res = await api.listAuditLogs(params);
      logs = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  function resetFilters() {
    filters = { action: '', agent_id: '', from_date: '', to_date: '' };
    fetchLogs();
  }

  onMount(fetchLogs);
</script>

<div class="p-6">
  <h1 class="text-2xl font-semibold mb-6">Audit Logs</h1>

  <div class="bg-white rounded-lg border border-gray-200 p-4 mb-6">
    <div class="grid grid-cols-5 gap-4">
      <div>
        <label class="block text-sm text-gray-600 mb-1">Action</label>
        <select bind:value={filters.action} class="w-full px-3 py-2 border border-gray-300 rounded">
          <option value="">All</option>
          <option value="skill_create">skill_create</option>
          <option value="skill_approve">skill_approve</option>
          <option value="skill_reject">skill_reject</option>
          <option value="skill_update">skill_update</option>
          <option value="skill_delete">skill_delete</option>
        </select>
      </div>
      <div>
        <label class="block text-sm text-gray-600 mb-1">Agent ID</label>
        <input
          bind:value={filters.agent_id}
          placeholder="Filter by agent"
          class="w-full px-3 py-2 border border-gray-300 rounded"
        />
      </div>
      <div>
        <label class="block text-sm text-gray-600 mb-1">From Date</label>
        <input
          type="date"
          bind:value={filters.from_date}
          class="w-full px-3 py-2 border border-gray-300 rounded"
        />
      </div>
      <div>
        <label class="block text-sm text-gray-600 mb-1">To Date</label>
        <input
          type="date"
          bind:value={filters.to_date}
          class="w-full px-3 py-2 border border-gray-300 rounded"
        />
      </div>
      <div class="flex items-end gap-2">
        <button
          on:click={fetchLogs}
          class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
          Search
        </button>
        <button
          on:click={resetFilters}
          class="px-4 py-2 text-gray-600 border border-gray-300 rounded hover:bg-gray-50">
          Reset
        </button>
      </div>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="text-red-500">{error}</div>
  {:else if logs.length === 0}
    <EmptyState message="No audit logs match your filters" />
  {:else}
    <div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
      <AuditTable {logs} />
    </div>
  {/if}
</div>
```

- [ ] **Step 3: Commit**

```bash
git add admin/src/routes/AuditLogs.svelte admin/src/components/AuditTable.svelte
git commit -m "feat(admin): add Audit Logs page"
```

---

## Task 8: Stats Dashboard Page

**Files:**
- Create: `admin/src/routes/Stats.svelte`

- [ ] **Step 1: Create admin/src/routes/Stats.svelte**

```svelte
<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import StatCard from '../components/StatCard.svelte';
  import AuditTable from '../components/AuditTable.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  let stats = null;
  let recentLogs = [];
  let loading = true;
  let error = '';

  onMount(async () => {
    try {
      const [pendingRes, allRes, logsRes] = await Promise.all([
        api.listSkills({ status: 'pending_review', limit: 1 }),
        api.listSkills({ limit: 1 }),
        api.listAuditLogs({ limit: 10 })
      ]);

      const pending = pendingRes.total || 0;
      const published = allRes.total || 0;

      stats = {
        total: published,
        pending,
        published: published - pending
      };

      recentLogs = logsRes.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-6">
  <h1 class="text-2xl font-semibold mb-6">Stats Dashboard</h1>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="text-red-500">{error}</div>
  {:else if stats}
    <div class="grid grid-cols-3 gap-6 mb-8">
      <StatCard title="Total Skills" value={stats.total} />
      <StatCard title="Pending Review" value={stats.pending} subtitle="Needs attention" />
      <StatCard title="Published" value={stats.published} subtitle="Live skills" />
    </div>

    <div class="bg-white rounded-lg border border-gray-200">
      <div class="px-4 py-3 border-b border-gray-200">
        <h2 class="font-medium">Recent Activity</h2>
      </div>
      {#if recentLogs.length > 0}
        <AuditTable logs={recentLogs} />
      {:else}
        <div class="p-8 text-center text-gray-500">No recent activity</div>
      {/if}
    </div>
  {/if}
</div>
```

- [ ] **Step 2: Commit**

```bash
git add admin/src/routes/Stats.svelte
git commit -m "feat(admin): add Stats Dashboard page"
```

---

## Task 9: App Router and Home Redirect

**Files:**
- Create: `admin/src/App.svelte`
- Create: `admin/src/routes/Home.svelte`

- [ ] **Step 1: Create admin/src/routes/Home.svelte**

```svelte
<script>
  import { navigate } from 'svelte-routing';
  import { onMount } from 'svelte';

  onMount(() => {
    navigate('/review', { replace: true });
  });
</script>
```

- [ ] **Step 2: Create admin/src/App.svelte**

```svelte
<script>
  import { Router, Route } from 'svelte-routing';
  import Nav from './components/Nav.svelte';
  import Toast from './components/Toast.svelte';
  import Home from './routes/Home.svelte';
  import Review from './routes/Review.svelte';
  import SkillDetail from './routes/SkillDetail.svelte';
  import AuditLogs from './routes/AuditLogs.svelte';
  import Stats from './routes/Stats.svelte';
</script>

<div class="min-h-screen bg-gray-50">
  <Nav />
  <main>
    <Route path="/" component={Home} />
    <Route path="/review" component={Review} />
    <Route path="/skills/:id" component={SkillDetail} />
    <Route path="/audit" component={AuditLogs} />
    <Route path="/stats" component={Stats} />
  </main>
  <Toast />
</div>
```

- [ ] **Step 3: Commit**

```bash
git add admin/src/App.svelte admin/src/routes/Home.svelte
git commit -m "feat(admin): add App router and home redirect"
```

---

## Task 10: Verify and Test

- [ ] **Step 1: Install dependencies and build**

Run in `admin/` directory:
```bash
cd admin && npm install && npm run build
```

Expected: Build succeeds without errors

- [ ] **Step 2: Verify dev server starts**

```bash
cd admin && npm run dev
```

Expected: Vite dev server starts on port 5173

- [ ] **Step 3: Final commit**

```bash
git add -A && git commit -m "feat(admin): complete admin dashboard v0.1"
```

---

## Self-Review Checklist

- [ ] All 4 pages implemented: Review, SkillDetail, AuditLogs, Stats
- [ ] All base components created: Badge, Nav, EmptyState, LoadingSpinner, Toast, StatCard
- [ ] API client connects to backend at /api
- [ ] Routing works with svelte-routing
- [ ] Build passes
