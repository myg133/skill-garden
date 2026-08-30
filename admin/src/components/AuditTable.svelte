<script>
  import { _ } from 'svelte-i18n';
  export let logs = [];

  function actionBadge(action) {
    if (!action) return 'bg-gray-100 text-gray-500 ring-1 ring-surface-600/10';
    if (action.includes('create')) return 'bg-emerald-50 text-emerald-600 ring-1 ring-emerald-600/20';
    if (action.includes('approve')) return 'bg-blue-50 text-blue-600 ring-1 ring-blue-600/20';
    if (action.includes('reject')) return 'bg-rose-50 text-rose-600 ring-1 ring-rose-600/20';
    if (action.includes('delete')) return 'bg-amber-50 text-amber-600 ring-1 ring-amber-600/20';
    if (action.includes('update')) return 'bg-blue-50 text-blue-600 ring-1 ring-blue-600/20';
    return 'bg-gray-100 text-gray-500 ring-1 ring-surface-600/10';
  }

  function identityTypeLabel(identityType) {
    if (!identityType) return '';
    switch (identityType) {
      case 'user': return 'User';
      case 'agent': return 'API Key';
      case 'external_agent': return 'External';
      case 'system': return 'System';
      default: return identityType;
    }
  }

  function identityTypeBadge(identityType) {
    if (!identityType) return 'bg-gray-50 text-gray-400';
    switch (identityType) {
      case 'user': return 'bg-indigo-50 text-indigo-600';
      case 'agent': return 'bg-cyan-50 text-cyan-600';
      case 'external_agent': return 'bg-violet-50 text-violet-600';
      case 'system': return 'bg-amber-50 text-amber-600';
      default: return 'bg-gray-50 text-gray-400';
    }
  }

  function detailsSummary(details) {
    if (!details || typeof details !== 'object') return '';
    // Extract human-readable summary from details JSON
    const parts = [];
    if (details.skill_name) parts.push(details.skill_name);
    if (details.key_name) parts.push(details.key_name);
    if (details.action === 'rejected' && details.reason) parts.push('reason: ' + details.reason);
    if (details.action === 'approved') parts.push('approved');
    if (details.comment) parts.push(details.comment);
    if (details.scopes) parts.push('scopes: ' + JSON.stringify(details.scopes));
    return parts.join(' | ') || '—';
  }
</script>

<table class="w-full">
  <thead>
    <tr class="border-b border-gray-100 bg-gray-50">
      <th class="px-4 py-2 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">{$_('auditLogs.timestamp')}</th>
      <th class="px-4 py-2 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">{$_('auditLogs.operator')}</th>
      <th class="px-4 py-2 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">{$_('auditLogs.action')}</th>
      <th class="px-4 py-2 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">{$_('auditLogs.resource')}</th>
      <th class="px-4 py-2 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">{$_('auditLogs.details')}</th>
      <th class="px-4 py-2 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">{$_('auditLogs.ip')}</th>
    </tr>
  </thead>
  <tbody class="divide-y divide-gray-50">
    {#each logs as log}
      <tr class="table-row hover:bg-gray-50">
        <td class="px-4 py-2 text-xs text-gray-500 font-mono">{log.timestamp ? new Date(log.timestamp).toLocaleString() : 'N/A'}</td>
        <td class="px-4 py-2 text-sm text-gray-600">
          <div class="flex items-center gap-2">
            <span>{log.identity_name || log.agent_id || '—'}</span>
            {#if log.identity_type}
              <span class="inline-flex px-1.5 py-0.5 rounded text-[10px] font-medium leading-none {identityTypeBadge(log.identity_type)}">
                {identityTypeLabel(log.identity_type)}
              </span>
            {/if}
          </div>
        </td>
        <td class="px-4 py-2">
          <span class="inline-flex px-2 py-0.5 rounded-full text-[10px] font-semibold leading-none {actionBadge(log.action)}">{log.action}</span>
        </td>
        <td class="px-4 py-2 text-sm text-gray-500 whitespace-nowrap">{log.resource_type || 'unknown'}: <span class="text-gray-400 font-mono text-xs">{log.resource_id || 'unknown'}</span></td>
        <td class="px-4 py-2 text-sm text-gray-500 max-w-xs truncate" title={detailsSummary(log.details)}>{detailsSummary(log.details)}</td>
        <td class="px-4 py-2 text-xs text-gray-400 font-mono">{log.ip_address || '—'}</td>
      </tr>
    {/each}
  </tbody>
</table>