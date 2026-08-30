<script>
  import { onMount } from 'svelte';
  import { Link } from 'svelte-routing';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  export let id = '';

  const ACT = ACTIONS.GroupDetail;
  const ACT_GRP = ACTIONS.Groups;

  let group = null;
  let members = [];
  let loading = true;
  let error = '';

  let editing = false;
  let editForm = {};

  let showAddMemberModal = false;
  let addMemberForm = { agent_id: '', role: 'member' };
  let addingMember = false;

  let editingMember = null;
  let editMemberRole = '';

  let permissions = null;

  const groupRoles = ['lead', 'member'];

  const permissionLabels = {
    'skill.read': 'Read Skills',
    'skill.write': 'Write Skills',
    'skill.delete': 'Delete Skills',
    'member.read': 'Read Members',
    'member.invite': 'Invite Members',
    'member.manage': 'Manage Members',
    'group.read': 'Read Group',
    'group.write': 'Update Group',
    'group.delete': 'Delete Group',
    'settings.read': 'Read Settings',
    'settings.write': 'Update Settings',
  };

  onMount(() => {
    loadData();
  });

  async function loadData() {
    loading = true;
    error = '';
    try {
      group = await api.getGroup(id);
      await loadMembers();
      await loadPermissions();
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function loadMembers() {
    try {
      const res = await api.listGroupMembers(id);
      members = Array.isArray(res) ? res : (res.data || []);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleUpdate() {
    if (!editForm.name?.trim()) return;
    try {
      const data = {
        name: editForm.name,
        slug: editForm.slug,
        description: editForm.description,
        group_type: editForm.group_type
      };
      await api.updateGroup(id, data);
      group = await api.getGroup(id);
      editing = false;
      addToast($_('groups.groupUpdated'), 'success');
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleDelete() {
    if (!confirm($_('groups.deleteThisGroup'))) return;
    try {
      await api.deleteGroup(id);
      addToast($_('groups.groupDeletedMsg'), 'success');
      window.history.back();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleAddMember() {
    if (!addMemberForm.agent_id.trim()) return;
    addingMember = true;
    try {
      await api.addGroupMember(id, { agent_id: addMemberForm.agent_id, role: addMemberForm.role });
      addMemberForm = { agent_id: '', role: 'member' };
      showAddMemberModal = false;
      addToast($_('groups.memberAdded'), 'success');
      await loadMembers();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      addingMember = false;
    }
  }

  async function handleUpdateMemberRole(member) {
    if (!editMemberRole) return;
    try {
      await api.updateGroupMember(id, member.agent_id, { role: editMemberRole });
      editingMember = null;
      editMemberRole = '';
      addToast($_('groups.roleUpdated'), 'success');
      await loadMembers();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleRemoveMember(member) {
    if (!confirm($_('groups.removeMemberConfirm', { values: { name: member.agent_id } }))) return;
    try {
      await api.removeGroupMember(id, member.agent_id);
      addToast($_('groups.memberRemoved'), 'success');
      await loadMembers();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function loadPermissions() {
    try {
      permissions = await api.listGroupPermissions(id);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleTogglePermission(roleName, permCode, currentGranted) {
    try {
      await api.updateGroupPermission(id, {
        role_name: roleName,
        permission_code: permCode,
        granted: !currentGranted,
      });
      addToast(!currentGranted ? $_('groups.permissionGranted') : $_('groups.permissionRevoked'), 'success');
      await loadPermissions();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  function getPermLabel(code) {
    return permissionLabels[code] || code;
  }

  function startEdit() {
    editForm = {
      name: group.name,
      slug: group.slug,
      description: group.description || '',
      group_type: group.group_type || 'team'
    };
    editing = true;
  }

  function getTypeColor(type) {
    switch (type) {
      case 'team': return 'bg-blue-100 text-blue-700';
      case 'project': return 'bg-purple-100 text-purple-700';
      case 'department': return 'bg-emerald-100 text-emerald-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }

  function getRoleColor(role) {
    switch (role) {
      case 'lead': return 'bg-amber-100 text-amber-700';
      case 'member': return 'bg-gray-100 text-gray-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }
</script>

<div class="p-8">
  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if group}
    <div class="mb-6">
      <Link to="/groups" class="text-blue-600 hover:text-blue-700 text-sm inline-flex items-center gap-1 font-semibold transition-colors">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/></svg>
        {$_('groups.backToGroups')}
      </Link>
    </div>

    <div class="bg-white rounded-2xl border border-gray-200 shadow-card mb-6">
      <div class="px-6 py-5 border-b border-gray-200">
        <div class="flex items-center justify-between">
          {#if editing}
            <div class="flex gap-3 items-center flex-wrap">
              <label for="group-name-input" class="sr-only">{$_('groups.groupName')}</label>
              <input
                id="group-name-input"
                type="text"
                bind:value={editForm.name}
                placeholder={$_('groups.groupName')}
                class="text-xl font-bold text-gray-800 px-3 py-1.5 border border-gray-200 rounded-xl input-focus outline-none transition-all bg-white"
              />
              <label for="group-slug-input" class="sr-only">{$_('groups.groupSlug')}</label>
              <input
                id="group-slug-input"
                type="text"
                bind:value={editForm.slug}
                placeholder={$_('groups.groupSlug')}
                class="text-sm text-gray-600 px-3 py-1.5 border border-gray-200 rounded-xl input-focus outline-none transition-all bg-white"
              />
              <button
                on:click={handleUpdate}
                class="btn-primary px-3 py-1.5 rounded-lg text-sm font-semibold"
              >
                {$_('common.save')}
              </button>
              <button
                on:click={() => editing = false}
                class="btn-secondary px-3 py-1.5 rounded-lg text-sm font-semibold"
              >
                {$_('common.cancel')}
              </button>
            </div>
          {:else}
            <div class="flex items-center gap-4">
              <div class="w-12 h-12 rounded-xl bg-gradient-to-br from-orange-500 to-red-600 flex items-center justify-center font-bold text-lg shadow-glow">
                {group.name[0]?.toUpperCase() || '?'}
              </div>
              <div>
                <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">{group.name}</h1>
                <div class="flex items-center gap-2 mt-1">
                  <p class="text-gray-400 text-xs font-mono">{group.slug}</p>
                  <span class="px-2 py-0.5 rounded-full text-xs font-medium {getTypeColor(group.group_type)}">
                    {group.group_type}
                  </span>
                </div>
              </div>
            </div>
            <div class="flex gap-2">
              {#if hasPermission(ACT_GRP.edit)}
                <button
                  on:click={startEdit}
                  class="btn-secondary px-4 py-2 rounded-xl text-sm font-semibold"
                >
                  {$_('common.edit')}
                </button>
              {/if}
              {#if hasPermission(ACT_GRP.delete)}
                <button
                  on:click={handleDelete}
                  class="px-4 py-2 rounded-xl text-sm font-semibold text-rose-600 hover:bg-rose-50 transition-colors"
                >
                  {$_('common.delete')}
                </button>
              {/if}
            </div>
          {/if}
        </div>
        <p class="text-gray-400 text-xs mt-1.5 font-mono">ID: {group.id}</p>
        {#if group.organization_id}
          <p class="text-gray-400 text-xs mt-0.5">Organization: {group.organization_id}</p>
        {/if}
      </div>

      {#if group.description}
        <div class="px-6 py-4 border-b border-gray-200">
          <p class="text-gray-600 text-sm">{group.description}</p>
        </div>
      {/if}

      <div class="px-6 py-5 grid grid-cols-3 gap-4">
        <div class="bg-gray-50 rounded-xl p-4 border border-gray-200">
          <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">{$_('groups.members')}</p>
          <p class="text-gray-800 font-extrabold text-2xl">{members.length}</p>
        </div>
        <div class="bg-gray-50 rounded-xl p-4 border border-gray-200">
          <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">{$_('common.type')}</p>
          <p class="text-gray-800 font-semibold text-sm capitalize">{group.group_type}</p>
        </div>
        <div class="bg-gray-50 rounded-xl p-4 border border-gray-200">
          <p class="text-gray-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">{$_('common.created')}</p>
          <p class="text-gray-800 font-semibold text-sm">{new Date(group.created_at).toLocaleDateString()}</p>
        </div>
      </div>
    </div>

    <div class="bg-white rounded-2xl border border-gray-200 shadow-card">
      <div class="px-6 py-4 border-b border-gray-100 flex items-center justify-between">
        <h2 class="font-semibold text-gray-800 text-sm">{$_('groups.members')} ({members.length})</h2>
        {#if hasPermission(ACT.addMember)}
          <button
            on:click={() => { showAddMemberModal = true; addMemberForm = { agent_id: '', role: 'member' }; }}
            class="btn-primary px-4 py-2 rounded-xl font-semibold text-sm flex items-center gap-2"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
            {$_('groups.addMember')}
          </button>
        {/if}
      </div>
      <div class="overflow-x-auto">
        {#if members.length === 0}
          <div class="px-6 py-16 text-center text-gray-400 text-sm font-medium">
            {$_('groups.noMembersYet')}
          </div>
        {:else}
          <table class="w-full">
            <thead class="bg-gray-50 border-b border-gray-100">
              <tr>
                <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('groups.identity')}</th>
                <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('groups.email')}</th>
                <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('groups.role')}</th>
                <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('groups.joined')}</th>
                <th class="px-6 py-3 text-right text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('groups.actions')}</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">
              {#each members as member (member.agent_id)}
                <tr class="hover:bg-gray-50 transition-colors">
                  <td class="px-6 py-4">
                    <div class="flex items-center gap-3">
                      <div class="w-8 h-8 rounded-full bg-blue-600 flex items-center justify-center text-white text-xs font-bold">
                        {(member.username || member.name || '?')[0]?.toUpperCase()}
                      </div>
                      <div>
                        <p class="text-sm font-semibold text-gray-800">{member.username || member.name || 'Unknown'}</p>
                        <p class="text-xs text-gray-400 font-mono">{member.agent_id ? member.agent_id.substring(0, 8) + '...' : ''}</p>
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
                    {#if editingMember === member.agent_id}
                      <div class="flex items-center gap-2">
                        <select
                          id="edit-member-role"
                          bind:value={editMemberRole}
                          class="px-2 py-1 border border-gray-200 rounded-lg text-xs input-focus outline-none bg-white"
                        >
                          {#each groupRoles as role}
                            <option value={role}>{role}</option>
                          {/each}
                        </select>
                        <button
                          on:click={() => handleUpdateMemberRole(member)}
                          class="text-emerald-600 hover:text-emerald-700 text-xs font-semibold"
                        >
                          {$_('common.save')}
                        </button>
                        <button
                          on:click={() => { editingMember = null; editMemberRole = ''; }}
                          class="text-gray-400 hover:text-gray-600 text-xs"
                        >
                          {$_('common.cancel')}
                        </button>
                      </div>
                    {:else}
                      <span class="px-2.5 py-1 rounded-full text-xs font-medium {getRoleColor(member.role)}">
                        {member.role}
                      </span>
                    {/if}
                  </td>
                  <td class="px-6 py-4 text-sm text-gray-500">
                    {member.joined_at ? new Date(member.joined_at).toLocaleDateString() : '-'}
                  </td>
                  <td class="px-6 py-4 text-right">
                    <div class="flex items-center justify-end gap-1">
                      {#if editingMember !== member.agent_id && hasPermission(ACT.manageRoles)}
                        <button
                          on:click={() => { editingMember = member.agent_id; editMemberRole = member.role; }}
                          class="p-2 rounded-lg text-gray-400 hover:text-blue-500 hover:bg-blue-50 transition-all"
                          title={$_('groups.editRole')}
                        >
                          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"/></svg>
                        </button>
                      {/if}
                      {#if hasPermission(ACT.removeMember)}
                        <button
                          on:click={() => handleRemoveMember(member)}
                          class="p-2 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all"
                          title={$_('groups.remove')}
                        >
                          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
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

    <div class="bg-white rounded-2xl border border-gray-200 shadow-card mt-6">
      <div class="px-6 py-4 border-b border-purple-100/60">
        <h2 class="font-semibold text-gray-800 text-sm">{$_('groups.groupPermissions')}</h2>
        <p class="text-gray-400 text-xs mt-1">{$_('groups.togglePermissionsDesc')}</p>
      </div>
      {#if permissions}
        <div class="grid grid-cols-2 gap-0">
          <div class="border-r border-purple-100/60">
            <div class="px-6 py-3 bg-amber-50/50 border-b border-purple-100/60">
              <h3 class="font-semibold text-gray-700 text-xs uppercase tracking-wider">Lead</h3>
            </div>
            <div class="divide-y divide-purple-100/40">
              {#each permissions.lead || [] as perm (perm.permission_code)}
                <div class="px-6 py-3 flex items-center justify-between hover:bg-gray-50/50 transition-colors">
                  <div class="flex-1">
                    <p class="text-sm font-medium text-gray-700">{getPermLabel(perm.permission_code)}</p>
                    <p class="text-xs text-gray-400 font-mono">{perm.permission_code}</p>
                  </div>
                  <button
                    on:click={() => handleTogglePermission('lead', perm.permission_code, perm.granted)}
                    class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {perm.granted ? 'bg-emerald-500' : 'bg-gray-300'}"
                    title={perm.granted ? 'Click to revoke' : 'Click to grant'}
                  >
                    <span class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform {perm.granted ? 'translate-x-6' : 'translate-x-1'} shadow-sm" />
                  </button>
                </div>
              {/each}
            </div>
          </div>
          <div>
            <div class="px-6 py-3 bg-gray-50/50 border-b border-purple-100/60">
              <h3 class="font-semibold text-gray-700 text-xs uppercase tracking-wider">Member</h3>
            </div>
            <div class="divide-y divide-purple-100/40">
              {#each permissions.member || [] as perm (perm.permission_code)}
                <div class="px-6 py-3 flex items-center justify-between hover:bg-gray-50/50 transition-colors">
                  <div class="flex-1">
                    <p class="text-sm font-medium text-gray-700">{getPermLabel(perm.permission_code)}</p>
                    <p class="text-xs text-gray-400 font-mono">{perm.permission_code}</p>
                  </div>
                  <button
                    on:click={() => handleTogglePermission('member', perm.permission_code, perm.granted)}
                    class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {perm.granted ? 'bg-emerald-500' : 'bg-gray-300'}"
                    title={perm.granted ? 'Click to revoke' : 'Click to grant'}
                  >
                    <span class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform {perm.granted ? 'translate-x-6' : 'translate-x-1'} shadow-sm" />
                  </button>
                </div>
              {/each}
            </div>
          </div>
        </div>
      {:else}
        <div class="px-6 py-16 text-center text-gray-400 text-sm font-medium">
          {$_('groups.loadingPermissions')}
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if showAddMemberModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  <div class="bg-white rounded-2xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <h2 class="text-lg font-bold text-gray-800 mb-5">{$_('groups.addMemberToGroup')}</h2>
    <div class="space-y-4">
      <div>
        <label for="add-member-id" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('groups.identityId')}</label>
        <input
          id="add-member-id"
          type="text"
          bind:value={addMemberForm.agent_id}
          placeholder="UUID of the identity"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium font-mono bg-white text-gray-900"
        />
        <p class="text-gray-400 text-xs mt-1">{$_('groups.enterIdentityUuid')}</p>
      </div>
      <div>
        <label for="add-member-role" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Role</label>
        <select
          id="add-member-role"
          bind:value={addMemberForm.role}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        >
          <option value="lead">{$_('groups.roleLead')}</option>
          <option value="member">{$_('groups.roleMember')}</option>
        </select>
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showAddMemberModal = false; addMemberForm = { agent_id: '', role: 'member' }; }}
          class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
        >
          {$_('common.cancel')}
        </button>
        <button
          on:click={handleAddMember}
          disabled={addingMember || !addMemberForm.agent_id.trim()}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {addingMember ? $_('common.loading') : $_('common.add')}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}