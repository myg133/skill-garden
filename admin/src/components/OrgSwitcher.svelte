<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { selectedOrg, userOrgs, isPersonalSpace } from '../stores/org.js';
  import { isAdmin } from '../stores/auth.js';
  import { hasPermission, isAnyAdmin, permissionStore } from '../stores/permission.js';

  // 是否展示 Personal Space 选项（仅 skill 相关页面需要，组织/分组管理页面不应包含个人空间）
  export let showPersonal = true;

  let open = false;
  let loading = true;

  $: showDropdown = ($isAdmin || ($permissionStore.loaded && isAnyAdmin())) && orgs.length > 0;

  $: orgs = $userOrgs || [];

  $: currentLabel = $selectedOrg
    ? $selectedOrg.name
    : 'Personal';

  $: roleLabel = $selectedOrg?.role
    ? orgRoleLabel($selectedOrg.role)
    : '';

  function orgRoleLabel(role) {
    const labels = {
      owner: 'Owner',
      admin: 'Admin',
      reviewer: 'Reviewer',
      developer: 'Developer',
      member: 'Member',
    };
    return labels[role] || role;
  }

  function selectOrg(org) {
    if (org === null) {
      $selectedOrg = null; // personal space
    } else {
      $selectedOrg = { id: org.id, name: org.name, slug: org.slug, role: org.role };
    }
    open = false;
  }

  function toggleDropdown() {
    open = !open;
  }

  function closeDropdown() {
    open = false;
  }

  onMount(async () => {
    try {
      const res = await api.getUserOrgs();
      const orgList = (res.data || res || []);
      $userOrgs = orgList;

      // Auto-select: default to first org if none selected
      if (!$selectedOrg && orgList.length > 0) {
        const first = orgList[0];
        $selectedOrg = { id: first.id, name: first.name, slug: first.slug, role: first.role };
      }
    } catch (e) {
      // silently fail, no orgs available
    } finally {
      loading = false;
    }
  });

  const roleEmoji = {
    owner: '👑',
    admin: '⚙️',
    reviewer: '👁️',
    developer: '💻',
    member: '👤',
  };

  function handleClickOutside(event) {
    if (open && !event.target.closest('.org-switcher')) {
      open = false;
    }
  }
</script>

<svelte:window on:click={handleClickOutside} />

{#if showDropdown}
  <div class="org-switcher relative">
    <button
      on:click={toggleDropdown}
      class="flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-semibold bg-white border border-gray-200 hover:border-gray-300 transition-all duration-200 shadow-sm"
    >
      <span class="text-base">{$isPersonalSpace ? '👤' : '🏢'}</span>
      <span class="max-w-[160px] truncate">{currentLabel}</span>
      {#if roleLabel}
        <span class="text-xs text-gray-400 font-normal">({roleLabel})</span>
      {/if}
      <svg class="w-3.5 h-3.5 text-gray-400 transition-transform {open ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
      </svg>
    </button>

    {#if open}
      <div class="absolute top-full left-0 mt-1.5 w-64 bg-white rounded-xl border border-gray-200 shadow-lg z-50 py-1.5">
        {#each orgs as org (org.id)}
          <button
            on:click={() => selectOrg(org)}
            class="w-full flex items-center gap-3 px-4 py-2.5 text-sm hover:bg-gray-50 transition-colors {($selectedOrg && $selectedOrg.id === org.id) ? 'bg-blue-50 text-blue-700' : 'text-gray-700'}"
          >
            <span class="text-base flex-shrink-0">🏢</span>
            <span class="truncate flex-1 text-left">{org.name}</span>
            <span class="text-xs text-gray-400 flex-shrink-0">{roleEmoji[org.role] || ''} {orgRoleLabel(org.role)}</span>
          </button>
        {/each}

        {#if showPersonal}
          <div class="border-t border-gray-100 my-1"></div>

          <button
            on:click={() => selectOrg(null)}
            class="w-full flex items-center gap-3 px-4 py-2.5 text-sm hover:bg-gray-50 transition-colors {!$selectedOrg ? 'bg-blue-50 text-blue-700' : 'text-gray-700'}"
          >
            <span class="text-base flex-shrink-0">👤</span>
            <span class="truncate flex-1 text-left">Personal Space</span>
          </button>
        {/if}
      </div>
    {/if}
  </div>
{/if}
