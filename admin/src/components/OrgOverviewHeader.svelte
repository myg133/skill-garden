<script>
  export let organization = null;
  export let editing = false;
  export let editName = '';
  export let memberCount = 0;
  export let activeSessionCount = 0;
  export let toolCount = 0;
  export let activeTab = 'overview';

  export let onStartEdit = () => {};
  export let onUpdate = () => {};
  export let onCancelEdit = () => {};
  export let onTabChange = () => {};
</script>

<div class="bg-white rounded-2xl border border-gray-200 shadow-card mb-6">
  <div class="px-6 py-5 border-b border-gray-200">
    <div class="flex items-center justify-between">
      {#if editing}
        <div class="flex gap-3 items-center">
          <input
            type="text"
            bind:value={editName}
            class="text-xl font-bold text-gray-800 px-3 py-1.5 border border-gray-200 rounded-xl input-focus outline-none transition-all bg-white"
          />
          <button on:click={onUpdate} class="btn-primary px-3 py-1.5 rounded-lg text-sm font-semibold">Save</button>
          <button on:click={onCancelEdit} class="btn-secondary px-3 py-1.5 rounded-lg text-sm font-semibold">Cancel</button>
        </div>
      {:else}
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">{organization.name}</h1>
        <button on:click={onStartEdit} class="btn-secondary px-4 py-2 rounded-xl text-sm font-semibold">Edit</button>
      {/if}
    </div>
    <p class="text-gray-400 text-xs mt-1.5 font-mono">ID: {organization.id}</p>
    {#if organization.slug}
      <p class="text-gray-400 text-xs mt-0.5">Slug: {organization.slug}</p>
    {/if}
    {#if organization.tenant_id}
      <p class="text-gray-400 text-xs mt-0.5">Tenant: {organization.tenant_id}</p>
    {/if}
  </div>
  <div class="px-6 py-5 grid grid-cols-4 gap-4">
    <div class="bg-gray-50 rounded-xl p-4 border border-gray-200 card">
      <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Created</p>
      <p class="text-gray-800 font-semibold text-sm">{new Date(organization.created_at).toLocaleString()}</p>
    </div>
    <div class="bg-gray-50 rounded-xl p-4 border border-gray-200 card">
      <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Members</p>
      <p class="text-gray-800 font-extrabold text-2xl">{memberCount}</p>
    </div>
    <div class="bg-gray-50 rounded-xl p-4 border border-gray-200 card">
      <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Active Sessions</p>
      <p class="text-gray-800 font-extrabold text-2xl">{activeSessionCount}</p>
    </div>
    <div class="bg-gray-50 rounded-xl p-4 border border-gray-200 card">
      <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Registered Tools</p>
      <p class="text-gray-800 font-extrabold text-2xl">{toolCount}</p>
    </div>
  </div>

  <!-- Tabs -->
  <div class="px-6 border-t border-gray-200 flex gap-0">
    {#each [
      { key: 'overview', label: 'Overview' },
      { key: 'members', label: 'Members' },
      { key: 'sessions', label: 'Sessions' },
      { key: 'tools', label: 'Tools' },
      { key: 'groups', label: 'Groups' }
    ] as tab}
      <button
        on:click={() => onTabChange(tab.key)}
        class="px-5 py-3 text-sm font-semibold border-b-2 transition-all {activeTab === tab.key ? 'border-blue-500 text-blue-700' : 'border-transparent text-gray-500 hover:text-gray-700'}"
      >
        {tab.label}
      </button>
    {/each}
  </div>
</div>
