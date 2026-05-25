<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import SkillRow from '../components/SkillRow.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import Badge from '../components/Badge.svelte';

  let skills = [];
  let loading = true;
  let error = '';

  onMount(async () => {
    try {
      const res = await api.listSkills({ status: 'pending_review', limit: 50 });
      skills = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-6">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-2xl font-semibold">Review Queue</h1>
    <Badge status="pending_review" />
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="text-red-500">{error}</div>
  {:else if skills.length === 0}
    <EmptyState message="No pending skills to review" />
  {:else}
    <div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
      <table class="w-full">
        <thead class="bg-gray-50">
          <tr>
            <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Name</th>
            <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Agent</th>
            <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Tags</th>
            <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Created</th>
            <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Actions</th>
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
