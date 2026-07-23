<script>
  import { onMount } from 'svelte';
  import { Link } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  let skills = [];
  let loading = true;
  let error = '';
  let filter = 'all'; // all | draft | pending | published | rejected
  let actionLoading = {};

  $: filteredSkills = skills.filter(s => {
    if (filter === 'all') return true;
    if (filter === 'pending') return s.status === 'pending_review';
    return s.status === filter;
  });

  $: totalSkills = skills.length;
  $: publishedCount = skills.filter(s => s.status === 'published').length;
  $: pendingCount = skills.filter(s => s.status === 'pending_review').length;
  $: rejectedCount = skills.filter(s => s.status === 'rejected').length;
  $: draftCount = skills.filter(s => s.status === 'draft').length;

  onMount(loadData);

  async function loadData() {
    loading = true;
    error = '';
    try {
      skills = await api.listMySkills();
      if (!Array.isArray(skills)) skills = skills.data || [];
    } catch (e) {
      error = e.message || 'Failed to load submissions';
    } finally {
      loading = false;
    }
  }

  async function submitForReview(skillId) {
    actionLoading[skillId] = 'submit';
    actionLoading = actionLoading;
    try {
      await api.submitSkillForReview(skillId);
      addToast('Skill submitted for review', 'success');
      await loadData();
    } catch (e) {
      addToast(e.message || 'Failed to submit', 'error');
    } finally {
      actionLoading[skillId] = null;
      actionLoading = actionLoading;
    }
  }

  async function handlePublish(skillId) {
    actionLoading[skillId] = 'publish';
    actionLoading = actionLoading;
    try {
      await api.publishSkill(skillId);
      addToast('Skill published', 'success');
      await loadData();
    } catch (e) {
      addToast(e.message || 'Failed to publish', 'error');
    } finally {
      actionLoading[skillId] = null;
      actionLoading = actionLoading;
    }
  }

  function statusLabel(s) {
    const map = {
      draft: 'Draft',
      pending_review: 'Pending Review',
      approved: 'Approved',
      published: 'Published',
      rejected: 'Rejected',
    };
    return map[s] || s || 'Unknown';
  }

  function statusClass(s) {
    const map = {
      draft: 'bg-gray-100 text-gray-700',
      pending_review: 'bg-amber-100 text-amber-700',
      approved: 'bg-emerald-100 text-emerald-700',
      published: 'bg-emerald-100 text-emerald-700',
      rejected: 'bg-red-100 text-red-700',
    };
    return map[s] || 'bg-gray-100 text-gray-700';
  }
</script>

<div class="p-8 max-w-6xl mx-auto">
  <!-- Header -->
  <div class="flex items-center justify-between mb-6">
    <div>
      <h1 class="text-2xl font-bold text-gray-900">Submissions</h1>
      <p class="text-gray-500 text-sm mt-1">Manage your skills and track review status</p>
    </div>
    <Link to="/user/skills" class="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition-colors">
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
      </svg>
      New Skill
    </Link>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-red-50 border border-red-200 rounded-xl p-4 text-red-700 text-sm">{error}</div>
  {:else}
    <!-- Stats -->
    <div class="grid grid-cols-4 gap-4 mb-6">
      <button on:click={() => filter = 'all'} class="bg-white rounded-xl border {filter === 'all' ? 'border-indigo-300 ring-1 ring-indigo-100' : 'border-gray-200'} p-4 text-left hover:border-indigo-300 transition-all duration-200">
        <p class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-0.5">All</p>
        <p class="text-2xl font-bold text-gray-900">{totalSkills}</p>
      </button>
      <button on:click={() => filter = 'published'} class="bg-white rounded-xl border {filter === 'published' ? 'border-emerald-300 ring-1 ring-emerald-100' : 'border-gray-200'} p-4 text-left hover:border-emerald-300 transition-all duration-200">
        <p class="text-xs font-medium text-emerald-500 uppercase tracking-wider mb-0.5">Published</p>
        <p class="text-2xl font-bold text-emerald-600">{publishedCount}</p>
      </button>
      <button on:click={() => filter = 'pending'} class="bg-white rounded-xl border {filter === 'pending' ? 'border-amber-300 ring-1 ring-amber-100' : 'border-gray-200'} p-4 text-left hover:border-amber-300 transition-all duration-200">
        <p class="text-xs font-medium text-amber-500 uppercase tracking-wider mb-0.5">Pending</p>
        <p class="text-2xl font-bold text-amber-600">{pendingCount}</p>
      </button>
      <button on:click={() => filter = 'rejected'} class="bg-white rounded-xl border {filter === 'rejected' ? 'border-red-300 ring-1 ring-red-100' : 'border-gray-200'} p-4 text-left hover:border-red-300 transition-all duration-200">
        <p class="text-xs font-medium text-red-500 uppercase tracking-wider mb-0.5">Rejected</p>
        <p class="text-2xl font-bold text-red-600">{rejectedCount}</p>
      </button>
    </div>

    <!-- Skills Table -->
    {#if filteredSkills.length > 0}
      <div class="bg-white rounded-xl border border-gray-200 overflow-hidden">
        <div class="overflow-x-auto">
          <table class="w-full">
            <thead>
              <tr class="border-b border-gray-100 bg-gray-50/50">
                <th class="text-left text-xs font-semibold text-gray-400 uppercase tracking-wider px-6 py-3">Skill Name</th>
                <th class="text-left text-xs font-semibold text-gray-400 uppercase tracking-wider px-6 py-3">Version</th>
                <th class="text-left text-xs font-semibold text-gray-400 uppercase tracking-wider px-6 py-3">Status</th>
                <th class="text-left text-xs font-semibold text-gray-400 uppercase tracking-wider px-6 py-3">Visibility</th>
                <th class="text-left text-xs font-semibold text-gray-400 uppercase tracking-wider px-6 py-3">Created</th>
                <th class="text-right text-xs font-semibold text-gray-400 uppercase tracking-wider px-6 py-3">Actions</th>
              </tr>
            </thead>
            <tbody>
              {#each filteredSkills as skill}
                <tr class="border-b border-gray-50 hover:bg-gray-50/50 transition-colors">
                  <td class="px-6 py-3.5">
                    <Link to="/user/skills/{skill.id}?from=submissions" class="text-sm font-medium text-indigo-600 hover:text-indigo-700">{skill.name || 'Unnamed'}</Link>
                  </td>
                  <td class="px-6 py-3.5 text-xs text-gray-500">v{skill.version || '0.1.0'}</td>
                  <td class="px-6 py-3.5">
                    <span class="inline-flex items-center gap-1.5 text-xs font-medium {statusClass(skill.status)} rounded-full px-2.5 py-0.5">
                      {statusLabel(skill.status)}
                    </span>
                    {#if skill.status === 'rejected' && skill.reject_reason}
                      <span class="block text-[10px] text-red-500 mt-0.5" title={skill.reject_reason}>Reason: {skill.reject_reason}</span>
                    {/if}
                  </td>
                  <td class="px-6 py-3.5 text-xs text-gray-500 capitalize">{skill.visibility || 'public'}</td>
                  <td class="px-6 py-3.5 text-xs text-gray-400">
                    {skill.created_at ? new Date(skill.created_at).toLocaleDateString() : '-'}
                  </td>
                  <td class="px-6 py-3.5 text-right">
                    <div class="flex items-center justify-end gap-2">
                      {#if skill.status === 'draft' || skill.status === 'rejected'}
                        <button
                          on:click={() => submitForReview(skill.id)}
                          disabled={actionLoading[skill.id]}
                          class="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-amber-50 text-amber-700 border border-amber-200 hover:bg-amber-100 disabled:opacity-50 transition-colors"
                        >
                          {#if actionLoading[skill.id] === 'submit'}
                            <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>
                          {/if}
                          Submit for Review
                        </button>
                      {/if}
                      {#if skill.status === 'approved'}
                        <button
                          on:click={() => handlePublish(skill.id)}
                          disabled={actionLoading[skill.id]}
                          class="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-emerald-50 text-emerald-700 border border-emerald-200 hover:bg-emerald-100 disabled:opacity-50 transition-colors"
                        >
                          {#if actionLoading[skill.id] === 'publish'}
                            <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>
                          {/if}
                          Publish
                        </button>
                      {/if}
                      {#if skill.status === 'pending_review'}
                        <span class="text-[10px] text-amber-500 italic">Awaiting review</span>
                      {/if}
                      {#if skill.status === 'published'}
                        <Link to="/user/skills/{skill.id}?from=submissions" class="text-xs text-indigo-600 hover:text-indigo-700 font-medium">
                          View
                        </Link>
                      {/if}
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>
    {:else}
      <div class="bg-white rounded-xl border border-dashed border-gray-300 p-12 text-center">
        <div class="w-12 h-12 rounded-full bg-gray-100 flex items-center justify-center mx-auto mb-4">
          <svg class="w-6 h-6 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
          </svg>
        </div>
        {#if filter !== 'all'}
          <p class="text-sm text-gray-500 mb-1">No {statusLabel(filter)} skills</p>
          <p class="text-xs text-gray-400 mb-4">Try a different filter or create a new skill</p>
          <button on:click={() => filter = 'all'} class="text-sm text-indigo-600 hover:text-indigo-700 font-medium">Show all</button>
        {:else}
          <p class="text-sm text-gray-500 mb-1">No submissions yet</p>
          <p class="text-xs text-gray-400 mb-4">Upload your first skill to get started</p>
          <Link to="/user/skills" class="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition-colors">
            Create Skill
          </Link>
        {/if}
      </div>
    {/if}
  {/if}
</div>
