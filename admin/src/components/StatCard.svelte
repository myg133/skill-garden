<script>
  export let variant = 'brand';
  export let title = '';
  export let value = 0;
  export let subtitle = '';
  export let icon = '';
</script>

<div
  class="bg-white rounded-2xl border border-gray-200 shadow-card p-5 flex items-center gap-4"
>
  <div
    class="w-12 h-12 rounded-xl flex items-center justify-center flex-shrink-0 {variant === 'brand' ? 'bg-blue-500' : variant === 'amber' ? 'bg-amber-500' : variant === 'green' ? 'bg-emerald-500' : 'bg-gray-500'}"
  >
    {#if icon}
      <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        {@html icon}
      </svg>
    {/if}
  </div>
  <div>
    <p class="text-2xl font-extrabold text-gray-800">{value}</p>
    <p class="text-sm font-semibold text-gray-500">{title}</p>
    {#if subtitle}
      <p class="text-xs text-gray-400 mt-0.5">{subtitle}</p>
    {/if}
  </div>
</div>
