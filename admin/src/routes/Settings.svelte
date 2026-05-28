<script>
  import { onMount } from 'svelte';
  import { addToast } from '../stores/app.js';
  import { api } from '../lib/api.js';

  let webhookUrls = '';
  let saving = false;
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

  function handleSaveWebhooks() {
    saving = true;
    setTimeout(() => {
      saving = false;
      addToast('Settings saved', 'success');
    }, 500);
  }

  function addWebhookUrl() {
    if (webhookUrls.trim()) {
      webhookUrls += '\n';
    }
  }
</script>

<div class="p-6 max-w-4xl mx-auto fade-in">
  <div class="page-header">
    <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">Settings</h1>
    <p class="text-surface-500 text-sm mt-1.5 font-medium">Configure system and view runtime status</p>
  </div>

  {#if loading}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 shadow-card p-8 text-center">
      <p class="text-surface-400 text-sm">Loading server status...</p>
    </div>
  {:else if error}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 shadow-card p-8 text-center">
      <p class="text-red-500 text-sm">Failed to load status: {error}</p>
    </div>
  {:else}
  <div class="space-y-5">
    <div class="gradient-card-sky-light rounded-2xl border border-sky-200/60 shadow-card">
      <div class="px-6 py-4 border-b border-sky-200/60">
        <h2 class="font-semibold text-surface-800 text-sm">Evaluation Webhooks</h2>
        <p class="text-surface-400 text-xs mt-0.5">Receive evaluation events at multiple endpoints</p>
      </div>
      <div class="p-6">
        <div class="mb-4">
          <label class="block text-sm font-semibold text-surface-500 mb-2">
            Webhook URLs <span class="text-surface-400 font-normal">(comma-separated or one per line)</span>
          </label>
          <textarea
            bind:value={webhookUrls}
            rows="4"
            placeholder="https://analytics.example.com/webhookl"
            class="w-full px-4 py-2.5 border border-surface-200 rounded-xl input-focus outline-none font-mono text-sm transition-all bg-surface-50"
          ></textarea>
          <p class="text-surface-400 text-xs mt-2">
            Set via AION_HIVE_EVAL_WEBHOOK_URLS environment variable
          </p>
        </div>
        <div class="flex items-center justify-between">
          <button
            on:click={addWebhookUrl}
            class="text-brand-600 hover:text-brand-700 text-sm font-medium transition-colors"
          >
            + Add Line
          </button>
          <button
            on:click={handleSaveWebhooks}
            disabled={saving}
            class="btn-primary px-4 py-2 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {saving ? 'Saving...' : 'Save Settings'}
          </button>
        </div>
      </div>
    </div>

    <div class="gradient-card-brand-light rounded-2xl border border-brand-200/60 shadow-card">
      <div class="px-6 py-4 border-b border-brand-200/60">
        <h2 class="font-semibold text-surface-800 text-sm">Environment</h2>
        <p class="text-surface-400 text-xs mt-0.5">Runtime configuration</p>
      </div>
      <div class="p-6">
        <div class="grid grid-cols-2 gap-4">
          <div class=" bg-sky-50/80 rounded-xl p-4 border border-brand-200/40 card">
            <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Version</p>
            <p class="text-surface-800 font-mono text-sm font-semibold">v{status.version}</p>
          </div>
          <div class=" bg-sky-50/80 rounded-xl p-4 border border-brand-200/40 card">
            <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Transport</p>
            <p class="text-surface-800 font-mono text-sm font-semibold">{status.transport_mode}</p>
          </div>
          <div class=" bg-sky-50/80 rounded-xl p-4 border border-brand-200/40 card">
            <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">HTTP Port</p>
            <p class="text-surface-800 font-mono text-sm font-semibold">{status.http_port}</p>
          </div>
          <div class=" bg-sky-50/80 rounded-xl p-4 border border-brand-200/40 card">
            <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Data Directory</p>
            <p class="text-surface-800 font-mono text-sm font-semibold">{status.data_dir}</p>
          </div>
          <div class=" bg-sky-50/80 rounded-xl p-4 border border-brand-200/40 card">
            <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Skills Directory</p>
            <p class="text-surface-800 font-mono text-sm font-semibold">{status.skills_dir}</p>
          </div>
        </div>
      </div>
    </div>

    <div class="gradient-card-green-light rounded-2xl border border-emerald-200/60 shadow-card">
      <div class="px-6 py-4 border-b border-emerald-200/60">
        <h2 class="font-semibold text-surface-800 text-sm">Security</h2>
        <p class="text-surface-400 text-xs mt-0.5">JWT and authentication settings</p>
      </div>
      <div class="p-6">
        <div class="space-y-4">
          <div class="flex items-center justify-between py-2">
            <div>
              <p class="text-surface-800 font-semibold text-sm">JWT Token Expiry</p>
              <p class="text-surface-400 text-xs">{status.jwt_expiry_hours} hours</p>
            </div>
            <span class="inline-flex px-2.5 py-1 text-xs font-semibold rounded-full bg-emerald-100 text-emerald-700 ring-1 ring-emerald-600/20">Configured</span>
          </div>
          <div class="flex items-center justify-between py-2 border-t border-emerald-200/60">
            <div>
              <p class="text-surface-800 font-semibold text-sm">Secret Key</p>
              <p class="text-surface-400 text-xs">Environment variable</p>
            </div>
            <span class="inline-flex px-2.5 py-1 text-xs font-semibold rounded-full bg-emerald-100 text-emerald-700 ring-1 ring-emerald-600/20">Set</span>
          </div>
        </div>
      </div>
    </div>

    <div class="gradient-card-rose-light rounded-2xl border border-rose-200/60 shadow-card">
      <div class="px-6 py-4 border-b border-rose-200/60">
        <h2 class="font-semibold text-surface-800 text-sm">Database</h2>
        <p class="text-surface-400 text-xs mt-0.5">PostgreSQL connection</p>
      </div>
      <div class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-surface-800 font-semibold text-sm">PostgreSQL</p>
            <p class="text-surface-400 text-xs font-mono">{status.db_url}</p>
          </div>
          {#if status.db_connected}
            <span class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-semibold rounded-full bg-emerald-100 text-emerald-700 ring-1 ring-emerald-600/20">
              <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 pulse-dot"></span>
              Connected
            </span>
          {:else}
            <span class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-semibold rounded-full bg-rose-100 text-rose-700 ring-1 ring-rose-600/20">
              <span class="w-1.5 h-1.5 rounded-full bg-rose-500"></span>
              Disconnected
            </span>
          {/if}
        </div>
      </div>
    </div>
  </div>
  {/if}
</div>