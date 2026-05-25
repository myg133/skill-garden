<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import Badge from '../components/Badge.svelte';
  import StatCard from '../components/StatCard.svelte';
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

<div class="p-6">
  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="text-red-500">{error}</div>
  {:else if skill}
    <div class="flex items-center justify-between mb-6">
      <div class="flex items-center gap-4">
        <h1 class="text-2xl font-semibold">{skill.name}</h1>
        <Badge status={skill.status} />
      </div>
      <ReviewActions {skill} />
    </div>

    <div class="grid grid-cols-4 gap-4 mb-6">
      <StatCard title="Installs" value={stats?.install_count || 0} />
      <StatCard title="Evaluations" value={stats?.evaluation_count || 0} />
      <StatCard title="Success Rate" value="{((stats?.success_rate || 0) * 100).toFixed(1)}%" />
      <StatCard title="Confidence" value={(stats?.confidence || 0).toFixed(2)} />
    </div>

    <div class="bg-white rounded-lg border border-gray-200 p-6 mb-6">
      <h2 class="text-lg font-medium mb-4">Description</h2>
      <p class="text-gray-700">{skill.description}</p>
    </div>

    <div class="bg-white rounded-lg border border-gray-200 p-6 mb-6">
      <h2 class="text-lg font-medium mb-4">Tags</h2>
      <div class="flex gap-2 flex-wrap">
        {#each skill.tags || [] as tag}
          <span class="px-3 py-1 bg-gray-100 text-gray-700 rounded-full text-sm">{tag}</span>
        {/each}
      </div>
    </div>

    <div class="bg-white rounded-lg border border-gray-200 p-6">
      <h2 class="text-lg font-medium mb-4">Content Preview</h2>
      <pre class="whitespace-pre-wrap text-sm text-gray-600 bg-gray-50 p-4 rounded overflow-auto max-h-64">{(skill.content || '').slice(0, 1000)}{(skill.content || '').length > 1000 ? '...' : ''}</pre>
    </div>
  {/if}
</div>