<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission } from '../stores/permission.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import { _ } from 'svelte-i18n';

  export let id = null;

  let tenant = null;
  let orgs = [];
  let admins = [];
  let loading = true;
  let error = '';

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
</script>

<div class="p-8">
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
        <h2 class="text-lg font-bold text-gray-800 mb-4">{$_('tenants.tenantAdmins') || 'Tenant Admins'}</h2>
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
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
