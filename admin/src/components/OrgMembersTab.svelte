<script>
  import Icon from './Icon.svelte';
  import { _ } from 'svelte-i18n';

  export let members = [];
  export let orgRoles = ['owner', 'admin', 'reviewer', 'developer', 'member'];

  export let canInviteMember = false;
  export let canManageRoles = false;
  export let canRemoveMember = false;

  export let onInvite = () => {};
  export let onUpdateRole = () => {};
  export let onRemoveMember = () => {};

  let editingMember = null;
  let editMemberRole = '';

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

  function getTypeColor(identityType) {
    switch (identityType) {
      case 'human': return 'bg-blue-100 text-blue-700';
      case 'agent': return 'bg-purple-100 text-purple-700';
      case 'service': return 'bg-amber-100 text-amber-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }

  function handleSaveRole(username) {
    onUpdateRole(username, editMemberRole);
    editingMember = null;
    editMemberRole = '';
  }
</script>

<div class="bg-white rounded-2xl border border-gray-200 shadow-card">
  <div class="px-6 py-4 border-b border-gray-100 flex items-center justify-between">
    <h2 class="font-semibold text-gray-800 text-sm">{$_('organizations.membersCount', { values: { count: members.length } })}</h2>
    {#if canInviteMember}
      <button
        on:click={onInvite}
        class="btn-primary px-4 py-2 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <Icon name="plus" size="w-4 h-4" />
        {$_('organizations.inviteMember')}
      </button>
    {/if}
  </div>
  <div class="overflow-x-auto">
    {#if members.length === 0}
      <div class="px-6 py-16 text-center text-gray-400 text-sm font-medium">{$_('organizations.noMembers')}</div>
    {:else}
      <table class="w-full">
        <thead class="bg-gray-50 border-b border-gray-100">
          <tr>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider w-12">ID</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.user')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.email')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.role')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('groups.title')}</th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.joined')}</th>
            <th class="px-6 py-3 text-right text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('organizations.table.actions')}</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each members as member (member.identity_id)}
            <tr class="hover:bg-gray-50 transition-colors">
              <td class="px-6 py-4">
                <span class="text-xs text-gray-400 font-mono" title={member.identity_id}>{member.identity_id ? member.identity_id.substring(0, 8) + '...' : '-'}</span>
              </td>
              <td class="px-6 py-4">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 rounded-full bg-blue-600 flex items-center justify-center text-white text-xs font-bold">
                    {(member.username || member.name || '?')[0]?.toUpperCase()}
                  </div>
                  <div>
                    <p class="text-sm font-semibold text-gray-800">{member.username || member.name}</p>
                    {#if member.display_name}
                      <p class="text-xs text-gray-400">{member.display_name}</p>
                    {/if}
                    <span class="text-[10px] text-gray-400">{member.identity_type || ''}</span>
                  </div>
                </div>
              </td>
              <td class="px-6 py-4">
                {#if member.email}
                  <span class="text-xs text-gray-600 font-mono">{member.email}</span>
                {:else}
                  <span class="text-gray-400 text-xs">-</span>
                {/if}
              </td>
              <td class="px-6 py-4">
                {#if editingMember === (member.username || member.name)}
                  <div class="flex items-center gap-2">
                    <select bind:value={editMemberRole} class="px-2 py-1 border border-gray-200 rounded-lg text-xs input-focus outline-none bg-white">
                      {#each orgRoles as role}
                        <option value={role}>{role}</option>
                      {/each}
                    </select>
                    <button on:click={() => handleSaveRole(member.username || member.name)} class="text-emerald-600 hover:text-emerald-700 text-xs font-semibold">Save</button>
                    <button on:click={() => { editingMember = null; editMemberRole = ''; }} class="text-gray-400 hover:text-gray-600 text-xs">Cancel</button>
                  </div>
                {:else}
                  <span class="px-2.5 py-1 rounded-full text-xs font-medium {getRoleColor(member.role)}">{member.role}</span>
                {/if}
              </td>
              <td class="px-6 py-4">
                {#if member.groups && member.groups.length > 0}
                  <div class="flex flex-wrap gap-1">
                    {#each member.groups as group}
                      <span class="px-2 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-600" title={group.role}>
                        {group.name}
                      </span>
                    {/each}
                  </div>
                {:else}
                  <span class="text-gray-400 text-xs">-</span>
                {/if}
              </td>
              <td class="px-6 py-4 text-sm text-gray-500">
                {member.joined_at ? new Date(member.joined_at).toLocaleDateString() : '-'}
              </td>
              <td class="px-6 py-4 text-right">
                <div class="flex items-center justify-end gap-1">
                  {#if editingMember !== (member.username || member.name) && canManageRoles}
                    <button
                      on:click={() => { editingMember = (member.username || member.name); editMemberRole = member.role; }}
                      class="p-2 rounded-lg text-gray-400 hover:text-blue-500 hover:bg-blue-50 transition-all" title="Edit role"
                    >
                      <Icon name="edit" size="w-4 h-4" />
                    </button>
                  {/if}
                  {#if canRemoveMember}
                    <button
                      on:click={() => onRemoveMember(member.username || member.name)}
                      class="p-2 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all" title="Remove"
                    >
                      <Icon name="trash" size="w-4 h-4" />
                    </button>
                  {/if}
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>
