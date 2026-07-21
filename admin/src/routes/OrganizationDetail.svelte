<script>
  import { onMount } from 'svelte';
  import { Link } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import Icon from '../components/Icon.svelte';
  import OrgOverviewHeader from '../components/OrgOverviewHeader.svelte';
  import OrgMembersTab from '../components/OrgMembersTab.svelte';
  import OrgGroupsTab from '../components/OrgGroupsTab.svelte';
  import Badge from '../components/Badge.svelte';

  export let id = '';

  const ACT_ORG = ACTIONS.OrganizationDetail;
  const ACT_GRP = ACTIONS.Groups;
  const ACT_GDT = ACTIONS.GroupDetail;

  // --- Core data ---
  let organization = null;
  let sessions = [];
  let orgTools = [];
  let members = [];
  let groups = [];
  let loading = true;
  let error = '';
  let loadingGroups = false;

  // --- UI state ---
  let editing = false;
  let editName = '';
  let activeTab = 'members';

  // --- Invite modal ---
  let showInviteModal = false;
  let inviteForm = { email: '', role: 'member' };
  let inviting = false;

  // --- Add group member modal ---
  let showAddMemberModal = false;
  let selectedGroupForAdd = null;
  let addMemberForm = { agent_id: '', role: 'member' };
  let addingMember = false;

  const orgRoles = ['owner', 'admin', 'reviewer', 'developer', 'member'];
  const groupTypes = ['team', 'project', 'department'];

  // --- Computed ---
  $: memberCount = members.length;
  $: activeSessionCount = sessions.filter(s => s.status === 'active').length;

  // --- Reactive tab loading ---
  $: if (activeTab === 'members' && organization) { loadMembers(); }
  $: if (activeTab === 'groups' && organization) { loadGroups(); }

  // --- Lifecycle ---
  onMount(() => loadData());

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

  // --- Members ---
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

  function startEdit() {
    editName = organization.name;
    editing = true;
  }

  function formatDuration(start, end) {
    if (!start) return 'N/A';
    const ms = new Date(end || Date.now()) - new Date(start);
    const mins = Math.floor(ms / 60000);
    if (mins < 60) return `${mins}m`;
    const hours = Math.floor(mins / 60);
    return `${hours}h ${mins % 60}m`;
  }


  // --- Invite ---
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

  async function handleUpdateRole(username, role) {
    if (!role) return;
    try {
      if (organization.slug) {
        await api.updateOrgMember(organization.slug, username, { role });
      } else {
        await api.updateOrgMemberById(organization.id, username, { role });
      }
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

  // Computed org identifier (slug or fallback to id) for group API calls
  $: orgRef = organization ? (organization.slug || organization.id) : '';

  // --- Groups (delegate to OrgGroupsTab via callback) ---
  async function loadGroups() {
    loadingGroups = true;
    try {
      const res = await api.listOrgGroups(orgRef);
      groups = res.data || [];
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      loadingGroups = false;
    }
  }

  async function handleGroupsAction(action, payload) {
    try {
      switch (action) {
        case 'create':
          await api.createOrgGroup(orgRef, { name: payload.name, slug: payload.slug, description: payload.description, group_type: payload.group_type });
          addToast('Group created', 'success');
          await loadGroups();
          break;
        case 'update':
          await api.updateOrgGroup(orgRef, payload.id, { name: payload.name, slug: payload.slug, description: payload.description, group_type: payload.group_type });
          addToast('Group updated', 'success');
          await loadGroups();
          break;
        case 'delete':
          await api.deleteOrgGroup(orgRef, payload.id);
          addToast('Group deleted', 'success');
          await loadGroups();
          break;
        case 'listMembers': {
          const res = await api.listOrgGroupMembers(orgRef, payload.groupId);
          return res.data || [];
        }
        case 'updateMember':
          await api.updateOrgGroupMember(orgRef, payload.groupId, payload.agentId, { role: payload.role });
          addToast('Member role updated', 'success');
          return (await api.listOrgGroupMembers(orgRef, payload.groupId)).data || [];
        case 'removeMember':
          await api.removeOrgGroupMember(orgRef, payload.groupId, payload.agentId);
          addToast('Member removed from group', 'success');
          return (await api.listOrgGroupMembers(orgRef, payload.groupId)).data || [];
      }
    } catch (e) {
      addToast(e.message, 'error');
      throw e;
    }
  }

  async function handleAddGroupMember() {
    if (!addMemberForm.agent_id.trim()) return;
    addingMember = true;
    try {
      await api.addGroupMember(selectedGroupForAdd.id, { agent_id: addMemberForm.agent_id, role: addMemberForm.role });
      addMemberForm = { agent_id: '', role: 'member' };
      showAddMemberModal = false;
      addToast('Member added to group', 'success');
      await loadGroups();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      addingMember = false;
    }
  }
</script>

<div class="p-8">
  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if organization}
    <!-- Back link -->
    <div class="mb-6">
      <Link to="/organizations" class="text-blue-600 hover:text-blue-700 text-sm inline-flex items-center gap-1 font-semibold transition-colors">
        <Icon name="chevron-left" size="w-4 h-4" />
        Back to Organizations
      </Link>
    </div>

    <!-- Header + Stats + Tabs -->
    <OrgOverviewHeader
      {organization}
      {editing}
      {editName}
      {memberCount}
      {activeSessionCount}
      toolCount={orgTools.length}
      {activeTab}
      canEdit={hasPermission(ACT_ORG.editSettings)}
      onStartEdit={startEdit}
      onUpdate={handleUpdate}
      onCancelEdit={() => editing = false}
      onTabChange={key => activeTab = key}
    />

    <!-- Members Tab -->
    {#if activeTab === 'members'}
      <OrgMembersTab
        {members}
        {orgRoles}
        canInviteMember={hasPermission(ACT_ORG.inviteMember)}
        canManageRoles={hasPermission(ACT_ORG.manageRoles)}
        canRemoveMember={hasPermission(ACT_ORG.removeMember)}
        onInvite={() => { showInviteModal = true; inviteForm = { email: '', role: 'member' }; }}
        onUpdateRole={handleUpdateRole}
        onRemoveMember={handleRemoveMember}
      />
    {/if}

    <!-- Sessions / Tools -->
    {#if activeTab === 'sessions' || activeTab === 'tools'}
      <div class="grid grid-cols-1 gap-5">
        {#if activeTab === 'sessions'}
          <div class="bg-white rounded-xl border border-gray-200 shadow-card">
            <div class="px-6 py-4 border-b border-gray-100">
              <h2 class="font-semibold text-gray-900 text-sm">Sessions</h2>
            </div>
            <div class="max-h-96 overflow-y-auto">
              {#if sessions.length === 0}
                <div class="px-6 py-12 text-center text-gray-400 text-sm font-medium">No sessions</div>
              {:else}
                <table class="w-full">
                  <thead>
                    <tr class="border-b border-gray-100 bg-gray-50">
                      <th class="px-6 py-3 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Identity</th>
                      <th class="px-6 py-3 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Status</th>
                      <th class="px-6 py-3 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Created</th>
                      <th class="px-6 py-3 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Last Active</th>
                      <th class="px-6 py-3 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Ended</th>
                      <th class="px-6 py-3 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Duration</th>
                      <th class="px-6 py-3 text-right text-xs font-semibold text-gray-400 uppercase tracking-wider">Actions</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-gray-50">
                    {#each sessions as session (session.id)}
                      <tr class="table-row">
                        <td class="px-6 py-4">
                          <p class="text-gray-900 text-sm font-semibold">{session.identity_display_name || session.identity_name || 'Unknown'}</p>
                          <p class="text-gray-400 text-xs font-mono mt-0.5 truncate max-w-[180px]">{session.id}</p>
                        </td>
                        <td class="px-6 py-4">
                          <Badge status={session.status} />
                        </td>
                        <td class="px-6 py-4 text-gray-500 text-sm whitespace-nowrap">{session.created_at ? new Date(session.created_at).toLocaleString() : 'N/A'}</td>
                        <td class="px-6 py-4 text-gray-500 text-sm whitespace-nowrap">{session.last_active_at ? new Date(session.last_active_at).toLocaleString() : 'N/A'}</td>
                        <td class="px-6 py-4 text-gray-500 text-sm whitespace-nowrap">{session.ended_at ? new Date(session.ended_at).toLocaleString() : '—'}</td>
                        <td class="px-6 py-4 text-gray-500 text-sm font-medium whitespace-nowrap">{formatDuration(session.created_at, session.ended_at)}</td>
                        <td class="px-6 py-4 text-right">
                          {#if session.status === 'active'}
                            <button on:click={() => handleEndSession(session.id)} class="text-red-500 hover:text-red-600 text-xs font-semibold transition-colors">End</button>
                          {:else}
                            <span class="text-gray-300 text-xs">—</span>
                          {/if}
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              {/if}
            </div>
          </div>
        {/if}

        {#if activeTab === 'tools'}
          <div class="bg-white rounded-xl border border-gray-200 shadow-card">
            <div class="px-6 py-4 border-b border-gray-100">
              <h2 class="font-semibold text-gray-900 text-sm">Registered Tools</h2>
            </div>
            <div class="p-4 max-h-80 overflow-y-auto">
              {#if orgTools.length === 0}
                <div class="py-12 text-center text-gray-400 text-sm font-medium">No tools registered</div>
              {:else}
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  {#each orgTools as tool (tool.id)}
                    <div class="bg-white rounded-lg border border-gray-200 p-4 hover:shadow-md transition-shadow">
                      <p class="text-gray-900 text-sm font-semibold truncate">{tool.name}</p>
                      <p class="text-gray-400 text-xs mt-1 font-mono truncate">{tool.tool_id}</p>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Groups Tab -->
    {#if activeTab === 'groups'}
      <OrgGroupsTab
        {groups}
        {organization}
        {loadingGroups}
        {groupTypes}
        {orgRoles}
        canCreateGroup={hasPermission(ACT_GRP.create)}
        canEditGroup={hasPermission(ACT_GRP.edit)}
        canDeleteGroup={hasPermission(ACT_GRP.delete)}
        canManageMembers={hasPermission(ACT_GDT.addMember)}
        onRefreshGroups={handleGroupsAction}
        onAddMember={group => { selectedGroupForAdd = group; showAddMemberModal = true; addMemberForm = { agent_id: '', role: 'member' }; }}
      />
    {/if}
  {/if}

  <!-- Invite Member Modal -->
  {#if showInviteModal}
  <div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
    <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
      <h2 class="text-lg font-bold text-gray-900 mb-5">Invite Member</h2>
      <div class="space-y-4">
        <div>
          <label for="invite-email" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Email</label>
          <input id="invite-email" type="email" bind:value={inviteForm.email} placeholder="Enter email address" class="w-full px-4 py-3 border border-gray-200 rounded-lg text-sm input-focus outline-none font-medium bg-white" />
        </div>
        <div>
          <label for="invite-role" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Role</label>
          <select id="invite-role" bind:value={inviteForm.role} class="w-full px-4 py-3 border border-gray-200 rounded-lg text-sm input-focus outline-none font-medium bg-white">
            {#each orgRoles as role}<option value={role}>{role}</option>{/each}
          </select>
        </div>
        <div class="flex gap-3 justify-end pt-1">
          <button on:click={() => { showInviteModal = false; inviteForm = { email: '', role: 'member' }; }} class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50">Cancel</button>
          <button on:click={handleInvite} disabled={inviting || !inviteForm.email.trim()} class="btn-primary px-5 py-2.5 rounded-lg font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed">{inviting ? 'Inviting...' : 'Invite'}</button>
        </div>
      </div>
    </div>
  </div>
  {/if}

  <!-- Add Group Member Modal -->
  {#if showAddMemberModal}
  <div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
    <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
      <h2 class="text-lg font-bold text-gray-900 mb-5">Add Member to {selectedGroupForAdd?.name}</h2>
      <div class="space-y-4">
        <div>
          <label for="add-member-id" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Identity ID (UUID)</label>
          <input id="add-member-id" type="text" bind:value={addMemberForm.agent_id} placeholder="e.g. 550e8400-e29b-41d4-a716-446655440000" class="w-full px-4 py-3 border border-gray-200 rounded-lg text-sm input-focus outline-none font-medium bg-white" />
        </div>
        <div>
          <label for="add-member-role" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Role</label>
          <select id="add-member-role" bind:value={addMemberForm.role} class="w-full px-4 py-3 border border-gray-200 rounded-lg text-sm input-focus outline-none font-medium bg-white">
            {#each orgRoles as role}<option value={role}>{role}</option>{/each}
          </select>
        </div>
        <div class="flex gap-3 justify-end pt-1">
          <button on:click={() => { showAddMemberModal = false; }} class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50">Cancel</button>
          <button on:click={handleAddGroupMember} disabled={addingMember || !addMemberForm.agent_id.trim()} class="btn-primary px-5 py-2.5 rounded-lg font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed">{addingMember ? 'Adding...' : 'Add'}</button>
        </div>
      </div>
    </div>
  </div>
  {/if}
</div>
