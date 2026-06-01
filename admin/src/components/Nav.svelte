<script>
  import { navigate } from 'svelte-routing';
  import { auth, selectedNav } from '../stores/auth.js';

  const STORAGE_KEY = 'nav_collapsed';

  function handleNavigate(href) {
    $selectedNav = href;
  }

  function handleLogout() {
    localStorage.removeItem(STORAGE_KEY);
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

  let collapsedGroups = loadCollapsed();

  $: expandedGroups = {
    overview: navGroups[0].children.some(c => c.href === $selectedNav) || !collapsedGroups.overview,
    users: navGroups[1].children.some(c => c.href === $selectedNav) || !collapsedGroups.users,
    org: navGroups[2].children.some(c => c.href === $selectedNav) || !collapsedGroups.org,
    skills: navGroups[3].children.some(c => c.href === $selectedNav) || !collapsedGroups.skills,
    system: navGroups[4].children.some(c => c.href === $selectedNav) || !collapsedGroups.system,
  };

  function toggleGroup(key) {
    collapsedGroups = { ...collapsedGroups, [key]: !collapsedGroups[key] };
    saveCollapsed(collapsedGroups);
  }

  const navGroups = [
    {
      key: 'overview',
      label: 'Overview',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zm10 0a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zm10 0a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"/>`,
      children: [
        { href: '/stats', label: 'Dashboard', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/>` }
      ]
    },
    {
      key: 'users',
      label: 'Users',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>`,
      children: [
        { href: '/identities', label: 'Identities', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"/>` },
        { href: '/profile', label: 'My Profile', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"/>` },
        { href: '/my-api-keys', label: 'My API Keys', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/>` },
        { href: '/api-keys', label: 'API Keys', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>` }
      ]
    },
    {
      key: 'org',
      label: 'Organizations',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"/>`,
      children: [
        { href: '/', label: 'Organizations', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"/>` },
        { href: '/groups', label: 'Groups', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z"/>` },
        { href: '/roles', label: 'Roles', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/>` },
        { href: '/org-tools', label: 'Org Tools', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>` }
      ]
    },
    {
      key: 'skills',
      label: 'Content',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"/>`,
      children: [
        { href: '/skills', label: 'Skills', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"/>` },
        { href: '/review', label: 'Review', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>` }
      ]
    },
    {
      key: 'system',
      label: 'System',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>`,
      children: [
        { href: '/tenants', label: 'Tenants', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"/>` },
        { href: '/sessions', label: 'Sessions', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>` },
        { href: '/audit', label: 'Audit Logs', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"/>` },
        { href: '/settings', label: 'Settings', icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>` }
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

<aside class="w-[244px] flex-shrink-0 flex flex-col glass-sidebar border-r border-indigo-100">
  <div class="h-16 flex items-center gap-3 px-5 border-b border-indigo-100">
    <div class="w-10 h-10 rounded-xl gradient-brand flex items-center justify-center text-xs font-bold shadow-[0_0_16px_rgba(99,102,241,0.3)]">
      @
    </div>
    <div>
      <p class="text-base font-bold tracking-tight leading-tight text-indigo-700">AionHive</p>
    </div>
  </div>

  <nav class="flex-1 py-5 px-3 overflow-y-auto">
    {#each navGroups as group}
      {@const expanded = expandedGroups[group.key]}
      {@const groupActive = isGroupActive(group)}
      <div class="mb-1">
        <button
          on:click={() => toggleGroup(group.key)}
          class="w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-[13px] font-semibold transition-all duration-200 group {groupActive && !expanded ? 'text-indigo-700 bg-indigo-50/70' : 'text-indigo-500 hover:text-indigo-700 hover:bg-indigo-50'}"
        >
          <svg class="w-[18px] h-[18px] flex-shrink-0 {groupActive ? 'text-indigo-600' : 'text-indigo-400'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            {@html group.icon}
          </svg>
          <span class="flex-1 text-left">{group.label}</span>
          <svg
            class="w-[14px] h-[14px] flex-shrink-0 text-indigo-400 transition-transform duration-200"
            style="transform: rotate({expanded ? '180deg' : '0deg'})"
            fill="none" stroke="currentColor" viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
          </svg>
        </button>

        {#if expanded}
          <div class="mt-0.5 ml-2 space-y-0.5 border-l-2 border-indigo-100 ml-[23px]">
            {#each group.children as child}
              {@const active = isActive(child.href)}
              <a
                href={child.href}
                on:click={() => handleNavigate(child.href)}
                class="relative flex items-center gap-3 pl-5 pr-3 py-2.5 rounded-r-xl text-[13px] font-medium transition-all duration-200 group {active ? 'nav-item-active' : 'text-indigo-500 hover:text-indigo-700 hover:bg-indigo-50 hover:shadow-[0_2px_8px_rgba(0,0,0,0.08)]'}"
              >
                <svg class="w-[16px] h-[16px] flex-shrink-0 {active ? 'text-indigo-300' : 'text-indigo-400 group-hover:text-indigo-600'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  {@html child.icon}
                </svg>
                <span>{child.label}</span>
              </a>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </nav>

  <div class="p-3 border-t border-indigo-100">
    <div class="flex items-center gap-2 px-2 py-1">
      <div class="w-8 h-8 rounded-lg gradient-brand flex items-center justify-center text-[10px] font-bold flex-shrink-0 shadow-[0_0_10px_rgba(99,102,241,0.3)]">
        {$auth.username?.[0]?.toUpperCase() || '?'}
      </div>
      <span class="text-[13px] text-indigo-700 font-medium truncate">{$auth.username}</span>
      <button
        on:click={handleLogout}
        class="ml-auto mr-1 p-1.5 rounded-lg text-indigo-400 hover:text-red-500 hover:bg-red-50 transition-all duration-200"
        title="Sign Out"
      >
        <svg class="w-[18px] h-[18px]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"/>
        </svg>
      </button>
    </div>
  </div>
</aside>