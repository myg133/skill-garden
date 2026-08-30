<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission, permissionStore } from '../stores/permission.js';
  import { getQuickActionsForRole, ROLE_TENANT_ADMIN } from '../config/nav-routes.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import Icon from '../components/Icon.svelte';
  import { _ } from 'svelte-i18n';

  export let id = null;

  let tenant = null;
  let orgs = [];
  let admins = [];
  let loading = true;
  let error = '';

  // 管理员管理状态
  let showAddAdmin = false;
  let newAdminEmail = '';
  let addingAdmin = false;
  let searchResults = [];
  let searching = false;
  let removingAdminId = null;

  // Quick actions for tenant_admin
  $: currentRole = $permissionStore.tenantRoles.some(t => t.role === ROLE_TENANT_ADMIN) ? ROLE_TENANT_ADMIN : null;
  $: quickActions = currentRole ? getQuickActionsForRole(currentRole) : [];

  onMount(async () => {
    if (id) {
      await loadTenantDetail(id);
    }
  });

  async function loadTenantDetail(tenantId) {
    loading = true;
    error = '';
    try {
      // 加载租户详情
      const tenantRes = await api.getTenant(tenantId);
      tenant = tenantRes.data || tenantRes;

      // 加载关联组织
      const orgsRes = await api.listOrganizations({ tenant_id: tenantId, limit: 100 });
      orgs = orgsRes.data || [];

      // 加载租户管理员
      const rolesRes = await api.listTenantRoleAssignments({ tenant_id: tenantId });
      admins = rolesRes.data || [];
    } catch (e) {
      error = e.message || 'Failed to load tenant details';
      addToast(error, 'error');
    } finally {
      loading = false;
    }
  }

  function roleColor(role) {
    const c = { tenant_admin: 'bg-amber-100 text-amber-700' };
    return c[role] || 'bg-gray-100 text-gray-600';
  }

  // 搜索用户
  async function searchUsers(query) {
    if (!query.trim()) {
      searchResults = [];
      return;
    }
    searching = true;
    try {
      const results = await api.searchIdentities(query, 5);
      searchResults = results || [];
    } catch (e) {
      searchResults = [];
    } finally {
      searching = false;
    }
  }

  function handleSearchInput(e) {
    newAdminEmail = e.target.value;
    clearTimeout(window._adminSearchTimer);
    window._adminSearchTimer = setTimeout(() => {
      searchUsers(newAdminEmail);
    }, 300);
  }

  function selectUser(user) {
    newAdminEmail = user.email || user.username || '';
    searchResults = [];
  }

  // 添加管理员
  async function handleAddAdmin() {
    if (!newAdminEmail.trim() || !tenant) return;
    addingAdmin = true;
    try {
      // 先搜索获取用户 ID
      const users = await api.searchIdentities(newAdminEmail, 1);
      const user = users && users.length > 0 ? users[0] : null;
      if (!user) {
        throw new Error('User not found');
      }
      await api.assignTenantRole(user.id, tenant.id, 'tenant_admin');
      addToast($_('tenants.adminAdded') || 'Administrator added', 'success');
      newAdminEmail = '';
      searchResults = [];
      showAddAdmin = false;
      await loadTenantDetail(tenant.id);
    } catch (e) {
      addToast(e.message || 'Failed to add administrator', 'error');
    } finally {
      addingAdmin = false;
    }
  }

  // 移除管理员
  async function handleRemoveAdmin(admin) {
    if (!tenant || !admin.identity) return;
    if (!confirm(`Remove ${admin.identity.name || admin.identity.email || 'this user'} as administrator?`)) return;
    removingAdminId = admin.id;
    try {
      await api.revokeTenantRole(admin.identity.id, tenant.id, 'tenant_admin');
      addToast($_('tenants.adminRemoved') || 'Administrator removed', 'success');
      await loadTenantDetail(tenant.id);
    } catch (e) {
      addToast(e.message || 'Failed to remove administrator', 'error');
    } finally {
      removingAdminId = null;
    }
  }
</script>

<div class="p-8">
  {#if quickActions.length > 0}
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-8">
    {#each quickActions as action (action.key)}
      <a
        href={action.href}
        class="bg-white rounded-xl border border-gray-200 shadow-card p-5 flex items-center gap-4 hover:shadow-md hover:border-blue-200 transition-all group"
      >
        <div class="w-10 h-10 rounded-lg bg-amber-50 flex items-center justify-center flex-shrink-0 group-hover:bg-amber-100 transition-colors">
          <Icon name={action.icon} size="w-5 h-5" className="text-amber-600" />
        </div>
        <span class="text-sm font-semibold text-gray-700 group-hover:text-amber-600 transition-colors">
          {$_(action.labelKey)}
        </span>
      </a>
    {/each}
  </div>
  {/if}

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">
      {error}
    </div>
  {:else if tenant}
    <!-- Tenant Header -->
    <div class="page-header mb-8">
      <div class="flex items-center gap-4">
        <div class="w-14 h-14 rounded-xl bg-blue-600 flex items-center justify-center font-bold text-white text-xl">
          {tenant.name[0]?.toUpperCase() || '?'}
        </div>
        <div>
          <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">{tenant.name}</h1>
          <p class="text-gray-500 text-sm mt-1">
            <span class="font-mono">{tenant.slug}</span>
            <span class="mx-2">·</span>
            <span class="px-2 py-0.5 rounded text-xs font-medium {tenant.status === 'active' ? 'bg-emerald-50 text-emerald-600' : 'bg-amber-50 text-amber-600'}">
              {tenant.status}
            </span>
          </p>
        </div>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
      <!-- Organizations Section -->
      <div class="bg-white rounded-xl border border-gray-200 p-6 shadow-card">
        <h2 class="text-lg font-bold text-gray-800 mb-4">{$_('tenants.organizations') || 'Organizations'}</h2>
        {#if orgs.length === 0}
          <div class="text-center py-8 text-gray-400">
            <p>No organizations found</p>
          </div>
        {:else}
          <div class="space-y-3">
            {#each orgs as org (org.id)}
              <a
                href="/organizations/{org.id}"
                class="flex items-center gap-3 p-3 rounded-lg bg-gray-50 hover:bg-gray-100 transition-colors"
              >
                <div class="w-10 h-10 rounded-lg bg-emerald-500 flex items-center justify-center text-white text-sm font-bold">
                  {org.name[0]?.toUpperCase() || '?'}
                </div>
                <div class="flex-1">
                  <p class="text-sm font-medium text-gray-800">{org.name}</p>
                  <p class="text-xs text-gray-400">{org.slug || ''}</p>
                </div>
                {#if org.my_role}
                  <span class="px-2 py-1 rounded text-xs font-medium bg-blue-50 text-blue-600">
                    {org.my_role}
                  </span>
                {/if}
                <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
                </svg>
              </a>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Admins Section -->
      <div class="bg-white rounded-xl border border-gray-200 p-6 shadow-card">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-lg font-bold text-gray-800">{$_('tenants.tenantAdmins') || 'Tenant Admins'}</h2>
          <button
            on:click={() => showAddAdmin = !showAddAdmin}
            class="px-3 py-1.5 text-xs font-semibold text-blue-600 bg-blue-50 hover:bg-blue-100 rounded-lg transition-colors flex items-center gap-1"
          >
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
            {$_('tenants.addAdmin') || 'Add Admin'}
          </button>
        </div>

        <!-- Add Admin Form -->
        {#if showAddAdmin}
          <div class="mb-4 p-4 bg-blue-50 rounded-xl border border-blue-100">
            <div class="mb-2">
              <label for="new-admin-email" class="block text-xs font-semibold text-gray-600 mb-1.5">
                {$_('tenants.adminEmail') || 'Administrator Email'}
              </label>
              <input
                id="new-admin-email"
                type="email"
                value={newAdminEmail}
                on:input={handleSearchInput}
                placeholder="admin@example.com"
                class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm bg-white"
              />
              <!-- 搜索结果 -->
              {#if searchResults.length > 0}
                <div class="mt-1 bg-white border border-gray-200 rounded-lg shadow-lg overflow-hidden">
                  {#each searchResults as user (user.id)}
                    <button
                      type="button"
                      class="w-full text-left px-3 py-2 hover:bg-blue-50 transition-colors border-b border-gray-100 last:border-b-0 text-sm"
                      on:click={() => selectUser(user)}
                    >
                      <p class="font-medium text-gray-800">{user.name || user.username || 'Unknown'}</p>
                      <p class="text-xs text-gray-400">{user.email || user.username || ''}</p>
                    </button>
                  {/each}
                </div>
              {/if}
              {#if searching}
                <p class="mt-1 text-xs text-gray-400">Searching...</p>
              {/if}
            </div>
            <div class="flex gap-2 justify-end">
              <button
                on:click={() => { showAddAdmin = false; newAdminEmail = ''; searchResults = []; }}
                class="px-3 py-1.5 text-xs font-medium text-gray-500 hover:text-gray-700"
              >
                {$_('common.cancel') || 'Cancel'}
              </button>
              <button
                on:click={handleAddAdmin}
                disabled={addingAdmin || !newAdminEmail.trim()}
                class="px-3 py-1.5 text-xs font-semibold text-white bg-blue-600 hover:bg-blue-700 rounded-lg disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {addingAdmin ? ($_('common.loading') || 'Loading...') : ($_('common.add') || 'Add')}
              </button>
            </div>
          </div>
        {/if}

        {#if admins.length === 0}
          <div class="text-center py-8 text-gray-400">
            <p>No administrators found</p>
          </div>
        {:else}
          <div class="space-y-3">
            {#each admins as admin (admin.id)}
              <div class="flex items-center gap-3 p-3 rounded-lg bg-gray-50">
                <div class="w-10 h-10 rounded-full bg-purple-600 flex items-center justify-center text-white text-sm font-bold">
                  {(admin.identity?.name || admin.identity?.email || '?')[0]?.toUpperCase()}
                </div>
                <div class="flex-1">
                  <p class="text-sm font-medium text-gray-800">
                    {admin.identity?.name || admin.identity?.email || 'Unknown'}
                  </p>
                  <p class="text-xs text-gray-400">{admin.identity?.email || ''}</p>
                </div>
                <span class="px-2 py-1 rounded text-xs font-medium {roleColor(admin.role_name)}">
                  {admin.role_name}
                </span>
                <button
                  on:click={() => handleRemoveAdmin(admin)}
                  disabled={removingAdminId === admin.id}
                  class="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-lg transition-colors disabled:opacity-50"
                  title="Remove administrator"
                >
                  {#if removingAdminId === admin.id}
                    <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
                  {:else}
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                  {/if}
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
