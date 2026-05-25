<script>
  import { navigate } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import { auth, isAuthenticated } from '../stores/auth.js';
  import { onMount } from 'svelte';

  let username = '';
  let password = '';
  let loading = false;
  let error = '';

  onMount(() => {
    if ($isAuthenticated) {
      navigate('/', { replace: true });
    }
  });

  async function handleSubmit() {
    if (!username.trim() || !password.trim()) {
      error = 'Please enter both username and password';
      return;
    }

    loading = true;
    error = '';

    try {
      const res = await api.adminLogin(username.trim(), password);
      auth.login(res.token, res.user.username);
      navigate('/', { replace: true });
    } catch (e) {
      error = e.message || 'Login failed';
      auth.setError(error);
    } finally {
      loading = false;
    }
  }
</script>

<div class="min-h-screen bg-gray-50 flex items-center justify-center px-4">
  <div class="max-w-md w-full">
    <div class="bg-white rounded-lg shadow-md p-8">
      <div class="text-center mb-8">
        <h1 class="text-2xl font-bold text-gray-900">SkillGarden Admin</h1>
        <p class="text-gray-500 mt-2">Sign in to access the admin panel</p>
      </div>

      <form on:submit|preventDefault={handleSubmit} class="space-y-6">
        <div>
          <label for="username" class="block text-sm font-medium text-gray-700 mb-1">
            Username
          </label>
          <input
            id="username"
            type="text"
            bind:value={username}
            placeholder="admin"
            class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
            disabled={loading}
          />
        </div>

        <div>
          <label for="password" class="block text-sm font-medium text-gray-700 mb-1">
            Password
          </label>
          <input
            id="password"
            type="password"
            bind:value={password}
            placeholder="Your password"
            class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
            disabled={loading}
          />
        </div>

        {#if error}
          <div class="bg-red-50 text-red-600 text-sm px-4 py-3 rounded-lg">
            {error}
          </div>
        {/if}

        <button
          type="submit"
          disabled={loading}
          class="w-full bg-blue-600 text-white py-2 px-4 rounded-lg font-medium hover:bg-blue-700 focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {loading ? 'Signing in...' : 'Sign In'}
        </button>
      </form>

      <p class="text-center text-sm text-gray-500 mt-6">
        Default credentials: admin / admin123
      </p>
    </div>
  </div>
</div>
