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

<div class="p-6 max-w-7xl mx-auto">
  <div class="flex items-center justify-between mb-6">
    <div>
      <h1 class="text-2xl font-semibold text-slate-900">Organizations</h1>
      <p class="text-slate-500 text-sm mt-1">Manage tenant organizations</p>
    </div>
    <button
      on:click={() => showCreateModal = true}
      class="bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-lg font-medium text-sm transition-colors"
    >
      + New Organization
    </button>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg">{error}</div>
  {:else if organizations.length === 0}
    <EmptyState message="No organizations yet" />
  {:else}
    <div class="bg-white rounded-xl border border-slate-200 overflow-hidden shadow-sm">
      <table class="w-full">
        <thead class="bg-slate-50 border-b border-slate-200">
          <tr>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Name</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">ID</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Created</th>
            <th class="px-6 py-3 text-right text-xs font-semibold text-slate-600 uppercase tracking-wider">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-100">
          {#each organizations as org (org.id)}
            <tr class="hover:bg-slate-50 transition-colors">
              <td class="px-6 py-4">
                <Link to="/organizations/{org.id}" class="text-slate-900 hover:text-indigo-600 font-medium">
                  {org.name}
                </Link>
              </td>
              <td class="px-6 py-4 text-slate-500 text-sm font-mono">{org.id}</td>
              <td class="px-6 py-4 text-slate-500 text-sm">{new Date(org.created_at).toLocaleDateString()}</td>
              <td class="px-6 py-4 text-right">
                <Link
                  to="/organizations/{org.id}"
                  class="text-indigo-600 hover:text-indigo-800 text-sm font-medium mr-4"
                >
                  View
                </Link>
                <button
                  on:click={() => handleDelete(org.id)}
                  class="text-red-600 hover:text-red-800 text-sm font-medium"
                >
                  Delete
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

{#if showCreateModal}
<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-2xl">
    <h2 class="text-lg font-semibold text-slate-900 mb-4">Create Organization</h2>
    <input
      type="text"
      bind:value={newOrgName}
      placeholder="Organization name"
      class="w-full px-4 py-2 border border-slate-300 rounded-lg mb-4 focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none"
    />
    <div class="flex gap-3 justify-end">
      <button
        on:click={() => { showCreateModal = false; newOrgName = ''; }}
        class="px-4 py-2 text-slate-600 hover:text-slate-800 font-medium"
      >
        Cancel
      </button>
      <button
        on:click={handleCreate}
        disabled={creating || !newOrgName.trim()}
        class="bg-indigo-600 hover:bg-indigo-700 disabled:bg-slate-300 text-white px-4 py-2 rounded-lg font-medium transition-colors"
      >
        {creating ? 'Creating...' : 'Create'}
      </button>
    </div>
  </div>
</div>
{/if}