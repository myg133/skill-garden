<script>
  import { navigate, useLocation } from 'svelte-routing';
  import { auth, selectedNav } from '../stores/auth.js';
  import Icon from './Icon.svelte';

  const location = useLocation();

  // 同步路由变化到 selectedNav，确保导航栏高亮与当前页面一致
  $: $selectedNav = $location.pathname;

  function handleNavigate(href) {
    $selectedNav = href;
  }

  function handleLogout() {
    auth.logout();
    navigate('/login', { replace: true });
  }

  function isActive(href) {
    // Home 只精确匹配，避免 /user/skills 等子路由误判
    if (href === '/user') {
      return $selectedNav === '/user';
    }
    return $selectedNav === href || ($selectedNav && $selectedNav.startsWith(href + '/'));
  }

  const userNavItems = [
    { href: '/user', label: 'Home', icon: 'dashboard' },
    { href: '/user/skills', label: 'Skills', icon: 'skills' },
    { href: '/user/submissions', label: 'Submissions', icon: 'review' },
    { href: '/profile', label: 'Profile', icon: 'profile' },
    { href: '/my-api-keys', label: 'API Keys', icon: 'my-api-keys' },
  ];
</script>

<aside class="flex-shrink-0 flex flex-col bg-white border-r border-gray-200 w-[200px]">
  <!-- Header -->
  <div class="h-16 flex items-center gap-3 px-5 border-b border-gray-200">
    <img src="/images/logo.png" alt="AionHive" class="w-9 h-9 rounded-lg flex-shrink-0" />
    <p class="text-base font-bold tracking-tight text-gray-900">AionHive</p>
  </div>

  <!-- Navigation -->
  <nav class="flex-1 py-4 px-3 overflow-y-auto">
    {#each userNavItems as item}
      <a
        href={item.href}
        on:click={() => handleNavigate(item.href)}
        class="flex items-center gap-3 px-3 py-2.5 mb-0.5 rounded-lg text-[13px] font-medium transition-all duration-200 {isActive(item.href) ? 'bg-blue-50 text-blue-700' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'}"
      >
        <Icon name={item.icon} size="w-[17px] h-[17px]" className={isActive(item.href) ? 'text-blue-600' : 'text-gray-400'} />
        {item.label}
      </a>
    {/each}
  </nav>

  <!-- Footer -->
  <div class="p-3 border-t border-gray-200">
    <div class="flex items-center gap-2 px-2 py-1">
      <div class="w-8 h-8 rounded-lg bg-indigo-600 flex items-center justify-center text-[10px] font-bold text-white flex-shrink-0">
        {$auth.username?.[0]?.toUpperCase() || '?'}
      </div>
      <span class="text-[13px] text-gray-700 font-medium truncate flex-1">{$auth.username}</span>
      <button
        on:click={handleLogout}
        class="p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all duration-200"
        title="Sign Out"
      >
        <Icon name="logout" size="w-[16px] h-[16px]" />
      </button>
    </div>
  </div>
</aside>
