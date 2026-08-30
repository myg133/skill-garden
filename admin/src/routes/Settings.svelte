<script>
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';

  let status = null;
  let loading = true;
  let error = null;

  onMount(async () => {
    try {
      status = await api.getAdminStatus();
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-6 max-w-4xl mx-auto fade-in">
  <div class="page-header">
    <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">{$_('settings.title')}</h1>
    <p class="text-gray-500 text-sm mt-1.5 font-medium">{$_('settings.general')}</p>
  </div>

  {#if loading}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card p-8 text-center">
      <p class="text-gray-400 text-sm">{$_('settings.loadingStatus')}</p>
    </div>
  {:else if error}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card p-8 text-center">
      <p class="text-red-500 text-sm">{$_('settings.failedToLoadStatus')}: {error}</p>
    </div>
  {:else}
  <div class="space-y-5">
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <div class="px-6 py-4 border-b border-gray-100">
        <h2 class="font-semibold text-gray-900 text-sm">{$_('settings.evaluationWebhooks')}</h2>
        <p class="text-gray-400 text-xs mt-0.5">{$_('settings.webhookDescription')}</p>
      </div>
      <div class="p-6">
        <div class="mb-4">
          <label for="webhook-urls" class="block text-sm font-semibold text-gray-500 mb-2">
            {$_('settings.webhookUrls')} <span class="text-gray-400 font-normal">{$_('settings.webhookUrlsHint')}</span>
          </label>
          <textarea
            id="webhook-urls"
            readonly
            rows="4"
            placeholder="https://analytics.example.com/webhook"
            class="w-full px-4 py-2.5 border border-gray-200 rounded-lg input-focus outline-none font-mono text-sm transition-all bg-gray-50 cursor-not-allowed"
          ></textarea>
          <p class="text-gray-400 text-xs mt-2">
            {$_('settings.envVariable', { values: { envVar: 'AION_HIVE_EVAL_WEBHOOK_URLS' }})}
          </p>
        </div>
      </div>
    </div>

    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <div class="px-6 py-4 border-b border-gray-100">
        <h2 class="font-semibold text-gray-900 text-sm">{$_('settings.environment')}</h2>
        <p class="text-gray-400 text-xs mt-0.5">{$_('settings.runtimeConfig')}</p>
      </div>
      <div class="p-6">
        <div class="grid grid-cols-2 gap-4">
          <div class="bg-gray-50 rounded-lg p-4 border border-gray-100">
            <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">{$_('settings.version')}</p>
            <p class="text-gray-900 font-mono text-sm font-semibold">v{status.version}</p>
          </div>
          <div class="bg-gray-50 rounded-lg p-4 border border-gray-100">
            <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">{$_('settings.transport')}</p>
            <p class="text-gray-900 font-mono text-sm font-semibold">{status.transport_mode}</p>
          </div>
          <div class="bg-gray-50 rounded-lg p-4 border border-gray-100">
            <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">{$_('settings.httpPort')}</p>
            <p class="text-gray-900 font-mono text-sm font-semibold">{status.http_port}</p>
          </div>
          <div class="bg-gray-50 rounded-lg p-4 border border-gray-100">
            <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">{$_('settings.dataDir')}</p>
            <p class="text-gray-900 font-mono text-sm font-semibold">{status.data_dir}</p>
          </div>
          <div class="bg-gray-50 rounded-lg p-4 border border-gray-100">
            <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">{$_('settings.skillsDir')}</p>
            <p class="text-gray-900 font-mono text-sm font-semibold">{status.skills_dir}</p>
          </div>
        </div>
      </div>
    </div>

    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <div class="px-6 py-4 border-b border-gray-100">
        <h2 class="font-semibold text-gray-900 text-sm">{$_('settings.security')}</h2>
        <p class="text-gray-400 text-xs mt-0.5">JWT and authentication settings</p>
      </div>
      <div class="p-6">
        <div class="space-y-4">
          <div class="flex items-center justify-between py-2">
            <div>
              <p class="text-gray-900 font-semibold text-sm">{$_('settings.jwtTokenExpiry')}</p>
              <p class="text-gray-400 text-xs">{status.jwt_expiry_hours} hours</p>
            </div>
            <span class="inline-flex px-2.5 py-1 text-xs font-semibold rounded-full bg-emerald-50 text-emerald-600 ring-1 ring-emerald-500/20">{$_('settings.configured')}</span>
          </div>
          <div class="flex items-center justify-between py-2 border-t border-gray-100">
            <div>
              <p class="text-gray-900 font-semibold text-sm">{$_('settings.secretKey')}</p>
              <p class="text-gray-400 text-xs">{$_('settings.envVar')}</p>
            </div>
            <span class="inline-flex px-2.5 py-1 text-xs font-semibold rounded-full bg-emerald-50 text-emerald-600 ring-1 ring-emerald-500/20">{$_('settings.set')}</span>
          </div>
        </div>
      </div>
    </div>

    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <div class="px-6 py-4 border-b border-gray-100">
        <h2 class="font-semibold text-gray-900 text-sm">{$_('settings.database')}</h2>
        <p class="text-gray-400 text-xs mt-0.5">{$_('settings.postgresqlConnection')}</p>
      </div>
      <div class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-gray-900 font-semibold text-sm">PostgreSQL</p>
            <p class="text-gray-400 text-xs font-mono">{status.db_url}</p>
          </div>
          {#if status.db_connected}
            <span class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-semibold rounded-full bg-emerald-50 text-emerald-600 ring-1 ring-emerald-500/20">
              <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 pulse-dot"></span>
              {$_('settings.connected')}
            </span>
          {:else}
            <span class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-semibold rounded-full bg-red-50 text-red-600 ring-1 ring-red-500/20">
              <span class="w-1.5 h-1.5 rounded-full bg-red-500"></span>
              {$_('settings.disconnected')}
            </span>
          {/if}
        </div>
      </div>
    </div>
  </div>
  {/if}
</div>
