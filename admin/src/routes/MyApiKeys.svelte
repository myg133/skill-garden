<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import SetupSkillModal from '../components/SetupSkillModal.svelte';

  let apiKeys = [];
  let organizations = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newKeyForm = { organization_id: '', name: '', expires_in_days: null };
  let creating = false;
  let newlyCreatedKey = '';
  let showSetupSkill = false;

  let revoking = null;

  onMount(async () => {
    await loadApiKeys();
    await loadOrganizations();
  });

  async function loadOrganizations() {
    try {
      organizations = await api.getUserOrgs().catch(() => []);
      organizations = Array.isArray(organizations) ? organizations : [];
    } catch (e) {
      organizations = [];
    }
  }

  async function loadApiKeys() {
    loading = true;
    error = '';
    try {
      const res = await api.listMyApiKeys({ limit: 100 });
      apiKeys = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function handleCreate() {
    if (!newKeyForm.name.trim()) return;
    creating = true;
    newlyCreatedKey = '';
    try {
      const payload = { name: newKeyForm.name };
      if (newKeyForm.organization_id) {
        payload.organization_id = newKeyForm.organization_id;
      }
      if (newKeyForm.expires_in_days) {
        payload.expires_in_days = parseInt(newKeyForm.expires_in_days);
      }
      const res = await api.createMyApiKey(payload);
      newlyCreatedKey = res.key || res.api_key || '';
      newKeyForm = { organization_id: '', name: '', expires_in_days: null };
      // 不关闭弹窗，让用户看到 key 并手动复制
      if (!newlyCreatedKey) {
        addToast('API Key created', 'success');
        showCreateModal = false;
      } else {
        addToast('API Key created — copy it now, it will not be shown again', 'success');
      }
      await loadApiKeys();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      creating = false;
    }
  }

  async function handleRevoke(id) {
    if (!confirm('Revoke this API Key? This action cannot be undone.')) return;
    revoking = id;
    try {
      await api.revokeMyApiKey(id);
      addToast('API Key revoked', 'success');
      await loadApiKeys();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      revoking = null;
    }
  }

  function getStatusColor(status) {
    switch (status) {
      case 'active': return 'bg-emerald-100 text-emerald-700';
      case 'disabled': return 'bg-gray-100 text-gray-500';
      case 'expired': return 'bg-amber-100 text-amber-700';
      case 'revoked': return 'bg-rose-100 text-rose-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }

  async function handleDisable(id) {
    try {
      await api.disableMyApiKey(id);
      addToast('API Key disabled', 'success');
      await loadApiKeys();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleEnable(id) {
    try {
      await api.enableMyApiKey(id);
      addToast('API Key enabled', 'success');
      await loadApiKeys();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  function copyKey(key) {
    navigator.clipboard.writeText(key).then(() => {
      addToast('Copied to clipboard', 'success');
    }).catch(() => {
      addToast('Failed to copy', 'error');
    });
  }

  function openSetupGuide() {
    showSetupSkill = true;
  }
</script>

<div class="p-8">
  <div class="page-header">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">My API Keys</h1>
        <p class="text-gray-500 text-sm mt-1.5 font-medium">Manage your personal API keys</p>
      </div>
      <div class="flex items-center gap-2">
        <button
          on:click={openSetupGuide}
          class="px-4 py-2.5 rounded-xl font-semibold text-sm text-blue-600 border border-blue-200 hover:bg-blue-50 transition-colors flex items-center gap-1.5"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          安装引导
        </button>
        <button
          on:click={() => { showCreateModal = true; newKeyForm = { organization_id: '', name: '', expires_in_days: null }; newlyCreatedKey = ''; }}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
          New API Key
        </button>
      </div>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else}
    <div class="bg-white rounded-2xl border border-gray-200 shadow-card">
      <div class="overflow-x-auto">
        {#if apiKeys.length === 0}
          <div class="px-6 py-16 text-center">
            <div class="w-12 h-12 rounded-2xl bg-indigo-100 flex items-center justify-center mx-auto mb-4">
              <svg class="w-6 h-6 text-indigo-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/></svg>
            </div>
            <p class="text-gray-500 text-sm font-medium">No API keys yet</p>
            <p class="text-gray-400 text-xs mt-1">Create your first API key to authenticate with the API</p>
          </div>
        {:else}
          <table class="w-full">
            <thead class="bg-gray-50 border-b border-gray-100">
              <tr>
                <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Name</th>
                <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Key Prefix</th>
                <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Status</th>
                <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Created</th>
                <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">Expires</th>
                <th class="px-6 py-3 text-right text-xs font-semibold text-gray-500 uppercase tracking-wider">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">
              {#each apiKeys as key (key.id)}
                <tr class="hover:bg-gray-50 transition-colors">
                  <td class="px-6 py-4">
                    <p class="text-sm font-semibold text-gray-800">{key.name}</p>
                  </td>
                  <td class="px-6 py-4">
                    <code class="text-xs font-mono bg-gray-100 px-2 py-1 rounded text-gray-800">{key.key_id || key.key_prefix || key.id}</code>
                  </td>
                  <td class="px-6 py-4">
                    <span class="px-2.5 py-1 rounded-full text-xs font-medium {getStatusColor(key.status)}">
                      {key.status || 'active'}
                    </span>
                  </td>
                  <td class="px-6 py-4 text-sm text-gray-500">
                    {key.created_at ? new Date(key.created_at).toLocaleDateString() : '-'}
                  </td>
                  <td class="px-6 py-4 text-sm text-gray-500">
                    {#if key.expires_at}
                      {new Date(key.expires_at).toLocaleDateString()}
                    {:else if key.expires_in_days}
                      in {key.expires_in_days} days
                    {:else}
                      Never
                    {/if}
                  </td>
                  <td class="px-6 py-4 text-right">
                    {#if key.status === 'disabled'}
                      <button
                        on:click={() => handleEnable(key.id)}
                        class="p-2 rounded-lg text-gray-400 hover:text-emerald-500 hover:bg-emerald-50 transition-all"
                        title="Enable"
                      >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
                      </button>
                    {:else if key.status !== 'revoked' && key.status !== 'expired'}
                      <button
                        on:click={() => handleDisable(key.id)}
                        class="p-2 rounded-lg text-gray-400 hover:text-amber-500 hover:bg-amber-50 transition-all"
                        title="Disable"
                      >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/></svg>
                      </button>
                    {/if}
                    <button
                      on:click={() => handleRevoke(key.id)}
                      disabled={revoking === key.id || key.status === 'revoked'}
                      class="p-2 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all disabled:opacity-30 disabled:cursor-not-allowed"
                      title="Revoke"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    </div>
  {/if}
</div>

{#if showCreateModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  <div class="bg-white rounded-2xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <h2 class="text-lg font-bold text-gray-800 mb-5">Create API Key</h2>
    {#if newlyCreatedKey}
      <div class="mb-4 p-4 bg-emerald-50 border border-emerald-200 rounded-xl">
        <p class="text-emerald-700 text-sm font-semibold mb-2">API Key Created</p>
        <p class="text-emerald-600 text-xs mb-3">Copy this key now — it will not be shown again.</p>
        <div class="flex items-center gap-2">
          <code class="flex-1 text-xs font-mono bg-white border border-emerald-200 rounded-lg px-3 py-2 text-gray-700 break-all">{newlyCreatedKey}</code>
          <button
            on:click={() => copyKey(newlyCreatedKey)}
            class="btn-secondary px-3 py-2 rounded-lg text-xs font-semibold flex-shrink-0"
          >
            Copy
          </button>
        </div>
      </div>
      <div class="flex justify-between items-center">
        <button
          on:click={openSetupGuide}
          class="px-3 py-2 text-xs font-semibold text-blue-600 hover:bg-blue-50 rounded-lg transition-colors flex items-center gap-1.5"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          安装引导
        </button>
        <button
          on:click={() => { showCreateModal = false; newlyCreatedKey = ''; }}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm"
        >
          Done
        </button>
      </div>
    {:else}
      <div class="space-y-4">
        <div>
          <label for="myapikey-org" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Organization</label>
          <select
            id="myapikey-org"
            bind:value={newKeyForm.organization_id}
            class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
          >
            <option value="">Personal（个人）</option>
            {#each organizations as org}
              <option value={org.id}>{org.name}</option>
            {/each}
          </select>
        </div>
        <div>
          <label for="myapikey-name" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Name</label>
          <input
            id="myapikey-name"
            type="text"
            bind:value={newKeyForm.name}
            placeholder="e.g. Production API Key"
            class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
          />
        </div>
        <div>
          <label for="myapikey-expires" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Expires In (days)</label>
          <input
            id="myapikey-expires"
            type="number"
            bind:value={newKeyForm.expires_in_days}
            placeholder="Leave empty for no expiration"
            min="1"
            class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
          />
        </div>
        <div class="flex gap-3 justify-end pt-1">
          <button
            on:click={() => { showCreateModal = false; }}
            class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
          >
            Cancel
          </button>
          <button
            on:click={handleCreate}
            disabled={creating || !newKeyForm.name.trim()}
            class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {creating ? 'Creating...' : 'Create'}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
{/if}

<SetupSkillModal bind:open={showSetupSkill} onClose={() => { showSetupSkill = false; }} />
