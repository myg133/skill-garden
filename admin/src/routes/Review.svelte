<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import SkillRow from '../components/SkillRow.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  let skills = [];
  let loading = true;
  let error = '';

  onMount(async () => {
    try {
      const res = await api.listSkills({ page_size: 100 });
      skills = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-8">
  <div class="page-header flex items-center justify-between">
    <div>
      <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">Review Queue</h1>
      <p class="text-surface-500 text-sm mt-1.5 font-medium">Review and approve pending skill submissions</p>
    </div>
    <div class="flex items-center gap-3">
      <span class="inline-flex items-center gap-1.5 px-3.5 py-2 bg-amber-50 text-amber-700 rounded-xl text-sm font-semibold ring-1 ring-amber-600/20">
        <span class="w-1.5 h-1.5 rounded-full bg-amber-500 pulse-dot"></span>
        {skills.length} pending
      </span>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if skills.length === 0}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 shadow-card">
      <EmptyState message="No pending skills to review" />
    </div>
  {:else}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 overflow-hidden shadow-card">
      <table class="w-full">
        <thead>
          <tr class="border-b border-surface-100 bg-gradient-to-r from-surface-50/80 to-transparent">
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Name</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Agent</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Tags</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Created</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Actions</th>
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
</div>