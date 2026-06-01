<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import Badge from '../components/Badge.svelte';

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

<div class="p-8">
  <div class="page-header">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">Sessions</h1>
        <p class="text-surface-500 text-sm mt-1.5 font-medium">Monitor active and past agent sessions</p>
      </div>
      <div class="flex gap-2.5">
        <select
          bind:value={filter}
          on:change={handleFilterChange}
          class="px-4 py-2.5 border border-surface-200 rounded-xl text-sm font-medium text-surface-600 input-focus outline-none bg-slate-50 select-caret"
        >
          <option value="all">All Sessions</option>
          <option value="active">Active</option>
          <option value="ended">Ended</option>
        </select>
        <button
          on:click={loadSessions}
          class="btn-secondary px-4 py-2.5 rounded-xl text-sm font-medium flex items-center gap-1.5"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
          Refresh
        </button>
      </div>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if sessions.length === 0}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 shadow-card">
      <EmptyState message="No sessions found" />
    </div>
  {:else}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 overflow-hidden shadow-card">
      <table class="w-full">
        <thead>
          <tr class="border-b border-surface-100 bg-gradient-to-r from-surface-50/80 to-transparent">
            <th class="px-5 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Session ID</th>
            <th class="px-5 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Agent</th>
            <th class="px-5 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Organization</th>
            <th class="px-5 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Status</th>
            <th class="px-5 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Duration</th>
            <th class="px-5 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Created</th>
            <th class="px-5 py-4 text-right text-xs font-semibold text-surface-400 uppercase tracking-wider">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-surface-50">
          {#each sessions as session (session.id)}
            <tr class="table-row">
              <td class="px-5 py-4 text-surface-700 text-xs font-mono font-medium">{session.id}</td>
              <td class="px-5 py-4 text-surface-500 text-sm">{session.agent_id}</td>
              <td class="px-5 py-4 text-surface-500 text-sm">{session.org_id}</td>
              <td class="px-5 py-4">
                <Badge status={session.status} />
              </td>
              <td class="px-5 py-4 text-surface-500 text-sm font-medium stat-number">
                {formatDuration(session.created_at, session.ended_at)}
              </td>
              <td class="px-5 py-4 text-surface-400 text-sm">
                {new Date(session.created_at).toLocaleString()}
              </td>
              <td class="px-5 py-4 text-right">
                {#if session.status === 'active'}
                  <button
                    on:click={() => handleEndSession(session.id)}
                    class="text-rose-500 hover:text-rose-600 text-sm font-semibold transition-colors"
                  >
                    End Session
                  </button>
                {:else}
                  <span class="text-surface-300 text-sm font-medium">Ended</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>