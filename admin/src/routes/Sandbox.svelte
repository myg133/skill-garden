<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import Badge from '../components/Badge.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  const ACT = ACTIONS.Sandbox;

  let containers = [];
  let health = null;
  let loading = true;
  let healthLoading = true;
  let error = null;
  let removing = null;

  onMount(() => {
    loadData();
  });

  async function loadData() {
    loading = true;
    healthLoading = true;
    error = null;
    try {
      const [sandboxRes, healthRes] = await Promise.allSettled([
        api.listSandboxes(),
        api.getSandboxHealth()
      ]);
      if (sandboxRes.status === 'fulfilled') {
        containers = sandboxRes.value.data || [];
      }
      if (healthRes.status === 'fulfilled') {
        health = healthRes.value;
      }
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
      healthLoading = false;
    }
  }

  async function handleRemove(key) {
    if (!confirm(`Remove sandbox "${key}"? This will stop and delete the container.`)) return;
    removing = key;
    try {
      const res = await api.removeSandbox(key);
      addToast(res.removed ? `Sandbox "${res.removed}" removed` : 'Sandbox removed', 'success');
      await loadData();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      removing = null;
    }
  }

  function statusStyle(status) {
    switch (status) {
      case 'ready': return 'bg-emerald-100 text-emerald-700 border-emerald-200';
      case 'busy': return 'bg-amber-100 text-amber-700 border-amber-200';
      case 'starting': return 'bg-blue-100 text-blue-700 border-blue-200';
      case 'stopped': return 'bg-gray-100 text-gray-600 border-gray-200';
      case 'error': return 'bg-red-100 text-red-700 border-red-200';
      default: return 'bg-gray-100 text-gray-600 border-gray-200';
    }
  }

  function formatTime(ts) {
    if (!ts) return '—';
    try {
      return new Date(ts).toLocaleString();
    } catch {
      return ts;
    }
  }

  function shortId(id) {
    return id ? id.substring(0, 12) : '—';
  }
</script>

<div class="p-8 max-w-7xl mx-auto fade-in">
  <div class="flex items-center justify-between mb-8">
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-indigo-900">Sandbox Containers</h1>
      <p class="mt-1 text-sm text-indigo-500">Docker sandbox management — list, health check, remove containers</p>
    </div>
    <button
      on:click={loadData}
      disabled={loading}
      class="btn-primary inline-flex items-center gap-2 px-4 py-2.5 rounded-xl text-sm font-semibold disabled:opacity-50"
    >
      <svg class="w-4 h-4 {loading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
      </svg>
      {loading ? 'Refreshing...' : 'Refresh'}
    </button>
  </div>

  <!-- Health Status -->
  <div class="mb-8">
    {#if healthLoading}
      <div class="card p-6"><LoadingSpinner /></div>
    {:else if health}
      <div class="card p-6">
        <h2 class="text-sm font-semibold text-indigo-700 mb-4">Docker Health</h2>
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div class="flex items-center gap-3 p-4 rounded-xl {health.docker_connected ? 'bg-emerald-50 border border-emerald-100' : 'bg-red-50 border border-red-100'}">
            <div class="w-3 h-3 rounded-full {health.docker_connected ? 'bg-emerald-500' : 'bg-red-500'}"></div>
            <div>
              <p class="text-xs text-indigo-500">Docker Daemon</p>
              <p class="text-sm font-semibold {health.docker_connected ? 'text-emerald-700' : 'text-red-700'}">
                {health.docker_connected ? 'Connected' : 'Disconnected'}
              </p>
            </div>
          </div>
          <div class="flex items-center gap-3 p-4 rounded-xl bg-indigo-50 border border-gray-100">
            <svg class="w-5 h-5 text-indigo-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/>
            </svg>
            <div>
              <p class="text-xs text-indigo-500">Active Containers</p>
              <p class="text-sm font-semibold text-indigo-700">{health.active_containers}</p>
            </div>
          </div>
          <div class="flex items-center gap-3 p-4 rounded-xl bg-indigo-50 border border-gray-100">
            <svg class="w-5 h-5 text-indigo-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
            </svg>
            <div>
              <p class="text-xs text-indigo-500">Tracked Containers</p>
              <p class="text-sm font-semibold text-indigo-700">{containers.length}</p>
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>

  <!-- Container List -->
  {#if loading}
    <div class="card p-12"><LoadingSpinner /></div>
  {:else if error}
    <div class="card p-6 text-center">
      <p class="text-red-600 font-medium">{error}</p>
      <button on:click={loadData} class="mt-4 btn-ghost text-sm">Retry</button>
    </div>
  {:else if containers.length === 0}
    <EmptyState title="No Sandbox Containers" message="No active sandbox containers found. Containers are created on-demand when tools are executed." />
  {:else}
    <div class="card overflow-hidden">
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-gray-100 text-left">
              <th class="px-5 py-3.5 text-xs font-semibold text-indigo-500 uppercase tracking-wider">ID / Key</th>
              <th class="px-5 py-3.5 text-xs font-semibold text-indigo-500 uppercase tracking-wider">Session</th>
              <th class="px-5 py-3.5 text-xs font-semibold text-indigo-500 uppercase tracking-wider">Container ID</th>
              <th class="px-5 py-3.5 text-xs font-semibold text-indigo-500 uppercase tracking-wider">Image</th>
              <th class="px-5 py-3.5 text-xs font-semibold text-indigo-500 uppercase tracking-wider">Status</th>
              <th class="px-5 py-3.5 text-xs font-semibold text-indigo-500 uppercase tracking-wider">Created</th>
              <th class="px-5 py-3.5 text-xs font-semibold text-indigo-500 uppercase tracking-wider">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each containers as c (c.container_id)}
              <tr class="border-b border-gray-50 hover:bg-blue-50 transition-colors">
                <td class="px-5 py-3.5">
                  <p class="font-mono text-xs text-indigo-700 truncate max-w-[160px]" title={c.id}>{c.id || '—'}</p>
                </td>
                <td class="px-5 py-3.5">
                  <p class="text-xs text-indigo-600">{c.session_id || '—'}</p>
                </td>
                <td class="px-5 py-3.5">
                  <code class="text-[11px] text-indigo-400 bg-indigo-50 px-1.5 py-0.5 rounded" title={c.container_id}>{shortId(c.container_id)}</code>
                </td>
                <td class="px-5 py-3.5">
                  <p class="text-xs text-indigo-600 truncate max-w-[200px]" title={c.image}>{c.image || '—'}</p>
                </td>
                <td class="px-5 py-3.5">
                  <span class="inline-flex px-2 py-0.5 text-[11px] font-medium rounded-full border {statusStyle(c.status)}">
                    {c.status || 'unknown'}
                  </span>
                </td>
                <td class="px-5 py-3.5">
                  <p class="text-xs text-indigo-500 whitespace-nowrap">{formatTime(c.created_at)}</p>
                </td>
                <td class="px-5 py-3.5">
                  {#if hasPermission(ACT.manage)}
                    <button
                      on:click={() => handleRemove(c.id)}
                      disabled={removing === c.id}
                      class="text-xs font-medium text-red-500 hover:text-red-700 hover:bg-red-50 px-2.5 py-1 rounded-lg transition-colors disabled:opacity-50"
                      title="Stop and remove container"
                    >
                      {removing === c.id ? 'Removing...' : 'Remove'}
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</div>
