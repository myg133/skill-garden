<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { hasPermission } from '../stores/permission.js';
  import SkillRow from '../components/SkillRow.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  let skills = [];
  let marketplaceSkills = [];
  let delistRequestSkills = [];
  let loading = true;
  let error = '';
  let activeQueue = 'internal'; // 'internal' | 'marketplace' | 'delist'

  $: canReviewInternal = hasPermission('skill:approve_review') || hasPermission('skill:reject_review');
  $: canReviewMarketplace = hasPermission('marketplace:review_approve') || hasPermission('marketplace:review_reject');

  onMount(async () => {
    try {
      const res = await api.listSkills({ page_size: 200 });
      const allSkills = res.data || [];
      skills = allSkills.filter(s => s.status === 'pending_review');
      marketplaceSkills = allSkills.filter(s => s.marketplace_status === 'pending_review');
      delistRequestSkills = allSkills.filter(s => s.marketplace_status === 'pending_delist');
      // Auto-select active queue based on permissions
      if (!canReviewInternal && canReviewMarketplace) activeQueue = 'marketplace';
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  });

  async function handleMarketplaceApprove(skillId, skillName) {
    try {
      await api.marketplaceReviewApprove(skillId);
      marketplaceSkills = marketplaceSkills.filter(s => s.id !== skillId);
    } catch (e) {
      // error handled by global error handler
    }
  }

  async function handleMarketplaceReject(skillId, skillName) {
    try {
      await api.marketplaceReviewReject(skillId);
      marketplaceSkills = marketplaceSkills.filter(s => s.id !== skillId);
    } catch (e) {
      // error handled by global error handler
    }
  }

  async function handleApproveDelist(skillId, skillName) {
    try {
      await api.marketplaceApproveDelist(skillId);
      delistRequestSkills = delistRequestSkills.filter(s => s.id !== skillId);
    } catch (e) {
      // error handled by global error handler
    }
  }

  async function handleRejectDelist(skillId, skillName) {
    try {
      await api.marketplaceRejectDelist(skillId);
      delistRequestSkills = delistRequestSkills.filter(s => s.id !== skillId);
    } catch (e) {
      // error handled by global error handler
    }
  }
</script>

<div class="p-8">
  <div class="page-header flex items-center justify-between">
    <div>
      <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">Review Queue</h1>
      <p class="text-gray-500 text-sm mt-1.5 font-medium">Review pending skill submissions</p>
    </div>
    <div class="flex items-center gap-3">
      {#if canReviewInternal && skills.length > 0}
        <span class="inline-flex items-center gap-1.5 px-3.5 py-2 bg-amber-50 text-amber-700 rounded-xl text-sm font-semibold ring-1 ring-amber-600/20">
          <span class="w-1.5 h-1.5 rounded-full bg-amber-500 pulse-dot"></span>
          Internal: {skills.length}
        </span>
      {/if}
      {#if canReviewMarketplace && marketplaceSkills.length > 0}
        <span class="inline-flex items-center gap-1.5 px-3.5 py-2 bg-blue-50 text-blue-700 rounded-xl text-sm font-semibold ring-1 ring-blue-600/20">
          <span class="w-1.5 h-1.5 rounded-full bg-blue-500 pulse-dot"></span>
          Market: {marketplaceSkills.length}
        </span>
      {/if}
      {#if canReviewMarketplace && delistRequestSkills.length > 0}
        <span class="inline-flex items-center gap-1.5 px-3.5 py-2 bg-orange-50 text-orange-700 rounded-xl text-sm font-semibold ring-1 ring-orange-600/20">
          <span class="w-1.5 h-1.5 rounded-full bg-orange-500 pulse-dot"></span>
          Delist: {delistRequestSkills.length}
        </span>
      {/if}
    </div>
  </div>

  <!-- Queue tabs -->
  {#if canReviewInternal && canReviewMarketplace}
    <div class="flex gap-2 mb-6">
      <button
        on:click={() => activeQueue = 'internal'}
        class="px-4 py-2 text-sm font-semibold rounded-xl transition-all duration-200 {activeQueue === 'internal' ? 'bg-amber-500 text-white shadow-sm shadow-amber-500/20' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}"
      >
        Internal Review ({skills.length})
      </button>
      <button
        on:click={() => activeQueue = 'marketplace'}
        class="px-4 py-2 text-sm font-semibold rounded-xl transition-all duration-200 {activeQueue === 'marketplace' ? 'bg-blue-500 text-white shadow-sm shadow-blue-500/20' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}"
      >
        Marketplace Review ({marketplaceSkills.length})
      </button>
      <button
        on:click={() => activeQueue = 'delist'}
        class="px-4 py-2 text-sm font-semibold rounded-xl transition-all duration-200 {activeQueue === 'delist' ? 'bg-orange-500 text-white shadow-sm shadow-orange-500/20' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}"
      >
        Delist Requests ({delistRequestSkills.length})
      </button>
    </div>
  {/if}

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else}
    <!-- Internal Review Queue -->
    {#if activeQueue === 'internal'}
      {#if !canReviewInternal}
        <div class="bg-white rounded-xl border border-gray-200 shadow-card">
          <EmptyState message="No permission to review internal submissions" />
        </div>
      {:else if skills.length === 0}
        <div class="bg-white rounded-xl border border-gray-200 shadow-card">
          <EmptyState message="No pending internal reviews" />
        </div>
      {:else}
        <div class="bg-white rounded-xl border border-gray-200 overflow-hidden shadow-card">
          <table class="w-full">
            <thead>
              <tr class="border-b border-gray-100 bg-gray-50">
                <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Name</th>
                <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Agent</th>
                <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Type</th>
                <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Tags</th>
                <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Created</th>
                <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Actions</th>
              </tr>
            </thead>
            <tbody>
              {#each skills as skill (skill.id)}
                <SkillRow {skill} />
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {:else if activeQueue === 'marketplace'}
      <!-- Marketplace Review Queue -->
      {#if !canReviewMarketplace}
        <div class="bg-white rounded-xl border border-gray-200 shadow-card">
          <EmptyState message="No permission to review marketplace submissions" />
        </div>
      {:else if marketplaceSkills.length === 0}
        <div class="bg-white rounded-xl border border-gray-200 shadow-card">
          <EmptyState message="No pending marketplace reviews" />
        </div>
      {:else}
        <div class="space-y-4">
          {#each marketplaceSkills as skill (skill.id)}
            <div class="bg-white rounded-xl border border-gray-200 shadow-card p-6">
              <div class="flex items-center justify-between">
                <div>
                  <h3 class="text-lg font-bold text-gray-800">{skill.name}</h3>
                  <p class="text-sm text-gray-500 mt-1">{skill.description || 'No description'}</p>
                  <div class="flex items-center gap-3 mt-3 text-xs text-gray-400">
                    <span>v{skill.version || '1.0.0'}</span>
                    <span>by {skill.author_name || 'Unknown'}</span>
                    <span>{new Date(skill.created_at).toLocaleDateString()}</span>
                  </div>
                  {#if skill.tags && skill.tags.length > 0}
                    <div class="flex gap-1.5 mt-2 flex-wrap">
                      {#each skill.tags as tag}
                        <span class="px-2 py-0.5 bg-gray-100 text-gray-600 rounded-md text-xs font-medium">{tag}</span>
                      {/each}
                    </div>
                  {/if}
                </div>
                <div class="flex items-center gap-2">
                  <a
                    href={`/skills/${skill.id}`}
                    class="px-3 py-1.5 text-xs font-semibold bg-gray-100 text-gray-600 rounded-lg hover:bg-gray-200 transition-colors"
                  >
                    View
                  </a>
                  <button
                    on:click={() => handleMarketplaceApprove(skill.id, skill.name)}
                    class="px-4 py-1.5 text-xs font-semibold bg-emerald-500 text-white rounded-lg hover:bg-emerald-600 transition-all duration-200 shadow-sm shadow-emerald-500/20 active:scale-[0.97]"
                  >
                    通过
                  </button>
                  <button
                    on:click={() => handleMarketplaceReject(skill.id, skill.name)}
                    class="px-4 py-1.5 text-xs font-semibold bg-rose-500 text-white rounded-lg hover:bg-rose-600 transition-all duration-200 shadow-sm shadow-rose-500/20 active:scale-[0.97]"
                  >
                    驳回
                  </button>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {:else if activeQueue === 'delist'}
      <!-- Delist Request Queue -->
      {#if !canReviewMarketplace}
        <div class="bg-white rounded-xl border border-gray-200 shadow-card">
          <EmptyState message="No permission to review delist requests" />
        </div>
      {:else if delistRequestSkills.length === 0}
        <div class="bg-white rounded-xl border border-gray-200 shadow-card">
          <EmptyState message="No pending delist requests" />
        </div>
      {:else}
        <div class="space-y-4">
          {#each delistRequestSkills as skill (skill.id)}
            <div class="bg-white rounded-xl border border-gray-200 shadow-card p-6">
              <div class="flex items-center justify-between">
                <div>
                  <h3 class="text-lg font-bold text-gray-800">{skill.name}</h3>
                  <p class="text-sm text-gray-500 mt-1">{skill.description || 'No description'}</p>
                  <div class="flex items-center gap-3 mt-3 text-xs text-gray-400">
                    <span>v{skill.version || '1.0.0'}</span>
                    <span>by {skill.author_name || 'Unknown'}</span>
                    <span>{new Date(skill.created_at).toLocaleDateString()}</span>
                  </div>
                  <div class="mt-2">
                    <span class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-semibold rounded-full bg-orange-50 text-orange-600 ring-1 ring-orange-600/20">
                      <span class="w-1.5 h-1.5 rounded-full bg-orange-500 pulse-dot"></span>
                      Delist Request
                    </span>
                  </div>
                  {#if skill.tags && skill.tags.length > 0}
                    <div class="flex gap-1.5 mt-2 flex-wrap">
                      {#each skill.tags as tag}
                        <span class="px-2 py-0.5 bg-gray-100 text-gray-600 rounded-md text-xs font-medium">{tag}</span>
                      {/each}
                    </div>
                  {/if}
                </div>
                <div class="flex items-center gap-2">
                  <a
                    href={`/skills/${skill.id}`}
                    class="px-3 py-1.5 text-xs font-semibold bg-gray-100 text-gray-600 rounded-lg hover:bg-gray-200 transition-colors"
                  >
                    View
                  </a>
                  <button
                    on:click={() => handleApproveDelist(skill.id, skill.name)}
                    class="px-4 py-1.5 text-xs font-semibold bg-emerald-500 text-white rounded-lg hover:bg-emerald-600 transition-all duration-200 shadow-sm shadow-emerald-500/20 active:scale-[0.97]"
                  >
                    批准下架
                  </button>
                  <button
                    on:click={() => handleRejectDelist(skill.id, skill.name)}
                    class="px-4 py-1.5 text-xs font-semibold bg-rose-500 text-white rounded-lg hover:bg-rose-600 transition-all duration-200 shadow-sm shadow-rose-500/20 active:scale-[0.97]"
                  >
                    驳回
                  </button>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  {/if}
</div>