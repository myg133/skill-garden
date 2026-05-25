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
      if (filters.from_date) params.from = filters.from_date;
      if (filters.to_date) params.to = filters.to_date;
      params.limit = 50;

      const res = await api.listAuditLogs(params);
      logs = res.data || [];
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

<div class="p-6">
  <h1 class="text-2xl font-semibold mb-6">Audit Logs</h1>

  <div class="bg-white rounded-lg border border-gray-200 p-4 mb-6">
    <div class="grid grid-cols-5 gap-4">
      <div>
        <label class="block text-sm text-gray-600 mb-1">Action</label>
        <select bind:value={filters.action} class="w-full px-3 py-2 border border-gray-300 rounded">
          <option value="">All</option>
          <option value="skill_create">skill_create</option>
          <option value="skill_approve">skill_approve</option>
          <option value="skill_reject">skill_reject</option>
          <option value="skill_update">skill_update</option>
          <option value="skill_delete">skill_delete</option>
        </select>
      </div>
      <div>
        <label class="block text-sm text-gray-600 mb-1">Agent ID</label>
        <input
          bind:value={filters.agent_id}
          placeholder="Filter by agent"
          class="w-full px-3 py-2 border border-gray-300 rounded"
        />
      </div>
      <div>
        <label class="block text-sm text-gray-600 mb-1">From Date</label>
        <input
          type="date"
          bind:value={filters.from_date}
          class="w-full px-3 py-2 border border-gray-300 rounded"
        />
      </div>
      <div>
        <label class="block text-sm text-gray-600 mb-1">To Date</label>
        <input
          type="date"
          bind:value={filters.to_date}
          class="w-full px-3 py-2 border border-gray-300 rounded"
        />
      </div>
      <div class="flex items-end gap-2">
        <button
          on:click={fetchLogs}
          class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
          Search
        </button>
        <button
          on:click={resetFilters}
          class="px-4 py-2 text-gray-600 border border-gray-300 rounded hover:bg-gray-50">
          Reset
        </button>
      </div>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="text-red-500">{error}</div>
  {:else if logs.length === 0}
    <EmptyState message="No audit logs match your filters" />
  {:else}
    <div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
      <AuditTable {logs} />
    </div>
  {/if}
</div>
