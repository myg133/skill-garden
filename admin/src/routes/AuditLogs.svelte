<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import AuditTable from '../components/AuditTable.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  let logs = [];
  let loading = true;
  let error = '';

  let filters = {
    action: '',
    agent_id: '',
    from_date: '',
    to_date: ''
  };

  async function fetchLogs() {
    loading = true;
    try {
      const params = {};
      if (filters.action) params.action = filters.action;
      if (filters.agent_id) params.agent_id = filters.agent_id;
      params.limit = 50;

      let res = await api.listAuditLogs(params);
      logs = res.data || [];

      if (filters.from_date || filters.to_date) {
        const from = filters.from_date ? new Date(filters.from_date).getTime() : 0;
        const to = filters.to_date ? new Date(filters.to_date + 'T23:59:59').getTime() : Infinity;
        logs = logs.filter(log => {
          const t = new Date(log.created_at).getTime();
          return t >= from && t <= to;
        });
      }
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  function resetFilters() {
    filters = { action: '', agent_id: '', from_date: '', to_date: '' };
    fetchLogs();
  }

  onMount(fetchLogs);
</script>

<div class="p-8">
  <div class="page-header">
    <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">Audit Logs</h1>
    <p class="text-surface-500 text-sm mt-1.5 font-medium">Track and search all skill operations</p>
  </div>

  <div class="gradient-card-sky rounded-2xl border border-sky-200/60 p-6 mb-6 shadow-card">
    <div class="grid grid-cols-5 gap-4 items-end">
      <div>
        <label class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Action</label>
        <select bind:value={filters.action} class="w-full px-4 py-2.5 border border-surface-200 rounded-xl text-sm font-medium text-surface-600 input-focus outline-none bg-sky-50 select-caret">
          <option value="">All Actions</option>
          <option value="skill_create">skill_create</option>
          <option value="skill_approve">skill_approve</option>
          <option value="skill_reject">skill_reject</option>
          <option value="skill_update">skill_update</option>
          <option value="skill_delete">skill_delete</option>
        </select>
      </div>
      <div>
        <label class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Agent ID</label>
        <input
          bind:value={filters.agent_id}
          placeholder="Filter by agent"
          class="w-full px-4 py-2.5 border border-surface-200 rounded-xl text-sm input-focus outline-none placeholder:text-surface-300 bg-sky-50"
        />
      </div>
      <div>
        <label class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">From</label>
        <input
          type="date"
          bind:value={filters.from_date}
          class="w-full px-4 py-2.5 border border-surface-200 rounded-xl text-sm text-surface-600 input-focus outline-none bg-sky-50"
        />
      </div>
      <div>
        <label class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">To</label>
        <input
          type="date"
          bind:value={filters.to_date}
          class="w-full px-4 py-2.5 border border-surface-200 rounded-xl text-sm text-surface-600 input-focus outline-none bg-sky-50"
        />
      </div>
      <div class="flex gap-2">
        <button
          on:click={fetchLogs}
          class="btn-primary px-5 py-2.5 rounded-xl text-sm font-semibold"
        >
          Search
        </button>
        <button
          on:click={resetFilters}
          class="btn-secondary px-4 py-2.5 rounded-xl text-sm font-medium"
        >
          Reset
        </button>
      </div>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if logs.length === 0}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 shadow-card">
      <EmptyState message="No audit logs match your filters" />
    </div>
  {:else}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 overflow-hidden shadow-card">
      <AuditTable {logs} />
    </div>
  {/if}
</div>