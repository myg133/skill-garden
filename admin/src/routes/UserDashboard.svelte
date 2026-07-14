<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { auth } from '../stores/auth.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  let skills = [];
  let userOrgs = [];
  let loading = true;
  let error = '';

  $: totalSkills = skills.length;
  $: publishedCount = skills.filter(s => s.status === 'published').length;
  $: pendingCount = skills.filter(s => s.status === 'pending_review').length;
  $: rejectedCount = skills.filter(s => s.status === 'rejected').length;

  onMount(async () => {
    try {
      const [skillsRes, orgsRes] = await Promise.all([
        api.listMySkills().catch(() => []),
        api.getUserOrgs().catch(() => []),
      ]);
      skills = Array.isArray(skillsRes) ? skillsRes : (skillsRes.data || []);
      userOrgs = Array.isArray(orgsRes) ? orgsRes : [];
    } catch (e) {
      error = e.message || 'Failed to load data';
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-8 max-w-5xl mx-auto">
  <!-- Welcome -->
  <div class="mb-8">
    <h1 class="text-2xl font-bold text-gray-900">Welcome, {$auth.username}</h1>
    <p class="text-gray-500 mt-1 text-sm">Your skill dashboard</p>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-red-50 border border-red-200 rounded-xl p-4 text-red-700 text-sm">{error}</div>
  {:else}
    <!-- Stats Cards -->
    <div class="grid grid-cols-2 sm:grid-cols-4 gap-4 mb-8">
      <div class="bg-white rounded-xl border border-gray-200 p-5">
        <p class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-1">Total Skills</p>
        <p class="text-2xl font-bold text-gray-900">{totalSkills}</p>
      </div>
      <div class="bg-white rounded-xl border border-gray-200 p-5">
        <p class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-1">Published</p>
        <p class="text-2xl font-bold text-emerald-600">{publishedCount}</p>
      </div>
      <div class="bg-white rounded-xl border border-gray-200 p-5">
        <p class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-1">Pending</p>
        <p class="text-2xl font-bold text-amber-600">{pendingCount}</p>
      </div>
      <div class="bg-white rounded-xl border border-gray-200 p-5">
        <p class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-1">Rejected</p>
        <p class="text-2xl font-bold text-red-600">{rejectedCount}</p>
      </div>
    </div>

    <!-- Organizations -->
    {#if userOrgs.length > 0}
      <div>
        <h2 class="text-sm font-semibold text-gray-500 uppercase tracking-wider mb-3">Your Organizations</h2>
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {#each userOrgs as org}
            <div class="bg-white rounded-xl border border-gray-200 p-4">
              <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-lg bg-indigo-50 flex items-center justify-center text-indigo-600 font-bold text-sm">
                  {org.name?.[0]?.toUpperCase() || '?'}
                </div>
                <div>
                  <p class="font-semibold text-sm text-gray-900">{org.name}</p>
                  <p class="text-xs text-gray-400 capitalize">{org.role || 'member'}</p>
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>
