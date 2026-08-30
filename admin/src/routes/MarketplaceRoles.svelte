<script>
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission } from '../stores/permission.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  const ROLE_LABEL = 'Marketplace Reviewer';

  let reviewers = [];
  let identities = [];
  let loading = true;
  let error = '';

  // Add modal
  let showAddModal = false;
  let addEmail = '';
  let adding = false;

  let identityMap = {};

  onMount(() => loadAll());

  async function loadAll() {
    loading = true;
    error = '';
    try {
      const [revRes, identRes] = await Promise.all([
        api.listMarketplaceReviewers().catch(() => ({ data: [] })),
        api.listIdentities({ limit: 200 }).catch(() => ({ data: [] })),
      ]);

      const raw = revRes.data || revRes || [];
      reviewers = Array.isArray(raw) ? raw : [];

      const identList = identRes.data || [];
      identities = Array.isArray(identList) ? identList : [];
      identityMap = {};
      for (const i of identities) {
        identityMap[i.id] = { name: i.name, email: i.email || '' };
      }
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  function getIdentityInfo(identityId) {
    return identityMap[identityId] || { name: identityId?.substring(0, 8) || '?', email: '' };
  }

  function openAddModal() {
    addEmail = '';
    showAddModal = true;
  }

  async function handleAdd() {
    if (!addEmail.trim()) return;
    adding = true;
    try {
      const identity = identities.find(i =>
        i.email && i.email.toLowerCase() === addEmail.trim().toLowerCase()
      );
      if (!identity) {
        addToast($_('marketplace.userNotFound'), 'error');
        adding = false;
        return;
      }

      const existing = reviewers.find(r => r.identity_id === identity.id);
      if (existing) {
        addToast($_('marketplace.alreadyReviewer', { values: { name: identity.name } }), 'warning');
        adding = false;
        return;
      }

      await api.assignMarketplaceReviewer(identity.id);
      addToast($_('marketplace.reviewerAssigned', { values: { name: identity.name } }), 'success');
      showAddModal = false;
      await loadAll();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      adding = false;
    }
  }

  async function handleRemove(reviewer) {
    const info = getIdentityInfo(reviewer.identity_id);
    if (!confirm($_('marketplace.confirmRemove', { values: { name: info.name, role: ROLE_LABEL } }))) return;

    try {
      await api.revokeMarketplaceReviewer(reviewer.identity_id);
      addToast($_('marketplace.reviewerRemoved', { values: { name: info.name } }), 'success');
      await loadAll();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }
</script>

<div class="p-8">
  <div class="page-header flex items-center justify-between">
    <div>
      <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">{$_('marketplace.marketplaceReviewers')}</h1>
      <p class="text-gray-500 text-sm mt-1.5 font-medium">
        {$_('marketplace.manageReviewers')}
      </p>
    </div>
    {#if hasPermission('marketplace:role_assign')}
      <button
        on:click={openAddModal}
        class="px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2 bg-blue-600 text-white hover:bg-blue-700 transition-colors shadow-sm"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        {$_('marketplace.addReviewer')}
      </button>
    {/if}
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-red-50 border border-red-100 text-red-600 px-5 py-4 rounded-xl text-sm font-medium">{error}</div>
  {:else if reviewers.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message={$_('marketplace.noReviewersYet')} />
    </div>
  {:else}
    <div class="bg-white rounded-xl border border-gray-200 overflow-hidden shadow-card">
      <table class="w-full">
        <thead>
          <tr class="border-b border-gray-100 bg-gray-50">
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">User</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Email</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Role</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('marketplace.assignedAt')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('groups.actions')}</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each reviewers as r (r.identity_id)}
            {@const info = getIdentityInfo(r.identity_id)}
            <tr class="hover:bg-gray-50 transition-colors">
              <td class="px-6 py-4">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 rounded-full bg-purple-600 flex items-center justify-center text-white text-xs font-bold">
                    {info.name[0]?.toUpperCase() || '?'}
                  </div>
                  <span class="text-sm font-semibold text-gray-900">{info.name}</span>
                </div>
              </td>
              <td class="px-6 py-4 text-sm text-gray-500">{info.email || '-'}</td>
              <td class="px-6 py-4">
                <span class="px-2.5 py-1 rounded-full text-xs font-medium bg-purple-100 text-purple-700">
                  {ROLE_LABEL}
                </span>
              </td>
              <td class="px-6 py-4 text-sm text-gray-500">
                {r.assigned_at ? new Date(r.assigned_at).toLocaleDateString() : '-'}
              </td>
              <td class="px-6 py-4">
                {#if hasPermission('marketplace:role_assign')}
                  <button
                    on:click={() => handleRemove(r)}
                    class="px-3 py-1.5 rounded-lg text-xs font-semibold text-red-600 hover:bg-red-50 border border-red-200 transition-colors"
                  >
                    {$_('groups.remove')}
                  </button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- Add Reviewer Modal -->
{#if showAddModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay" role="button" tabindex="-1" on:click|self={() => showAddModal = false} on:keydown|self={(e) => e.key === 'Escape' && (showAddModal = false)}>
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <div class="flex items-center justify-between mb-5">
      <h2 class="text-lg font-bold text-gray-900">{$_('marketplace.addReviewer')}</h2>
      <button on:click={() => showAddModal = false} class="p-1.5 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-gray-100">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
      </button>
    </div>

    <div class="space-y-4">
      <div>
        <label for="add-reviewer-email" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('marketplace.userEmail')} *</label>
        <input
          id="add-reviewer-email"
          type="email"
          bind:value={addEmail}
          placeholder={$_('marketplace.userEmailPlaceholder')}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>

      <div class="bg-blue-50 border border-blue-100 rounded-xl p-4 text-sm text-blue-700">
        <p class="font-semibold mb-1">{$_('marketplace.marketplaceReviewer')}</p>
        <p class="text-blue-600 text-xs">{$_('marketplace.reviewerDescription')}</p>
      </div>
    </div>

    <div class="flex justify-end gap-3 pt-5">
      <button
        on:click={() => showAddModal = false}
        class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
      >
        {$_('common.cancel')}
      </button>
      <button
        on:click={handleAdd}
        disabled={adding || !addEmail.trim()}
        class="px-5 py-2.5 rounded-xl font-semibold text-sm bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-sm"
      >
        {adding ? $_('common.loading') : $_('common.confirm')}
      </button>
    </div>
  </div>
</div>
{/if}
