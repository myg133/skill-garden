<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  let identities = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newIdentity = { identity_type: 'user', name: '', email: '', external_id: '' };
  let creating = false;

  const identityTypes = ['user', 'agent', 'system'];

  onMount(async () => {
    await loadIdentities();
  });

  async function loadIdentities() {
    loading = true;
    error = '';
    try {
      const res = await api.listIdentities({ limit: 100 });
      identities = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function handleCreate() {
    if (!newIdentity.identity_type || !newIdentity.name.trim()) return;
    creating = true;
    try {
      await api.createIdentity(newIdentity);
      newIdentity = { identity_type: 'user', name: '', email: '', external_id: '' };
      showCreateModal = false;
      addToast('Identity created', 'success');
      await loadIdentities();
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
      await loadIdentities();
    } catch (e) {
      addToast(e.message, 'error');
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
</script>

<div class="p-8">
  <div class="page-header">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">Identities</h1>
        <p class="text-gray-500 text-sm mt-1.5 font-medium">Manage users, agents and system identities</p>
      </div>
      <button
        on:click={() => showCreateModal = true}
        class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        New Identity
      </button>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if identities.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message="No identities yet">
        <button
          on:click={() => showCreateModal = true}
          class="mt-4 btn-primary px-5 py-2.5 rounded-lg font-semibold text-sm"
        >
          Create your first identity
        </button>
      </EmptyState>
    </div>
  {:else}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card overflow-hidden">
      <table class="w-full">
        <thead class="bg-gray-50 border-b border-gray-200">
          <tr>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Identity</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Type</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Email</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Status</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Created</th>
            <th class="px-6 py-4 text-right text-xs font-semibold text-gray-500 uppercase tracking-wider">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each identities as identity (identity.id)}
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
              <td class="px-6 py-4 text-sm text-gray-600">{identity.email || '-'}</td>
              <td class="px-6 py-4">
                <span class="px-2.5 py-1 rounded-full text-xs font-medium {identity.status === 'active' ? 'bg-emerald-50 text-emerald-600' : 'bg-amber-50 text-amber-600'}">
                  {identity.status}
                </span>
              </td>
              <td class="px-6 py-4 text-sm text-gray-500">{new Date(identity.created_at).toLocaleDateString()}</td>
              <td class="px-6 py-4 text-right">
                <button
                  on:click={() => handleDelete(identity.id)}
                  class="p-2 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all"
                  title="Delete"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
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
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <h2 class="text-lg font-bold text-gray-900 mb-5">Create Identity</h2>
    <div class="space-y-4">
      <div>
        <label for="identity-type" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Type</label>
        <select
          id="identity-type"
          bind:value={newIdentity.identity_type}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
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
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div>
        <label for="identity-email" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Email</label>
        <input
          id="identity-email"
          type="email"
          bind:value={newIdentity.email}
          placeholder="email@example.com"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div>
        <label for="identity-external-id" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">External ID (optional)</label>
        <input
          id="identity-external-id"
          type="text"
          bind:value={newIdentity.external_id}
          placeholder="External system ID"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium"
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
