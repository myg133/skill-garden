<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  let sessions = [];
  let loading = true;
  let error = '';
  let filter = 'all';

  onMount(async () => {
    await loadSessions();
  });

  async function loadSessions() {
    loading = true;
    error = '';
    try {
      const params = filter !== 'all' ? { status: filter } : {};
      const res = await api.listSessions(params);
      sessions = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function handleEndSession(id) {
    if (!confirm('End this session?')) return;
    try {
      await api.endSession(id);
      addToast('Session ended', 'success');
      await loadSessions();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  function handleFilterChange() {
    loadSessions();
  }

  function formatDuration(start, end) {
    if (!start) return 'N/A';
    const ms = new Date(end || Date.now()) - new Date(start);
    const mins = Math.floor(ms / 60000);
    if (mins < 60) return `${mins}m`;
    const hours = Math.floor(mins / 60);
    return `${hours}h ${mins % 60}m`;
  }
</script>

<div class="p-6 max-w-7xl mx-auto">
  <div class="flex items-center justify-between mb-6">
    <div>
      <h1 class="text-2xl font-semibold text-slate-900">Sessions</h1>
      <p class="text-slate-500 text-sm mt-1">Manage agent sessions</p>
    </div>
    <div class="flex gap-2">
      <select
        bind:value={filter}
        on:change={handleFilterChange}
        class="px-4 py-2 border border-slate-300 rounded-lg text-sm focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none"
      >
        <option value="all">All Sessions</option>
        <option value="active">Active</option>
        <option value="ended">Ended</option>
      </select>
      <button
        on:click={loadSessions}
        class="px-4 py-2 border border-slate-300 rounded-lg text-sm hover:bg-slate-50 transition-colors"
      >
        Refresh
      </button>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg">{error}</div>
  {:else if sessions.length === 0}
    <EmptyState message="No sessions found" />
  {:else}
    <div class="bg-white rounded-xl border border-slate-200 overflow-hidden shadow-sm">
      <table class="w-full">
        <thead class="bg-slate-50 border-b border-slate-200">
          <tr>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Session ID</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Agent</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Organization</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Status</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Duration</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase tracking-wider">Created</th>
            <th class="px-6 py-3 text-right text-xs font-semibold text-slate-600 uppercase tracking-wider">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-100">
          {#each sessions as session (session.id)}
            <tr class="hover:bg-slate-50 transition-colors">
              <td class="px-6 py-4 text-slate-900 text-sm font-mono">{session.id}</td>
              <td class="px-6 py-4 text-slate-600 text-sm">{session.agent_id}</td>
              <td class="px-6 py-4 text-slate-600 text-sm">{session.org_id}</td>
              <td class="px-6 py-4">
                <span class="px-2 py-1 text-xs rounded-full {session.status === 'active' ? 'bg-green-100 text-green-700' : 'bg-slate-100 text-slate-600'}">
                  {session.status}
                </span>
              </td>
              <td class="px-6 py-4 text-slate-600 text-sm">
                {formatDuration(session.created_at, session.ended_at)}
              </td>
              <td class="px-6 py-4 text-slate-500 text-sm">
                {new Date(session.created_at).toLocaleString()}
              </td>
              <td class="px-6 py-4 text-right">
                {#if session.status === 'active'}
                  <button
                    on:click={() => handleEndSession(session.id)}
                    class="text-red-600 hover:text-red-800 text-sm font-medium"
                  >
                    End
                  </button>
                {:else}
                  <span class="text-slate-400 text-sm">Ended</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>