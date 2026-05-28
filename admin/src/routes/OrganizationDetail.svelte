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
        api.listSessions({ limit: 50 }),
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

<div class="p-8">
  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if organization}
    <div class="mb-6">
      <Link to="/organizations" class="text-brand-600 hover:text-brand-700 text-sm inline-flex items-center gap-1 font-semibold transition-colors">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/></svg>
        Back to Organizations
      </Link>
    </div>

    <div class="gradient-card-brand-light rounded-2xl border border-brand-200/60 shadow-card mb-6">
      <div class="px-6 py-5 border-b border-brand-200/60">
        <div class="flex items-center justify-between">
          {#if editing}
            <div class="flex gap-3 items-center">
              <input
                type="text"
                bind:value={editName}
                class="text-xl font-bold text-surface-800 px-3 py-1.5 border border-surface-200 rounded-xl input-focus outline-none transition-all bg-white"
              />
              <button
                on:click={handleUpdate}
                class="btn-primary px-3 py-1.5 rounded-lg text-sm font-semibold"
              >
                Save
              </button>
              <button
                on:click={() => editing = false}
                class="btn-secondary px-3 py-1.5 rounded-lg text-sm font-semibold"
              >
                Cancel
              </button>
            </div>
          {:else}
            <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">{organization.name}</h1>
            <button
              on:click={startEdit}
              class="btn-secondary px-4 py-2 rounded-xl text-sm font-semibold"
            >
              Edit
            </button>
          {/if}
        </div>
        <p class="text-surface-400 text-xs mt-1.5 font-mono">ID: {organization.id}</p>
      </div>
      <div class="px-6 py-5 grid grid-cols-3 gap-4">
        <div class="bg-slate-50/80 rounded-xl p-4 border border-brand-200/40 card">
          <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Created</p>
          <p class="text-surface-800 font-semibold text-sm">{new Date(organization.created_at).toLocaleString()}</p>
        </div>
        <div class="bg-slate-50/80 rounded-xl p-4 border border-brand-200/40 card">
          <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Active Sessions</p>
          <p class="text-surface-800 font-extrabold text-2xl">{sessions.filter(s => s.status === 'active').length}</p>
        </div>
        <div class="bg-slate-50/80 rounded-xl p-4 border border-brand-200/40 card">
          <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Registered Tools</p>
          <p class="text-surface-800 font-extrabold text-2xl">{orgTools.length}</p>
        </div>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-5">
      <div class="gradient-card-sky rounded-2xl border border-sky-100/60 shadow-card">
        <div class="px-6 py-4 border-b border-sky-100/60">
          <h2 class="font-semibold text-surface-800 text-sm">Sessions</h2>
        </div>
        <div class="divide-y divide-surface-50 max-h-80 overflow-y-auto">
          {#if sessions.length === 0}
            <div class="px-6 py-12 text-center text-surface-400 text-sm font-medium">No sessions</div>
          {:else}
            {#each sessions as session (session.id)}
              <div class="px-6 py-4 flex items-center justify-between table-row">
                <div>
                  <p class="text-surface-800 text-sm font-mono font-semibold">{session.id}</p>
                  <p class="text-surface-400 text-xs mt-0.5">Agent: {session.agent_id}</p>
                </div>
                <div class="flex items-center gap-3">
                  <span class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-semibold rounded-full {session.status === 'active' ? 'bg-emerald-50 text-emerald-700 ring-1 ring-emerald-600/20' : 'bg-surface-100 text-surface-600 ring-1 ring-surface-600/10'}">
                    {#if session.status === 'active'}
                      <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 pulse-dot"></span>
                    {/if}
                    {session.status}
                  </span>
                  {#if session.status === 'active'}
                    <button
                      on:click={() => handleEndSession(session.id)}
                      class="text-rose-500 hover:text-rose-600 text-xs font-semibold transition-colors"
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

      <div class="gradient-card-rose rounded-2xl border border-rose-100/60 shadow-card">
        <div class="px-6 py-4 border-b border-rose-100/60">
          <h2 class="font-semibold text-surface-800 text-sm">Registered Tools</h2>
        </div>
        <div class="divide-y divide-surface-50 max-h-80 overflow-y-auto">
          {#if orgTools.length === 0}
            <div class="px-6 py-12 text-center text-surface-400 text-sm font-medium">No tools registered</div>
          {:else}
            {#each orgTools as tool (tool.id)}
              <div class="px-6 py-4 flex items-center justify-between table-row">
                <div>
                  <p class="text-surface-800 text-sm font-semibold">{tool.name}</p>
                  <p class="text-surface-400 text-xs mt-0.5 font-mono">{tool.tool_id}</p>
                </div>
                <span class="inline-flex px-2.5 py-1 text-xs font-semibold rounded-full bg-brand-50 text-brand-700 ring-1 ring-brand-600/10">
                  {tool.status || 'pending'}
                </span>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>