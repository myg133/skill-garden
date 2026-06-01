<script>
  import { onMount } from 'svelte';
  import { Link, navigate } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  export let id = '';

  let organization = null;
  let sessions = [];
  let orgTools = [];
  let members = [];
  let loading = true;
  let error = '';
  let editing = false;
  let editName = '';
  let activeTab = 'overview';

  let showInviteModal = false;
  let inviteForm = { email: '', role: 'member' };
  let inviting = false;

  let editingMember = null;
  let editMemberRole = '';

  let groups = [];
  let loadingGroups = false;
  let showCreateGroupModal = false;
  let newGroup = { name: '', slug: '', description: '', group_type: 'team' };
  let creating = false;

  let editingGroup = null;
  let editGroupForm = {};

  let showGroupMembersModal = false;
  let selectedGroup = null;
  let groupMembers = [];
  let loadingGroupMembers = false;
  let editingGroupMember = null;
  let editGroupMemberRole = '';
  let showAddMemberModal = false;
  let addMemberForm = { agent_id: '', role: 'member' };
  let addingMember = false;

  const orgRoles = ['owner', 'admin', 'reviewer', 'developer', 'member'];
  const groupTypes = ['team', 'project', 'department'];

  onMount(async () => {
    await loadData();
  });

  async function loadData() {
    loading = true;
    error = '';
    try {
      const [orgRes, sessionsRes, toolsRes] = await Promise.all([
        api.getOrganization(id),
        api.listSessions({ limit: 50 }),
        api.listApprovedTools(id)
      ]);
      organization = orgRes;
      sessions = sessionsRes.data || [];
      orgTools = toolsRes.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function loadMembers() {
    if (!organization) return;
    try {
      const res = organization.slug
        ? await api.listOrgMembers(organization.slug)
        : await api.listOrgMembersById(organization.id);
      members = Array.isArray(res) ? res : (res.members || res.data || []);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleUpdate() {
    if (!editName.trim()) return;
    try {
      await api.updateOrganization(id, { name: editName });
      organization.name = editName;
      editing = false;
      addToast('Organization updated', 'success');
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleEndSession(sessionId) {
    if (!confirm('End this session?')) return;
    try {
      await api.endSession(sessionId);
      addToast('Session ended', 'success');
      await loadData();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleInvite() {
    if (!inviteForm.email.trim() || !inviteForm.role) return;
    inviting = true;
    try {
      if (organization.slug) {
        await api.inviteOrgMember(organization.slug, inviteForm);
      } else {
        await api.inviteOrgMemberById(organization.id, inviteForm);
      }
      inviteForm = { email: '', role: 'member' };
      showInviteModal = false;
      addToast('Member invited successfully', 'success');
      await loadMembers();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      inviting = false;
    }
  }

  async function handleUpdateRole(username) {
    if (!editMemberRole) return;
    try {
      if (organization.slug) {
        await api.updateOrgMember(organization.slug, username, { role: editMemberRole });
      } else {
        await api.updateOrgMemberById(organization.id, username, { role: editMemberRole });
      }
      editingMember = null;
      editMemberRole = '';
      addToast('Role updated', 'success');
      await loadMembers();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleRemoveMember(username) {
    if (!confirm(`Remove ${username} from this organization?`)) return;
    try {
      if (organization.slug) {
        await api.removeOrgMember(organization.slug, username);
      } else {
        await api.removeOrgMemberById(organization.id, username);
      }
      addToast('Member removed', 'success');
      await loadMembers();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  function startEdit() {
    editName = organization.name;
    editing = true;
  }

  function getRoleColor(role) {
    switch (role) {
      case 'owner': return 'bg-purple-100 text-purple-700';
      case 'admin': return 'bg-blue-100 text-blue-700';
      case 'reviewer': return 'bg-amber-100 text-amber-700';
      case 'developer': return 'bg-emerald-100 text-emerald-700';
      case 'member': return 'bg-surface-100 text-surface-700';
      default: return 'bg-surface-100 text-surface-700';
    }
  }

  function getTypeColor(identityType) {
    switch (identityType) {
      case 'human': return 'bg-blue-100 text-blue-700';
      case 'agent': return 'bg-purple-100 text-purple-700';
      case 'service': return 'bg-amber-100 text-amber-700';
      default: return 'bg-surface-100 text-surface-700';
    }
  }

  $: if (activeTab === 'members' && organization) {
    loadMembers();
  }

  $: if (activeTab === 'groups' && organization) {
    loadGroups();
  }

  async function loadGroups() {
    if (!organization?.slug) {
      addToast('Organization slug is required for group operations', 'warning');
      return;
    }
    loadingGroups = true;
    try {
      const res = await api.listOrgGroups(organization.slug);
      groups = res.data || [];
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      loadingGroups = false;
    }
  }

  async function handleCreateGroup() {
    if (!newGroup.name.trim() || !newGroup.slug.trim()) return;
    creating = true;
    try {
      await api.createOrgGroup(organization.slug, newGroup);
      newGroup = { name: '', slug: '', description: '', group_type: 'team' };
      showCreateGroupModal = false;
      addToast('Group created', 'success');
      await loadGroups();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      creating = false;
    }
  }

  async function handleUpdateGroup() {
    if (!editingGroup || !editGroupForm.name?.trim()) return;
    try {
      await api.updateOrgGroup(organization.slug, editingGroup, editGroupForm);
      editingGroup = null;
      addToast('Group updated', 'success');
      await loadGroups();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleDeleteGroup(groupId) {
    if (!confirm('Delete this group?')) return;
    try {
      await api.deleteOrgGroup(organization.slug, groupId);
      addToast('Group deleted', 'success');
      await loadGroups();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function openGroupMembers(group) {
    selectedGroup = group;
    showGroupMembersModal = true;
    loadingGroupMembers = true;
    try {
      const res = await api.listOrgGroupMembers(organization.slug, group.id);
      groupMembers = res.data || [];
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      loadingGroupMembers = false;
    }
  }

  async function handleUpdateGroupMemberRole(username) {
    if (!editGroupMemberRole) return;
    try {
      await api.updateOrgGroupMember(organization.slug, selectedGroup.id, username, { role: editGroupMemberRole });
      editingGroupMember = null;
      editGroupMemberRole = '';
      addToast('Member role updated', 'success');
      const res = await api.listOrgGroupMembers(organization.slug, selectedGroup.id);
      groupMembers = res.data || [];
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleRemoveGroupMember(username) {
    if (!confirm(`Remove ${username} from this group?`)) return;
    try {
      await api.removeOrgGroupMember(organization.slug, selectedGroup.id, username);
      addToast('Member removed from group', 'success');
      const res = await api.listOrgGroupMembers(organization.slug, selectedGroup.id);
      groupMembers = res.data || [];
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleAddGroupMember() {
    if (!addMemberForm.agent_id.trim()) return;
    addingMember = true;
    try {
      await api.addGroupMember(selectedGroup.id, { agent_id: addMemberForm.agent_id, role: addMemberForm.role });
      addMemberForm = { agent_id: '', role: 'member' };
      showAddMemberModal = false;
      addToast('Member added to group', 'success');
      const res = await api.listOrgGroupMembers(organization.slug, selectedGroup.id);
      groupMembers = res.data || [];
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      addingMember = false;
    }
  }

  function startEditGroup(group) {
    editingGroup = group.id;
    editGroupForm = { name: group.name, slug: group.slug, description: group.description || '', group_type: group.group_type || 'team' };
  }

  $: memberCount = members.length;
  $: activeSessionCount = sessions.filter(s => s.status === 'active').length;
</script>

<div class="p-8">
  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if organization}
    <div class="mb-6">
      <Link to="/organizations" class="text-brand-600 hover:text-brand-700 text-sm inline-flex items-center gap-1 font-semibold transition-colors">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/></svg>
        Back to Organizations
      </Link>
    </div>

    <div class="gradient-card-brand-light rounded-2xl border border-brand-200/60 shadow-card mb-6">
      <div class="px-6 py-5 border-b border-brand-200/60">
        <div class="flex items-center justify-between">
          {#if editing}
            <div class="flex gap-3 items-center">
              <input
                type="text"
                bind:value={editName}
                class="text-xl font-bold text-surface-800 px-3 py-1.5 border border-surface-200 rounded-xl input-focus outline-none transition-all bg-white"
              />
              <button
                on:click={handleUpdate}
                class="btn-primary px-3 py-1.5 rounded-lg text-sm font-semibold"
              >
                Save
              </button>
              <button
                on:click={() => editing = false}
                class="btn-secondary px-3 py-1.5 rounded-lg text-sm font-semibold"
              >
                Cancel
              </button>
            </div>
          {:else}
            <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">{organization.name}</h1>
            <button
              on:click={startEdit}
              class="btn-secondary px-4 py-2 rounded-xl text-sm font-semibold"
            >
              Edit
            </button>
          {/if}
        </div>
        <p class="text-surface-400 text-xs mt-1.5 font-mono">ID: {organization.id}</p>
        {#if organization.slug}
          <p class="text-surface-400 text-xs mt-0.5">Slug: {organization.slug}</p>
        {/if}
        {#if organization.tenant_id}
          <p class="text-surface-400 text-xs mt-0.5">Tenant: {organization.tenant_id}</p>
        {/if}
      </div>
      <div class="px-6 py-5 grid grid-cols-4 gap-4">
        <div class="bg-slate-50/80 rounded-xl p-4 border border-brand-200/40 card">
          <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Created</p>
          <p class="text-surface-800 font-semibold text-sm">{new Date(organization.created_at).toLocaleString()}</p>
        </div>
        <div class="bg-slate-50/80 rounded-xl p-4 border border-brand-200/40 card">
          <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Members</p>
          <p class="text-surface-800 font-extrabold text-2xl">{memberCount}</p>
        </div>
        <div class="bg-slate-50/80 rounded-xl p-4 border border-brand-200/40 card">
          <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Active Sessions</p>
          <p class="text-surface-800 font-extrabold text-2xl">{activeSessionCount}</p>
        </div>
        <div class="bg-slate-50/80 rounded-xl p-4 border border-brand-200/40 card">
          <p class="text-surface-400 text-[11px] uppercase tracking-wider font-semibold mb-1.5">Registered Tools</p>
          <p class="text-surface-800 font-extrabold text-2xl">{orgTools.length}</p>
        </div>
      </div>

      <div class="px-6 border-t border-brand-200/60 flex gap-0">
        <button
          on:click={() => activeTab = 'overview'}
          class="px-5 py-3 text-sm font-semibold border-b-2 transition-all {activeTab === 'overview' ? 'border-brand-500 text-brand-700' : 'border-transparent text-surface-500 hover:text-surface-700'}"
        >
          Overview
        </button>
        <button
          on:click={() => activeTab = 'members'}
          class="px-5 py-3 text-sm font-semibold border-b-2 transition-all {activeTab === 'members' ? 'border-brand-500 text-brand-700' : 'border-transparent text-surface-500 hover:text-surface-700'}"
        >
          Members
        </button>
        <button
          on:click={() => activeTab = 'sessions'}
          class="px-5 py-3 text-sm font-semibold border-b-2 transition-all {activeTab === 'sessions' ? 'border-brand-500 text-brand-700' : 'border-transparent text-surface-500 hover:text-surface-700'}"
        >
          Sessions
        </button>
        <button
          on:click={() => activeTab = 'tools'}
          class="px-5 py-3 text-sm font-semibold border-b-2 transition-all {activeTab === 'tools' ? 'border-brand-500 text-brand-700' : 'border-transparent text-surface-500 hover:text-surface-700'}"
        >
          Tools
        </button>
        <button
          on:click={() => activeTab = 'groups'}
          class="px-5 py-3 text-sm font-semibold border-b-2 transition-all {activeTab === 'groups' ? 'border-brand-500 text-brand-700' : 'border-transparent text-surface-500 hover:text-surface-700'}"
        >
          Groups
        </button>
      </div>
    </div>

    {#if activeTab === 'members'}
      <div class="gradient-card-sky rounded-2xl border border-indigo-200/60 shadow-card">
        <div class="px-6 py-4 border-b border-indigo-100/60 flex items-center justify-between">
          <h2 class="font-semibold text-surface-800 text-sm">Organization Members ({memberCount})</h2>
          <button
            on:click={() => { showInviteModal = true; inviteForm = { email: '', role: 'member' }; }}
            class="btn-primary px-4 py-2 rounded-xl font-semibold text-sm flex items-center gap-2"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
            Invite Member
          </button>
        </div>
        <div class="overflow-x-auto">
          {#if members.length === 0}
            <div class="px-6 py-16 text-center text-surface-400 text-sm font-medium">No members yet</div>
          {:else}
            <table class="w-full">
              <thead class="bg-surface-50 border-b border-surface-100">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-semibold text-surface-500 uppercase tracking-wider w-12">ID</th>
                  <th class="px-6 py-3 text-left text-xs font-semibold text-surface-500 uppercase tracking-wider">User</th>
                  <th class="px-6 py-3 text-left text-xs font-semibold text-surface-500 uppercase tracking-wider">Email</th>
                  <th class="px-6 py-3 text-left text-xs font-semibold text-surface-500 uppercase tracking-wider">Role</th>
                  <th class="px-6 py-3 text-left text-xs font-semibold text-surface-500 uppercase tracking-wider">Joined</th>
                  <th class="px-6 py-3 text-right text-xs font-semibold text-surface-500 uppercase tracking-wider">Actions</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-surface-100">
                {#each members as member (member.identity_id)}
                  <tr class="hover:bg-surface-50 transition-colors">
                    <td class="px-6 py-4">
                      <span class="text-xs text-surface-400 font-mono" title={member.identity_id}>{member.identity_id ? member.identity_id.substring(0, 8) + '...' : '-'}</span>
                    </td>
                    <td class="px-6 py-4">
                      <div class="flex items-center gap-3">
                        <div class="w-8 h-8 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-white text-xs font-bold">
                          {(member.username || member.name || '?')[0]?.toUpperCase()}
                        </div>
                        <div>
                          <p class="text-sm font-semibold text-surface-800">{member.username || member.name}</p>
                          {#if member.display_name}
                            <p class="text-xs text-surface-400">{member.display_name}</p>
                          {/if}
                          <span class="text-[10px] text-surface-400">{member.identity_type || ''}</span>
                        </div>
                      </div>
                    </td>
                    <td class="px-6 py-4">
                      {#if member.email}
                        <span class="text-xs text-surface-600 font-mono">{member.email}</span>
                      {:else}
                        <span class="text-surface-400 text-xs">-</span>
                      {/if}
                    </td>
                    <td class="px-6 py-4">
                      {#if editingMember === (member.username || member.name)}
                        <div class="flex items-center gap-2">
                          <select
                            bind:value={editMemberRole}
                            class="px-2 py-1 border border-surface-200 rounded-lg text-xs input-focus outline-none bg-white"
                          >
                            {#each orgRoles as role}
                              <option value={role}>{role}</option>
                            {/each}
                          </select>
                          <button
                            on:click={() => handleUpdateRole(member.username || member.name)}
                            class="text-emerald-600 hover:text-emerald-700 text-xs font-semibold"
                          >
                            Save
                          </button>
                          <button
                            on:click={() => { editingMember = null; editMemberRole = ''; }}
                            class="text-surface-400 hover:text-surface-600 text-xs"
                          >
                            Cancel
                          </button>
                        </div>
                      {:else}
                        <span class="px-2.5 py-1 rounded-full text-xs font-medium {getRoleColor(member.role)}">
                          {member.role}
                        </span>
                      {/if}
                    </td>
                    <td class="px-6 py-4 text-sm text-surface-500">
                      {member.joined_at ? new Date(member.joined_at).toLocaleDateString() : '-'}
                    </td>
                    <td class="px-6 py-4 text-right">
                      <div class="flex items-center justify-end gap-1">
                        {#if editingMember !== (member.username || member.name)}
                          <button
                            on:click={() => { editingMember = (member.username || member.name); editMemberRole = member.role; }}
                            class="p-2 rounded-lg text-surface-400 hover:text-brand-500 hover:bg-brand-50 transition-all"
                            title="Edit role"
                          >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"/></svg>
                          </button>
                        {/if}
                        <button
                          on:click={() => handleRemoveMember(member.username || member.name)}
                          class="p-2 rounded-lg text-surface-400 hover:text-red-500 hover:bg-red-50 transition-all"
                          title="Remove"
                        >
                          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                        </button>
                      </div>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>
      </div>
    {/if}

    {#if activeTab === 'overview' || activeTab === 'sessions' || activeTab === 'tools'}
      <div class="grid grid-cols-1 {activeTab === 'overview' ? '' : 'lg:grid-cols-1'} gap-5">
        {#if activeTab === 'sessions' || activeTab === 'overview'}
          <div class="gradient-card-sky rounded-2xl border border-sky-100/60 shadow-card">
            <div class="px-6 py-4 border-b border-sky-100/60">
              <h2 class="font-semibold text-surface-800 text-sm">Sessions</h2>
            </div>
            <div class="divide-y divide-surface-50 max-h-80 overflow-y-auto">
              {#if sessions.length === 0}
                <div class="px-6 py-12 text-center text-surface-400 text-sm font-medium">No sessions</div>
              {:else}
                {#each sessions as session (session.id)}
                  <div class="px-6 py-4 flex items-center justify-between table-row">
                    <div>
                      <p class="text-surface-800 text-sm font-mono font-semibold">{session.id}</p>
                      <p class="text-surface-400 text-xs mt-0.5">Agent: {session.agent_id}</p>
                    </div>
                    <div class="flex items-center gap-3">
                      <span class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-semibold rounded-full {session.status === 'active' ? 'bg-emerald-50 text-emerald-700 ring-1 ring-emerald-600/20' : 'bg-surface-100 text-surface-600 ring-1 ring-surface-600/10'}">
                        {#if session.status === 'active'}
                          <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 pulse-dot"></span>
                        {/if}
                        {session.status}
                      </span>
                      {#if session.status === 'active'}
                        <button
                          on:click={() => handleEndSession(session.id)}
                          class="text-rose-500 hover:text-rose-600 text-xs font-semibold transition-colors"
                        >
                          End
                        </button>
                      {/if}
                    </div>
                  </div>
                {/each}
              {/if}
            </div>
          </div>
        {/if}

        {#if activeTab === 'tools' || activeTab === 'overview'}
          <div class="gradient-card-rose rounded-2xl border border-rose-100/60 shadow-card">
            <div class="px-6 py-4 border-b border-rose-100/60">
              <h2 class="font-semibold text-surface-800 text-sm">Registered Tools</h2>
            </div>
            <div class="p-4 max-h-80 overflow-y-auto">
              {#if orgTools.length === 0}
                <div class="py-12 text-center text-surface-400 text-sm font-medium">No tools registered</div>
              {:else}
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  {#each orgTools as tool (tool.id)}
                    <div class="bg-sky-50 rounded-xl border border-indigo-200 p-4 hover:shadow-md transition-shadow">
                      <p class="text-surface-800 text-sm font-semibold truncate">{tool.name}</p>
                      <p class="text-surface-400 text-xs mt-1 font-mono truncate">{tool.tool_id}</p>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    {#if activeTab === 'groups'}
      <div class="gradient-card-sky rounded-2xl border border-indigo-200/60 shadow-card">
        <div class="px-6 py-4 border-b border-indigo-100/60 flex items-center justify-between">
          <h2 class="font-semibold text-surface-800 text-sm">Groups ({groups.length})</h2>
          <button
            on:click={() => { showCreateGroupModal = true; newGroup = { name: '', slug: '', description: '', group_type: 'team' }; }}
            class="btn-primary px-4 py-2 rounded-xl font-semibold text-sm flex items-center gap-2"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
            New Group
          </button>
        </div>
        {#if loadingGroups}
          <div class="p-8 text-center text-surface-400 text-sm">Loading...</div>
        {:else if groups.length === 0}
          <div class="px-6 py-16 text-center text-surface-400 text-sm font-medium">No groups yet</div>
        {:else}
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead class="bg-surface-50 border-b border-surface-100">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-semibold text-surface-500 uppercase tracking-wider">Group</th>
                  <th class="px-6 py-3 text-left text-xs font-semibold text-surface-500 uppercase tracking-wider">Type</th>
                  <th class="px-6 py-3 text-left text-xs font-semibold text-surface-500 uppercase tracking-wider">Slug</th>
                  <th class="px-6 py-3 text-left text-xs font-semibold text-surface-500 uppercase tracking-wider">Description</th>
                  <th class="px-6 py-3 text-right text-xs font-semibold text-surface-500 uppercase tracking-wider">Actions</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-surface-100">
                {#each groups as group (group.id)}
                  <tr class="hover:bg-surface-50 transition-colors">
                    <td class="px-6 py-4">
                      <button
                        on:click={() => openGroupMembers(group)}
                        class="text-brand-600 hover:text-brand-700 font-semibold text-sm text-left hover:underline"
                      >
                        {group.name}
                      </button>
                    </td>
                    <td class="px-6 py-4">
                      <span class="px-2.5 py-1 rounded-full text-xs font-medium bg-blue-100 text-blue-700">
                        {group.group_type || 'team'}
                      </span>
                    </td>
                    <td class="px-6 py-4">
                      <code class="text-xs font-mono bg-surface-100 px-2 py-1 rounded">{group.slug}</code>
                    </td>
                    <td class="px-6 py-4 text-sm text-surface-600 max-w-xs truncate">{group.description || '-'}</td>
                    <td class="px-6 py-4 text-right">
                      <div class="flex items-center justify-end gap-1">
                        <button
                          on:click={() => openGroupMembers(group)}
                          class="p-2 rounded-lg text-surface-400 hover:text-brand-500 hover:bg-brand-50 transition-all"
                          title="Manage members"
                        >
                          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z"/></svg>
                        </button>
                        <button
                          on:click={() => startEditGroup(group)}
                          class="p-2 rounded-lg text-surface-400 hover:text-brand-500 hover:bg-brand-50 transition-all"
                          title="Edit"
                        >
                          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"/></svg>
                        </button>
                        <button
                          on:click={() => handleDeleteGroup(group.id)}
                          class="p-2 rounded-lg text-surface-400 hover:text-red-500 hover:bg-red-50 transition-all"
                          title="Delete"
                        >
                          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                        </button>
                      </div>
                    </td>
                  </tr>
                  {#if editingGroup === group.id}
                    <tr>
                      <td colspan="5" class="px-6 py-4 bg-indigo-50/50">
                        <div class="flex gap-3 items-end">
                          <div class="flex-1">
                            <label for="edit-group-name" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-1">Name</label>
                            <input id="edit-group-name" type="text" bind:value={editGroupForm.name} class="w-full px-3 py-2 border border-surface-200 rounded-lg text-sm input-focus outline-none bg-white" />
                          </div>
                          <div class="flex-1">
                            <label for="edit-group-type" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-1">Type</label>
                            <select id="edit-group-type" bind:value={editGroupForm.group_type} class="w-full px-3 py-2 border border-surface-200 rounded-lg text-sm input-focus outline-none bg-white">
                              {#each groupTypes as gt}<option value={gt}>{gt}</option>{/each}
                            </select>
                          </div>
                          <div class="flex-1">
                            <label for="edit-group-desc" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-1">Description</label>
                            <input id="edit-group-desc" type="text" bind:value={editGroupForm.description} class="w-full px-3 py-2 border border-surface-200 rounded-lg text-sm input-focus outline-none bg-white" />
                          </div>
                          <button on:click={handleUpdateGroup} class="btn-primary px-4 py-2 rounded-lg text-sm font-semibold">Save</button>
                          <button on:click={() => editingGroup = null} class="px-4 py-2 text-surface-500 font-semibold text-sm hover:bg-surface-100 rounded-lg">Cancel</button>
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
    {/if}
  {/if}
</div>

{#if showInviteModal}
<div class="fixed inset-0 bg-surface-900/50 backdrop-blur-sm flex items-center justify-center z-50 modal-overlay">
  <div class="bg-sky-50 rounded-2xl p-6 w-full max-w-md shadow-elevated-lg border border-indigo-200 modal-content">
    <h2 class="text-lg font-bold text-surface-800 mb-5">Invite Member</h2>
    <div class="space-y-4">
      <div>
        <label for="invite-email" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Email</label>
        <input
          id="invite-email"
          type="email"
          bind:value={inviteForm.email}
          placeholder="Enter email address"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div>
        <label for="invite-role" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Role</label>
        <select
          id="invite-role"
          bind:value={inviteForm.role}
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
        >
          {#each orgRoles as role}
            <option value={role}>{role}</option>
          {/each}
        </select>
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showInviteModal = false; inviteForm = { email: '', role: 'member' }; }}
          class="px-4 py-2.5 text-surface-500 hover:text-surface-800 font-semibold text-sm transition-all rounded-lg hover:bg-surface-50"
        >
          Cancel
        </button>
        <button
          on:click={handleInvite}
          disabled={inviting || !inviteForm.email.trim()}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {inviting ? 'Inviting...' : 'Invite'}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}

{#if showAddMemberModal}
<div class="fixed inset-0 bg-surface-900/50 backdrop-blur-sm flex items-center justify-center z-50 modal-overlay">
  <div class="bg-sky-50 rounded-2xl p-6 w-full max-w-md shadow-elevated-lg border border-indigo-200 modal-content">
    <h2 class="text-lg font-bold text-surface-800 mb-5">Add Member to {selectedGroup?.name}</h2>
    <div class="space-y-4">
      <div>
        <label for="add-member-id" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Identity ID (UUID)</label>
        <input
          id="add-member-id"
          type="text"
          bind:value={addMemberForm.agent_id}
          placeholder="e.g. 550e8400-e29b-41d4-a716-446655440000"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div>
        <label for="add-member-role" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Role</label>
        <select
          id="add-member-role"
          bind:value={addMemberForm.role}
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
        >
          {#each orgRoles as role}
            <option value={role}>{role}</option>
          {/each}
        </select>
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showAddMemberModal = false; }}
          class="px-4 py-2.5 text-surface-500 hover:text-surface-800 font-semibold text-sm transition-all rounded-lg hover:bg-surface-50"
        >
          Cancel
        </button>
        <button
          on:click={handleAddGroupMember}
          disabled={addingMember || !addMemberForm.agent_id.trim()}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {addingMember ? 'Adding...' : 'Add'}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}

{#if showCreateGroupModal}
<div class="fixed inset-0 bg-surface-900/50 backdrop-blur-sm flex items-center justify-center z-50 modal-overlay">
  <div class="bg-sky-50 rounded-2xl p-6 w-full max-w-md shadow-elevated-lg border border-indigo-200 modal-content">
    <h2 class="text-lg font-bold text-surface-800 mb-5">Create Group</h2>
    <div class="space-y-4">
      <div>
        <label for="create-group-name" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Name</label>
        <input
          id="create-group-name"
          type="text"
          bind:value={newGroup.name}
          placeholder="Group name"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div>
        <label for="create-group-slug" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Slug</label>
        <input
          id="create-group-slug"
          type="text"
          bind:value={newGroup.slug}
          placeholder="group-slug"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div>
        <label for="create-group-type" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Type</label>
        <select
          id="create-group-type"
          bind:value={newGroup.group_type}
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
        >
          {#each groupTypes as gt}
            <option value={gt}>{gt}</option>
          {/each}
        </select>
      </div>
      <div>
        <label for="create-group-desc" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Description</label>
        <input
          id="create-group-desc"
          type="text"
          bind:value={newGroup.description}
          placeholder="Optional description"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showCreateGroupModal = false; }}
          class="px-4 py-2.5 text-surface-500 hover:text-surface-800 font-semibold text-sm transition-all rounded-lg hover:bg-surface-50"
        >
          Cancel
        </button>
        <button
          on:click={handleCreateGroup}
          disabled={creating || !newGroup.name.trim() || !newGroup.slug.trim()}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {creating ? 'Creating...' : 'Create'}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}

{#if showGroupMembersModal}
<div class="fixed inset-0 bg-surface-900/50 backdrop-blur-sm flex items-center justify-center z-50 modal-overlay">
  <div class="bg-sky-50 rounded-2xl p-6 w-full max-w-2xl shadow-elevated-lg border border-indigo-200 modal-content">
    <div class="flex items-center justify-between mb-5">
      <h2 class="text-lg font-bold text-surface-800">Group Members: {selectedGroup?.name}</h2>
      <div class="flex items-center gap-2">
        <button
          on:click={() => { showAddMemberModal = true; addMemberForm = { agent_id: '', role: 'member' }; }}
          class="btn-primary px-3 py-1.5 rounded-lg text-xs font-semibold flex items-center gap-1.5"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
          Add Member
        </button>
        <button
          on:click={() => { showGroupMembersModal = false; selectedGroup = null; }}
          class="p-2 rounded-lg text-surface-400 hover:text-surface-600 hover:bg-surface-100 transition-all"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
        </button>
      </div>
    </div>
    {#if loadingGroupMembers}
      <div class="text-center py-8 text-surface-400">Loading...</div>
    {:else if groupMembers.length === 0}
      <div class="text-center py-8 text-surface-400 text-sm font-medium">No members in this group</div>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead class="bg-surface-50 border-b border-surface-200">
            <tr>
              <th class="px-4 py-3 text-left text-xs font-semibold text-surface-500 uppercase tracking-wider">User</th>
              <th class="px-4 py-3 text-left text-xs font-semibold text-surface-500 uppercase tracking-wider">Role</th>
              <th class="px-4 py-3 text-right text-xs font-semibold text-surface-500 uppercase tracking-wider">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-surface-100">
            {#each groupMembers as member (member.agent_id || member.username)}
              <tr class="hover:bg-surface-50 transition-colors">
                <td class="px-4 py-3">
                  <div class="flex items-center gap-3">
                    <div class="w-7 h-7 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-white text-xs font-bold">
                      {(member.username || member.agent_id || '?')[0]?.toUpperCase()}
                    </div>
                    <span class="text-sm font-semibold text-surface-800">{member.username || member.agent_id}</span>
                  </div>
                </td>
                <td class="px-4 py-3">
                  {#if editingGroupMember === (member.username || member.agent_id)}
                    <div class="flex items-center gap-2">
                      <select
                        bind:value={editGroupMemberRole}
                        class="px-2 py-1 border border-surface-200 rounded-lg text-xs input-focus outline-none bg-white"
                      >
                        {#each orgRoles as role}
                          <option value={role}>{role}</option>
                        {/each}
                      </select>
                      <button
                        on:click={() => handleUpdateGroupMemberRole(member.username || member.agent_id)}
                        class="text-emerald-600 hover:text-emerald-700 text-xs font-semibold"
                      >
                        Save
                      </button>
                      <button
                        on:click={() => { editingGroupMember = null; editGroupMemberRole = ''; }}
                        class="text-surface-400 hover:text-surface-600 text-xs"
                      >
                        Cancel
                      </button>
                    </div>
                  {:else}
                    <span class="px-2.5 py-1 rounded-full text-xs font-medium {getRoleColor(member.role)}">
                      {member.role}
                    </span>
                  {/if}
                </td>
                <td class="px-4 py-3 text-right">
                  <div class="flex items-center justify-end gap-1">
                    {#if editingGroupMember !== (member.username || member.agent_id)}
                      <button
                        on:click={() => { editingGroupMember = (member.username || member.agent_id); editGroupMemberRole = member.role; }}
                        class="p-2 rounded-lg text-surface-400 hover:text-brand-500 hover:bg-brand-50 transition-all"
                        title="Edit role"
                      >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"/></svg>
                      </button>
                    {/if}
                    <button
                      on:click={() => handleRemoveGroupMember(member.username || member.agent_id)}
                      class="p-2 rounded-lg text-surface-400 hover:text-red-500 hover:bg-red-50 transition-all"
                      title="Remove from group"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7a4 4 0 11-8 0 4 4 0 018 0zM9 9a1 1 0 000-2 1 1 0 000 2zm0 7c-.828 0-1.5-.895-1.5-2s.672-2 1.5-2 1.5.895 1.5 2-.895 1.5-1.5 1.5z"/></svg>
                    </button>
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