<script>
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  let roles = [];
  let loading = true;
  let error = '';

  onMount(async () => {
    await loadRoles();
  });

  async function loadRoles() {
    loading = true;
    error = '';
    try {
      const res = await api.listRoles();
      roles = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  function getTypeColor(type) {
    switch (type) {
      case 'system': return 'bg-red-100 text-red-700';
      case 'tenant': return 'bg-purple-100 text-purple-700';
      case 'organization': return 'bg-blue-100 text-blue-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }

  function getScopeColor(scope) {
    switch (scope) {
      case 'global': return 'bg-amber-100 text-amber-700';
      case 'tenant': return 'bg-emerald-100 text-emerald-700';
      case 'org': return 'bg-cyan-100 text-cyan-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }
</script>

<div class="p-8">
  <div class="page-header">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">Roles</h1>
        <p class="text-gray-500 text-sm mt-1.5 font-medium">Role definitions and permissions</p>
      </div>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if roles.length === 0}
    <div class="bg-white rounded-2xl border border-gray-200 shadow-card">
      <EmptyState message="No roles defined" />
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {#each roles as role (role.id)}
        <div class="bg-white rounded-2xl border border-gray-200 p-6 card">
          <div class="flex items-start gap-4">
            <div class="w-11 h-11 rounded-xl bg-gradient-to-br from-violet-500 to-indigo-600 flex items-center justify-center font-bold text-lg shadow-glow flex-shrink-0">
              {role.name[0]?.toUpperCase() || '?'}
            </div>
            <div class="flex-1 min-w-0">
              <h3 class="text-gray-800 font-semibold text-[15px] truncate mb-0.5">{role.name}</h3>
              <div class="flex gap-2 mt-1">
                <span class="px-2 py-0.5 rounded-full text-xs font-medium {getTypeColor(role.role_type)}">
                  {role.role_type}
                </span>
                <span class="px-2 py-0.5 rounded-full text-xs font-medium {getScopeColor(role.scope_level)}">
                  {role.scope_level || 'unknown'}
                </span>
              </div>
            </div>
          </div>
          {#if role.description}
            <p class="mt-4 text-gray-600 text-sm">{role.description}</p>
          {/if}
          <div class="mt-4 pt-4 border-t border-gray-100">
            <p class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Permissions</p>
            <div class="flex flex-wrap gap-1">
              {#if role.permissions.includes('*')}
                <span class="px-2 py-1 rounded bg-amber-100 text-amber-700 text-xs font-medium">All (*)</span>
              {:else}
                {#each (role.permissions || []).slice(0, 5) as perm}
                  <span class="px-2 py-1 rounded bg-indigo-100 text-indigo-700 text-xs font-mono">{perm}</span>
                {/each}
                {#if (role.permissions || []).length > 5}
                  <span class="px-2 py-1 rounded bg-gray-100 text-gray-600 text-xs">+{role.permissions.length - 5} more</span>
                {/if}
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
