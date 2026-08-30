<script>
  import { api } from '../lib/api.js';
  import { navigate, link } from 'svelte-routing';
  import { _ } from 'svelte-i18n';

  let username = '';
  let displayName = '';
  let email = '';
  let password = '';
  let confirmPassword = '';
  let tenantName = '';
  let error = '';
  let loading = false;
  let showPassword = false;

  async function handleRegister() {
    if (!username || !email || !password) {
      error = 'Username, email and password are required';
      return;
    }
    if (password !== confirmPassword) {
      error = $_('auth.passwordMismatch');
      return;
    }
    if (password.length < 6) {
      error = 'Password must be at least 6 characters';
      return;
    }
    // SaaS mode: tenant name is required
    if (!tenantName || tenantName.trim().length < 2) {
      error = 'Tenant name is required (at least 2 characters)';
      return;
    }
    if (tenantName.length > 50) {
      error = 'Tenant name must not exceed 50 characters';
      return;
    }
    loading = true;
    error = '';
    try {
      await api.userRegister(username, password, displayName || undefined, email || undefined, tenantName.trim());
      navigate('/login', { replace: true });
    } catch (e) {
      error = e.message || $_('auth.registrationFailed');
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
      <img src="/images/logo.png" alt="AionHive" class="w-20 h-20 rounded-2xl mb-5 shadow-glow float-anim mx-auto block" />
      <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">AionHive</h1>
      <p class="text-surface-500 text-sm mt-2 font-medium">{$_('auth.createAccount')}</p>
    </div>

    <div class="bg-white/90 backdrop-blur-xl rounded-2xl shadow-elevated-lg p-8 border border-surface-200/60 ring-1 ring-brand-500/5">
      <form on:submit|preventDefault={handleRegister} class="space-y-4">
        <div>
          <label for="reg-username" class="block text-xs font-semibold text-surface-600 uppercase tracking-wider mb-2">{$_('auth.username')} <span class="text-rose-500">*</span></label>
          <input
            id="reg-username"
            type="text"
            bind:value={username}
            placeholder={$_('auth.chooseUsername')}
            class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-surface-50/80 text-surface-800 placeholder:text-surface-500"
          />
        </div>

        <div>
          <label for="reg-display-name" class="block text-xs font-semibold text-surface-600 uppercase tracking-wider mb-2">{$_('profile.displayName')}</label>
          <input
            id="reg-display-name"
            type="text"
            bind:value={displayName}
            placeholder={$_('auth.displayNameOptional')}
            class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-surface-50/80 text-surface-800 placeholder:text-surface-500"
          />
        </div>

        <!-- SaaS mode: tenant name field -->
        <div>
          <label for="reg-tenant-name" class="block text-xs font-semibold text-surface-600 uppercase tracking-wider mb-2">
            {$_('auth.tenantName') || 'Tenant Name'} <span class="text-rose-500">*</span>
          </label>
          <input
            id="reg-tenant-name"
            type="text"
            bind:value={tenantName}
            placeholder={$_('auth.tenantNamePlaceholder') || 'Your Organization Name'}
            class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-surface-50/80 text-surface-800 placeholder:text-surface-500"
          />
          <p class="text-xs text-surface-500 mt-1">2-50 characters, will be your workspace name</p>
        </div>

        <div>
          <label for="reg-email" class="block text-xs font-semibold text-surface-600 uppercase tracking-wider mb-2">{$_('auth.email')} <span class="text-rose-500">*</span></label>
          <input
            id="reg-email"
            type="email"
            bind:value={email}
            required
            placeholder={$_('auth.enterEmail')}
            class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-surface-50/80 text-surface-800 placeholder:text-surface-500"
          />
        </div>

        <div>
          <label for="reg-password" class="block text-xs font-semibold text-surface-600 uppercase tracking-wider mb-2">{$_('auth.password')} <span class="text-rose-500">*</span></label>
          <div class="relative">
            {#if showPassword}
            <input
              id="reg-password"
              type="text"
              bind:value={password}
              placeholder={$_('auth.passwordAtLeast6')}
              class="w-full px-4 pr-12 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-surface-50/80 text-surface-800 placeholder:text-surface-500"
            />
            {:else}
            <input
              id="reg-password"
              type="password"
              bind:value={password}
              placeholder={$_('auth.passwordAtLeast6')}
              class="w-full px-4 pr-12 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-surface-50/80 text-surface-800 placeholder:text-surface-500"
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

        <div>
          <label for="reg-confirm-password" class="block text-xs font-semibold text-surface-600 uppercase tracking-wider mb-2">{$_('auth.confirmPassword')} <span class="text-rose-500">*</span></label>
          {#if showPassword}
          <input
            id="reg-confirm-password"
            type="text"
            bind:value={confirmPassword}
            placeholder={$_('auth.enterConfirmPassword')}
            class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-surface-50/80 text-surface-800 placeholder:text-surface-500"
          />
          {:else}
          <input
            id="reg-confirm-password"
            type="password"
            bind:value={confirmPassword}
            placeholder={$_('auth.enterConfirmPassword')}
            class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-surface-50/80 text-surface-800 placeholder:text-surface-500"
          />
          {/if}
        </div>

        {#if error}
          <div class="text-rose-600 text-sm font-medium bg-rose-50 px-4 py-3 rounded-xl border border-rose-100">
            {error}
          </div>
        {/if}

        <button
          type="submit"
          disabled={loading}
          class="w-full py-3 rounded-xl font-semibold text-sm flex items-center justify-center gap-2 shadow-lg transition-all duration-300 text-white"
          style="background: linear-gradient(135deg, #4f46e5, #3730a3); border: none; cursor: pointer;"
          class:opacity-60={loading}
          class:cursor-not-allowed={loading}
        >
          {#if loading}
            <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/>
            </svg>
            {$_('auth.registering')}
          {:else}
            {$_('auth.createAccountButton')}
          {/if}
        </button>
      </form>
    </div>

    <p class="text-center text-surface-500 text-sm mt-5 font-medium">
      {$_('auth.alreadyHaveAccount')} <a href="/login" use:link class="text-brand-600 hover:text-brand-700 font-semibold">{$_('auth.login')}</a>
    </p>

    <p class="text-center text-surface-500 text-xs mt-6 font-medium">
      {$_('app.name')} {$_('app.version')}
    </p>
  </div>
</div>