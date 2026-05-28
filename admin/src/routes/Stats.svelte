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
        api.listSkills({ page_size: 100 }),
        api.listSkills({ page_size: 1 }),
        api.listAuditLogs({ limit: 10 })
      ]);

      const allSkills = allRes.data || [];
      const allTotal = allRes.total || allSkills.length;
      const pendingCount = allSkills.filter(s => s.status === 'pending_review').length;

      stats = {
        total: allTotal,
        pending: pendingCount,
        published: Math.max(0, allTotal - pendingCount)
      };

      recentLogs = logsRes.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-8">
  <div class="page-header">
    <div>
      <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">Dashboard</h1>
      <p class="text-surface-500 text-sm mt-1.5 font-medium">Overview of skill activity and metrics</p>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if stats}
    <div class="grid grid-cols-1 md:grid-cols-3 gap-5 mb-8">
      <StatCard
        variant="brand"
        title="Total Skills"
        value={stats.total}
        subtitle="All registered skills"
        icon='<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"/>'
      />
      <StatCard
        variant="amber"
        title="Pending Review"
        value={stats.pending}
        subtitle="Awaiting approval"
        icon='<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>'
      />
      <StatCard
        variant="green"
        title="Published"
        value={stats.published}
        subtitle="Live in production"
        icon='<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>'
      />
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-5 mb-8">
      <div class="lg:col-span-2 bg-sky-50 rounded-2xl border border-indigo-200 shadow-card overflow-hidden">
        <div class="px-6 py-4 border-b border-surface-100 flex items-center justify-between">
          <div>
            <h2 class="font-semibold text-surface-800 text-sm">Recent Activity</h2>
            <p class="text-surface-400 text-xs mt-0.5">Latest audit log entries</p>
          </div>
          <span class="w-2 h-2 rounded-full bg-emerald-500 pulse-dot"></span>
        </div>
        {#if recentLogs.length > 0}
          <AuditTable logs={recentLogs} />
        {:else}
          <div class="p-12 text-center">
            <p class="text-surface-400 text-sm">No recent activity</p>
          </div>
        {/if}
      </div>

      <div class="bg-sky-50 rounded-2xl border border-indigo-200 shadow-card p-6">
        <h3 class="font-semibold text-surface-800 text-sm mb-4">Quick Stats</h3>
        <div class="space-y-4">
          <div class="flex items-center justify-between py-2">
            <span class="text-surface-500 text-sm">Approval Rate</span>
            <span class="font-semibold text-emerald-600 text-sm">
              {stats.total > 0 ? ((stats.published / stats.total) * 100).toFixed(0) : 0}%
            </span>
          </div>
          <hr class="divider-soft" />
          <div class="flex items-center border-indigo-200 justify-between py-2">
            <span class="text-surface-500 text-sm">Pending Queue</span>
            <span class="font-semibold text-amber-600 text-sm">{stats.pending}</span>
          </div>
          <hr class="divider-soft" />
          <div class="flex items-center border-indigo-200 justify-between py-2">
            <span class="text-surface-500 text-sm">Total Published</span>
            <span class="font-semibold text-brand-600 text-sm">{stats.published}</span>
          </div>
          <hr class="divider-soft" />
          <div class="flex items-center border-indigo-200 justify-between py-2">
            <span class="text-surface-500 text-sm">Total Skills</span>
            <span class="font-semibold text-surface-800 text-sm">{stats.total}</span>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>