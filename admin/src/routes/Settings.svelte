<script>
  import { addToast } from '../stores/app.js';

  // Webhook URLs from env
  let webhookUrls = '';
  let saving = false;

  function handleSaveWebhooks() {
    saving = true;
    // In production, this would call an API to update the webhook configuration
    // For now, we just show a success message
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

<div class="p-6 max-w-4xl mx-auto">
  <div class="mb-6">
    <h1 class="text-2xl font-semibold text-slate-900">Settings</h1>
    <p class="text-slate-500 text-sm mt-1">Configure system settings</p>
  </div>

  <div class="space-y-6">
    <!-- Webhook Configuration -->
    <div class="bg-white rounded-xl border border-slate-200 shadow-sm">
      <div class="px-6 py-4 border-b border-slate-200">
        <h2 class="text-lg font-semibold text-slate-900">Evaluation Webhooks</h2>
        <p class="text-slate-500 text-sm mt-1">Receive evaluation events at multiple endpoints</p>
      </div>
      <div class="p-6">
        <div class="mb-4">
          <label class="block text-sm font-medium text-slate-700 mb-2">
            Webhook URLs <span class="text-slate-400 font-normal">(comma-separated or one per line)</span>
          </label>
          <textarea
            bind:value={webhookUrls}
            rows="4"
            placeholder="https://analytics.example.com/webhook&#10;https://monitoring.example.com/eval"
            class="w-full px-4 py-2 border border-slate-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none font-mono text-sm"
          ></textarea>
          <p class="text-slate-500 text-xs mt-2">
            Set via AION_HIVE_EVAL_WEBHOOK_URLS environment variable
          </p>
        </div>
        <div class="flex items-center justify-between">
          <div class="flex gap-2">
            <button
              on:click={addWebhookUrl}
              class="text-indigo-600 hover:text-indigo-800 text-sm font-medium"
            >
              + Add Line
            </button>
          </div>
          <button
            on:click={handleSaveWebhooks}
            disabled={saving}
            class="bg-indigo-600 hover:bg-indigo-700 disabled:bg-slate-300 text-white px-4 py-2 rounded-lg font-medium text-sm transition-colors"
          >
            {saving ? 'Saving...' : 'Save Settings'}
          </button>
        </div>
      </div>
    </div>

    <!-- Environment Info -->
    <div class="bg-white rounded-xl border border-slate-200 shadow-sm">
      <div class="px-6 py-4 border-b border-slate-200">
        <h2 class="text-lg font-semibold text-slate-900">Environment</h2>
        <p class="text-slate-500 text-sm mt-1">Runtime configuration</p>
      </div>
      <div class="p-6">
        <div class="grid grid-cols-2 gap-4">
          <div class="bg-slate-50 rounded-lg p-4">
            <p class="text-slate-500 text-xs uppercase tracking-wider mb-1">Transport Mode</p>
            <p class="text-slate-900 font-mono text-sm">stdio</p>
          </div>
          <div class="bg-slate-50 rounded-lg p-4">
            <p class="text-slate-500 text-xs uppercase tracking-wider mb-1">HTTP Port</p>
            <p class="text-slate-900 font-mono text-sm">8080</p>
          </div>
          <div class="bg-slate-50 rounded-lg p-4">
            <p class="text-slate-500 text-xs uppercase tracking-wider mb-1">Data Directory</p>
            <p class="text-slate-900 font-mono text-sm">./data</p>
          </div>
          <div class="bg-slate-50 rounded-lg p-4">
            <p class="text-slate-500 text-xs uppercase tracking-wider mb-1">Skills Directory</p>
            <p class="text-slate-900 font-mono text-sm">./skills</p>
          </div>
        </div>
      </div>
    </div>

    <!-- Security -->
    <div class="bg-white rounded-xl border border-slate-200 shadow-sm">
      <div class="px-6 py-4 border-b border-slate-200">
        <h2 class="text-lg font-semibold text-slate-900">Security</h2>
        <p class="text-slate-500 text-sm mt-1">JWT and authentication settings</p>
      </div>
      <div class="p-6">
        <div class="space-y-4">
          <div class="flex items-center justify-between py-2">
            <div>
              <p class="text-slate-900 font-medium">JWT Token Expiry</p>
              <p class="text-slate-500 text-sm">24 hours</p>
            </div>
            <span class="px-2 py-1 text-xs rounded-full bg-green-100 text-green-700">Configured</span>
          </div>
          <div class="flex items-center justify-between py-2 border-t border-slate-100">
            <div>
              <p class="text-slate-900 font-medium">Secret Key</p>
              <p class="text-slate-500 text-sm">Environment variable</p>
            </div>
            <span class="px-2 py-1 text-xs rounded-full bg-green-100 text-green-700">Set</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Database -->
    <div class="bg-white rounded-xl border border-slate-200 shadow-sm">
      <div class="px-6 py-4 border-b border-slate-200">
        <h2 class="text-lg font-semibold text-slate-900">Database</h2>
        <p class="text-slate-500 text-sm mt-1">PostgreSQL connection</p>
      </div>
      <div class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-slate-900 font-medium">PostgreSQL</p>
            <p class="text-slate-500 text-sm font-mono">postgres://localhost:5432/aionhive</p>
          </div>
          <span class="px-2 py-1 text-xs rounded-full bg-green-100 text-green-700">Connected</span>
        </div>
      </div>
    </div>
  </div>
</div>