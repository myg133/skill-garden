<script>
  import Badge from './Badge.svelte';
  import ReviewActions from './ReviewActions.svelte';
  import { Link } from 'svelte-routing';

  export let skill;
</script>

<tr class="border-b border-gray-100 hover:bg-gray-50">
  <td class="px-4 py-3">
    <Link to="/skills/{skill.id}" class="text-blue-600 hover:text-blue-800 font-medium">
      {skill.name}
    </Link>
  </td>
  <td class="px-4 py-3 text-gray-600 text-sm">{skill.agent_id}</td>
  <td class="px-4 py-3">
    <div class="flex gap-1 flex-wrap">
      {#each (skill.tags || []).slice(0, 3) as tag}
        <span class="px-2 py-0.5 bg-gray-100 text-gray-600 text-xs rounded">{tag}</span>
      {/each}
    </div>
  </td>
  <td class="px-4 py-3 text-gray-600 text-sm">{skill.created_at ? new Date(skill.created_at).toLocaleDateString() : 'N/A'}</td>
  <td class="px-4 py-3">
    <ReviewActions {skill} />
  </td>
</tr>

