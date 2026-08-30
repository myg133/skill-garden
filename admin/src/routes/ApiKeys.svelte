<script>
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  const ACT = ACTIONS.ApiKeys;

  let apiKeys = [];
  let identities = [];
  let organizations = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newApiKey = { identity_id: '', organization_id: '', name: '', scopes: [], rate_limit: 1000 };
  let creating = false;

  let createdKey = null;

  onMount(async () => {
    await Promise.all([loadApiKeys(), loadIdentities(), loadOrganizations()]);
  });

  async function loadApiKeys() {
    loading = true;
    error = '';
    try {
      const res = await api.listApiKeys({ limit: 100 });
      apiKeys = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function loadIdentities() {
    try {
      const res = await api.listIdentities({ limit: 100 });
      identities = res.data || [];
    } catch (e) {
      addToast('身份列表加载失败', 'warning');
    }
  }

  async function loadOrganizations() {
    try {
      const res = await api.listOrganizations({ limit: 100 });
      organizations = res.data || [];
    } catch (e) {
      addToast('组织列表加载失败', 'warning');
    }
  }

  async function handleCreate() {
    if (!newApiKey.identity_id) return;
    creating = true;
    try {
      const payload = {
        ...newApiKey,
        organization_id: newApiKey.organization_id || null
      };
      const res = await api.createApiKey(payload);
      createdKey = res;
      newApiKey = { identity_id: '', organization_id: '', name: '', scopes: [], rate_limit: 1000 };
      addToast($_('apiKeys.apiKeyCreated'), 'success');
      await loadApiKeys();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      creating = false;
    }
  }

  async function handleDelete(id) {
    if (!confirm($_('apiKeys.deleteApiKey'))) return;
    try {
      await api.deleteApiKey(id);
      addToast('API Key revoked', 'success');
      await loadApiKeys();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  function getIdentityName(key) {
    if (key.identity_name) return key.identity_name;
    const identity = identities.find(i => i.id === key.identity_id);
    return identity ? identity.name : key.identity_id;
  }

  function getOrgName(key) {
    if (key.organization_name) return key.organization_name;
    const org = organizations.find(o => o.id === key.organization_id);
    return org ? org.name : key.organization_id;
  }

  function formatId(id) {
    if (!id) return '';
    const s = String(id);
    return s.length > 12 ? s.slice(0, 8) + '...' : s;
  }

  function getStatusColor(status) {
    switch (status) {
      case 'active': return 'bg-emerald-100 text-emerald-700';
      case 'disabled': return 'bg-gray-100 text-gray-500';
      case 'expired': return 'bg-amber-100 text-amber-700';
      case 'revoked': return 'bg-red-100 text-red-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }

  async function handleDisable(id) {
    try {
      await api.disableApiKey(id);
      addToast($_('apiKeys.apiKeyDisabled'), 'success');
      await loadApiKeys();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleEnable(id) {
    try {
      await api.enableApiKey(id);
      addToast($_('apiKeys.apiKeyEnabled'), 'success');
      await loadApiKeys();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }
</script>

<div class="p-8">
  <div class="page-header">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">{$_('apiKeys.title')}</h1>
        <p class="text-gray-500 text-sm mt-1.5 font-medium">{$_('apiKeys.manageApiKeys')}</p>
      </div>
      {#if hasPermission(ACT.create)}
        <button
          on:click={() => { showCreateModal = true; createdKey = null; }}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
          {$_('apiKeys.newApiKey')}
        </button>
      {/if}
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if apiKeys.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message={$_('apiKeys.noApiKeys')}>
        {#if hasPermission(ACT.create)}
          <button
            on:click={() => showCreateModal = true}
            class="mt-4 btn-primary px-5 py-2.5 rounded-lg font-semibold text-sm"
          >
            {$_('apiKeys.createFirst')}
          </button>
        {/if}
      </EmptyState>
    </div>
  {:else}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card overflow-hidden">
      <table class="w-full">
        <thead class="bg-gray-50 border-b border-gray-200">
          <tr>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('common.name')}</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('groups.identity')}</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.title')}</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('apiKeys.keyPrefix')}</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('common.status')}</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('apiKeys.rateLimitPerMin')}</th>
            <th class="px-6 py-4 text-right text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('groups.actions')}</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each apiKeys as key (key.id)}
            <tr class="hover:bg-gray-50 transition-colors">
              <td class="px-6 py-4">
                <p class="text-sm font-semibold text-gray-800">{key.name || $_('apiKeys.unnamed')}</p>
              </td>
              <td class="px-6 py-4">
                <div class="text-sm text-gray-800 font-medium">{getIdentityName(key)}</div>
                {#if key.identity_name}
                  <div class="text-xs text-gray-400 font-mono mt-0.5" title={key.identity_id}>{formatId(key.identity_id)}</div>
                {/if}
              </td>
              <td class="px-6 py-4">
                <div class="text-sm text-gray-800 font-medium">{getOrgName(key)}</div>
                {#if key.organization_id}
                  <div class="text-xs text-gray-400 font-mono mt-0.5" title={key.organization_id}>{formatId(key.organization_id)}</div>
                {/if}
              </td>
              <td class="px-6 py-4">
                <code class="text-xs font-mono bg-gray-100 px-2 py-1 rounded text-gray-800">{key.key_prefix}***</code>
              </td>
              <td class="px-6 py-4">
                <span class="px-2.5 py-1 rounded-full text-xs font-medium {getStatusColor(key.status)}">
                  {key.status}
                </span>
              </td>
              <td class="px-6 py-4 text-sm text-gray-600">{key.rate_limit}/min</td>
              <td class="px-6 py-4 text-right">
                {#if hasPermission(ACT.revoke)}
                  {#if key.status === 'disabled'}
                    <button
                      on:click={() => handleEnable(key.id)}
                      class="p-2 rounded-lg text-gray-400 hover:text-emerald-500 hover:bg-emerald-50 transition-all"
                      title={$_('apiKeys.enable')}
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
                    </button>
                  {:else if key.status !== 'revoked' && key.status !== 'expired'}
                    <button
                      on:click={() => handleDisable(key.id)}
                      class="p-2 rounded-lg text-gray-400 hover:text-amber-500 hover:bg-amber-50 transition-all"
                      title={$_('apiKeys.disable')}
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/></svg>
                    </button>
                  {/if}
                  <button
                    on:click={() => handleDelete(key.id)}
                    class="p-2 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all"
                    title={$_('apiKeys.revoke')}
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
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

{#if showCreateModal}
  <div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <h2 class="text-lg font-bold text-gray-800 mb-5">{$_('apiKeys.createApiKey')}</h2>
    {#if createdKey}
      <div class="space-y-4">
        <div class="bg-emerald-50 border border-emerald-200 rounded-xl p-4">
          <p class="text-sm font-semibold text-emerald-800 mb-2">{$_('apiKeys.apiKeyCreatedSuccess')}</p>
          <p class="text-xs text-emerald-600 mb-3">{$_('apiKeys.copyKeyNow')}</p>
          <code class="block bg-white border border-emerald-300 rounded-lg p-3 text-sm font-mono break-all">{createdKey.key}</code>
        </div>
        <button
          on:click={() => { showCreateModal = false; createdKey = null; }}
          class="w-full btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm"
        >
          {$_('apiKeys.done')}
        </button>
      </div>
    {:else}
      <div class="space-y-4">
        <div>
          <label for="apikey-identity" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Identity</label>
          <select
            id="apikey-identity"
            bind:value={newApiKey.identity_id}
            class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
          >
            <option value="" disabled selected hidden>{$_('apiKeys.selectIdentity')}</option>
            {#each identities as identity}
              <option value={identity.id}>{identity.name} ({identity.identity_type})</option>
            {/each}
          </select>
        </div>
        <div>
          <label for="apikey-org" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Organization</label>
          <select
            id="apikey-org"
            bind:value={newApiKey.organization_id}
            class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
          >
            <option value="">{$_('apiKeys.personal')}</option>
            {#each organizations as org}
              <option value={org.id}>{org.name}</option>
            {/each}
          </select>
        </div>
        <div>
          <label for="apikey-name" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('common.name')} ({$_('common.optional')})</label>
          <input
            id="apikey-name"
            type="text"
            bind:value={newApiKey.name}
            placeholder="My API Key"
            class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
          />
        </div>
        <div>
          <label for="apikey-rate-limit" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('apiKeys.rateLimitPerMin')}</label>
          <input
            id="apikey-rate-limit"
            type="number"
            bind:value={newApiKey.rate_limit}
            min="1"
            class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
          />
        </div>
        <div class="flex gap-3 justify-end pt-1">
          <button
            on:click={() => { showCreateModal = false; newApiKey = { identity_id: '', organization_id: '', name: '', scopes: [], rate_limit: 1000 }; }}
            class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
          >
            {$_('common.cancel')}
          </button>
          <button
            on:click={handleCreate}
            disabled={creating || !newApiKey.identity_id}
            class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {creating ? $_('common.loading') : $_('common.create')}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
{/if}
