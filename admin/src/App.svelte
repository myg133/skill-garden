<script>
  import { onMount } from 'svelte';
  import { writable } from 'svelte/store';
  import { Router, Route } from 'svelte-routing';

  import { isAuthenticated, isAdmin } from './stores/auth.js';
  import { isAnyAdmin, permissionStore, isSuperAdmin, isTenantAdmin, isOrgAdmin, getDefaultRoute, getFirstTenantId, getFirstOrgId } from './stores/permission.js';
  import { isLoading as i18nLoading } from './i18n/index.js';
  import Nav from './components/Nav.svelte';
  import UserNav from './components/UserNav.svelte';
  import OrgSwitcher from './components/OrgSwitcher.svelte';
  import RoleBadges from './components/RoleBadges.svelte';
  import LanguageSwitcher from './components/LanguageSwitcher.svelte';
  import Toast from './components/Toast.svelte';
  import Review from './routes/Review.svelte';
  import SkillDetail from './routes/SkillDetail.svelte';
  import AuditLogs from './routes/AuditLogs.svelte';
  import Stats from './routes/Stats.svelte';
  import Login from './routes/Login.svelte';
  import Register from './routes/Register.svelte';
  import Organizations from './routes/Organizations.svelte';
  import OrganizationDetail from './routes/OrganizationDetail.svelte';
  import Marketplace from './routes/Marketplace.svelte';
  import Sessions from './routes/Sessions.svelte';
  import OrgTools from './routes/OrgTools.svelte';
  import Settings from './routes/Settings.svelte';
  import Tenants from './routes/Tenants.svelte';
  import TenantDetail from './routes/TenantDetail.svelte';
  import OrgMembers from './routes/OrgMembers.svelte';
  import Identities from './routes/Identities.svelte';
  import Groups from './routes/Groups.svelte';
  import GroupDetail from './routes/GroupDetail.svelte';
  import Roles from './routes/Roles.svelte';
  import SystemRoles from './routes/SystemRoles.svelte';
  import MarketplaceRoles from './routes/MarketplaceRoles.svelte';
  import ApiKeys from './routes/ApiKeys.svelte';
  import Skills from './routes/Skills.svelte';
  import Profile from './routes/Profile.svelte';
  import MyApiKeys from './routes/MyApiKeys.svelte';
  import Sandbox from './routes/Sandbox.svelte';
  import UserDashboard from './routes/UserDashboard.svelte';
  import MySubmissions from './routes/MySubmissions.svelte';

  export let url = '';

  $: showLogin = !$isAuthenticated;

  // 跟踪当前路径（Router 外部用，拦截 history API 和 popstate）
  const currentPath = writable(window.location.pathname);

  // 只在需要组织上下文的页面显示组织切换器（排除详情页如 /skills/:id、/organizations/:id）
  $: showOrgSwitcher = /^\/(skills|review|organizations|groups|org-tools)(\?|$)/.test($currentPath);

  // 仅 skill 相关页面保留 Personal Space 选项；组织/分组管理页面不展示个人空间
  $: showPersonalOption = /^\/(skills|review)(\/|$)/.test($currentPath);

  // 权限未加载时不急着决定布局，避免 user/admin 布局切换的闪烁
  // loaded 后：管理员或组织角色用户进入 admin 布局
  $: hasOrgRole = ($permissionStore.orgRoles || []).length > 0;
  $: showAdminLayout = $permissionStore.loaded && ($isAdmin || isAnyAdmin() || hasOrgRole);

  // 已登录但权限还在加载中 → 展示加载状态
  $: permissionsLoading = $isAuthenticated && !$permissionStore.loaded;

  // 角色判断
  $: isSA = $permissionStore.loaded && isSuperAdmin();
  $: isTA = $permissionStore.loaded && isTenantAdmin();
  $: isOA = $permissionStore.loaded && isOrgAdmin();

  // 首次登录时记录是否已处理默认路由重定向
  let defaultRouteHandled = false;
  let pendingRedirect = null;

  // 当权限加载完成后，检查是否需要重定向到默认页面
  $: if ($permissionStore.loaded && !defaultRouteHandled && $isAuthenticated) {
    const path = window.location.pathname;
    // 仅在根路径或用户仪表盘时重定向
    if (path === '/' || path === '/user') {
      const defaultRoute = getDefaultRoute();
      if (defaultRoute && defaultRoute !== path) {
        pendingRedirect = defaultRoute;
      }
    }
    defaultRouteHandled = true;
  }

  // 执行重定向
  $: if (pendingRedirect && typeof window !== 'undefined') {
    window.location.href = pendingRedirect;
    pendingRedirect = null;
  }

  onMount(async () => {
    // 拦截 svelte-routing 的 navigation
    const updatePath = () => currentPath.set(window.location.pathname);
    const origPush = history.pushState.bind(history);
    const origReplace = history.replaceState.bind(history);
    history.pushState = (...args) => { origPush(...args); updatePath(); };
    history.replaceState = (...args) => { origReplace(...args); updatePath(); };
    window.addEventListener('popstate', updatePath);

    // 页面刷新后重新拉取权限，确保 permissionStore 有最新数据
    if ($isAuthenticated) {
      await permissionStore.refresh();
    }

    return () => {
      history.pushState = origPush;
      history.replaceState = origReplace;
      window.removeEventListener('popstate', updatePath);
    };
  });
</script>

<Router {url}>
  {#if $i18nLoading}
    <!-- i18n 加载中，防止 $_() 调用报错 -->
    <div class="min-h-screen flex items-center justify-center bg-surface-950">
      <div class="w-10 h-10 rounded-full border-2 border-blue-500 border-t-transparent animate-spin"></div>
    </div>
  {:else if showLogin}
    <div class="min-h-screen bg-surface-950">
      <!-- 登录页面语言切换器 -->
      <div class="absolute top-4 right-4 z-50">
        <LanguageSwitcher />
      </div>
      <Route path="/login" component={Login} />
      <Route path="/register" component={Register} />
      <Route path="*" component={Login} />
    </div>
  {:else if permissionsLoading}
    <!-- 已登录但权限数据加载中，防止 user/admin 布局闪烁 -->
    <div class="flex h-screen overflow-hidden bg-gray-50 items-center justify-center">
      <div class="flex flex-col items-center gap-3">
        <div class="w-10 h-10 rounded-full border-2 border-blue-500 border-t-transparent animate-spin"></div>
        <p class="text-sm text-gray-400 font-medium" data-i18n="app.loading">Loading…</p>
      </div>
    </div>
  {:else if showAdminLayout}
    <!-- ========== Admin Layout ========== -->
    <div class="flex h-screen overflow-hidden bg-gray-50">
      <Nav />
      <div class="flex-1 flex flex-col overflow-hidden">
        <!-- Top bar with OrgSwitcher -->
        <div class="h-14 flex items-center px-6 border-b border-gray-200 bg-white flex-shrink-0">
          {#if showOrgSwitcher}
            <OrgSwitcher showPersonal={showPersonalOption} />
          {/if}
          <RoleBadges className="ml-auto flex-shrink-0" />
          <LanguageSwitcher className="ml-4 flex-shrink-0" />
        </div>
        <!-- Content area -->
        <div class="flex-1 overflow-y-auto relative">
          <main class="relative">
            <Route path="/" component={Organizations} />
            <Route path="/marketplace" component={Marketplace} />
            <Route path="/review" component={Review} />
            <Route path="/skills/:id" component={SkillDetail} />
            <Route path="/skills" component={Skills} />
            <Route path="/audit" component={AuditLogs} />
            <Route path="/stats" component={Stats} />
            <Route path="/organizations" component={Organizations} />
            <Route path="/organizations/:id" component={OrganizationDetail} />
            <Route path="/sessions" component={Sessions} />
            <Route path="/org-tools" component={OrgTools} />
            <Route path="/settings" component={Settings} />
            <Route path="/tenants" component={Tenants} />
            <!-- 租户详情页 - 用于 tenant_admin 默认着陆 -->
            <Route path="/tenants/:id" let:params>
              <TenantDetail id={params.id} />
            </Route>
            <!-- 组织成员页 - 用于 org_admin 快捷访问 -->
            <Route path="/org-members" component={OrgMembers} />
            <Route path="/identities" component={Identities} />
            <Route path="/groups" component={Groups} />
            <Route path="/groups/:id" component={GroupDetail} />
            <Route path="/system-roles" component={SystemRoles} />
            <Route path="/marketplace-roles" component={MarketplaceRoles} />
            <Route path="/roles" component={Roles} />
            <Route path="/api-keys" component={ApiKeys} />
            <Route path="/profile" component={Profile} />
            <Route path="/my-api-keys" component={MyApiKeys} />
            <Route path="/sandboxes" component={Sandbox} />
          </main>
        </div>
      </div>
      <Toast />
    </div>
  {:else}
    <!-- ========== User Layout ========== -->
    <div class="flex h-screen overflow-hidden bg-gray-50">
      <UserNav />
      <div class="flex-1 flex flex-col overflow-hidden">
        <!-- Top bar with OrgSwitcher + role badges -->
        <div class="h-14 flex items-center px-6 border-b border-gray-200 bg-white flex-shrink-0">
          {#if showOrgSwitcher}
            <OrgSwitcher showPersonal={showPersonalOption} />
          {/if}
          <RoleBadges className="ml-auto flex-shrink-0" />
          <LanguageSwitcher className="ml-4 flex-shrink-0" />
        </div>
        <div class="flex-1 overflow-y-auto relative">
          <main class="relative">
            <Route path="/" component={UserDashboard} />
            <Route path="/user" component={UserDashboard} />
            <Route path="/user/marketplace" component={Marketplace} />
            <Route path="/user/skills/:id" let:params>
              <SkillDetail id={params.id} />
            </Route>
            <Route path="/user/skills" component={Skills} />
            <Route path="/user/submissions" component={MySubmissions} />
            <Route path="/profile" component={Profile} />
            <Route path="/my-api-keys" component={MyApiKeys} />
            <!-- Organization management for users with org roles -->
            <Route path="/organizations/:id" let:params>
              <OrganizationDetail id={params.id} />
            </Route>
            <Route path="/organizations" component={Organizations} />
            <Route path="/groups" component={Groups} />
            <Route path="*" component={UserDashboard} />
          </main>
        </div>
      </div>
      <Toast />
    </div>
  {/if}
</Router>