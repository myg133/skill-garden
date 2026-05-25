<script>
  import { toasts } from '../stores/app.js';
</script>

<div class="fixed top-4 right-4 z-50 flex flex-col gap-2">
  {#each $toasts as toast (toast.id)}
    <div class="px-4 py-3 rounded-lg shadow-lg text-white {
      toast.type === 'success' ? 'bg-green-500' : 'bg-red-500'
    }">
      {toast.message}
    </div>
  {/each}
</div>
