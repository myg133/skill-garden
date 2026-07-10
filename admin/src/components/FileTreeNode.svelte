<script>
  export let node;
  export let selectedFilePath = '';
  export let selectFile = () => {};
  export let formatSize = (b) => b;
  export let fileIconColor = () => 'text-gray-400';
  export let depth = 0;

  let collapsed = node.type === 'dir';

  function handleClick() {
    if (node.type === 'dir') {
      collapsed = !collapsed;
    } else {
      selectFile(node.path);
    }
  }

  $: isSkillMd = node.name === 'SKILL.md';
  $: indentPx = depth * 16 + 8;
</script>

{#if node.type === 'dir'}
  <div>
    <button
      class="w-full flex items-center gap-1.5 py-1 rounded text-xs text-left transition-colors hover:bg-gray-200/60 text-gray-600"
      style="padding-left: {indentPx}px; padding-right: 6px;"
      on:click={handleClick}
    >
      <svg
        class="w-3 h-3 flex-shrink-0 text-gray-400 transition-transform duration-150"
        class:rotate-90={!collapsed}
        fill="none" stroke="currentColor" viewBox="0 0 24 24"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
      </svg>
      <svg class="w-3.5 h-3.5 flex-shrink-0 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        {#if collapsed}
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
        {:else}
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"/>
        {/if}
      </svg>
      <span class="truncate font-medium">{node.name}</span>
    </button>
    {#if !collapsed}
      {#each node.children as child (child.path || child.name)}
        <svelte:self
          node={child}
          {selectedFilePath}
          {selectFile}
          {formatSize}
          {fileIconColor}
          depth={depth + 1}
        />
      {/each}
    {/if}
  </div>
{:else}
  <button
    class="w-full flex items-center gap-2 py-1 rounded text-xs text-left transition-colors
           {selectedFilePath === node.path
             ? 'bg-blue-100 text-blue-700 font-semibold'
             : 'text-gray-700 hover:bg-gray-200'}"
    style="padding-left: {indentPx}px; padding-right: 6px;"
    on:click={handleClick}
  >
    <span class="text-xs flex-shrink-0 {isSkillMd ? 'text-blue-500' : fileIconColor(node.name)}">
      {#if isSkillMd}
        <svg class="w-3.5 h-3.5 inline" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
        </svg>
      {:else}
        <svg class="w-3.5 h-3.5 inline" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"/>
        </svg>
      {/if}
    </span>
    <span class="truncate">{node.name}</span>
    <span class="text-[10px] text-gray-400 ml-auto flex-shrink-0">{formatSize(node.size)}</span>
  </button>
{/if}
