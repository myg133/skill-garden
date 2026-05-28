<script>
  import { onMount } from 'svelte';
  import { Link } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  let organizations = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newOrgName = '';
  let creating = false;

  onMount(async () => {
    await loadOrganizations();
  });

  async function loadOrganizations() {
    loading = true;
    error = '';
    try {
      const res = await api.listOrganizations({ limit: 100 });
      organizations = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function handleCreate() {
    if (!newOrgName.trim()) return;
    creating = true;
    try {
      await api.createOrganization({ name: newOrgName });
      newOrgName = '';
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
        <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">Organizations</h1>
        <p class="text-surface-500 text-sm mt-1.5 font-medium">Manage tenant organizations and their tools</p>
      </div>
      <button
        on:click={() => showCreateModal = true}
        class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        New Organization
      </button>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if organizations.length === 0}
    <div class="bg-sky-50 backdrop-blur-sm rounded-2xl border border-indigo-200/60 shadow-card">
      <EmptyState message="No organizations yet">
        <button
          on:click={() => showCreateModal = true}
          class="mt-4 btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm"
        >
          Create your first organization
        </button>
      </EmptyState>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {#each organizations as org (org.id)}
        <Link
          to="/organizations/{org.id}"
          class="group bg-sky-50 backdrop-blur-sm rounded-2xl border border-indigo-200/60 p-6 card card-interactive block"
        >
          <div class="flex items-start gap-4">
            <div class="w-11 h-11 rounded-xl bg-gradient-to-br from-brand-500 to-purple-600 flex items-center justify-center font-bold text-lg shadow-glow flex-shrink-0 group-hover:scale-105 transition-transform duration-300">
              {org.name[0]?.toUpperCase() || '?'}
            </div>
            <div class="flex-1 min-w-0">
              <h3 class="text-surface-800 font-semibold text-[15px] truncate mb-0.5 group-hover:text-brand-600 transition-colors">
                {org.name}
              </h3>
              <p class="text-surface-400 text-xs font-mono truncate">{org.id}</p>
            </div>
          </div>
          <div class="mt-4 pt-4 border-t border-surface-100 flex items-center justify-between">
            <p class="text-surface-400 text-xs">
              Created {new Date(org.created_at).toLocaleDateString()}
            </p>
            <svg class="w-4 h-4 text-surface-300 group-hover:text-brand-500 group-hover:translate-x-0.5 transition-all" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
            </svg>
          </div>
        </Link>
      {/each}
    </div>
  {/if}
</div>

{#if showCreateModal}
<div class="fixed inset-0 bg-surface-900/50 backdrop-blur-sm flex items-center justify-center z-50 modal-overlay">
  <div class="bg-sky-50 rounded-2xl p-6 w-full max-w-md shadow-elevated-lg border border-indigo-200 modal-content">
    <h2 class="text-lg font-bold text-surface-800 mb-5">Create Organization</h2>
    <div class="space-y-4">
      <div>
        <label class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Name</label>
        <input
          type="text"
          bind:value={newOrgName}
          placeholder="Organization name"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showCreateModal = false; newOrgName = ''; }}
          class="px-4 py-2.5 text-surface-500 hover:text-surface-800 font-semibold text-sm transition-all rounded-lg hover:bg-surface-50"
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