<script>
  import { navigate, useLocation } from 'svelte-routing';
  import { auth, selectedNav } from '../stores/auth.js';
  import Icon from './Icon.svelte';

  const STORAGE_KEY = 'nav_collapsed';
  const SIDEBAR_KEY = 'sidebar_collapsed';
  const location = useLocation();

  // 同步路由变化到 selectedNav，确保导航栏高亮与当前页面一致
  $: $selectedNav = $location.pathname;

  function handleNavigate(href) {
    $selectedNav = href;
    // Reset all manual expand preferences so only the current route's group stays open
    const path = href;
    for (const group of navGroups) {
      const isInGroup = group.children.some(c => pathInGroup(c.href, path));
      if (!isInGroup) {
        manualCollapsed[group.key] = true;
      } else {
        delete manualCollapsed[group.key];
      }
    }
    saveCollapsed(manualCollapsed);
  }

  // Check if a given path belongs to a nav child route
  function pathInGroup(childHref, currentPath) {
    if (childHref === currentPath) return true;
    if (childHref === '/') return currentPath === '/';
    return currentPath.startsWith(childHref + '/') || currentPath.startsWith(childHref + '?');
  }

  function handleLogout() {
    localStorage.removeItem(STORAGE_KEY);
    localStorage.removeItem(SIDEBAR_KEY);
    auth.logout();
    navigate('/login', { replace: true });
  }

  function loadCollapsed() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      return saved ? JSON.parse(saved) : {};
    } catch {
      return {};
    }
  }

  function saveCollapsed(state) {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch {}
  }

  // Stores ONLY manual user preference, not auto-derived state
  // Key=true means user explicitly collapsed; Key=false means user explicitly expanded
  // Default (undefined) means collapsed when not the current route's group
  let manualCollapsed = loadCollapsed();
  let sidebarCollapsed = false;

  // Load sidebar state
  try {
    const saved = localStorage.getItem(SIDEBAR_KEY);
    sidebarCollapsed = saved === 'true';
  } catch {}

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    localStorage.setItem(SIDEBAR_KEY, String(sidebarCollapsed));
  }

  // Derive expanded state: current route's group ALWAYS expanded;
  // other groups use manual preference (default: collapsed)
  $: currentPath = $selectedNav;
  $: expandedGroups = {};
  $: {
    for (const group of navGroups) {
      const isCurrentInGroup = group.children.some(c => pathInGroup(c.href, currentPath));
      if (isCurrentInGroup) {
        // Auto-expand: current route belongs to this group
        expandedGroups[group.key] = true;
      } else {
        // Not current route's group: use manual preference, default collapsed
        expandedGroups[group.key] = manualCollapsed[group.key] === false;
      }
    }
  }

  function toggleGroup(key) {
    // Save the OPPOSITE of current expanded state as the user's manual preference
    manualCollapsed = { ...manualCollapsed, [key]: expandedGroups[key] };
    saveCollapsed(manualCollapsed);
  }

  const navGroups = [
    {
      key: 'overview',
      label: 'Overview',
      icon: 'overview',
      children: [
        { href: '/stats', label: 'Dashboard', icon: 'dashboard' }
      ]
    },
    {
      key: 'users',
      label: 'Users',
      icon: 'users',
      children: [
        { href: '/identities', label: 'Identities', icon: 'identities' },
        { href: '/profile', label: 'My Profile', icon: 'profile' },
        { href: '/my-api-keys', label: 'My API Keys', icon: 'my-api-keys' },
        { href: '/api-keys', label: 'API Keys', icon: 'api-keys' }
      ]
    },
    {
      key: 'org',
      label: 'Organizations',
      icon: 'organizations',
      children: [
        { href: '/', label: 'Organizations', icon: 'organizations' },
        { href: '/groups', label: 'Groups', icon: 'groups' },
        { href: '/roles', label: 'Roles', icon: 'roles' },
        { href: '/org-tools', label: 'Org Tools', icon: 'org-tools' }
      ]
    },
    {
      key: 'skills',
      label: 'Content',
      icon: 'skills',
      children: [
        { href: '/skills', label: 'Skills', icon: 'skills' },
        { href: '/review', label: 'Review', icon: 'review' }
      ]
    },
    {
      key: 'system',
      label: 'System',
      icon: 'settings',
      children: [
        { href: '/tenants', label: 'Tenants', icon: 'tenants' },
        { href: '/sessions', label: 'Sessions', icon: 'sessions' },
        { href: '/audit', label: 'Audit Logs', icon: 'audit-logs' },
        { href: '/settings', label: 'Settings', icon: 'settings' }
      ]
    },
    {
      key: 'infra',
      label: 'Infrastructure',
      icon: 'infrastructure',
      children: [
        { href: '/sandboxes', label: 'Sandboxes', icon: 'sandbox' }
      ]
    }
  ];

  function isActive(href) {
    return $selectedNav === href;
  }

  function isGroupActive(group) {
    return group.children.some(child => isActive(child.href));
  }
</script>

<aside class="relative flex-shrink-0 flex flex-col bg-white border-r border-gray-200 transition-all duration-300 ease-in-out {sidebarCollapsed ? 'w-[64px]' : 'w-[244px]'}">
  <!-- Header -->
  <div class="h-16 flex items-center gap-3 px-3 border-b border-gray-200 {sidebarCollapsed ? 'justify-center' : 'px-5'}">
    <img src="/images/logo.png" alt="AionHive" class="w-10 h-10 rounded-lg flex-shrink-0" />
    {#if !sidebarCollapsed}
      <div class="overflow-hidden">
        <p class="text-base font-bold tracking-tight leading-tight text-gray-900 whitespace-nowrap">AionHive</p>
      </div>
    {/if}
  </div>

  <!-- Toggle button -->
  <button
    on:click={toggleSidebar}
    class="absolute top-3 -right-3 w-6 h-6 rounded-full bg-white border border-gray-200 shadow-sm flex items-center justify-center text-indigo-400 hover:text-indigo-600 hover:border-gray-300 transition-all z-10"
    style="transform: translateX(50%);"
    title={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
  >
    <svg class="w-3 h-3 transition-transform duration-300 {sidebarCollapsed ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
    </svg>
  </button>

  <!-- Navigation -->
  <nav class="flex-1 py-5 px-3 overflow-y-auto">
    {#each navGroups as group}
      {@const expanded = expandedGroups[group.key]}
      {@const groupActive = isGroupActive(group)}
      <div class="mb-1">
        <!-- Group header - only interactive when not collapsed -->
        {#if sidebarCollapsed}
          <button
            on:click={() => toggleGroup(group.key)}
            class="w-full flex items-center justify-center py-2.5 rounded-lg transition-all duration-200 {groupActive ? 'text-blue-600 bg-blue-50' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'}"
            title="{group.label}{expanded ? '' : ' (click to expand)'}">
            <Icon name={group.icon} size="w-[18px] h-[18px]" className={groupActive ? 'text-blue-600' : 'text-gray-400'} />
          </button>
        {:else}
          <button
            on:click={() => toggleGroup(group.key)}
            class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-[13px] font-semibold transition-all duration-200 group {groupActive && !expanded ? 'text-blue-600 bg-blue-50' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'}"
          >
            <Icon name={group.icon} size="w-[18px] h-[18px]" className={groupActive ? 'text-blue-600' : 'text-gray-400'} />
            <span class="flex-1 text-left">{group.label}</span>
            <Icon name="chevron-down" size="w-[14px] h-[14px]" className={"text-gray-400 transition-transform duration-200" + (expanded ? ' rotate-180' : '')} />
          </button>
        {/if}

        <!-- Children (only when expanded and not collapsed) -->
        {#if expanded && !sidebarCollapsed}
          <div class="mt-0.5 ml-2 space-y-0.5 border-l-2 border-gray-100 ml-[23px]">
            {#each group.children as child}
              {@const active = isActive(child.href)}
              <a
                href={child.href}
                on:click={() => handleNavigate(child.href)}
                class="relative flex items-center gap-3 pl-5 pr-3 py-2 rounded-r-lg text-[13px] font-medium transition-all duration-200 group {active ? 'nav-item-active' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'}"
              >
                <Icon name={child.icon} size="w-[16px] h-[16px]" className={active ? 'text-blue-600' : 'text-gray-400 group-hover:text-gray-600'} />
                <span>{child.label}</span>
              </a>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </nav>

  <!-- Footer -->
  <div class="p-3 border-t border-gray-200">
    {#if sidebarCollapsed}
      <div class="flex flex-col items-center gap-2">
        <div class="w-8 h-8 rounded-lg bg-blue-600 flex items-center justify-center text-[10px] font-bold text-white flex-shrink-0" title={$auth.username}>
          {$auth.username?.[0]?.toUpperCase() || '?'}
        </div>
        <button
          on:click={handleLogout}
          class="p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all duration-200"
          title="Sign Out"
        >
          <Icon name="logout" size="w-[18px] h-[18px]" />
        </button>
      </div>
    {:else}
      <div class="flex items-center gap-2 px-2 py-1">
        <div class="w-8 h-8 rounded-lg bg-blue-600 flex items-center justify-center text-[10px] font-bold text-white flex-shrink-0">
          {$auth.username?.[0]?.toUpperCase() || '?'}
        </div>
        <span class="text-[13px] text-gray-700 font-medium truncate">{$auth.username}</span>
        <button
          on:click={handleLogout}
          class="ml-auto mr-1 p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all duration-200"
          title="Sign Out"
        >
          <Icon name="logout" size="w-[18px] h-[18px]" />
        </button>
      </div>
    {/if}
  </div>
</aside>