<script>
  import Icon from './Icon.svelte';
  import { _ } from 'svelte-i18n';

  export let groups = [];
  export let loadingGroups = false;
  export let groupTypes = ['team', 'project', 'department'];
  export let orgRoles = ['owner', 'admin', 'reviewer', 'developer', 'member'];

  export let canCreateGroup = false;
  export let canEditGroup = false;
  export let canDeleteGroup = false;
  export let canManageMembers = false;

  export let onRefreshGroups = () => {};
  export let onAddMember = () => {};

  // --- Create group modal state ---
  let showCreateGroupModal = false;
  let newGroup = { name: '', slug: '', description: '', group_type: 'team' };
  let creating = false;

  // --- Edit group inline state ---
  let editingGroup = null;
  let editGroupForm = {};

  // --- Group members modal state ---
  let showGroupMembersModal = false;
  let selectedGroup = null;
  let groupMembers = [];
  let loadingGroupMembers = false;
  let editingGroupMember = null;
  let editGroupMemberRole = '';

  function getRoleColor(role) {
    switch (role) {
      case 'owner': return 'bg-purple-100 text-purple-700';
      case 'admin': return 'bg-blue-100 text-blue-700';
      case 'reviewer': return 'bg-amber-100 text-amber-700';
      case 'developer': return 'bg-emerald-100 text-emerald-700';
      case 'member': return 'bg-gray-100 text-gray-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }

  function handleCreateGroup() {
    if (!newGroup.name.trim() || !newGroup.slug.trim()) return;
    creating = true;
    onRefreshGroups('create', newGroup)
      .then(() => {
        newGroup = { name: '', slug: '', description: '', group_type: 'team' };
        showCreateGroupModal = false;
      })
      .finally(() => { creating = false; });
  }

  function handleUpdateGroup() {
    if (!editingGroup || !editGroupForm.name?.trim()) return;
    onRefreshGroups('update', { id: editingGroup, ...editGroupForm }).then(() => {
      editingGroup = null;
    });
  }

  function handleDeleteGroup(groupId) {
    if (!confirm('Delete this group?')) return;
    onRefreshGroups('delete', { id: groupId });
  }

  function startEditGroup(group) {
    editingGroup = group.id;
    editGroupForm = { name: group.name, slug: group.slug, description: group.description || '', group_type: group.group_type || 'team' };
  }

  function openGroupMembers(group) {
    selectedGroup = group;
    showGroupMembersModal = true;
    loadingGroupMembers = true;
    onRefreshGroups('listMembers', { groupId: group.id })
      .then(data => { groupMembers = data || []; })
      .finally(() => { loadingGroupMembers = false; });
  }

  function handleUpdateGroupMemberRole(agentId) {
    if (!editGroupMemberRole) return;
    onRefreshGroups('updateMember', { groupId: selectedGroup.id, agentId, role: editGroupMemberRole })
      .then(data => {
        editingGroupMember = null;
        editGroupMemberRole = '';
        if (data) groupMembers = data;
      });
  }

  function handleRemoveGroupMember(agentId) {
    if (!confirm('Remove this member from the group?')) return;
    onRefreshGroups('removeMember', { groupId: selectedGroup.id, agentId })
      .then(data => { if (data) groupMembers = data; });
  }

  function handleAddGroupMember() {
    onAddMember(selectedGroup);
  }
</script>

<div class="bg-white rounded-2xl border border-gray-200 shadow-card">
  <div class="px-6 py-4 border-b border-gray-100 flex items-center justify-between">
    <h2 class="font-semibold text-gray-800 text-sm">{$_('organizations.groups', { values: { count: groups.length } })}</h2>
    {#if canCreateGroup}
      <button
        on:click={() => { showCreateGroupModal = true; newGroup = { name: '', slug: '', description: '', group_type: 'team' }; }}
        class="btn-primary px-4 py-2 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <Icon name="plus" size="w-4 h-4" />
        {$_('organizations.createGroup')}
      </button>
    {/if}
  </div>
  {#if loadingGroups}
    <div class="p-8 text-center text-gray-400 text-sm">{$_('organizations.loading')}</div>
  {:else if groups.length === 0}
    <div class="px-6 py-16 text-center text-gray-400 text-sm font-medium">{$_('organizations.noGroups')}</div>
  {:else}
    <div class="overflow-x-auto">
      <table class="w-full">
        <thead class="bg-gray-50 border-b border-gray-100">
          <tr>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.group')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.type')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.slug')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.description')}</th>
            <th class="px-6 py-3 text-right text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.actions')}</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each groups as group (group.id)}
            <tr class="hover:bg-gray-50 transition-colors">
              <td class="px-6 py-4">
                <button on:click={() => openGroupMembers(group)} class="text-blue-600 hover:text-blue-700 font-semibold text-sm text-left hover:underline">
                  {group.name}
                </button>
              </td>
              <td class="px-6 py-4">
                <span class="px-2.5 py-1 rounded-full text-xs font-medium bg-blue-100 text-blue-700">{group.group_type || 'team'}</span>
              </td>
              <td class="px-6 py-4"><code class="text-xs font-mono bg-gray-100 px-2 py-1 rounded">{group.slug}</code></td>
              <td class="px-6 py-4 text-sm text-gray-600 max-w-xs truncate">{group.description || '-'}</td>
              <td class="px-6 py-4 text-right">
                <div class="flex items-center justify-end gap-1">
                  <button on:click={() => openGroupMembers(group)} class="p-2 rounded-lg text-gray-400 hover:text-blue-500 hover:bg-blue-50 transition-all" title="Manage members">
                    <Icon name="people" size="w-4 h-4" />
                  </button>
                  {#if canEditGroup}
                    <button on:click={() => startEditGroup(group)} class="p-2 rounded-lg text-gray-400 hover:text-blue-500 hover:bg-blue-50 transition-all" title="Edit">
                      <Icon name="edit" size="w-4 h-4" />
                    </button>
                  {/if}
                  {#if canDeleteGroup}
                    <button on:click={() => handleDeleteGroup(group.id)} class="p-2 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all" title="Delete">
                      <Icon name="trash" size="w-4 h-4" />
                    </button>
                  {/if}
                </div>
              </td>
            </tr>
            <!-- Inline edit row -->
            {#if editingGroup === group.id}
              <tr>
                <td colspan="5" class="px-6 py-4 bg-blue-50">
                  <div class="flex gap-3 items-end">
                    <div class="flex-1">
                      <label for="edit-group-name" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-1">{$_('common.name')}</label>
                      <input id="edit-group-name" type="text" bind:value={editGroupForm.name} class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm input-focus outline-none bg-white" />
                    </div>
                    <div class="flex-1">
                      <label for="edit-group-type" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-1">{$_('common.type')}</label>
                      <select id="edit-group-type" bind:value={editGroupForm.group_type} class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm input-focus outline-none bg-white">
                        {#each groupTypes as gt}<option value={gt}>{gt}</option>{/each}
                      </select>
                    </div>
                    <div class="flex-1">
                      <label for="edit-group-desc" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-1">{$_('common.description')}</label>
                      <input id="edit-group-desc" type="text" bind:value={editGroupForm.description} class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm input-focus outline-none bg-white" />
                    </div>
                    <button on:click={handleUpdateGroup} class="btn-primary px-4 py-2 rounded-lg text-sm font-semibold">{$_('common.save')}</button>
                    <button on:click={() => editingGroup = null} class="px-4 py-2 text-gray-500 font-semibold text-sm hover:bg-gray-100 rounded-lg">{$_('common.cancel')}</button>
                  </div>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- Create Group Modal -->
{#if showCreateGroupModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  <div class="bg-white rounded-2xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <h2 class="text-lg font-bold text-gray-800 mb-5">{$_('organizations.createGroup')}</h2>
    <div class="space-y-4">
      <div>
        <label for="group-name" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('common.name')}</label>
        <input id="group-name" type="text" bind:value={newGroup.name} placeholder="Group name" class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900" />
      </div>
      <div>
        <label for="group-slug" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('common.slug')}</label>
        <input id="group-slug" type="text" bind:value={newGroup.slug} placeholder="group-slug" class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900" />
      </div>
      <div>
        <label for="group-type" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('common.type')}</label>
        <select id="group-type" bind:value={newGroup.group_type} class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900">
          {#each groupTypes as gt}<option value={gt}>{gt}</option>{/each}
        </select>
      </div>
      <div>
        <label for="group-desc" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('common.description')}</label>
        <input id="group-desc" type="text" bind:value={newGroup.description} placeholder="Optional description" class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900" />
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button on:click={() => { showCreateGroupModal = false; }} class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50">{$_('common.cancel')}</button>
        <button on:click={handleCreateGroup} disabled={creating || !newGroup.name.trim() || !newGroup.slug.trim()} class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed">{creating ? $_('common.loading') : $_('common.create')}</button>
      </div>
    </div>
  </div>
</div>
{/if}

<!-- Group Members Modal -->
{#if showGroupMembersModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  <div class="bg-white rounded-2xl p-6 w-full max-w-2xl shadow-elevated-lg border border-gray-200 modal-content">
      <div class="flex items-center justify-between mb-5">
      <h2 class="text-lg font-bold text-gray-800">Group Members: {selectedGroup?.name}</h2>
      <div class="flex items-center gap-2">
        {#if canManageMembers}
          <button on:click={handleAddGroupMember} class="btn-primary px-3 py-1.5 rounded-lg text-xs font-semibold flex items-center gap-1.5">
            <Icon name="plus" size="w-3.5 h-3.5" />
            {$_('groups.addMember')}
          </button>
        {/if}
        <button on:click={() => { showGroupMembersModal = false; selectedGroup = null; }} class="p-2 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-all">
          <Icon name="close" size="w-5 h-5" />
        </button>
      </div>
    </div>
    {#if loadingGroupMembers}
      <div class="text-center py-8 text-gray-400">{$_('organizations.loading')}</div>
    {:else if groupMembers.length === 0}
      <div class="text-center py-8 text-gray-400 text-sm font-medium">{$_('groups.noMembersYet')}</div>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead class="bg-gray-50 border-b border-gray-200">
            <tr>
              <th class="px-4 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.user')}</th>
              <th class="px-4 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.role')}</th>
              <th class="px-4 py-3 text-right text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.actions')}</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-100">
            {#each groupMembers as member (member.agent_id || member.username)}
              <tr class="hover:bg-gray-50 transition-colors">
                <td class="px-4 py-3">
                  <div class="flex items-center gap-3">
                    <div class="w-7 h-7 rounded-full bg-blue-600 flex items-center justify-center text-white text-xs font-bold">
                      {(member.username || member.agent_id || '?')[0]?.toUpperCase()}
                    </div>
                    <span class="text-sm font-semibold text-gray-800">{member.username || member.agent_id}</span>
                  </div>
                </td>
                <td class="px-4 py-3">
                  {#if editingGroupMember === (member.username || member.agent_id)}
                    <div class="flex items-center gap-2">
                      <select bind:value={editGroupMemberRole} class="px-2 py-1 border border-gray-200 rounded-lg text-xs input-focus outline-none bg-white">
                        {#each orgRoles as role}<option value={role}>{role}</option>{/each}
                      </select>
                      <button on:click={() => handleUpdateGroupMemberRole(member.username || member.agent_id)} class="text-emerald-600 hover:text-emerald-700 text-xs font-semibold">{$_('common.save')}</button>
                      <button on:click={() => { editingGroupMember = null; editGroupMemberRole = ''; }} class="text-gray-400 hover:text-gray-600 text-xs">{$_('common.cancel')}</button>
                    </div>
                  {:else}
                    <span class="px-2.5 py-1 rounded-full text-xs font-medium {getRoleColor(member.role)}">{member.role}</span>
                  {/if}
                </td>
                <td class="px-4 py-3 text-right">
                  <div class="flex items-center justify-end gap-1">
                    {#if editingGroupMember !== (member.username || member.agent_id) && canManageMembers}
                      <button on:click={() => { editingGroupMember = (member.username || member.agent_id); editGroupMemberRole = member.role; }} class="p-2 rounded-lg text-gray-400 hover:text-blue-500 hover:bg-blue-50 transition-all" title="Edit role">
                        <Icon name="edit" size="w-4 h-4" />
                      </button>
                    {/if}
                    {#if canManageMembers}
                      <button on:click={() => handleRemoveGroupMember(member.username || member.agent_id)} class="p-2 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all" title="Remove from group">
                        <Icon name="trash" size="w-4 h-4" />
                      </button>
                    {/if}
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</div>
{/if}
