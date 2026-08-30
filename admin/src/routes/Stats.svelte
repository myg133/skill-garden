<script>
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';
  import { permissionStore } from '../stores/permission.js';
  import { getQuickActionsForRole, ROLE_SUPER_ADMIN } from '../config/nav-routes.js';
  import StatCard from '../components/StatCard.svelte';
  import AuditTable from '../components/AuditTable.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import Icon from '../components/Icon.svelte';

  let stats = null;
  let recentLogs = [];
  let sandboxHealth = null;
  let loading = true;
  let error = '';

  // Quick actions for super_admin
  $: currentRole = $permissionStore.systemRoles.includes(ROLE_SUPER_ADMIN) ? ROLE_SUPER_ADMIN : null;
  $: quickActions = currentRole ? getQuickActionsForRole(currentRole) : [];

  onMount(async () => {
    try {
      const [skillsRes, countsRes, sandboxRes] = await Promise.all([
        api.listSkills({ page_size: 1 }),
        api.listSkills({ page_size: 200 }),
        api.getSandboxHealth().catch(() => null)
      ]);

      const allTotal = skillsRes.total || 0;
      const allSkills = countsRes.data || [];
      const pendingCount = allSkills.filter(s => s.status === 'pending_review').length;
      const publishedCount = allTotal - pendingCount;

      stats = {
        total: allTotal,
        pending: pendingCount,
        published: publishedCount >= 0 ? publishedCount : 0
      };

      sandboxHealth = sandboxRes;

      // 审计日志独立请求，权限不足时静默
      try {
        const logsRes = await api.listAuditLogs({ limit: 10 });
        recentLogs = logsRes.data || [];
      } catch {}
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
      <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">{$_('stats.title')}</h1>
      <p class="text-gray-500 text-sm mt-1.5 font-medium">{$_('stats.overview')}</p>
    </div>
  </div>

  {#if quickActions.length > 0}
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-8">
    {#each quickActions as action (action.key)}
      <a
        href={action.href}
        class="bg-white rounded-xl border border-gray-200 shadow-card p-5 flex items-center gap-4 hover:shadow-md hover:border-blue-200 transition-all group"
      >
        <div class="w-10 h-10 rounded-lg bg-blue-50 flex items-center justify-center flex-shrink-0 group-hover:bg-blue-100 transition-colors">
          <Icon name={action.icon} size="w-5 h-5" className="text-blue-600" />
        </div>
        <span class="text-sm font-semibold text-gray-700 group-hover:text-blue-600 transition-colors">
          {$_(action.labelKey)}
        </span>
      </a>
    {/each}
  </div>
  {/if}

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if stats}
    <div class="grid grid-cols-1 md:grid-cols-3 gap-5 mb-8">
      <StatCard
        variant="brand"
        title={$_('skills.title')}
        value={stats.total}
        subtitle={$_('stats.totalSkills')}
        icon='<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"/>'
      />
      <StatCard
        variant="amber"
        title={$_('skills.pending')}
        value={stats.pending}
        subtitle={$_('review.pendingReviews')}
        icon='<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>'
      />
      <StatCard
        variant="green"
        title={$_('skills.published')}
        value={stats.published}
        subtitle={$_('skills.published')}
        icon='<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>'
      />
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-5 mb-8">
      <div class="lg:col-span-2 bg-white rounded-xl border border-gray-200 shadow-card overflow-hidden">
        <div class="px-6 py-4 border-b border-gray-100 flex items-center justify-between">
          <div>
            <h2 class="font-semibold text-gray-900 text-sm">{$_('stats.recentActivity')}</h2>
            <p class="text-gray-400 text-xs mt-0.5">{$_('stats.latestAuditEntries')}</p>
          </div>
          <span class="w-2 h-2 rounded-full bg-emerald-500 pulse-dot"></span>
        </div>
        {#if recentLogs.length > 0}
          <div class="overflow-x-auto">
            <AuditTable logs={recentLogs} />
          </div>
        {:else}
          <div class="p-12 text-center">
            <p class="text-gray-400 text-sm">{$_('stats.noRecentActivity')}</p>
          </div>
        {/if}
      </div>

      <div class="flex flex-col gap-5">
        <!-- System Health -->
        <div class="bg-white rounded-xl border border-gray-200 shadow-card p-6">
          <h3 class="font-semibold text-gray-900 text-sm mb-4">{$_('stats.systemHealth')}</h3>
          <div class="space-y-3">
            <div class="flex items-center justify-between py-2">
              <div class="flex items-center gap-2">
                <Icon name="sandbox" size="w-4 h-4" className="text-gray-400" />
                <span class="text-gray-500 text-sm">{$_('stats.sandbox')}</span>
              </div>
              {#if sandboxHealth}
                <span class="inline-flex items-center gap-1.5 text-xs font-semibold {sandboxHealth.docker_connected ? 'text-emerald-600' : 'text-amber-600'}">
                  <span class="w-1.5 h-1.5 rounded-full {sandboxHealth.docker_connected ? 'bg-emerald-500' : 'bg-amber-500'}"></span>
                  {sandboxHealth.docker_connected ? $_('stats.connected') : $_('stats.disconnected')}
                </span>
              {:else}
                <span class="text-gray-400 text-xs">{$_('stats.unavailable')}</span>
              {/if}
            </div>
            {#if sandboxHealth?.active_containers !== undefined}
              <hr class="divider-soft" />
              <div class="flex items-center justify-between py-2">
                <span class="text-gray-500 text-sm">{$_('stats.activeContainers')}</span>
                <span class="font-semibold text-blue-600 text-sm">{sandboxHealth.active_containers}</span>
              </div>
            {/if}
          </div>
        </div>

        <!-- Quick Stats -->
        <div class="bg-white rounded-xl border border-gray-200 shadow-card p-6">
          <h3 class="font-semibold text-gray-900 text-sm mb-4">{$_('stats.quickStats')}</h3>
          <div class="space-y-4">
          <div class="flex items-center justify-between py-2">
            <span class="text-gray-500 text-sm">{$_('stats.approvalRate')}</span>
            <span class="font-semibold text-emerald-600 text-sm">
              {stats.total > 0 ? ((stats.published / stats.total) * 100).toFixed(0) : 0}%
            </span>
          </div>
          <hr class="divider-soft" />
          <div class="flex items-center justify-between py-2">
            <span class="text-gray-500 text-sm">{$_('stats.pendingQueue')}</span>
            <span class="font-semibold text-amber-600 text-sm">{stats.pending}</span>
          </div>
          <hr class="divider-soft" />
          <div class="flex items-center justify-between py-2">
            <span class="text-gray-500 text-sm">{$_('stats.totalPublished')}</span>
            <span class="font-semibold text-blue-600 text-sm">{stats.published}</span>
          </div>
          <hr class="divider-soft" />
          <div class="flex items-center justify-between py-2">
            <span class="text-gray-500 text-sm">{$_('stats.totalSkills')}</span>
            <span class="font-semibold text-gray-900 text-sm">{stats.total}</span>
          </div>
        </div>
      </div>
      </div>
    </div>
  {/if}
</div>