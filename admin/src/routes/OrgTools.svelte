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
  let newTool = { name: '', tool_type: 'cli', version: '1.0.0', config: '{}' };
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
    if (!selectedOrgId || !newTool.name.trim()) return;
    registering = true;
    try {
      let config;
      try {
        config = JSON.parse(newTool.config);
      } catch {
        addToast('Invalid JSON in config', 'error');
        registering = false;
        return;
      }
      await api.registerOrgTool({
        org_id: selectedOrgId,
        name: newTool.name,
        tool_type: newTool.tool_type,
        version: newTool.version,
        config
      });
      newTool = { name: '', tool_type: 'cli', version: '1.0.0', config: '{}' };
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

<div class="p-6 max-w-7xl mx-auto">
  <div class="flex items-center justify-between mb-6">
    <div>
      <h1 class="text-2xl font-semibold text-slate-900">Org Tools</h1>
      <p class="text-slate-500 text-sm mt-1">Manage organization tools</p>
    </div>
    <button
      on:click={() => showRegisterModal = true}
      class="bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-lg font-medium text-sm transition-colors"
    >
      + Register Tool
    </button>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg">{error}</div>
  {:else if tools.length === 0}
    <EmptyState message="No tools registered yet" />
  {:else}
    <div class="bg-white rounded-xl border border-slate-200 overflow-hidden shadow-sm">
      <table class="w-full">
        <thead class="bg-slate-50 border-b border-slate-200">
          <tr>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Name</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Organization</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Type</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Version</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Registered</th>
            <th class="px-6 py-3 text-right text-xs font-semibold text-slate-600 uppercase tracking-wider">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-100">
          {#each tools as tool (tool.id)}
            <tr class="hover:bg-slate-50 transition-colors">
              <td class="px-6 py-4 text-slate-900 font-medium">{tool.name}</td>
              <td class="px-6 py-4 text-slate-600 text-sm">{tool.org_id}</td>
              <td class="px-6 py-4">
                <span class="px-2 py-1 text-xs rounded-full bg-slate-100 text-slate-600">
                  {tool.tool_type}
                </span>
              </td>
              <td class="px-6 py-4 text-slate-600 text-sm">{tool.version || '1.0.0'}</td>
              <td class="px-6 py-4 text-slate-500 text-sm">
                {new Date(tool.created_at).toLocaleDateString()}
              </td>
              <td class="px-6 py-4 text-right">
                <button
                  on:click={() => handleDelete(tool.id)}
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

{#if showRegisterModal}
<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
  <div class="bg-white rounded-xl p-6 w-full max-w-lg shadow-2xl">
    <h2 class="text-lg font-semibold text-slate-900 mb-4">Register Organization Tool</h2>

    <div class="space-y-4">
      <div>
        <label class="block text-sm font-medium text-slate-700 mb-1">Organization</label>
        <select
          bind:value={selectedOrgId}
          class="w-full px-4 py-2 border border-slate-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none"
        >
          {#each organizations as org (org.id)}
            <option value={org.id}>{org.name}</option>
          {/each}
        </select>
      </div>

      <div>
        <label class="block text-sm font-medium text-slate-700 mb-1">Tool Name</label>
        <input
          type="text"
          bind:value={newTool.name}
          placeholder="e.g., github-cli, docker-tool"
          class="w-full px-4 py-2 border border-slate-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none"
        />
      </div>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium text-slate-700 mb-1">Type</label>
          <select
            bind:value={newTool.tool_type}
            class="w-full px-4 py-2 border border-slate-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none"
          >
            <option value="cli">CLI</option>
            <option value="api">API</option>
            <option value="docker">Docker</option>
            <option value="script">Script</option>
          </select>
        </div>
        <div>
          <label class="block text-sm font-medium text-slate-700 mb-1">Version</label>
          <input
            type="text"
            bind:value={newTool.version}
            placeholder="1.0.0"
            class="w-full px-4 py-2 border border-slate-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none"
          />
        </div>
      </div>

      <div>
        <label class="block text-sm font-medium text-slate-700 mb-1">Config (JSON)</label>
          <textarea
          bind:value={newTool.config}
          rows="3"
          placeholder={`{"command": "gh", "args": ["issue", "list"]}`}
          class="w-full px-4 py-2 border border-slate-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none font-mono text-sm"
        ></textarea>
      </div>
    </div>

    <div class="flex gap-3 justify-end mt-6">
      <button
        on:click={() => { showRegisterModal = false; }}
        class="px-4 py-2 text-slate-600 hover:text-slate-800 font-medium"
      >
        Cancel
      </button>
      <button
        on:click={handleRegister}
        disabled={registering || !newTool.name.trim() || !selectedOrgId}
        class="bg-indigo-600 hover:bg-indigo-700 disabled:bg-slate-300 text-white px-4 py-2 rounded-lg font-medium transition-colors"
      >
        {registering ? 'Registering...' : 'Register'}
      </button>
    </div>
  </div>
</div>
{/if}