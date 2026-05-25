<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import StatCard from '../components/StatCard.svelte';
  import AuditTable from '../components/AuditTable.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  let stats = null;
  let recentLogs = [];
  let loading = true;
  let error = '';

  onMount(async () => {
    try {
      const [pendingRes, allRes, logsRes] = await Promise.all([
        api.listSkills({ status: 'pending_review', limit: 1 }),
        api.listSkills({ limit: 1 }),
        api.listAuditLogs({ limit: 10 })
      ]);

      const pending = pendingRes.total || 0;
      const published = allRes.total || 0;

      stats = {
        total: published,
        pending,
        published: Math.max(0, published - pending)
      };

      recentLogs = logsRes.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-6">
  <h1 class="text-2xl font-semibold mb-6">Stats Dashboard</h1>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="text-red-500">{error}</div>
  {:else if stats}
    <div class="grid grid-cols-3 gap-6 mb-8">
      <StatCard title="Total Skills" value={stats.total} />
      <StatCard title="Pending Review" value={stats.pending} subtitle="Needs attention" />
      <StatCard title="Published" value={stats.published} subtitle="Live skills" />
    </div>

    <div class="bg-white rounded-lg border border-gray-200">
      <div class="px-4 py-3 border-b border-gray-200">
        <h2 class="font-medium">Recent Activity</h2>
      </div>
      {#if recentLogs.length > 0}
        <AuditTable logs={recentLogs} />
      {:else}
        <div class="p-8 text-center text-gray-500">No recent activity</div>
      {/if}
    </div>
  {/if}
</div>
