<script>
  import { permissionStore } from '../stores/permission.js';

  export let className = '';

  // 合并 systemRoles + tenantRoles + orgRoles（去重按最高角色显示）
  $: displayRoles = (() => {
    const roles = [...($permissionStore.systemRoles || [])];
    const seen = new Set(roles);
    for (const t of ($permissionStore.tenantRoles || [])) {
      if (t.role === 'tenant_admin' && !seen.has('tenant_admin')) {
        roles.push(`tenant_admin:${t.tenant_name || ''}`);
        seen.add('tenant_admin');
      }
    }
    // 展示用户的组织角色（去重，按最高角色显示）
    const orgRoles = $permissionStore.orgRoles || [];
    const orgRoleOrder = ['owner', 'admin', 'reviewer', 'developer', 'member'];
    for (const r of orgRoleOrder) {
      if (orgRoles.some(o => o.role === r) && !seen.has(r)) {
        roles.push(`org:${r}`);
        seen.add(r);
      }
    }
    return roles;
  })();

  function roleLabel(role) {
    if (role.startsWith('org:')) {
      const r = role.slice(4);
      const labels = { owner: 'Org Owner', admin: 'Org Admin', reviewer: 'Org Reviewer', developer: 'Org Developer', member: 'Org Member' };
      return labels[r] || r;
    }
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
    if (role.startsWith('org:')) {
      const r = role.slice(4);
      const colors = { owner: 'bg-amber-100 text-amber-700', admin: 'bg-blue-100 text-blue-700', reviewer: 'bg-purple-100 text-purple-700', developer: 'bg-emerald-100 text-emerald-700', member: 'bg-gray-100 text-gray-600' };
      return colors[r] || 'bg-gray-100 text-gray-600';
    }
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
