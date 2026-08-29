<script>
  import { toasts, removeToast } from '../stores/app.js';

  const typeClasses = {
    success: 'bg-emerald-600',
    error: 'bg-rose-600',
    warning: 'bg-amber-500',
    info: 'bg-sky-600',
  };

  function iconByType(type) {
    if (type === 'success') return 'M5 13l4 4L19 7';
    if (type === 'error')   return 'M6 18L18 6M6 6l12 12';
    if (type === 'warning') return 'M12 9v2m0 4h.01M12 3l9.66 16.5H2.34L12 3z';
    return 'M13 16h-1v-4h-1m1-4h.01M12 2a10 10 0 100 20 10 10 0 000-20z'; // info
  }
</script>

<div class="fixed bottom-6 right-6 z-50 flex flex-col gap-2 max-w-sm">
  {#each $toasts as toast (toast.id)}
    <div class="px-5 py-3 pr-9 rounded-2xl shadow-elevated text-sm font-medium scale-in text-white relative {typeClasses[toast.type] || typeClasses.error}">
      <div class="flex items-center gap-2.5">
        <svg class="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={iconByType(toast.type)}/>
        </svg>
        <span class="leading-snug">{toast.message}</span>
      </div>
      <button
        class="absolute top-2 right-2 p-0.5 rounded-full opacity-70 hover:opacity-100 transition-opacity"
        on:click={() => removeToast(toast.id)}
        aria-label="关闭"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
        </svg>
      </button>
    </div>
  {/each}
</div>