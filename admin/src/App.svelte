<script>
  import { Router, Route, navigate } from 'svelte-routing';
  import { isAuthenticated } from './stores/auth.js';
  import Nav from './components/Nav.svelte';
  import Toast from './components/Toast.svelte';
  import Review from './routes/Review.svelte';
  import SkillDetail from './routes/SkillDetail.svelte';
  import AuditLogs from './routes/AuditLogs.svelte';
  import Stats from './routes/Stats.svelte';
  import Login from './routes/Login.svelte';
  import Organizations from './routes/Organizations.svelte';
  import OrganizationDetail from './routes/OrganizationDetail.svelte';
  import Sessions from './routes/Sessions.svelte';
  import OrgTools from './routes/OrgTools.svelte';
  import Settings from './routes/Settings.svelte';

  export let url = '';

  // Always show Login when not authenticated
  $: showLogin = !$isAuthenticated;
</script>

<Router {url}>
  <div class="min-h-screen bg-gray-50">
    {#if showLogin}
      <Route path="/login" component={Login} />
      <Route path="*" component={Login} />
    {:else}
      <Nav />
      <main>
        <Route path="/" component={Organizations} />
        <Route path="/review" component={Review} />
        <Route path="/skills/:id" component={SkillDetail} />
        <Route path="/audit" component={AuditLogs} />
        <Route path="/stats" component={Stats} />
        <Route path="/organizations" component={Organizations} />
        <Route path="/organizations/:id" component={OrganizationDetail} />
        <Route path="/sessions" component={Sessions} />
        <Route path="/org-tools" component={OrgTools} />
        <Route path="/settings" component={Settings} />
      </main>
    {/if}
    <Toast />
  </div>
</Router>
