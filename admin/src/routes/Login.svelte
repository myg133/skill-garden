<script>
  import { auth } from '../stores/auth.js';
  import { api } from '../lib/api.js';

  let username = '';
  let password = '';
  let error = '';
  let loading = false;
  let showPassword = false;

  async function handleLogin() {
    if (!username || !password) {
      error = 'Please fill in all fields';
      return;
    }
    loading = true;
    error = '';
    try {
      const res = await api.adminLogin(username, password);
      auth.login(res.token, res.user?.username || username);
    } catch (e) {
      error = e.message || 'Login failed';
    } finally {
      loading = false;
    }
  }
</script>

<div class="min-h-screen relative overflow-hidden flex items-center justify-center p-6" style="background: linear-gradient(160deg, #dbeafe 0%, #e0f2fe 30%, #f0f9ff 60%, #f8fafc 100%);">
  <div class="absolute inset-0 bg-dot-pattern opacity-40"></div>

  <div class="absolute top-1/4 -left-24 w-80 h-80 rounded-full bg-purple-400/15 blur-3xl"></div>
  <div class="absolute bottom-1/4 -right-24 w-96 h-96 rounded-full bg-indigo-400/12 blur-3xl"></div>
  <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[520px] h-[520px] rounded-full bg-violet-400/8 blur-3xl"></div>

  <div class="max-w-[420px] w-full relative slide-up">
    <div class="text-center mb-8">
      <div class="inline-flex w-16 h-16 rounded-2xl gradient-brand items-center justify-center text-2xl font-bold mb-5 shadow-glow pulse-glow float-anim ring-1 ring-brand-400/20">
        @
      </div>
      <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">AionHive</h1>
      <p class="text-surface-500 text-sm mt-2 font-medium">Admin Console — Sign in to continue</p>
    </div>

    <div class="bg-white/90 backdrop-blur-xl rounded-2xl shadow-elevated-lg p-8 border border-surface-200/60 ring-1 ring-brand-500/5">
      <form on:submit|preventDefault={handleLogin} class="space-y-5">
        <div>
          <label class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Username</label>
          <div class="relative">
            <svg class="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"/>
            </svg>
            <input
              type="text"
              bind:value={username}
              placeholder="Enter your username"
              class="w-full pl-10 pr-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-surface-50/80 placeholder:text-surface-300"
            />
          </div>
        </div>

        <div>
          <label class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Password</label>
          <div class="relative">
            <svg class="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
            </svg>
            {#if showPassword}
            <input
              type="text"
              bind:value={password}
              placeholder="Enter your password"
              class="w-full pl-10 pr-12 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-surface-50/80 placeholder:text-surface-300"
            />
            {:else}
            <input
              type="password"
              bind:value={password}
              placeholder="Enter your password"
              class="w-full pl-10 pr-12 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-surface-50/80 placeholder:text-surface-300"
            />
            {/if}
            <button
              type="button"
              on:click={() => showPassword = !showPassword}
              class="absolute right-3 top-1/2 -translate-y-1/2 p-1 rounded-lg hover:bg-surface-100 transition-colors text-surface-400 hover:text-surface-600"
              tabindex="-1"
            >
              {#if showPassword}
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M15 12a3 3 0 01-3 3m0 0l-6 6m6-6l6 6"/>
                </svg>
              {:else}
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/>
                </svg>
              {/if}
            </button>
          </div>
        </div>

        {#if error}
          <div class="text-rose-600 text-sm font-medium bg-rose-50 px-4 py-3 rounded-xl border border-rose-100">
            {error}
          </div>
        {/if}

        <button
          type="submit"
          disabled={loading}
          class="w-full py-3 rounded-xl font-semibold text-sm flex items-center justify-center gap-2 shadow-lg transition-all duration-300"
          style="background: linear-gradient(135deg, #4f46e5, #3730a3); border: none; cursor: pointer;"
          class:opacity-60={loading}
          class:cursor-not-allowed={loading}
        >
          {#if loading}
            <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/>
            </svg>
            Signing in...
          {:else}
            Sign In
          {/if}
        </button>
      </form>
    </div>

    <p class="text-center text-surface-500 text-xs mt-6 font-medium">
      AionHive v0.3.0
    </p>
  </div>
</div>