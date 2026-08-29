<script>
  import { navigate } from 'svelte-routing';
  import { onMount } from 'svelte';

  onMount(() => {
    navigate('/review', { replace: true });
  });
</script>
