<script>
  import { toasts } from '../stores/app.js';
</script>

<div class="fixed bottom-6 right-6 z-50 flex flex-col gap-2">
  {#each $toasts as toast (toast.id)}
    <div class="px-5 py-3 rounded-2xl shadow-elevated text-sm font-medium scale-in {toast.type === 'success' ? 'bg-emerald-600' : 'bg-rose-600'}">
      <div class="flex items-center gap-2.5">
        {#if toast.type === 'success'}
          <svg class="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
        {:else}
          <svg class="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
        {/if}
        {toast.message}
      </div>
    </div>
  {/each}
</div>