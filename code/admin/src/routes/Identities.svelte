<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission, hasSystemRole } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  const ACT = ACTIONS.Identities;
  const VALID_SYSTEM_ROLES = ['super_admin', 'marketplace_admin', 'marketplace_reviewer'];

  let identities = [];
  let roleAssignments = {}; // identity_id -> Set of role_names
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newIdentity = { identity_type: 'user', name: '', email: '', external_id: '' };
  let creating = false;
  let roleModalIdentity = null; // { id, name }
  let roleModalLoading = false;
  let currentRoles = [];
  let pendingRoleAction = null; // { action: 'assign'|'revoke', role: string }

  const identityTypes = ['user', 'agent', 'system'];

  onMount(async () => {
    await loadAll();
  });

  async function loadAll() {
    loading = true;
    error = '';
    try {
      const [identRes, rolesRes] = await Promise.all([
        api.listIdentities({ limit: 100 }),
        api.listSystemRoleAssignments()
      ]);
      identities = identRes.data || [];
      // Build role map: identity_id -> Set of role_names
      roleAssignments = {};
      const data = rolesRes.data || rolesRes || [];
      if (Array.isArray(data)) {
        for (const a of data) {
          const iid = a.identity_id;
          if (!roleAssignments[iid]) roleAssignments[iid] = new Set();
          roleAssignments[iid].add(a.role_name);
        }
      }
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  function getRoles(identityId) {
    return roleAssignments[identityId] || new Set();
  }

  async function handleCreate() {
    if (!newIdentity.identity_type || !newIdentity.name.trim()) return;
    creating = true;
    try {
      await api.createIdentity(newIdentity);
      newIdentity = { identity_type: 'user', name: '', email: '', external_id: '' };
      showCreateModal = false;
      addToast('Identity created', 'success');
      await loadAll();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      creating = false;
    }
  }

  async function handleDelete(id) {
    if (!confirm('Delete this identity?')) return;
    try {
      await api.deleteIdentity(id);
      addToast('Identity deleted', 'success');
      await loadAll();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleToggleStatus(identity) {
    const isActive = identity.status === 'active';
    const action = isActive ? 'disable' : 'enable';
    if (!confirm(`${action === 'disable' ? '禁用' : '启用'} ${identity.name || identity.username}？`)) return;
    try {
      await api.updateIdentityStatus(identity.id, !isActive);
      addToast(`${identity.name || identity.username} ${action === 'disable' ? '已禁用' : '已启用'}`, 'success');
      identity.status = isActive ? 'suspended' : 'active';
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  function openRoleModal(identity) {
    roleModalIdentity = identity;
    currentRoles = [...getRoles(identity.id)];
    pendingRoleAction = null;
    roleModalLoading = false;
  }

  function closeRoleModal() {
    roleModalIdentity = null;
    currentRoles = [];
  }

  async function handleRoleAction(action, roleName) {
    if (!roleModalIdentity) return;
    roleModalLoading = true;
    pendingRoleAction = { action, role: roleName };
    try {
      if (action === 'assign') {
        await api.assignSystemRole(roleModalIdentity.id, roleName);
      } else {
        await api.revokeSystemRole(roleModalIdentity.id, roleName);
      }
      addToast(`${action === 'assign' ? 'Assigned' : 'Revoked'} ${roleName} for ${roleModalIdentity.name}`, 'success');
      // Refresh the full role list
      await loadAll();
      // Update local modal state
      currentRoles = [...getRoles(roleModalIdentity.id)];
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      roleModalLoading = false;
      pendingRoleAction = null;
    }
  }

  function getTypeColor(type) {
    switch (type) {
      case 'user': return 'bg-blue-100 text-blue-700';
      case 'agent': return 'bg-purple-100 text-purple-700';
      case 'system': return 'bg-amber-100 text-amber-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }

  function getRoleColor(role) {
    switch (role) {
      case 'super_admin': return 'bg-red-100 text-red-700';
      case 'marketplace_admin': return 'bg-emerald-100 text-emerald-700';
      case 'marketplace_reviewer': return 'bg-purple-100 text-purple-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }
</script>

<div class="p-8">
  <div class="page-header">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">Identities</h1>
        <p class="text-gray-500 text-sm mt-1.5 font-medium">Manage users, agents and system identities</p>
      </div>
      {#if hasPermission(ACT.create)}
      <button
        on:click={() => showCreateModal = true}
        class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        New Identity
      </button>
      {/if}
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if identities.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message="No identities yet">
        {#if hasPermission(ACT.create)}
        <button
          on:click={() => showCreateModal = true}
          class="mt-4 btn-primary px-5 py-2.5 rounded-lg font-semibold text-sm"
        >
          Create your first identity
        </button>
        {/if}
      </EmptyState>
    </div>
  {:else}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card overflow-hidden">
      <table class="w-full">
        <thead class="bg-gray-50 border-b border-gray-200">
          <tr>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Identity</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Type</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">System Roles</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Email</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Status</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Created</th>
            <th class="px-6 py-4 text-right text-xs font-semibold text-gray-500 uppercase tracking-wider">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each identities as identity (identity.id)}
            {@const roles = getRoles(identity.id)}
            <tr class="hover:bg-gray-50 transition-colors">
              <td class="px-6 py-4">
                <div class="flex items-center gap-3">
                  <div class="w-9 h-9 rounded-full bg-blue-600 flex items-center justify-center text-white text-xs font-bold">
                    {identity.name[0]?.toUpperCase() || '?'}
                  </div>
                  <div>
                    <p class="text-sm font-semibold text-gray-900">{identity.name}</p>
                    <p class="text-xs text-gray-400 font-mono">{identity.id}</p>
                  </div>
                </div>
              </td>
              <td class="px-6 py-4">
                <span class="px-2.5 py-1 rounded-full text-xs font-medium {getTypeColor(identity.identity_type)}">
                  {identity.identity_type}
                </span>
              </td>
              <td class="px-6 py-4">
                <div class="flex items-center gap-1.5">
                  {#if roles.size > 0}
                    {#each [...roles] as role}
                      <span class="px-2 py-0.5 rounded-md text-xs font-medium {getRoleColor(role)}">{role}</span>
                    {/each}
                  {:else}
                    <span class="text-xs text-gray-400">—</span>
                  {/if}
                  {#if hasSystemRole('super_admin')}
                    <button
                      on:click|stopPropagation={() => openRoleModal(identity)}
                      class="ml-1 p-1 rounded-md text-gray-400 hover:text-brand-600 hover:bg-brand-50 transition-all"
                      title="Manage system roles"
                    >
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"/></svg>
                    </button>
                  {/if}
                </div>
              </td>
              <td class="px-6 py-4 text-sm text-gray-600">{identity.email || '-'}</td>
              <td class="px-6 py-4">
                <span class="px-2.5 py-1 rounded-full text-xs font-medium {identity.status === 'active' ? 'bg-emerald-50 text-emerald-600' : 'bg-amber-50 text-amber-600'}">
                  {identity.status}
                </span>
              </td>
              <td class="px-6 py-4 text-sm text-gray-500">{new Date(identity.created_at).toLocaleDateString()}</td>
              <td class="px-6 py-4 text-right">
                <div class="flex items-center justify-end gap-1">
                {#if identity.identity_type === 'user' && hasPermission(ACT.edit)}
                <button
                  on:click={() => handleToggleStatus(identity)}
                  class="p-2 rounded-lg {identity.status === 'active' ? 'text-amber-500 hover:text-amber-600 hover:bg-amber-50' : 'text-emerald-500 hover:text-emerald-600 hover:bg-emerald-50'} transition-all"
                  title={identity.status === 'active' ? 'Disable' : 'Enable'}
                >
                  {#if identity.status === 'active'}
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/></svg>
                  {:else}
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                  {/if}
                </button>
                {/if}
                {#if hasPermission(ACT.delete)}
                <button
                  on:click={() => handleDelete(identity.id)}
                  class="p-2 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all"
                  title="Delete"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                </button>
                {/if}
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- Role Management Modal -->
{#if roleModalIdentity}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay" role="button" tabindex="-1" on:click|self={closeRoleModal} on:keydown|self={(e) => e.key === 'Escape' && closeRoleModal()}>
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <div class="flex items-center justify-between mb-5">
      <div>
        <h2 class="text-lg font-bold text-gray-900">System Roles</h2>
        <p class="text-sm text-gray-500 mt-0.5">{roleModalIdentity.name}</p>
      </div>
      <button on:click={closeRoleModal} class="p-1.5 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-gray-100">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
      </button>
    </div>

    <div class="space-y-3">
      <!-- Current roles -->
      <div>
        <p class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Current Roles</p>
        {#if currentRoles.length > 0}
          <div class="flex flex-wrap gap-2">
            {#each currentRoles as role}
              <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium {getRoleColor(role)}">
                {role}
                <button
                  on:click={() => handleRoleAction('revoke', role)}
                  disabled={roleModalLoading || role === 'super_admin'}
                  class="ml-1 hover:opacity-70 disabled:opacity-30"
                  title={role === 'super_admin' ? 'Cannot revoke your own super_admin' : 'Revoke role'}
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                </button>
              </span>
            {/each}
          </div>
        {:else}
          <p class="text-sm text-gray-400">No system roles assigned</p>
        {/if}
      </div>

      <!-- Available roles to assign -->
      <div>
        <p class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Available Roles</p>
        <div class="space-y-2">
          {#each VALID_SYSTEM_ROLES.filter(r => !currentRoles.includes(r)) as role}
            <div class="flex items-center justify-between p-3 rounded-lg border border-gray-200 hover:border-blue-200 hover:bg-blue-50/30 transition-all">
              <div>
                <p class="text-sm font-semibold text-gray-800">{role}</p>
                <p class="text-xs text-gray-500 mt-0.5">
                  {role === 'super_admin' ? 'Full system access, all admin routes' : role === 'marketplace_admin' ? 'Marketplace management, manage reviewers' : 'Review marketplace skills (approve/reject/delist)'}
                </p>
              </div>
              <button
                on:click={() => handleRoleAction('assign', role)}
                disabled={roleModalLoading}
                class="px-3 py-1.5 rounded-lg text-xs font-semibold bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 transition-all"
              >
                {roleModalLoading && pendingRoleAction?.role === role && pendingRoleAction?.action === 'assign' ? '...' : 'Assign'}
              </button>
            </div>
          {/each}
        </div>
        {#if VALID_SYSTEM_ROLES.every(r => currentRoles.includes(r))}
          <p class="text-sm text-gray-400 mt-2">All system roles already assigned</p>
        {/if}
      </div>
    </div>
  </div>
</div>
{/if}

{#if showCreateModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <h2 class="text-lg font-bold text-gray-900 mb-5">Create Identity</h2>
    <div class="space-y-4">
      <div>
        <label for="identity-type" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Type</label>
        <select
          id="identity-type"
          bind:value={newIdentity.identity_type}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        >
          {#each identityTypes as type}
            <option value={type}>{type}</option>
          {/each}
        </select>
      </div>
      <div>
        <label for="identity-name" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Name</label>
        <input
          id="identity-name"
          type="text"
          bind:value={newIdentity.name}
          placeholder="Identity name"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>
      <div>
        <label for="identity-email" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Email</label>
        <input
          id="identity-email"
          type="email"
          bind:value={newIdentity.email}
          placeholder="email@example.com"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>
      <div>
        <label for="identity-external-id" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">External ID (optional)</label>
        <input
          id="identity-external-id"
          type="text"
          bind:value={newIdentity.external_id}
          placeholder="External system ID"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showCreateModal = false; newIdentity = { identity_type: 'user', name: '', email: '', external_id: '' }; }}
          class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
        >
          Cancel
        </button>
        <button
          on:click={handleCreate}
          disabled={creating || !newIdentity.name.trim()}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {creating ? 'Creating...' : 'Create'}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}
