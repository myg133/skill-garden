<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import Badge from '../components/Badge.svelte';
  import ReviewActions from '../components/ReviewActions.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  export let id;

  let skill = null;
  let stats = null;
  let loading = true;
  let error = '';

  onMount(async () => {
    try {
      const [skillRes, statsRes] = await Promise.all([
        api.getSkill(id),
        api.getSkillStats(id)
      ]);
      skill = skillRes.data;
      stats = statsRes.data;
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-8">
  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if skill}
    <div class="page-header">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
          <div class="w-12 h-12 rounded-2xl gradient-brand flex items-center justify-center font-bold text-lg shadow-glow">
            {skill.name[0]?.toUpperCase() || 'S'}
          </div>
          <div>
            <div class="flex items-center gap-3 mb-1">
              <h1 class="text-[28px] font-extrabold text-surface-900 tracking-tight">{skill.name}</h1>
              <Badge status={skill.status} />
            </div>
            <p class="text-surface-400 text-sm font-medium">Skill details and statistics</p>
          </div>
        </div>
        <ReviewActions {skill} />
      </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
      <div class="gradient-card-brand-light rounded-2xl border border-brand-200/60 p-5 card">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 rounded-xl bg-brand-100 flex items-center justify-center">
            <svg class="w-4 h-4 text-brand-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
            </svg>
          </div>
          <span class="text-surface-500 text-[11px] font-semibold uppercase tracking-wider">Installs</span>
        </div>
        <p class="text-[28px] font-extrabold text-brand-600 stat-number">{stats?.install_count || 0}</p>
      </div>

      <div class="gradient-card-purple-light rounded-2xl border border-purple-200/60 p-5 card">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 rounded-xl bg-purple-100 flex items-center justify-center">
            <svg class="w-4 h-4 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/>
            </svg>
          </div>
          <span class="text-surface-500 text-[11px] font-semibold uppercase tracking-wider">Evaluations</span>
        </div>
        <p class="text-[28px] font-extrabold text-purple-600 stat-number">{stats?.evaluation_count || 0}</p>
      </div>

      <div class="gradient-card-green-light rounded-2xl border border-emerald-200/60 p-5 card">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 rounded-xl bg-emerald-100 flex items-center justify-center">
            <svg class="w-4 h-4 text-emerald-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
            </svg>
          </div>
          <span class="text-surface-500 text-[11px] font-semibold uppercase tracking-wider">Success Rate</span>
        </div>
        <p class="text-[28px] font-extrabold text-emerald-600 stat-number">{((stats?.success_rate || 0) * 100).toFixed(1)}%</p>
      </div>

      <div class="gradient-card-amber-light rounded-2xl border border-amber-200/60 p-5 card">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 rounded-xl bg-amber-100 flex items-center justify-center">
            <svg class="w-4 h-4 text-amber-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"/>
            </svg>
          </div>
          <span class="text-surface-500 text-[11px] font-semibold uppercase tracking-wider">Confidence</span>
        </div>
        <p class="text-[28px] font-extrabold text-amber-600 stat-number">{(stats?.confidence || 0).toFixed(2)}</p>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-5 mb-5">
      <div class="gradient-card-sky-light rounded-2xl border border-sky-200/60 shadow-card">
        <div class="px-6 py-4 border-b border-sky-200/60">
          <h2 class="font-semibold text-surface-800 text-sm">Description</h2>
        </div>
        <div class="p-6">
          <p class="text-surface-600 text-sm leading-relaxed">{skill.description || 'No description'}</p>
        </div>
      </div>

      <div class="gradient-card-rose-light rounded-2xl border border-rose-200/60 shadow-card">
        <div class="px-6 py-4 border-b border-rose-200/60">
          <h2 class="font-semibold text-surface-800 text-sm">Tags</h2>
        </div>
        <div class="p-6">
          {#if (skill.tags || []).length > 0}
            <div class="flex gap-2 flex-wrap">
              {#each skill.tags as tag}
                <span class="px-3 py-1.5 bg-slate-100 text-surface-600 text-xs font-medium rounded-lg border border-indigo-200 tag-pill">
                  {tag}
                </span>
              {/each}
            </div>
          {:else}
            <p class="text-surface-400 text-sm">No tags</p>
          {/if}
        </div>
      </div>
    </div>

    <div class="gradient-card-brand-light rounded-2xl border border-brand-200/60 shadow-card">
      <div class="px-6 py-4 border-b border-brand-200/60">
        <h2 class="font-semibold text-surface-800 text-sm">Content Preview</h2>
      </div>
      <div class="p-6">
        <pre class="whitespace-pre-wrap text-sm text-surface-600 bg-slate-100 p-5 rounded-xl overflow-auto max-h-96 font-mono text-[13px] leading-relaxed border border-indigo-200">{(skill.content || '').slice(0, 2000)}{(skill.content || '').length > 2000 ? '\n\n...' : ''}</pre>
      </div>
    </div>
  {/if}
</div>