<script>
  import { onMount } from 'svelte';
  import { Link } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  let groups = [];
  let organizations = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newGroup = { organization_id: '', name: '', slug: '', description: '', group_type: 'team' };
  let creating = false;

  let showPermissionConfig = false;
  let defaultPermissions = null;
  let permissionToggles = {};

  const permissionLabels = {
    'group:read': 'Read Group',
    'group:update': 'Update Group',
    'group:delete': 'Delete Group',
    'group:member_read': 'Read Members',
    'group:member_add': 'Add Members',
    'group:member_remove': 'Remove Members',
    'group:member_role_assign': 'Assign Roles',
    'skill:read': 'Read Skills',
    'skill:read_content': 'Read Skill Content',
    'skill:update': 'Update Skills',
    'skill:delete': 'Delete Skills',
    'skill:install': 'Install Skills',
    'skill:version_create': 'Create Versions',
    'skill:version_rollback': 'Rollback Versions',
    'skill:submit_review': 'Submit Review',
    'skill:approve_review': 'Approve Review',
    'skill:reject_review': 'Reject Review',
    'skill:change_visibility': 'Change Visibility',
    'skill:associate_group': 'Associate Group',
    'skill:dissociate_group': 'Dissociate Group',
  };

  const groupTypes = ['team', 'project', 'department'];

  onMount(async () => {
    await Promise.all([loadGroups(), loadOrganizations(), loadDefaultPermissions()]);
  });

  async function loadGroups() {
    loading = true;
    error = '';
    try {
      const res = await api.listGroups({ limit: 100 });
      groups = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function loadOrganizations() {
    try {
      const res = await api.listOrganizations({ limit: 100 });
      organizations = res.data || [];
    } catch (e) {
      console.error('Failed to load organizations:', e);
    }
  }

  async function loadDefaultPermissions() {
    try {
      defaultPermissions = await api.listGroupDefaultPermissions();
      resetPermissionToggles();
    } catch (e) {
      console.error('Failed to load default permissions:', e);
    }
  }

  function resetPermissionToggles() {
    permissionToggles = {};
    if (defaultPermissions) {
      for (const [role, perms] of Object.entries(defaultPermissions)) {
        for (const permCode of perms) {
          permissionToggles[`${role}:${permCode}`] = true;
        }
      }
    }
  }

  function getPermLabel(code) {
    return permissionLabels[code] || code;
  }

  function openCreateModal() {
    showCreateModal = true;
    showPermissionConfig = false;
    resetPermissionToggles();
  }

  async function handleCreate() {
    if (!newGroup.organization_id || !newGroup.name.trim() || !newGroup.slug.trim()) return;
    creating = true;
    try {
      const data = { ...newGroup };
      if (showPermissionConfig && permissionToggles && defaultPermissions) {
        const overrides = [];
        for (const [role, perms] of Object.entries(defaultPermissions)) {
          for (const permCode of perms) {
            if (!permissionToggles[`${role}:${permCode}`]) {
              overrides.push({ role_name: role, permission_code: permCode, granted: false });
            }
          }
        }
        if (overrides.length > 0) {
          data.permission_overrides = overrides;
        }
      }
      await api.createGroup(data);
      newGroup = { organization_id: '', name: '', slug: '', description: '', group_type: 'team' };
      showCreateModal = false;
      showPermissionConfig = false;
      addToast('Group created', 'success');
      await loadGroups();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      creating = false;
    }
  }

  async function handleDelete(id) {
    if (!confirm('Delete this group?')) return;
    try {
      await api.deleteGroup(id);
      addToast('Group deleted', 'success');
      await loadGroups();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  function getTypeColor(type) {
    switch (type) {
      case 'team': return 'bg-blue-100 text-blue-700';
      case 'project': return 'bg-purple-100 text-purple-700';
      case 'department': return 'bg-emerald-100 text-emerald-700';
      default: return 'bg-surface-100 text-surface-700';
    }
  }

  function getOrgName(orgId) {
    const org = organizations.find(o => o.id === orgId);
    return org ? org.name : orgId;
  }
</script>

<div class="p-8">
  <div class="page-header">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">Groups</h1>
        <p class="text-surface-500 text-sm mt-1.5 font-medium">Manage teams, projects and departments within organizations</p>
      </div>
      <button
        on:click={openCreateModal}
        class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        New Group
      </button>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if groups.length === 0}
    <div class="bg-sky-50 backdrop-blur-sm rounded-2xl border border-indigo-200/60 shadow-card">
      <EmptyState message="No groups yet">
        <button
          on:click={openCreateModal}
          class="mt-4 btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm"
        >
          Create your first group
        </button>
      </EmptyState>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {#each groups as group (group.id)}
        <div class="relative">
          <Link
            to="/groups/{group.id}"
            class="block bg-sky-50 backdrop-blur-sm rounded-2xl border border-indigo-200/60 p-6 card card-interactive"
          >
            <div class="flex items-start gap-4">
              <div class="w-11 h-11 rounded-xl bg-gradient-to-br from-orange-500 to-red-600 flex items-center justify-center font-bold text-lg shadow-glow flex-shrink-0">
                {group.name[0]?.toUpperCase() || '?'}
              </div>
              <div class="flex-1 min-w-0">
                <h3 class="text-surface-800 font-semibold text-[15px] truncate mb-0.5">{group.name}</h3>
                <p class="text-surface-400 text-xs font-mono truncate">{group.slug}</p>
              </div>
              <span class="px-2 py-1 rounded-full text-xs font-medium {getTypeColor(group.group_type)}">
                {group.group_type}
              </span>
            </div>
            <div class="mt-4 pt-4 border-t border-surface-100">
              <p class="text-surface-500 text-xs mb-2">Organization: {getOrgName(group.organization_id)}</p>
              {#if group.description}
                <p class="text-surface-600 text-sm">{group.description}</p>
              {/if}
            </div>
          </Link>
          <button
            on:click={() => handleDelete(group.id)}
            class="absolute top-3 right-3 p-2 rounded-lg text-surface-400 hover:text-red-500 hover:bg-red-50 transition-all z-10"
            title="Delete"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if showCreateModal}
<div class="fixed inset-0 bg-surface-900/50 backdrop-blur-sm flex items-center justify-center z-50 modal-overlay">
  <div class="bg-sky-50 rounded-2xl p-6 w-full max-w-lg shadow-elevated-lg border border-indigo-200 modal-content max-h-[90vh] overflow-y-auto">
    <h2 class="text-lg font-bold text-surface-800 mb-5">Create Group</h2>
    <div class="space-y-4">
      <div>
        <label for="group-org" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Organization</label>
        <select
          id="group-org"
          bind:value={newGroup.organization_id}
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
        >
          <option value="" disabled selected hidden>Select organization</option>
          {#each organizations as org}
            <option value={org.id}>{org.name}</option>
          {/each}
        </select>
      </div>
      <div>
        <label for="group-name" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Name</label>
        <input
          id="group-name"
          type="text"
          bind:value={newGroup.name}
          placeholder="Group name"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div>
        <label for="group-slug" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Slug</label>
        <input
          id="group-slug"
          type="text"
          bind:value={newGroup.slug}
          placeholder="group-slug"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>
      <div>
        <label for="group-type" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Type</label>
        <select
          id="group-type"
          bind:value={newGroup.group_type}
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
        >
          {#each groupTypes as type}
            <option value={type}>{type}</option>
          {/each}
        </select>
      </div>
      <div>
        <label for="group-desc" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Description (optional)</label>
        <textarea
          id="group-desc"
          bind:value={newGroup.description}
          placeholder="Group description"
          rows="2"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium resize-none"
        ></textarea>
      </div>

      <div class="border-t border-surface-200 pt-4">
        <button
          on:click={() => showPermissionConfig = !showPermissionConfig}
          class="w-full flex items-center justify-between px-4 py-3 rounded-xl border border-surface-200 bg-white hover:bg-surface-50 transition-colors text-sm font-medium text-surface-700"
        >
          <span>Configure Default Permissions</span>
          <svg class="w-4 h-4 transition-transform {showPermissionConfig ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/></svg>
        </button>

        {#if showPermissionConfig && defaultPermissions}
          <div class="mt-4 bg-white rounded-xl border border-surface-200 overflow-hidden">
            <div class="grid grid-cols-2 divide-x divide-surface-200">
              <div>
                <div class="px-4 py-2.5 bg-amber-50 border-b border-surface-200">
                  <h3 class="font-semibold text-surface-700 text-xs uppercase tracking-wider">Lead</h3>
                </div>
                <div class="max-h-[300px] overflow-y-auto divide-y divide-surface-100">
                  {#each defaultPermissions.lead as permCode}
                    <div class="px-4 py-2.5 flex items-center justify-between hover:bg-surface-50 transition-colors">
                      <span class="text-xs text-surface-700 font-medium">{getPermLabel(permCode)}</span>
                      <button
                        on:click={() => permissionToggles[`lead:${permCode}`] = !permissionToggles[`lead:${permCode}`]}
                        class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors {permissionToggles[`lead:${permCode}`] ? 'bg-emerald-500' : 'bg-surface-300'}"
                        title={permissionToggles[`lead:${permCode}`] ? 'Click to deny' : 'Click to grant'}
                      >
                        <span class="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform {permissionToggles[`lead:${permCode}`] ? 'translate-x-[18px]' : 'translate-x-[3px]'} shadow-sm" />
                      </button>
                    </div>
                  {/each}
                </div>
              </div>
              <div>
                <div class="px-4 py-2.5 bg-surface-50 border-b border-surface-200">
                  <h3 class="font-semibold text-surface-700 text-xs uppercase tracking-wider">Member</h3>
                </div>
                <div class="max-h-[300px] overflow-y-auto divide-y divide-surface-100">
                  {#each defaultPermissions.member as permCode}
                    <div class="px-4 py-2.5 flex items-center justify-between hover:bg-surface-50 transition-colors">
                      <span class="text-xs text-surface-700 font-medium">{getPermLabel(permCode)}</span>
                      <button
                        on:click={() => permissionToggles[`member:${permCode}`] = !permissionToggles[`member:${permCode}`]}
                        class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors {permissionToggles[`member:${permCode}`] ? 'bg-emerald-500' : 'bg-surface-300'}"
                        title={permissionToggles[`member:${permCode}`] ? 'Click to deny' : 'Click to grant'}
                      >
                        <span class="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform {permissionToggles[`member:${permCode}`] ? 'translate-x-[18px]' : 'translate-x-[3px]'} shadow-sm" />
                      </button>
                    </div>
                  {/each}
                </div>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showCreateModal = false; showPermissionConfig = false; newGroup = { organization_id: '', name: '', slug: '', description: '', group_type: 'team' }; }}
          class="px-4 py-2.5 text-surface-500 hover:text-surface-800 font-semibold text-sm transition-all rounded-lg hover:bg-surface-50"
        >
          Cancel
        </button>
        <button
          on:click={handleCreate}
          disabled={creating || !newGroup.organization_id || !newGroup.name.trim() || !newGroup.slug.trim()}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {creating ? 'Creating...' : 'Create'}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}
