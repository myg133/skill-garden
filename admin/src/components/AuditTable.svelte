<script>
  export let logs = [];
</script>

<table class="w-full">
  <thead class="bg-gray-50">
    <tr>
      <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Timestamp</th>
      <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Agent</th>
      <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Action</th>
      <th class="px-4 py-3 text-left text-sm font-medium text-gray-600">Resource</th>
    </tr>
  </thead>
  <tbody>
    {#each logs as log}
      <tr class="border-b border-gray-100 hover:bg-gray-50">
        <td class="px-4 py-3 text-sm text-gray-600">{log.created_at ? new Date(log.created_at).toLocaleString() : 'N/A'}</td>
        <td class="px-4 py-3 text-sm text-gray-600">{log.agent_id || '-'}</td>
        <td class="px-4 py-3 text-sm">
          <span class="px-2 py-1 bg-blue-100 text-blue-700 rounded text-xs">{log.action}</span>
        </td>
        <td class="px-4 py-3 text-sm text-gray-600">{log.resource_type || 'unknown'}: {log.resource_id || 'unknown'}</td>
      </tr>
    {/each}
  </tbody>
</table>
