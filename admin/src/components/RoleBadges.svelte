<script>
  import { permissionStore } from '../stores/permission.js';

  export let className = '';

  // 合并 systemRoles 和 tenantRoles 中的 admin 角色用于右上角徽章显示
  // tenant_admin 可能来自 system scope 或 tenant scope，合并后去重
  $: displayRoles = (() => {
    const roles = [...($permissionStore.systemRoles || [])];
    const seen = new Set(roles);
    for (const t of ($permissionStore.tenantRoles || [])) {
      if (t.role === 'tenant_admin' && !seen.has('tenant_admin')) {
        roles.push(`tenant_admin:${t.tenant_name || ''}`);
        seen.add('tenant_admin');
      }
    }
    return roles;
  })();

  function roleLabel(role) {
    const baseRole = role.startsWith('tenant_admin:') ? 'tenant_admin' : role;
    switch (baseRole) {
      case 'super_admin': return 'Super Admin';
      case 'marketplace_admin': return 'Mkt Admin';
      case 'marketplace_reviewer': return 'Reviewer';
      case 'tenant_admin': return 'Tenant Admin';
      default: return baseRole;
    }
  }

  function roleColorClass(role) {
    const baseRole = role.startsWith('tenant_admin:') ? 'tenant_admin' : role;
    switch (baseRole) {
      case 'super_admin': return 'bg-red-100 text-red-700';
      case 'marketplace_admin': return 'bg-emerald-100 text-emerald-700';
      case 'marketplace_reviewer': return 'bg-purple-100 text-purple-700';
      case 'tenant_admin': return 'bg-blue-100 text-blue-700';
      default: return 'bg-gray-100 text-gray-600';
    }
  }
</script>

<div class="flex items-center gap-2 {className}">
  {#if $permissionStore.loaded}
    {#each displayRoles as role}
      <span class="px-2.5 py-0.5 rounded-full text-[11px] font-semibold {roleColorClass(role)}">
        {roleLabel(role)}
      </span>
    {/each}
    {#if !displayRoles.length}
      <span class="text-xs text-gray-400">No roles</span>
    {/if}
  {:else}
    <span class="text-xs text-gray-400">Loading roles…</span>
  {/if}
</div>
