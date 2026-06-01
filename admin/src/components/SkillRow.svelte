<script>
  import Badge from './Badge.svelte';
  import ReviewActions from './ReviewActions.svelte';
  import { Link } from 'svelte-routing';

  export let skill;
</script>

<tr class="table-row hover:bg-surface-800/50">
  <td class="px-6 py-4">
    <Link to="/skills/{skill.id}" class="text-brand-400 hover:text-brand-300 font-semibold text-sm transition-colors">
      {skill.name}
    </Link>
  </td>
  <td class="px-6 py-4 text-surface-400 text-sm font-mono text-xs">{skill.author_agent_id}</td>
  <td class="px-6 py-4">
    <div class="flex gap-1.5 flex-wrap">
      {#each (skill.tags || []).slice(0, 3) as tag}
        <span class="px-2.5 py-1 bg-surface-800 text-surface-400 text-xs font-medium rounded-lg">{tag}</span>
      {/each}
      {#if (skill.tags || []).length > 3}
        <span class="px-2.5 py-1 bg-surface-800 text-surface-500 text-xs font-medium rounded-lg">+{skill.tags.length - 3}</span>
      {/if}
    </div>
  </td>
  <td class="px-6 py-4 text-surface-400 text-sm">{skill.created ? new Date(skill.created).toLocaleDateString() : 'N/A'}</td>
  <td class="px-6 py-4">
    <ReviewActions {skill} />
  </td>
</tr>