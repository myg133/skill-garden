<script>
  import { navigate } from 'svelte-routing';
  import { auth, selectedNav } from '../stores/auth.js';

  function handleNavigate(href) {
    $selectedNav = href;
  }

  function handleLogout() {
    auth.logout();
    navigate('/login', { replace: true });
  }

  const links = [
    {
      href: '/',
      label: 'Organizations',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2-2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"/>`
    },
    {
      href: '/sessions',
      label: 'Sessions',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>`
    },
    {
      href: '/org-tools',
      label: 'Org Tools',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>`
    },
    {
      href: '/review',
      label: 'Review',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>`
    },
    {
      href: '/stats',
      label: 'Dashboard',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/>`
    },
    {
      href: '/audit',
      label: 'Audit Logs',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"/>`
    },
    {
      href: '/settings',
      label: 'Settings',
      icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>`
    },
  ];

  function isActive(href) {
    return $selectedNav === href;
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

  <nav class="flex-1 py-5 px-3 space-y-1 overflow-y-auto">
    {#each links as link}
      {@const active = isActive(link.href)}
      <a
        href={link.href}
        on:click={() => handleNavigate(link.href)}
        class="relative flex items-center gap-3 px-3 py-2.5 rounded-xl text-[13px] font-semibold transition-all duration-200 group {active ? 'nav-item-active' : 'text-indigo-500 hover:text-indigo-700 hover:bg-indigo-50 hover:shadow-[0_2px_8px_rgba(0,0,0,0.08)]'}"
      >
        <svg class="w-[18px] h-[18px] flex-shrink-0 {active ? 'text-indigo-600' : 'text-indigo-400 group-hover:text-indigo-600'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          {@html link.icon}
        </svg>
        <span>{link.label}</span>
      </a>
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