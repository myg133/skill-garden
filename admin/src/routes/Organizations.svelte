<script>
  import { onMount } from 'svelte';
  import { Link } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission, hasSystemRole, permissionStore } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  const ACT = ACTIONS.Organizations;
  let organizations = [];
  let tenants = [];
  let tenantFilter = '';
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newOrgName = '';
  let newOrgTenantId = '';
  let creating = false;

  // 角色判断
  $: isSystemAdmin = hasSystemRole('super_admin') || hasSystemRole('marketplace_admin');
  $: isTenantAdmin = ($permissionStore.tenantRoles || []).some(t => t.role === 'tenant_admin');
  $: isOrgOnlyUser = !isSystemAdmin && !isTenantAdmin && ($permissionStore.orgRoles || []).length > 0;
  $: canManageTenants = isSystemAdmin || isTenantAdmin;

  function orgRoleColor(role) {
    const c = { owner: 'bg-amber-100 text-amber-700', admin: 'bg-blue-100 text-blue-700', reviewer: 'bg-purple-100 text-purple-700', developer: 'bg-emerald-100 text-emerald-700', member: 'bg-gray-100 text-gray-600' };
    return c[role] || 'bg-gray-100 text-gray-600';
  }

  onMount(async () => {
    if (canManageTenants) {
      await Promise.all([loadOrganizations(), loadTenants()]);
    } else {
      await loadOrganizations();
    }
  });

  async function loadTenants() {
    if (!canManageTenants) return;
    try {
      const res = await api.listTenants({ limit: 100 });
      tenants = res.data || [];
    } catch (e) {
      // 静默失败
    }
  }

  async function loadOrganizations() {
    loading = true;
    error = '';
    try {
      const params = { limit: 100 };
      if (tenantFilter) params.tenant_id = tenantFilter;
      const res = await api.listOrganizations(params);
      organizations = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  function getTenantName(org) {
    return org.tenant_name || (org.tenant_id ? org.tenant_id.substring(0, 8) : '-');
  }

  function handleClearFilter() {
    tenantFilter = '';
    loadOrganizations();
  }

  async function handleCreate() {
    if (!newOrgName.trim()) return;
    creating = true;
    try {
      await api.createOrganization({
        name: newOrgName,
        tenant_id: newOrgTenantId || null
      });
      newOrgName = '';
      newOrgTenantId = '';
      showCreateModal = false;
      addToast('Organization created', 'success');
      await loadOrganizations();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      creating = false;
    }
  }

  async function handleDelete(id) {
    if (!confirm('Delete this organization?')) return;
    try {
      await api.deleteOrganization(id);
      addToast('Organization deleted', 'success');
      await loadOrganizations();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }
</script>

<div class="p-8">
  <div class="page-header">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">Organizations</h1>
        <p class="text-gray-500 text-sm mt-1.5 font-medium">
          {#if isSystemAdmin}
            Manage all organizations across tenants
          {:else if isTenantAdmin}
            Manage organizations in your tenant
          {:else}
            Your organizations
          {/if}
        </p>
      </div>
      <div class="flex items-center gap-3">
        {#if canManageTenants}
        <select
          bind:value={tenantFilter}
          on:change={() => loadOrganizations()}
          aria-label="Filter by tenant"
          class="px-4 py-2.5 bg-white border border-gray-200 rounded-xl text-sm text-gray-700 focus:outline-none focus:ring-2 focus:ring-brand-500/30 cursor-pointer"
        >
          <option value="">All tenants</option>
          {#each tenants as tenant}
            <option value={tenant.id}>{tenant.name}</option>
          {/each}
        </select>
        {#if tenantFilter}
          <button
            on:click={handleClearFilter}
            class="px-3 py-2.5 text-gray-500 hover:text-gray-700 text-sm font-medium transition-colors"
          >
            Clear filter
          </button>
        {/if}
        {/if}
        {#if hasPermission(ACT.create)}
        <button
          on:click={() => showCreateModal = true}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
          New Organization
        </button>
        {/if}
      </div>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if organizations.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message="No organizations yet">
        {#if hasPermission(ACT.create)}
        <button
          on:click={() => showCreateModal = true}
          class="mt-4 btn-primary px-5 py-2.5 rounded-lg font-semibold text-sm"
        >
          Create your first organization
        </button>
        {/if}
      </EmptyState>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {#each organizations as org (org.id)}
        <Link
          to="/organizations/{org.id}"
          class="group bg-white rounded-xl border border-gray-200 p-5 card card-interactive block"
        >
          <div class="flex items-start gap-4">
            <div class="w-10 h-10 rounded-lg bg-blue-600 flex items-center justify-center font-bold text-white text-sm flex-shrink-0 group-hover:scale-105 transition-transform duration-300">
              {org.name[0]?.toUpperCase() || '?'}
            </div>
            <div class="flex-1 min-w-0">
              <h3 class="text-gray-900 font-semibold text-[15px] truncate mb-0.5 group-hover:text-blue-600 transition-colors">
                {org.name}
              </h3>
              <div class="flex items-center gap-2 mt-1">
                {#if org.my_role}
                  <span class="inline-flex items-center px-1.5 py-0.5 text-[10px] font-semibold rounded {orgRoleColor(org.my_role)}">{org.my_role}</span>
                {/if}
              </div>
            </div>
          </div>
          <div class="mt-4 pt-4 border-t border-gray-100 flex items-center justify-between">
            <div>
              <p class="text-gray-400 text-xs">Created {new Date(org.created_at).toLocaleDateString()}</p>
              <p class="text-gray-400 text-xs mt-0.5">{getTenantName(org)}</p>
            </div>
            <svg class="w-4 h-4 text-gray-300 group-hover:text-blue-600 group-hover:translate-x-0.5 transition-all" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
            </svg>
          </div>
        </Link>
      {/each}
    </div>
  {/if}
</div>

{#if showCreateModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <h2 class="text-lg font-bold text-gray-900 mb-5">Create Organization</h2>
    <div class="space-y-4">
      <div>
        <label for="org-name" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Name</label>
        <input
          id="org-name"
          type="text"
          bind:value={newOrgName}
          placeholder="Organization name"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>
      {#if canManageTenants}
      <div>
        <label for="org-tenant" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Tenant</label>
        <select
          id="org-tenant"
          bind:value={newOrgTenantId}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        >
          <option value="">No tenant</option>
          {#each tenants as tenant}
            <option value={tenant.id}>{tenant.name}</option>
          {/each}
        </select>
      </div>
      {/if}
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showCreateModal = false; newOrgName = ''; }}
          class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
        >
          Cancel
        </button>
        <button
          on:click={handleCreate}
          disabled={creating || !newOrgName.trim()}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {creating ? 'Creating...' : 'Create'}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}