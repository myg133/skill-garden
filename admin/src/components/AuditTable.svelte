<script>
  export let logs = [];

  function actionBadge(action) {
    if (!action) return 'bg-surface-100 text-surface-500 ring-1 ring-surface-600/10';
    if (action.includes('create')) return 'bg-emerald-50 text-emerald-600 ring-1 ring-emerald-600/20';
    if (action.includes('approve')) return 'bg-brand-50 text-brand-600 ring-1 ring-brand-600/20';
    if (action.includes('reject')) return 'bg-rose-50 text-rose-600 ring-1 ring-rose-600/20';
    if (action.includes('delete')) return 'bg-amber-50 text-amber-600 ring-1 ring-amber-600/20';
    if (action.includes('update')) return 'bg-sky-50 text-sky-600 ring-1 ring-sky-600/20';
    return 'bg-surface-100 text-surface-500 ring-1 ring-surface-600/10';
  }
</script>

<table class="w-full">
  <thead>
    <tr class="border-b border-surface-100 bg-gradient-to-r from-surface-50/80 to-transparent">
      <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Timestamp</th>
      <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Agent</th>
      <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Action</th>
      <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Resource</th>
    </tr>
  </thead>
  <tbody class="divide-y divide-surface-50">
    {#each logs as log}
      <tr class="table-row hover:bg-surface-50/70">
        <td class="px-6 py-3.5 text-sm text-surface-500 font-mono text-xs">{log.created_at ? new Date(log.created_at).toLocaleString() : 'N/A'}</td>
        <td class="px-6 py-3.5 text-sm text-surface-600">{log.agent_id || '—'}</td>
        <td class="px-6 py-3.5">
          <span class="inline-flex px-2.5 py-1 rounded-full text-xs font-semibold {actionBadge(log.action)}">{log.action}</span>
        </td>
        <td class="px-6 py-3.5 text-sm text-surface-500">{log.resource_type || 'unknown'}: <span class="text-surface-400 font-mono text-xs">{log.resource_id || 'unknown'}</span></td>
      </tr>
    {/each}
  </tbody>
</table>