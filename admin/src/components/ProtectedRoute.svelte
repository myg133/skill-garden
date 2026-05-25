<script>
  import { navigate } from 'svelte-routing';
  import { isAuthenticated } from '../stores/auth.js';
  import { onMount } from 'svelte';

  export let component = null;

  onMount(() => {
    if (!$isAuthenticated) {
      navigate('/login', { replace: true });
    }
  });

  $: if (!$isAuthenticated && typeof window !== 'undefined') {
    navigate('/login', { replace: true });
  }
</script>

{#if $isAuthenticated && component}
  <svelte:component this={component} />
{/if}
