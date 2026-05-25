<script>
  import { Link, navigate } from 'svelte-routing';
  import { auth } from '../stores/auth.js';

  function handleLogout() {
    auth.logout();
    navigate('/login', { replace: true });
  }
</script>

<nav class="bg-slate-900 border-b border-slate-700 px-6 py-3">
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-8">
      <Link to="/" class="text-lg font-semibold text-white">AionHive</Link>
      <div class="flex gap-1">
        <Link to="/" class="text-slate-300 hover:text-white hover:bg-slate-800 px-3 py-2 rounded-md text-sm font-medium transition-colors">Organizations</Link>
        <Link to="/sessions" class="text-slate-300 hover:text-white hover:bg-slate-800 px-3 py-2 rounded-md text-sm font-medium transition-colors">Sessions</Link>
        <Link to="/org-tools" class="text-slate-300 hover:text-white hover:bg-slate-800 px-3 py-2 rounded-md text-sm font-medium transition-colors">Org Tools</Link>
        <Link to="/review" class="text-slate-300 hover:text-white hover:bg-slate-800 px-3 py-2 rounded-md text-sm font-medium transition-colors">Review</Link>
        <Link to="/stats" class="text-slate-300 hover:text-white hover:bg-slate-800 px-3 py-2 rounded-md text-sm font-medium transition-colors">Stats</Link>
        <Link to="/settings" class="text-slate-300 hover:text-white hover:bg-slate-800 px-3 py-2 rounded-md text-sm font-medium transition-colors">Settings</Link>
      </div>
    </div>
    <div class="flex items-center gap-4">
      <span class="text-sm text-slate-400">{$auth.agentId}</span>
      <button
        on:click={handleLogout}
        class="text-sm text-slate-300 hover:text-white px-3 py-1 rounded border border-slate-600 hover:border-slate-500 transition-colors"
      >
        Logout
      </button>
    </div>
  </div>
</nav>
