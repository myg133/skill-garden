<script>
  import { onMount } from 'svelte';
  import { Link, navigate } from 'svelte-routing';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  const ACT = ACTIONS.Tenants;
  let tenants = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newTenant = { name: '', slug: '' };
  let creating = false;

  // Detail view state
  let selectedTenant = null;
  let tenantOrgs = [];
  let tenantAdmins = [];
  let loadingDetail = false;

  onMount(async () => {
    await loadTenants();
  });

  async function loadTenants() {
    loading = true;
    error = '';
    try {
      const res = await api.listTenants({ limit: 100 });
      tenants = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function loadTenantDetail(tenant) {
    selectedTenant = tenant;
    loadingDetail = true;
    try {
      // Load organizations for this tenant
      const orgsRes = await api.listOrganizations({ tenant_id: tenant.id, limit: 100 });
      tenantOrgs = orgsRes.data || [];
      
      // Load tenant role assignments (admins)
      const rolesRes = await api.listTenantRoleAssignments({ tenant_id: tenant.id });
      tenantAdmins = rolesRes.data || [];
    } catch (e) {
      console.error('Failed to load tenant detail:', e);
      tenantOrgs = [];
      tenantAdmins = [];
    } finally {
      loadingDetail = false;
    }
  }

  function closeDetail() {
    selectedTenant = null;
    tenantOrgs = [];
    tenantAdmins = [];
  }

  async function handleCreate() {
    if (!newTenant.name.trim() || !newTenant.slug.trim()) return;
    creating = true;
    try {
      await api.createTenant(newTenant);
      newTenant = { name: '', slug: '' };
      showCreateModal = false;
      addToast($_('tenants.tenantCreated'), 'success');
      await loadTenants();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      creating = false;
    }
  }

  async function handleDelete(id) {
    if (!confirm('Delete this tenant?')) return;
    try {
      await api.deleteTenant(id);
      addToast('Tenant deleted', 'success');
      await loadTenants();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }
</script>

<div class="p-8">
  <div class="page-header">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">{$_('tenants.title')}</h1>
        <p class="text-gray-500 text-sm mt-1.5 font-medium">{$_('tenants.description')}</p>
      </div>
      {#if hasPermission(ACT.create)}
      <button
        on:click={() => showCreateModal = true}
        class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        {$_('tenants.newTenant')}
      </button>
      {/if}
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if tenants.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message={$_('tenants.noTenants')}>
        {#if hasPermission(ACT.create)}
        <button
          on:click={() => showCreateModal = true}
          class="mt-4 btn-primary px-5 py-2.5 rounded-lg font-semibold text-sm"
        >
          {$_('tenants.createFirst')}
        </button>
        {/if}
      </EmptyState>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {#each tenants as tenant (tenant.id)}
        <div class="group bg-white rounded-xl border border-gray-200 p-5 card card-interactive">
          <div class="flex items-start gap-4">
            <div class="w-10 h-10 rounded-lg bg-blue-600 flex items-center justify-center font-bold text-white text-sm flex-shrink-0 group-hover:scale-105 transition-transform duration-300">
              {tenant.name[0]?.toUpperCase() || '?'}
            </div>
            <div class="flex-1 min-w-0">
              <h3 class="text-gray-900 font-semibold text-[15px] truncate mb-0.5">{tenant.name}</h3>
              <p class="text-gray-400 text-xs font-mono truncate">{tenant.slug}</p>
            </div>
            <span class="px-2 py-1 rounded text-xs font-medium {tenant.status === 'active' ? 'bg-emerald-50 text-emerald-600' : 'bg-amber-50 text-amber-600'}">
              {tenant.status}
            </span>
          </div>
          <div class="mt-4 pt-4 border-t border-gray-100 flex items-center justify-between">
            <p class="text-gray-400 text-xs">
              Created {new Date(tenant.created_at).toLocaleDateString()}
            </p>
            <div class="flex items-center gap-1">
              <button
                on:click={() => loadTenantDetail(tenant)}
                class="px-3 py-1.5 rounded-lg text-xs font-medium text-blue-600 hover:bg-blue-50 transition-all"
                title="View details"
              >
                {$_('common.view')}
              </button>
              {#if hasPermission(ACT.delete)}
              <button
                on:click={() => handleDelete(tenant.id)}
                class="p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all"
                title="Delete"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
              </button>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Tenant Detail Modal -->
{#if selectedTenant}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay" on:click={closeDetail} on:keydown={(e) => e.key === 'Escape' && closeDetail()} role="dialog" aria-modal="true">
  <div class="bg-white rounded-2xl p-6 w-full max-w-2xl shadow-elevated-lg border border-gray-200 modal-content max-h-[90vh] overflow-y-auto" on:click|stopPropagation role="document">
    <div class="flex items-center justify-between mb-6">
      <div class="flex items-center gap-3">
        <div class="w-12 h-12 rounded-xl bg-blue-600 flex items-center justify-center font-bold text-white text-lg">
          {selectedTenant.name[0]?.toUpperCase() || '?'}
        </div>
        <div>
          <h2 class="text-xl font-bold text-gray-800">{selectedTenant.name}</h2>
          <p class="text-gray-400 text-sm">{selectedTenant.slug}</p>
        </div>
      </div>
      <button on:click={closeDetail} class="p-2 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-all">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
      </button>
    </div>

    {#if loadingDetail}
      <LoadingSpinner />
    {:else}
      <!-- Related Organizations -->
      <div class="mb-6">
        <h3 class="text-sm font-semibold text-gray-700 uppercase tracking-wider mb-3">{$_('tenants.organizations')}</h3>
        {#if tenantOrgs.length === 0}
          <div class="bg-gray-50 rounded-xl p-4 text-center text-gray-400 text-sm">
            {$_('organizations.noOrganizations')}
          </div>
        {:else}
          <div class="space-y-2">
            {#each tenantOrgs as org (org.id)}
              <Link to="/organizations/{org.id}" class="block bg-gray-50 rounded-xl p-4 hover:bg-gray-100 transition-colors">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-3">
                    <div class="w-8 h-8 rounded-lg bg-emerald-500 flex items-center justify-center text-white text-xs font-bold">
                      {org.name[0]?.toUpperCase() || '?'}
                    </div>
                    <div>
                      <p class="text-sm font-medium text-gray-800">{org.name}</p>
                      <p class="text-xs text-gray-400">{org.slug || ''}</p>
                    </div>
                  </div>
                  <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/></svg>
                </div>
              </Link>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Tenant Admins -->
      <div>
        <h3 class="text-sm font-semibold text-gray-700 uppercase tracking-wider mb-3">{$_('tenants.tenantAdmins')}</h3>
        {#if tenantAdmins.length === 0}
          <div class="bg-gray-50 rounded-xl p-4 text-center text-gray-400 text-sm">
            {$_('tenants.noTenantAdmins')}
          </div>
        {:else}
          <div class="space-y-2">
            {#each tenantAdmins as assignment (assignment.id)}
              <div class="bg-gray-50 rounded-xl p-4 flex items-center gap-3">
                <div class="w-8 h-8 rounded-full bg-purple-600 flex items-center justify-center text-white text-xs font-bold">
                  {(assignment.identity?.name || assignment.identity?.email || '?')[0]?.toUpperCase()}
                </div>
                <div class="flex-1">
                  <p class="text-sm font-medium text-gray-800">{assignment.identity?.name || assignment.identity?.email || 'Unknown'}</p>
                  <p class="text-xs text-gray-400">{assignment.identity?.email || ''}</p>
                </div>
                <span class="px-2 py-1 rounded text-xs font-medium bg-amber-50 text-amber-600">
                  {assignment.role_name}
                </span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
{/if}

<!-- Create Tenant Modal -->
{#if showCreateModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay" on:click={() => showCreateModal = false} on:keydown={(e) => e.key === 'Escape' && (showCreateModal = false)} role="dialog" aria-modal="true">
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content" on:click|stopPropagation role="document">
    <h2 class="text-lg font-bold text-gray-900 mb-5">{$_('tenants.createTenant')}</h2>
    <div class="space-y-4">
      <div>
        <label for="tenant-name" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('tenants.name')}</label>
        <input
          id="tenant-name"
          type="text"
          bind:value={newTenant.name}
          placeholder={$_('tenants.companyName')}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>
      <div>
        <label for="tenant-slug" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('tenants.slug')}</label>
        <input
          id="tenant-slug"
          type="text"
          bind:value={newTenant.slug}
          placeholder={$_('tenants.companySlug')}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showCreateModal = false; newTenant = { name: '', slug: '' }; }}
          class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
        >
          {$_('common.cancel')}
        </button>
        <button
          on:click={handleCreate}
          disabled={creating || !newTenant.name.trim() || !newTenant.slug.trim()}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {creating ? $_('common.loading') : $_('common.create')}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}

<style>
  .card-interactive {
    transition: all 0.2s ease;
  }
  .card-interactive:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    transform: translateY(-2px);
  }
</style>
