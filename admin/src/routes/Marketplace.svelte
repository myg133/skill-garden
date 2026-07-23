<script>
  import { onMount } from 'svelte';
  import { Link } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { isAdmin } from '../stores/auth.js';
  import { hasPermission, permissionStore, isAnyAdmin } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import Badge from '../components/Badge.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  $: skillLinkBase = ($isAdmin || ($permissionStore.loaded && (isAnyAdmin() || ($permissionStore.orgRoles || []).length > 0))) ? '/skills' : '/user/skills';
  const ACT = ACTIONS.Marketplace;

  // Role detection for delist
  $: systemRoles = $permissionStore.systemRoles || [];
  $: isMarketplaceAdmin = systemRoles.includes('marketplace_admin');
  $: isMarketplaceReviewer = systemRoles.includes('marketplace_reviewer');
  $: canDelist = isMarketplaceAdmin || isMarketplaceReviewer || hasPermission(ACT.unfeature);

  let skills = [];
  let loading = true;
  let error = '';
  let keyword = '';
  let tagFilter = '';
  let page = 1;
  let total = 0;
  let pageSize = 20;
  let allTags = [];

  onMount(() => {
    loadSkills();
  });

  async function loadSkills() {
    loading = true;
    error = '';
    try {
      const params = { limit: pageSize, offset: (page - 1) * pageSize };
      if (keyword.trim()) params.keyword = keyword.trim();
      if (tagFilter) params.tag = tagFilter;

      const res = await api.listMarketplaceSkills(params);
      skills = Array.isArray(res) ? res : (res.data || []);
      total = res.total || skills.length;

      if (allTags.length === 0 && skills.length > 0) {
        const tagsSet = new Set();
        skills.forEach(s => (s.tags || []).forEach(t => tagsSet.add(t)));
        allTags = [...tagsSet].sort();
      }
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  function handleSearch() {
    page = 1;
    loadSkills();
  }

  function handleTagFilter(tag) {
    tagFilter = tag;
    page = 1;
    loadSkills();
  }

  function handleClearFilters() {
    keyword = '';
    tagFilter = '';
    page = 1;
    loadSkills();
  }

  function goToPage(p) {
    page = p;
    loadSkills();
  }

  function handleKeydown(e) {
    if (e.key === 'Enter') handleSearch();
  }

  async function handleDelist(skill) {
    if (!confirm(`确定要将 "${skill.name}" 从市场中下架吗？该 Skill 将不再对公众可见。`)) return;
    try {
      if (isMarketplaceAdmin || isMarketplaceReviewer) {
        await api.marketplaceDelist(skill.id);
      } else {
        await api.adminUnpublishSkill(skill.id);
      }
      addToast(`${skill.name} 已下架`, 'success');
      skills = skills.filter(s => s.id !== skill.id);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  function fmtDate(d) {
    if (!d) return 'N/A';
    return new Date(d).toLocaleDateString();
  }

  function truncate(text, max = 80) {
    if (!text) return '';
    return text.length > max ? text.slice(0, max) + '...' : text;
  }

  $: totalPages = Math.max(1, Math.ceil(total / pageSize));
</script>

<div class="p-8">
  <div class="page-header flex items-center justify-between mb-6">
    <div>
      <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">Marketplace</h1>
      <p class="text-gray-500 text-sm mt-1.5 font-medium">Discover skills from the community</p>
    </div>
    <span class="inline-flex items-center gap-1.5 px-3.5 py-2 bg-white text-blue-700 rounded-xl text-sm font-semibold ring-1 ring-sky-600/20">
      <span class="w-1.5 h-1.5 rounded-full bg-blue-500"></span>
      {total} skills
    </span>
  </div>

  <!-- Search & Filters -->
  <div class="flex flex-wrap items-center gap-3 mb-6">
    <div class="relative flex-1 min-w-[280px] max-w-md">
      <svg class="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
      </svg>
      <input
        type="text"
        bind:value={keyword}
        on:keydown={handleKeydown}
        placeholder="Search skills by name or description..."
        class="w-full pl-10 pr-4 py-2.5 bg-white border border-gray-200 rounded-lg text-sm text-gray-900 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 transition-all"
      />
    </div>

    <select
      bind:value={tagFilter}
      on:change={() => handleTagFilter(tagFilter)}
      aria-label="Filter by tag"
      class="px-4 py-2.5 bg-white border border-gray-200 rounded-lg text-sm text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500/20 cursor-pointer"
    >
      <option value="" disabled selected hidden>Filter by tag</option>
      <option value="">All tags</option>
      {#each allTags as tag}
        <option value={tag}>{tag}</option>
      {/each}
    </select>

    <button
      on:click={handleSearch}
      class="px-5 py-2.5 bg-blue-600 text-white rounded-lg text-sm font-semibold hover:bg-blue-700 transition-colors shadow-sm"
    >
      Search
    </button>

    {#if keyword || tagFilter}
      <button
        on:click={handleClearFilters}
        class="px-4 py-2.5 text-gray-500 hover:text-gray-700 text-sm font-medium transition-colors"
      >
        Clear filters
      </button>
    {/if}
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-red-50 border border-red-100 text-red-600 px-5 py-4 rounded-xl text-sm font-medium">{error}</div>
  {:else if skills.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message="No marketplace skills available yet" />
    </div>
  {:else}
    <!-- Card Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
      {#each skills as skill (skill.id)}
        <div class="bg-white rounded-xl border border-gray-200 shadow-card hover:shadow-card-lg transition-all duration-200 hover:border-blue-200 flex flex-col overflow-hidden">
          <!-- Card header -->
          <div class="px-5 pt-5 pb-3 flex items-start justify-between">
            <div class="min-w-0 flex-1">
              <Link
                to="{skillLinkBase}/{skill.id}?from=marketplace"
                state={{ readonly: true }}
                class="text-base font-bold text-gray-900 hover:text-blue-600 transition-colors truncate block"
              >
                {skill.name}
              </Link>
              {#if skill.author_name || skill.author_agent_id}
                <p class="text-gray-400 text-xs mt-0.5">
                  by {skill.author_name || skill.author_agent_id}
                </p>
              {/if}
            </div>
            <Badge status={skill.status || 'published'} />
          </div>

          <!-- Description -->
          <div class="px-5 pb-3 flex-1">
            <p class="text-gray-500 text-sm leading-relaxed line-clamp-2">
              {truncate(skill.description, 120) || 'No description'}
            </p>
          </div>

          <!-- Tags -->
          {#if (skill.tags || []).length > 0}
            <div class="px-5 pb-3 flex flex-wrap gap-1.5">
              {#each (skill.tags || []).slice(0, 4) as tag}
                <span class="px-2 py-0.5 bg-blue-50 text-blue-600 text-[11px] font-medium rounded-full">{tag}</span>
              {/each}
              {#if (skill.tags || []).length > 4}
                <span class="px-2 py-0.5 bg-gray-100 text-gray-500 text-[11px] font-medium rounded-full">+{skill.tags.length - 4}</span>
              {/if}
            </div>
          {/if}

          <!-- Stats row -->
          <div class="px-5 pb-3 flex items-center gap-4 text-xs text-gray-400">
            <span class="flex items-center gap-1">
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/></svg>
              {skill.install_count || 0} installs
            </span>
            <span class="flex items-center gap-1">
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"/></svg>
              v{skill.version || '1.0.0'}
            </span>
            <span>{fmtDate(skill.created || skill.created_at)}</span>
          </div>

          <!-- Actions -->
          <div class="px-5 py-3 border-t border-gray-100 bg-gray-50 flex items-center justify-between">
            <Link
              to="{skillLinkBase}/{skill.id}?from=marketplace"
              state={{ readonly: true }}
              class="inline-flex items-center gap-1.5 px-4 py-2 text-blue-600 rounded-lg text-sm font-semibold hover:bg-blue-50 transition-colors"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/></svg>
              Details
            </Link>

            {#if canDelist}
              <button
                on:click={() => handleDelist(skill)}
                class="px-2.5 py-1 text-[11px] font-semibold bg-amber-100 text-amber-700 rounded-lg hover:bg-amber-200 transition-colors"
                title="Delist from marketplace"
              >下架</button>
            {/if}
          </div>
        </div>
      {/each}
    </div>

    <!-- Pagination -->
    {#if totalPages > 1}
      <div class="flex items-center justify-between mt-6 px-2">
        <span class="text-gray-500 text-sm">
          Page {page} of {totalPages} ({total} total)
        </span>
        <div class="flex gap-1.5">
          <button
            on:click={() => goToPage(page - 1)}
            disabled={page <= 1}
            class="px-3.5 py-2 bg-white border border-gray-200 rounded-lg text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
          >
            Previous
          </button>
          {#each Array(totalPages) as _, i}
            {@const pageNum = i + 1}
            {#if pageNum === 1 || pageNum === totalPages || (pageNum >= page - 2 && pageNum <= page + 2)}
              <button
                on:click={() => goToPage(pageNum)}
                class="w-9 h-9 rounded-lg text-sm font-semibold transition-colors {pageNum === page ? 'bg-blue-600 text-white shadow-sm' : 'bg-white border border-gray-200 text-gray-600 hover:bg-gray-50'}"
              >
                {pageNum}
              </button>
            {:else if pageNum === page - 3 || pageNum === page + 3}
              <span class="w-9 h-9 flex items-center justify-center text-gray-400 text-sm">...</span>
            {/if}
          {/each}
          <button
            on:click={() => goToPage(page + 1)}
            disabled={page >= totalPages}
            class="px-3.5 py-2 bg-white border border-gray-200 rounded-lg text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
          >
            Next
          </button>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
