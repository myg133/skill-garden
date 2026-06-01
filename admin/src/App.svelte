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
  import Register from './routes/Register.svelte';
  import Organizations from './routes/Organizations.svelte';
  import OrganizationDetail from './routes/OrganizationDetail.svelte';
  import Sessions from './routes/Sessions.svelte';
  import OrgTools from './routes/OrgTools.svelte';
  import Settings from './routes/Settings.svelte';
  import Tenants from './routes/Tenants.svelte';
  import Identities from './routes/Identities.svelte';
  import Groups from './routes/Groups.svelte';
  import GroupDetail from './routes/GroupDetail.svelte';
  import Roles from './routes/Roles.svelte';
  import ApiKeys from './routes/ApiKeys.svelte';
  import Skills from './routes/Skills.svelte';
  import Profile from './routes/Profile.svelte';
  import MyApiKeys from './routes/MyApiKeys.svelte';

  export let url = '';

  $: showLogin = !$isAuthenticated;
</script>

<Router {url}>
  {#if showLogin}
    <div class="min-h-screen bg-surface-950">
      <Route path="/login" component={Login} />
      <Route path="/register" component={Register} />
      <Route path="*" component={Login} />
    </div>
  {:else}
    <div class="flex h-screen overflow-hidden" style="background: linear-gradient(160deg, #dbeafe 0%, #e0f2fe 30%, #f0f9ff 60%, #f8fafc 100%);">
      <Nav />
      <div class="flex-1 overflow-y-auto relative">
        <main class="relative fade-in">
          <Route path="/" component={Organizations} />
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
          <Route path="/identities" component={Identities} />
          <Route path="/groups" component={Groups} />
          <Route path="/groups/:id" component={GroupDetail} />
          <Route path="/roles" component={Roles} />
          <Route path="/api-keys" component={ApiKeys} />
          <Route path="/profile" component={Profile} />
          <Route path="/my-api-keys" component={MyApiKeys} />
        </main>
      </div>
      <Toast />
    </div>
  {/if}
</Router>