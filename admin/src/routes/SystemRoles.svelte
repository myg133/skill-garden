<script>
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission, hasSystemRole } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  const ROLE_LABELS = {
    super_admin: 'Super Admin',
    marketplace_admin: 'Marketplace Admin',
    tenant_admin: 'Tenant Admin',
    marketplace_reviewer: 'Marketplace Reviewer',
  };

  const ASSIGNABLE_ROLES = [
    { value: 'marketplace_admin', label: 'Marketplace Admin' },
    { value: 'tenant_admin', label: 'Tenant Admin' },
  ];

  let assignments = [];
  let identities = [];
  let tenants = [];
  let loading = true;
  let error = '';

  // Add modal
  let showAddModal = false;
  let addForm = { email: '', role: 'marketplace_admin', tenant_id: '' };
  let adding = false;

  // role -> identity_name lookup
  let identityMap = {}; // identity_id -> { name, email }

  onMount(() => loadAll());

  async function loadAll() {
    loading = true;
    error = '';
    try {
      const [assignRes, identRes, tenantRes] = await Promise.all([
        api.listSystemRoleAssignments().catch(() => ({ data: [] })),
        api.listIdentities({ limit: 200 }).catch(() => ({ data: [] })),
        api.listTenants({ limit: 100 }).catch(() => ({ data: [] })),
      ]);

      const raw = assignRes.data || assignRes || [];
      assignments = Array.isArray(raw) ? raw : [];

      const identList = identRes.data || [];
      identities = Array.isArray(identList) ? identList : [];
      identityMap = {};
      for (const i of identities) {
        identityMap[i.id] = { name: i.name, email: i.email || '' };
      }

      const tList = tenantRes.data || [];
      tenants = Array.isArray(tList) ? tList : [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  function getIdentityInfo(identityId) {
    return identityMap[identityId] || { name: identityId?.substring(0, 8) || '?', email: '' };
  }

  function getRoleLabel(role) {
    return ROLE_LABELS[role] || role;
  }

  function getRoleColor(role) {
    switch (role) {
      case 'super_admin': return 'bg-red-100 text-red-700';
      case 'marketplace_admin': return 'bg-emerald-100 text-emerald-700';
      case 'tenant_admin': return 'bg-blue-100 text-blue-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }

  function openAddModal() {
    addForm = { email: '', role: 'marketplace_admin', tenant_id: '' };
    showAddModal = true;
  }

  async function handleAdd() {
    if (!addForm.email.trim()) return;
    adding = true;
    try {
      // Find identity by email
      const identity = identities.find(i =>
        i.email && i.email.toLowerCase() === addForm.email.trim().toLowerCase()
      );
      if (!identity) {
        addToast('User not found with this email. Please check and try again.', 'error');
        adding = false;
        return;
      }

      if (addForm.role === 'tenant_admin' && !addForm.tenant_id) {
        addToast('Please select a tenant for tenant_admin role.', 'warning');
        adding = false;
        return;
      }

      // Check if already assigned
      const existing = assignments.find(
        a => a.identity_id === identity.id && a.role_name === addForm.role
      );
      if (existing) {
        addToast(`${identity.name} already has the ${addForm.role} role.`, 'warning');
        adding = false;
        return;
      }

      if (addForm.role === 'tenant_admin') {
        await api.assignTenantRole(identity.id, addForm.tenant_id, addForm.role);
      } else {
        await api.assignSystemRole(identity.id, addForm.role);
      }

      addToast(`${getRoleLabel(addForm.role)} assigned to ${identity.name}`, 'success');
      showAddModal = false;
      await loadAll();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      adding = false;
    }
  }

  async function handleRemove(assignment) {
    const info = getIdentityInfo(assignment.identity_id);
    if (!confirm(`Remove "${getRoleLabel(assignment.role_name)}" from ${info.name}?`)) return;

    try {
      if (assignment.role_name === 'tenant_admin') {
        // Tenant roles handled differently - need tenant_id from assignment
        await api.revokeTenantRole(assignment.identity_id, assignment.tenant_id, assignment.role_name);
      } else {
        await api.revokeSystemRole(assignment.identity_id, assignment.role_name);
      }
      addToast(`Role revoked from ${info.name}`, 'success');
      await loadAll();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  // Separate assignments: system role vs tenant role
  $: systemAssignments = assignments.filter(a =>
    ['super_admin', 'marketplace_admin'].includes(a.role_name) || !a.tenant_id
  );
  $: tenantAssignments = assignments.filter(a =>
    a.role_name === 'tenant_admin' && a.tenant_id
  );
</script>

<div class="p-8">
  <div class="page-header flex items-center justify-between">
    <div>
      <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">{$_('systemRoles.title')}</h1>
      <p class="text-gray-500 text-sm mt-1.5 font-medium">
        {$_('systemRoles.description')}
      </p>
    </div>
    {#if hasSystemRole('super_admin')}
      <button
        on:click={openAddModal}
        class="px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2 bg-blue-600 text-white hover:bg-blue-700 transition-colors shadow-sm"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        {$_('systemRoles.addAdministrator')}
      </button>
    {/if}
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-red-50 border border-red-100 text-red-600 px-5 py-4 rounded-xl text-sm font-medium">{error}</div>
  {:else if systemAssignments.length === 0 && tenantAssignments.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message={$_('roles.noAssignments')} />
    </div>
  {:else}
    <!-- System Role Assignments -->
    {#if systemAssignments.length > 0}
    <div class="bg-white rounded-xl border border-gray-200 overflow-hidden shadow-card mb-6">
      <div class="px-6 py-3 bg-gray-50 border-b border-gray-200">
        <span class="text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.systemRolesSection')}</span>
      </div>
      <table class="w-full">
        <thead>
          <tr class="border-b border-gray-100 bg-gray-50/50">
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.user')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.email')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.role')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.assignedAt')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.actions')}</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each systemAssignments as a (a.identity_id + a.role_name)}
            {@const info = getIdentityInfo(a.identity_id)}
            <tr class="hover:bg-gray-50 transition-colors">
              <td class="px-6 py-4">
                <span class="text-sm font-semibold text-gray-900">{info.name}</span>
              </td>
              <td class="px-6 py-4 text-sm text-gray-500">{info.email || '-'}</td>
              <td class="px-6 py-4">
                <span class="px-2.5 py-1 rounded-full text-xs font-medium {getRoleColor(a.role_name)}">
                  {getRoleLabel(a.role_name)}
                </span>
              </td>
              <td class="px-6 py-4 text-sm text-gray-500">
                {a.assigned_at ? new Date(a.assigned_at).toLocaleDateString() : '-'}
              </td>
              <td class="px-6 py-4">
                <button
                  on:click={() => handleRemove(a)}
                  class="px-3 py-1.5 rounded-lg text-xs font-semibold text-red-600 hover:bg-red-50 border border-red-200 transition-colors"
                >
                  {$_('systemRoles.remove')}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {/if}

    <!-- Tenant Role Assignments -->
    {#if tenantAssignments.length > 0}
    <div class="bg-white rounded-xl border border-gray-200 overflow-hidden shadow-card">
      <div class="px-6 py-3 bg-gray-50 border-b border-gray-200">
        <span class="text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.tenantRolesSection')}</span>
      </div>
      <table class="w-full">
        <thead>
          <tr class="border-b border-gray-100 bg-gray-50/50">
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.user')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.email')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.role')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.tenant')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.assignedAt')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('systemRoles.actions')}</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each tenantAssignments as a (a.identity_id + a.role_name + (a.tenant_id || ''))}
            {@const info = getIdentityInfo(a.identity_id)}
            {@const tenantName = tenants.find(t => t.id === a.tenant_id)?.name || a.tenant_id?.substring(0, 8) || '-'}
            <tr class="hover:bg-gray-50 transition-colors">
              <td class="px-6 py-4">
                <span class="text-sm font-semibold text-gray-900">{info.name}</span>
              </td>
              <td class="px-6 py-4 text-sm text-gray-500">{info.email || '-'}</td>
              <td class="px-6 py-4">
                <span class="px-2.5 py-1 rounded-full text-xs font-medium {getRoleColor(a.role_name)}">
                  {getRoleLabel(a.role_name)}
                </span>
              </td>
              <td class="px-6 py-4 text-sm text-gray-600">{tenantName}</td>
              <td class="px-6 py-4 text-sm text-gray-500">
                {a.assigned_at ? new Date(a.assigned_at).toLocaleDateString() : '-'}
              </td>
              <td class="px-6 py-4">
                <button
                  on:click={() => handleRemove(a)}
                  class="px-3 py-1.5 rounded-lg text-xs font-semibold text-red-600 hover:bg-red-50 border border-red-200 transition-colors"
                >
                  {$_('systemRoles.remove')}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {/if}
  {/if}
</div>

<!-- Add Administrator Modal -->
{#if showAddModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay" role="button" tabindex="-1" on:click|self={() => showAddModal = false} on:keydown|self={(e) => e.key === 'Escape' && (showAddModal = false)}>
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <div class="flex items-center justify-between mb-5">
      <h2 class="text-lg font-bold text-gray-900">{$_('systemRoles.addAdministrator')}</h2>
      <button on:click={() => showAddModal = false} class="p-1.5 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-gray-100">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
      </button>
    </div>

    <div class="space-y-4">
      <div>
        <label for="add-email" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('systemRoles.userEmail')} *</label>
        <input
          id="add-email"
          type="email"
          bind:value={addForm.email}
          placeholder={$_('systemRoles.enterEmail')}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>

      <div>
        <label for="add-role-select" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Role *</label>
        <select
          id="add-role-select"
          bind:value={addForm.role}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        >
          {#each ASSIGNABLE_ROLES as role}
            <option value={role.value}>{role.label}</option>
          {/each}
        </select>
      </div>

      {#if addForm.role === 'tenant_admin'}
      <div>
        <label for="add-tenant-select" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('systemRoles.tenant')} *</label>
        <select
          id="add-tenant-select"
          bind:value={addForm.tenant_id}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        >
          <option value="">{$_('systemRoles.selectTenant')}</option>
          {#each tenants as t (t.id)}
            <option value={t.id}>{t.name}</option>
          {/each}
        </select>
      </div>
      {/if}
    </div>

    <div class="flex justify-end gap-3 pt-5">
      <button
        on:click={() => showAddModal = false}
        class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
      >
        {$_('common.cancel')}
      </button>
      <button
        on:click={handleAdd}
        disabled={adding || !addForm.email.trim() || (addForm.role === 'tenant_admin' && !addForm.tenant_id)}
        class="px-5 py-2.5 rounded-xl font-semibold text-sm bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-sm"
      >
        {adding ? $_('common.loading') : $_('common.confirm')}
      </button>
    </div>
  </div>
</div>
{/if}
