<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  let tools = [];
  let organizations = [];
  let loading = true;
  let error = '';
  let showRegisterModal = false;
  let selectedOrgId = '';
  let newTool = { name: '', tool_id: '', description: '', schema: '{}', implementation: '{}' };
  let registering = false;

  onMount(async () => {
    await Promise.all([loadTools(), loadOrganizations()]);
  });

  async function loadTools() {
    loading = true;
    error = '';
    try {
      const res = await api.listOrgTools({ limit: 100 });
      tools = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function loadOrganizations() {
    try {
      const res = await api.listOrganizations({ limit: 100 });
      organizations = res.data || [];
      if (organizations.length > 0) {
        selectedOrgId = organizations[0].id;
      }
    } catch (e) {
      console.error('Failed to load organizations:', e);
    }
  }

  async function handleRegister() {
    if (!selectedOrgId || !newTool.name.trim() || !newTool.tool_id.trim()) return;
    registering = true;
    try {
      let schema, implementation;
      try {
        schema = JSON.parse(newTool.schema);
        implementation = JSON.parse(newTool.implementation);
      } catch {
        addToast('Invalid JSON in schema or implementation', 'error');
        registering = false;
        return;
      }
      await api.registerOrgTool({
        org_id: selectedOrgId,
        tool_id: newTool.tool_id,
        name: newTool.name,
        description: newTool.description,
        schema,
        implementation
      });
      newTool = { name: '', tool_id: '', description: '', schema: '{}', implementation: '{}' };
      showRegisterModal = false;
      addToast('Tool registered', 'success');
      await loadTools();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      registering = false;
    }
  }

  async function handleDelete(id) {
    if (!confirm('Delete this tool?')) return;
    try {
      await api.deleteOrgTool(id);
      addToast('Tool deleted', 'success');
      await loadTools();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }
</script>

<div class="p-8">
  <div class="page-header flex items-center justify-between">
    <div>
      <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">Org Tools</h1>
      <p class="text-surface-500 text-sm mt-1.5 font-medium">Manage organization tools and schemas</p>
    </div>
    <button
      on:click={() => showRegisterModal = true}
      class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
      Register Tool
    </button>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if tools.length === 0}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 shadow-card">
      <EmptyState message="No tools registered yet" />
    </div>
  {:else}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 overflow-hidden shadow-card">
      <table class="w-full">
        <thead>
          <tr class="border-b border-surface-100 bg-gradient-to-r from-surface-50/80 to-transparent">
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Name</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Tool ID</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Description</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Status</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Registered</th>
            <th class="px-6 py-4 text-right text-xs font-semibold text-surface-400 uppercase tracking-wider">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-surface-50">
          {#each tools as tool (tool.id)}
            <tr class="table-row">
              <td class="px-6 py-4 text-surface-800 font-semibold text-sm">{tool.name}</td>
              <td class="px-6 py-4 text-surface-500 text-sm font-mono text-xs">{tool.tool_id}</td>
              <td class="px-6 py-4 text-surface-500 text-sm truncate max-w-[200px]">{tool.description}</td>
              <td class="px-6 py-4">
                <span class="inline-flex px-2.5 py-1 text-xs font-semibold rounded-full bg-surface-100 text-surface-600 ring-1 ring-surface-600/10">
                  {tool.status}
                </span>
              </td>
              <td class="px-6 py-4 text-surface-400 text-sm">
                {new Date(tool.created_at).toLocaleDateString()}
              </td>
              <td class="px-6 py-4 text-right">
                <button
                  on:click={() => handleDelete(tool.id)}
                  class="text-rose-500 hover:text-rose-600 text-sm font-semibold transition-colors"
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

<!-- Register Modal -->
  {#if showRegisterModal}
    <div class="fixed inset-0 bg-surface-900/40 backdrop-blur-sm flex items-center justify-center z-50 fade-in" on:click={() => showRegisterModal = false}>
      <div class="bg-sky-50 rounded-2xl p-6 w-full max-w-lg shadow-elevated border border-indigo-200 max-h-[85vh] overflow-y-auto" on:click|stopPropagation>
        <div class="flex items-center justify-between mb-5">
          <h2 class="text-lg font-semibold text-surface-800">Register Tool</h2>
          <button
            on:click={() => showRegisterModal = false}
            class="p-2 text-surface-400 hover:text-surface-600 hover:bg-surface-100 rounded-xl transition-all duration-200"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M6 18L18 6M6 6l12 12"/></svg>
          </button>
        </div>
        <div class="space-y-4">
      <div>
        <label class="block text-sm font-semibold text-surface-500 mb-2">Organization</label>
        <select
          bind:value={selectedOrgId}
          class="w-full px-4 py-2.5 border border-surface-200 rounded-xl text-sm focus:ring-2 focus:ring-brand-500/30 focus:border-brand-500 outline-none bg-white transition-all"
        >
          {#each organizations as org (org.id)}
            <option value={org.id}>{org.name}</option>
          {/each}
        </select>
      </div>

      <div>
        <label class="block text-sm font-semibold text-surface-500 mb-2">Tool Name</label>
        <input
          type="text"
          bind:value={newTool.name}
          placeholder="e.g., github-cli, docker-tool"
          class="w-full px-4 py-2.5 border border-surface-200 rounded-xl text-sm focus:ring-2 focus:ring-brand-500/30 focus:border-brand-500 outline-none transition-all"
        />
      </div>

      <div>
        <label class="block text-sm font-semibold text-surface-500 mb-2">Tool ID</label>
        <input
          type="text"
          bind:value={newTool.tool_id}
          placeholder="e.g., github_issue_lister"
          class="w-full px-4 py-2.5 border border-surface-200 rounded-xl text-sm focus:ring-2 focus:ring-brand-500/30 focus:border-brand-500 outline-none transition-all"
        />
      </div>

      <div>
        <label class="block text-sm font-semibold text-surface-500 mb-2">Description</label>
        <input
          type="text"
          bind:value={newTool.description}
          placeholder="Describe what this tool does"
          class="w-full px-4 py-2.5 border border-surface-200 rounded-xl text-sm focus:ring-2 focus:ring-brand-500/30 focus:border-brand-500 outline-none transition-all"
        />
      </div>

      <div>
        <label class="block text-sm font-semibold text-surface-500 mb-2">Schema (JSON)</label>
        <textarea
          bind:value={newTool.schema}
          rows="3"
          placeholder={`{"type": "object", "properties": {}}`}
          class="w-full px-4 py-2.5 border border-surface-200 rounded-xl text-sm focus:ring-2 focus:ring-brand-500/30 focus:border-brand-500 outline-none font-mono transition-all"
        ></textarea>
      </div>

      <div>
        <label class="block text-sm font-semibold text-surface-500 mb-2">Implementation (JSON)</label>
        <textarea
          bind:value={newTool.implementation}
          rows="3"
          placeholder={`{"command": "gh", "args": ["issue", "list"]}`}
          class="w-full px-4 py-2.5 border border-surface-200 rounded-xl text-sm focus:ring-2 focus:ring-brand-500/30 focus:border-brand-500 outline-none font-mono transition-all"
        ></textarea>
      </div>
    </div>

    <div class="flex gap-3 justify-end mt-6">
      <button
            on:click={() => showRegisterModal = false}
            class="btn-secondary px-5 py-2.5 rounded-xl text-sm font-medium"
          >
            Cancel
          </button>
          <button
            on:click={handleRegister}
            disabled={!newTool.name || !newTool.tool_id || registering}
            class="btn-primary px-6 py-2.5 rounded-xl text-sm font-semibold disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {registering ? 'Registering...' : 'Register'}
          </button>
    </div>
  </div>
</div>
{/if}