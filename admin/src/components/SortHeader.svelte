<script>
  export let label = '';
  export let sortKey = '';
  export let currentSort = { key: '', dir: 'asc' };
  export let onSort = () => {};

  $: isActive = currentSort.key === sortKey;
  $: arrowClass = isActive
    ? (currentSort.dir === 'asc' ? 'rotate-0' : 'rotate-180')
    : 'opacity-0 group-hover:opacity-30';

  function handleClick() {
    if (!sortKey) return;
    const dir = isActive && currentSort.dir === 'asc' ? 'desc' : 'asc';
    onSort(sortKey, dir);
  }
</script>

{#if sortKey}
  <button
    on:click={handleClick}
    class="group flex items-center gap-1 text-xs font-semibold text-gray-400 uppercase tracking-wider hover:text-gray-600 transition-colors"
  >
    {label}
    <svg
      class="w-3 h-3 {arrowClass} transition-all duration-200 {isActive ? 'text-blue-500' : ''}"
      fill="none" stroke="currentColor" viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 15l7-7 7 7"/>
    </svg>
  </button>
{:else}
  <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{label}</span>
{/if}
