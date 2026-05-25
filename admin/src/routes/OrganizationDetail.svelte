<script>
  import { onMount } from 'svelte';
  import { Link, navigate } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  export let id = '';

  let organization = null;
  let sessions = [];
  let orgTools = [];
  let loading = true;
  let error = '';
  let editing = false;
  let editName = '';

  onMount(async () => {
    await loadData();
  });

  async function loadData() {
    loading = true;
    error = '';
    try {
      const [orgRes, sessionsRes, toolsRes] = await Promise.all([
        api.getOrganization(id),
        api.listSessions({ org_id: id, limit: 50 }),
        api.listApprovedTools(id)
      ]);
      organization = orgRes;
      sessions = sessionsRes.data || [];
      orgTools = toolsRes.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function handleUpdate() {
    if (!editName.trim()) return;
    try {
      await api.updateOrganization(id, { name: editName });
      organization.name = editName;
      editing = false;
      addToast('Organization updated', 'success');
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleEndSession(sessionId) {
    if (!confirm('End this session?')) return;
    try {
      await api.endSession(sessionId);
      addToast('Session ended', 'success');
      await loadData();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  function startEdit() {
    editName = organization.name;
    editing = true;
  }
</script>

<div class="p-6 max-w-7xl mx-auto">
  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg">{error}</div>
  {:else if organization}
    <div class="mb-6">
      <Link to="/organizations" class="text-indigo-600 hover:text-indigo-800 text-sm mb-4 inline-flex items-center gap-1">
        ← Back to Organizations
      </Link>
    </div>

    <div class="bg-white rounded-xl border border-slate-200 shadow-sm mb-6">
      <div class="px-6 py-4 border-b border-slate-200">
        <div class="flex items-center justify-between">
          {#if editing}
            <div class="flex gap-3 items-center">
              <input
                type="text"
                bind:value={editName}
                class="text-xl font-semibold text-slate-900 px-3 py-1 border border-slate-300 rounded-lg focus:ring-2 focus:ring-indigo-500"
              />
              <button
                on:click={handleUpdate}
                class="bg-indigo-600 hover:bg-indigo-700 text-white px-3 py-1 rounded-lg text-sm font-medium"
              >
                Save
              </button>
              <button
                on:click={() => editing = false}
                class="text-slate-600 hover:text-slate-800 px-3 py-1 text-sm"
              >
                Cancel
              </button>
            </div>
          {:else}
            <h1 class="text-2xl font-semibold text-slate-900">{organization.name}</h1>
            <button
              on:click={startEdit}
              class="text-slate-500 hover:text-slate-700 text-sm font-medium"
            >
              Edit
            </button>
          {/if}
        </div>
        <p class="text-slate-500 text-sm mt-1">ID: {organization.id}</p>
      </div>
      <div class="px-6 py-4 grid grid-cols-3 gap-6">
        <div class="bg-slate-50 rounded-lg p-4">
          <p class="text-slate-500 text-xs uppercase tracking-wider mb-1">Created</p>
          <p class="text-slate-900 font-medium">{new Date(organization.created_at).toLocaleString()}</p>
        </div>
        <div class="bg-slate-50 rounded-lg p-4">
          <p class="text-slate-500 text-xs uppercase tracking-wider mb-1">Active Sessions</p>
          <p class="text-slate-900 font-medium">{sessions.filter(s => s.status === 'active').length}</p>
        </div>
        <div class="bg-slate-50 rounded-lg p-4">
          <p class="text-slate-500 text-xs uppercase tracking-wider mb-1">Registered Tools</p>
          <p class="text-slate-900 font-medium">{orgTools.length}</p>
        </div>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Sessions -->
      <div class="bg-white rounded-xl border border-slate-200 shadow-sm">
        <div class="px-6 py-4 border-b border-slate-200">
          <h2 class="text-lg font-semibold text-slate-900">Sessions</h2>
        </div>
        <div class="divide-y divide-slate-100">
          {#if sessions.length === 0}
            <div class="px-6 py-8 text-center text-slate-500">No sessions</div>
          {:else}
            {#each sessions as session (session.id)}
              <div class="px-6 py-3 flex items-center justify-between">
                <div>
                  <p class="text-slate-900 text-sm font-mono">{session.id}</p>
                  <p class="text-slate-500 text-xs">Agent: {session.agent_id}</p>
                </div>
                <div class="flex items-center gap-3">
                  <span class="px-2 py-1 text-xs rounded-full {session.status === 'active' ? 'bg-green-100 text-green-700' : 'bg-slate-100 text-slate-600'}">
                    {session.status}
                  </span>
                  {#if session.status === 'active'}
                    <button
                      on:click={() => handleEndSession(session.id)}
                      class="text-red-600 hover:text-red-800 text-xs font-medium"
                    >
                      End
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>

      <!-- Org Tools -->
      <div class="bg-white rounded-xl border border-slate-200 shadow-sm">
        <div class="px-6 py-4 border-b border-slate-200">
          <h2 class="text-lg font-semibold text-slate-900">Registered Tools</h2>
        </div>
        <div class="divide-y divide-slate-100">
          {#if orgTools.length === 0}
            <div class="px-6 py-8 text-center text-slate-500">No tools registered</div>
          {:else}
            {#each orgTools as tool (tool.id)}
              <div class="px-6 py-3 flex items-center justify-between">
                <div>
                  <p class="text-slate-900 text-sm font-medium">{tool.name}</p>
                  <p class="text-slate-500 text-xs">{tool.tool_type}</p>
                </div>
                <span class="px-2 py-1 text-xs rounded-full bg-indigo-100 text-indigo-700">
                  {tool.version || 'v1'}
                </span>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>