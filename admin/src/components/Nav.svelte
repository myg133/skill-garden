<script>
  import { useLocation, navigate } from 'svelte-routing';
  import { _ } from 'svelte-i18n';
  import { auth, selectedNav } from '../stores/auth.js';
  import { adminNavRoutes } from '../config/nav-routes.js';
  import Icon from './Icon.svelte';

  import { hasPermission, permissionStore } from '../stores/permission.js';

  const STORAGE_KEY = 'nav_collapsed';
  const SIDEBAR_KEY = 'sidebar_collapsed';
  const location = useLocation();

  // 同步路由变化到 selectedNav，保留 from 参数用于恢复来源 tab 上下文
  $: $selectedNav = $location.pathname + ($location.search || '');

  function handleNavigate(href) {
    navigate(href);
    $selectedNav = href;
    // 延迟到 filteredGroups 有值时才处理折叠
    const groups = filteredGroups || [];
    for (const group of groups) {
      const isInGroup = group.children.some(c => pathInGroup(c.href, href));
      if (!isInGroup) {
        manualCollapsed[group.key] = true;
      } else {
        delete manualCollapsed[group.key];
      }
    }
    saveCollapsed(manualCollapsed);
  }

  function pathInGroup(childHref, currentPath) {
    if (childHref === currentPath) return true;
    if (childHref === '/') return currentPath === '/';
    return currentPath.startsWith(childHref + '/') || currentPath.startsWith(childHref + '?');
  }

  function handleLogout() {
    localStorage.removeItem(STORAGE_KEY);
    localStorage.removeItem(SIDEBAR_KEY);
    auth.logout();
    // 退出后直接整页刷新到登录页，避免 SPA 路由状态残留导致布局未切换
    window.location.href = '/login';
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

  let manualCollapsed = loadCollapsed();
  let sidebarCollapsed = false;

  try {
    const saved = localStorage.getItem(SIDEBAR_KEY);
    sidebarCollapsed = saved === 'true';
  } catch {}

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    localStorage.setItem(SIDEBAR_KEY, String(sidebarCollapsed));
  }

  function canSee(child, state) {
    if (!child.need) return true;
    if (child.systemRole) {
      return state.systemRoles.includes(child.systemRole) || state.systemRoles.includes('super_admin');
    }
    return hasPermission(child.need);
  }

  // 权限未加载时不显示任何菜单（避免闪烁先全量再过滤）
  // loaded 后按权限过滤；使用 key 避免响应式重建导致 DOM 丢失点击事件
  $: filteredGroups = $permissionStore.loaded
    ? adminNavRoutes
        .map(g => ({ ...g, children: g.tabs.filter(c => canSee(c, $permissionStore)) }))
        .filter(g => g.children.length > 0)
    : [];

  // 当前实际路由路径
  $: currentPath = $location.pathname;

  // 用 $: 预计算高亮 map（模板直接查表，避免函数内 store 订阅失效）
  // 支持 ?from=xxx 参数保留来源 tab 高亮
  $: activeMap = (() => {
    const path = $location.pathname;
    const from = new URLSearchParams($location.search || '').get('from') || '';
    const m = {};
    for (const group of filteredGroups) {
      for (const child of group.children) {
        if (path === child.href) {
          // 精确匹配
          m[child.href] = true;
        } else if (from && child.href.endsWith('/' + from)) {
          // ?from=marketplace → 高亮 /marketplace tab
          m[child.href] = true;
        } else {
          m[child.href] = false;
        }
      }
    }
    return m;
  })();

  $: groupActiveMap = (() => {
    const path = $location.pathname;
    const from = new URLSearchParams($location.search || '').get('from') || '';
    const m = {};
    // 从 query 参数还原来源 tab
    const effectivePath = from && /^\/.+\/.+/.test(path)
      ? '/' + from
      : path;
    for (const group of filteredGroups) {
      m[group.key] = group.children.some(c => pathInGroup(c.href, effectivePath));
    }
    return m;
  })();

  $: expandedGroups = {};
  $: {
    for (const group of filteredGroups) {
      const from = new URLSearchParams($location.search || '').get('from') || '';
      const effectivePath = from && /^\/.+\/.+/.test(currentPath)
        ? '/' + from
        : currentPath;
      const isCurrentInGroup = group.children.some(c => pathInGroup(c.href, effectivePath));
      expandedGroups[group.key] = isCurrentInGroup
        ? true
        : manualCollapsed[group.key] === false;
    }
  }

  function toggleGroup(key) {
    manualCollapsed = { ...manualCollapsed, [key]: expandedGroups[key] };
    saveCollapsed(manualCollapsed);
  }
</script>

<aside class="relative flex-shrink-0 flex flex-col bg-white border-r border-gray-200 transition-all duration-300 ease-in-out {sidebarCollapsed ? 'w-[64px]' : 'w-[244px]'}">
  <!-- Header -->
  <div class="h-16 flex items-center gap-3 px-3 border-b border-gray-200 {sidebarCollapsed ? 'justify-center' : 'px-5'}">
    <img src="/images/logo.png" alt="AionHive" class="w-10 h-10 rounded-lg flex-shrink-0" />
    {#if !sidebarCollapsed}
      <div class="overflow-hidden">
        <p class="text-base font-bold tracking-tight leading-tight text-gray-900 whitespace-nowrap">{$_('app.name')}</p>
      </div>
    {/if}
  </div>

  <!-- Toggle button -->
  <button
    on:click={toggleSidebar}
    class="absolute top-3 -right-3 w-6 h-6 rounded-full bg-white border border-gray-200 shadow-sm flex items-center justify-center text-indigo-400 hover:text-indigo-600 hover:border-gray-300 transition-all z-10"
    style="transform: translateX(50%);"
    title={sidebarCollapsed ? $_('nav.expandSidebar') : $_('nav.collapseSidebar')}
  >
    <svg class="w-3 h-3 transition-transform duration-300 {sidebarCollapsed ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
    </svg>
  </button>

  <!-- Navigation -->
  <nav class="flex-1 py-5 px-3 overflow-y-auto">
    {#if !$permissionStore.loaded}
      <!-- 权限加载中的骨架屏，避免渲染全量再闪变为过滤结果 -->
      {#if !sidebarCollapsed}
        {#each [1, 2, 3, 4] as _}
          <div class="mb-3">
            <div class="flex items-center gap-3 px-3 py-2.5">
              <div class="w-[18px] h-[18px] rounded bg-gray-200 animate-pulse" />
              <div class="h-3.5 rounded bg-gray-200 animate-pulse flex-1 max-w-[120px]" />
            </div>
          </div>
        {/each}
      {:else}
        {#each [1, 2, 3, 4] as _}
          <div class="mb-3 flex justify-center">
            <div class="w-[18px] h-[18px] rounded bg-gray-200 animate-pulse" />
          </div>
        {/each}
      {/if}
    {:else}
      {#each filteredGroups as group}
        {@const expanded = expandedGroups[group.key]}
        {@const groupActive = groupActiveMap[group.key]}
        <div class="mb-1">
          {#if sidebarCollapsed}
            <button
              on:click={() => toggleGroup(group.key)}
              class="w-full flex items-center justify-center py-2.5 rounded-lg transition-all duration-200 {groupActive ? 'text-blue-600 bg-blue-50' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'}"
              title="{$_(group.labelKey)}{expanded ? '' : ' (click to expand)'}">
              <Icon name={group.icon} size="w-[18px] h-[18px]" className={groupActive ? 'text-blue-600' : 'text-gray-400'} />
            </button>
          {:else}
            <button
              on:click={() => toggleGroup(group.key)}
              class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-[13px] font-semibold transition-all duration-200 group {groupActive && !expanded ? 'text-blue-600 bg-blue-50' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'}"
            >
              <Icon name={group.icon} size="w-[18px] h-[18px]" className={groupActive ? 'text-blue-600' : 'text-gray-400'} />
              <span class="flex-1 text-left">{$_(group.labelKey)}</span>
              <Icon name="chevron-down" size="w-[14px] h-[14px]" className={"text-gray-400 transition-transform duration-200" + (expanded ? ' rotate-180' : '')} />
            </button>
          {/if}

          {#if expanded && !sidebarCollapsed}
            <div class="mt-0.5 ml-2 space-y-0.5 border-l-2 border-gray-100 ml-[23px]">
              {#each group.children as child}
                {@const active = activeMap[child.href]}
                <button
                  on:click={() => handleNavigate(child.href)}
                  class="w-full relative flex items-center gap-3 pl-5 pr-3 py-2 rounded-r-lg text-[13px] font-medium transition-all duration-200 group {active ? 'nav-item-active' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'}"
                >
                  <Icon name={child.icon} size="w-[16px] h-[16px]" className={active ? 'text-blue-600' : 'text-gray-400 group-hover:text-gray-600'} />
                  <span>{$_(child.labelKey)}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    {/if}
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
          title={$_('auth.signOut')}
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
          title={$_('auth.signOut')}
        >
          <Icon name="logout" size="w-[18px] h-[18px]" />
        </button>
      </div>
    {/if}
  </div>
</aside>