<script>
  import { useLocation, navigate } from 'svelte-routing';
  import { auth, selectedNav } from '../stores/auth.js';
  import { permissionStore } from '../stores/permission.js';
  import Icon from './Icon.svelte';

  const location = useLocation();

  // 同步路由变化到 selectedNav，确保导航栏高亮与当前页面一致
  // 保留 from 参数用于恢复来源 tab 上下文
  $: $selectedNav = $location.pathname + ($location.search || '');

  function handleNavigate(href) {
    navigate(href);
    $selectedNav = href;
  }

  function handleLogout() {
    auth.logout();
    window.location.href = '/login';
  }

  // 有组织角色的用户可见组织相关入口
  $: hasOrgAccess = ($permissionStore.orgRoles || []).length > 0;

  const userNavItems = [
    { href: '/user', label: 'Home', icon: 'dashboard' },
    { href: '/user/marketplace', label: 'Marketplace', icon: 'marketplace' },
    { href: '/user/skills', label: 'My Skills', icon: 'skills' },
    { href: '/user/submissions', label: 'Submissions', icon: 'review' },
    { href: '/profile', label: 'Profile', icon: 'profile' },
    { href: '/my-api-keys', label: 'API Keys', icon: 'my-api-keys' },
  ];

  const orgNavItems = [
    { href: '/organizations', label: 'Organizations', icon: 'organizations' },
  ];

  // 用 $: 预计算高亮 map，模板直接查表，避免函数内 store 订阅失效
  // 支持 ?from=xxx 参数保留来源 tab 高亮
  $: activeMap = (() => {
    const path = $location.pathname;
    const from = new URLSearchParams($location.search || '').get('from') || '';
    const m = {};
    const allItems = [...userNavItems, ...orgNavItems];
    for (const item of allItems) {
      if (item.href === '/user') {
        m[item.href] = path === '/user';
      } else if (path === item.href) {
        // 精确匹配
        m[item.href] = true;
      } else if (from && item.href.endsWith('/' + from)) {
        // ?from=marketplace → 高亮 /user/marketplace tab
        m[item.href] = true;
      } else {
        m[item.href] = false;
      }
    }
    return m;
  })();
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
      <button
        on:click={() => handleNavigate(item.href)}
        class="w-full flex items-center gap-3 px-3 py-2.5 mb-0.5 rounded-lg text-[13px] font-medium transition-all duration-200 {activeMap[item.href] ? 'bg-blue-50 text-blue-700' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'}"
      >
        <Icon name={item.icon} size="w-[17px] h-[17px]" className={activeMap[item.href] ? 'text-blue-600' : 'text-gray-400'} />
        {item.label}
      </button>
    {/each}

    {#if hasOrgAccess}
      <div class="mt-4 mb-1 px-3 text-[10px] font-semibold text-gray-400 uppercase tracking-wider">Organizations</div>
      {#each orgNavItems as item}
        <button
          on:click={() => handleNavigate(item.href)}
          class="w-full flex items-center gap-3 px-3 py-2.5 mb-0.5 rounded-lg text-[13px] font-medium transition-all duration-200 {activeMap[item.href] ? 'bg-blue-50 text-blue-700' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'}"
        >
          <Icon name={item.icon} size="w-[17px] h-[17px]" className={activeMap[item.href] ? 'text-blue-600' : 'text-gray-400'} />
          {item.label}
        </button>
      {/each}
    {/if}
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
