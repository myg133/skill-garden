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
<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
  <div class="bg-white rounded-lg p-6 w-full max-w-md">
    <h3 class="text-lg font-semibold mb-4">Reject "{skillName}"</h3>
    <textarea
      bind:value={reason}
      placeholder="Rejection reason (min 10 characters)"
      rows="4"
      class="w-full px-3 py-2 border border-gray-300 rounded mb-2"
    ></textarea>
    {#if error}
      <p class="text-red-500 text-sm mb-2">{error}</p>
    {/if}
    <div class="flex justify-end gap-2">
      <button
        on:click={() => dispatch('cancel')}
        class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded">
        Cancel
      </button>
      <button
        on:click={handleSubmit}
        class="px-4 py-2 text-white bg-red-600 rounded hover:bg-red-700">
        Reject
      </button>
    </div>
  </div>
</div>
{/if}
