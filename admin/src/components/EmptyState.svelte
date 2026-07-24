<script>
  export let message = 'No data found';
</script>

<div class="flex flex-col items-center justify-center py-16 text-center">
  <div class="w-16 h-16 rounded-2xl bg-gray-100 flex items-center justify-center mb-5">
    <svg class="w-8 h-8 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"/>
    </svg>
  </div>
  <p class="text-gray-500 font-medium text-sm">{message}</p>
  <p class="text-gray-400 text-xs mt-1">Check back later or adjust your filters</p>
</div>