<script>
  import { createEventDispatcher } from 'svelte';

  export let show = false;
  export let skillName = '';

  const dispatch = createEventDispatcher();

  let reason = '';
  let error = '';

  function resetForm() {
    reason = '';
    error = '';
  }

  function handleSubmit() {
    if (reason.length < 10) {
      error = 'Reason must be at least 10 characters';
      return;
    }
    dispatch('submit', reason);
  }
</script>
$: if (!show) resetForm();

{#if show}
<div class="fixed inset-0 bg-surface-900/40 backdrop-blur-sm flex items-center justify-center z-50 modal-overlay">
  <div class="bg-slate-50 rounded-2xl p-6 w-full max-w-md shadow-elevated-lg border border-indigo-200 modal-content">
    <div class="flex items-center gap-3 mb-5">
      <div class="w-10 h-10 rounded-xl bg-rose-100 flex items-center justify-center">
        <svg class="w-5 h-5 text-rose-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4.5c-.77-.833-2.694-.833-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z"/>
        </svg>
      </div>
      <div>
        <h3 class="font-semibold text-surface-800 text-[15px]">Reject Skill</h3>
        <p class="text-surface-400 text-xs font-medium">&ldquo;{skillName}&rdquo;</p>
      </div>
    </div>

    <div class="mb-4">
      <label class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Rejection Reason</label>
      <textarea
        bind:value={reason}
        placeholder="Explain why this skill is being rejected (min 10 characters)..."
        rows="4"
        class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none transition-all resize-none font-medium bg-surface-50 text-surface-700 placeholder:text-surface-400"
      ></textarea>
      <div class="flex justify-between mt-1.5">
        <span class="text-surface-400 text-[11px]">{reason.length} / 10 min</span>
        {#if error}
          <span class="text-rose-500 text-[11px] font-medium">{error}</span>
        {/if}
      </div>
    </div>

    <div class="flex justify-end gap-3 pt-1">
      <button
        on:click={() => dispatch('cancel')}
        class="px-4 py-2.5 text-surface-500 hover:text-surface-700 hover:bg-surface-100 rounded-xl font-semibold text-sm transition-all"
      >
        Cancel
      </button>
      <button
        on:click={handleSubmit}
        class="px-5 py-2.5 bg-rose-500 hover:bg-rose-600 rounded-xl font-semibold text-sm transition-all shadow-sm shadow-rose-500/20"
      >
        Reject Skill
      </button>
    </div>
  </div>
</div>
{/if}