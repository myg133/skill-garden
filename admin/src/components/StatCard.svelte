<script>
  export let title = '';
  export let value = 0;
  export let subtitle = '';
</script>

<div class="bg-white rounded-lg border border-gray-200 p-4">
  <p class="text-sm text-gray-500 mb-1">{title}</p>
  <p class="text-2xl font-semibold text-gray-900">{value}</p>
  {#if subtitle}
    <p class="text-xs text-gray-400 mt-1">{subtitle}</p>
  {/if}
</div>
