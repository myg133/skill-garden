<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  let tenants = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newTenant = { name: '', slug: '' };
  let creating = false;

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

  async function handleCreate() {
    if (!newTenant.name.trim() || !newTenant.slug.trim()) return;
    creating = true;
    try {
      await api.createTenant(newTenant);
      newTenant = { name: '', slug: '' };
      showCreateModal = false;
      addToast('Tenant created', 'success');
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
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">Tenants</h1>
        <p class="text-gray-500 text-sm mt-1.5 font-medium">Manage tenant organizations and companies</p>
      </div>
      <button
        on:click={() => showCreateModal = true}
        class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        New Tenant
      </button>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if tenants.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message="No tenants yet">
        <button
          on:click={() => showCreateModal = true}
          class="mt-4 btn-primary px-5 py-2.5 rounded-lg font-semibold text-sm"
        >
          Create your first tenant
        </button>
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
            <button
              on:click={() => handleDelete(tenant.id)}
              class="p-2 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all"
              title="Delete"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if showCreateModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <h2 class="text-lg font-bold text-gray-900 mb-5">Create Tenant</h2>
    <div class="space-y-4">
      <div>
        <label for="tenant-name" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Name</label>
        <input
          id="tenant-name"
          type="text"
          bind:value={newTenant.name}
          placeholder="Company name"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div>
        <label for="tenant-slug" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Slug</label>
        <input
          id="tenant-slug"
          type="text"
          bind:value={newTenant.slug}
          placeholder="company-slug"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showCreateModal = false; newTenant = { name: '', slug: '' }; }}
          class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
        >
          Cancel
        </button>
        <button
          on:click={handleCreate}
          disabled={creating || !newTenant.name.trim() || !newTenant.slug.trim()}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {creating ? 'Creating...' : 'Create'}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}
