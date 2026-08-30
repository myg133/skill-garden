<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission, permissionStore } from '../stores/permission.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import { _ } from 'svelte-i18n';

  let selectedOrgId = null;
  let members = [];
  let loading = true;
  let error = '';
  let showAddModal = false;
  let searchQuery = '';
  let searchResults = [];
  let searching = false;
  let newMemberEmail = '';
  let newMemberRole = 'member';
  let adding = false;

  // 获取用户所属的组织列表
  $: orgRoles = $permissionStore.orgRoles || [];
  $: orgAdminOrgs = orgRoles.filter(r => r.role === 'org_admin' || r.role === 'owner');

  onMount(async () => {
    // 如果只有一个组织，直接加载
    if (orgAdminOrgs.length === 1) {
      selectedOrgId = orgAdminOrgs[0].org_id;
      await loadMembers(selectedOrgId);
    } else if (orgAdminOrgs.length > 1) {
      // 如果有多个组织，使用当前选中的组织
      const savedOrg = localStorage.getItem('selected_org');
      if (savedOrg) {
        try {
          const org = JSON.parse(savedOrg);
          if (org && org.id && org.id !== '__personal__') {
            selectedOrgId = org.id;
          }
        } catch {}
      }
      if (!selectedOrgId) {
        selectedOrgId = orgAdminOrgs[0]?.org_id;
      }
      if (selectedOrgId) {
        await loadMembers(selectedOrgId);
      } else {
        loading = false;
      }
    } else {
      loading = false;
    }
  });

  async function loadMembers(orgId) {
    loading = true;
    error = '';
    try {
      const res = await api.listOrgMembers(orgId);
      members = res.data || [];
    } catch (e) {
      error = e.message || 'Failed to load members';
    } finally {
      loading = false;
    }
  }

  async function handleSelectOrg(orgId) {
    selectedOrgId = orgId;
    await loadMembers(orgId);
  }

  function roleColor(role) {
    const c = {
      owner: 'bg-amber-100 text-amber-700',
      admin: 'bg-blue-100 text-blue-700',
      reviewer: 'bg-purple-100 text-purple-700',
      developer: 'bg-emerald-100 text-emerald-700',
      member: 'bg-gray-100 text-gray-600'
    };
    return c[role] || 'bg-gray-100 text-gray-600';
  }

  async function handleAddMember() {
    if (!newMemberEmail.trim()) return;
    adding = true;
    try {
      await api.addOrgMember(selectedOrgId, {
        identity_email: newMemberEmail,
        role: newMemberRole
      });
      addToast('Member added successfully', 'success');
      showAddModal = false;
      newMemberEmail = '';
      newMemberRole = 'member';
      await loadMembers(selectedOrgId);
    } catch (e) {
      addToast(e.message || 'Failed to add member', 'error');
    } finally {
      adding = false;
    }
  }

  async function handleRemoveMember(memberId) {
    if (!confirm('Remove this member?')) return;
    try {
      await api.removeOrgMember(selectedOrgId, memberId);
      addToast('Member removed', 'success');
      await loadMembers(selectedOrgId);
    } catch (e) {
      addToast(e.message || 'Failed to remove member', 'error');
    }
  }

  async function handleUpdateRole(memberId, newRole) {
    try {
      await api.updateOrgMember(selectedOrgId, memberId, { role: newRole });
      addToast('Role updated', 'success');
      await loadMembers(selectedOrgId);
    } catch (e) {
      addToast(e.message || 'Failed to update role', 'error');
    }
  }
</script>

<div class="p-8">
  <div class="page-header mb-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">{$_('orgMembers.title') || 'Organization Members'}</h1>
        <p class="text-gray-500 text-sm mt-1.5">{$_('orgMembers.description') || 'Manage your organization members'}</p>
      </div>
      {#if selectedOrgId && hasPermission('org:member:manage')}
        <button
          on:click={() => showAddModal = true}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
          </svg>
          {$_('orgMembers.addMember') || 'Add Member'}
        </button>
      {/if}
    </div>
  </div>

  <!-- Organization Selector (if multiple orgs) -->
  {#if orgAdminOrgs.length > 1}
    <div class="mb-6 flex items-center gap-3">
      <label class="text-sm font-medium text-gray-600">{$_('orgMembers.selectOrg') || 'Organization'}:</label>
      <select
        value={selectedOrgId}
        on:change={(e) => handleSelectOrg(e.target.value)}
        class="px-4 py-2.5 bg-white border border-gray-200 rounded-xl text-sm text-gray-700 focus:outline-none focus:ring-2 focus:ring-brand-500/30"
      >
        {#each orgAdminOrgs as org}
          <option value={org.org_id}>{org.org_name}</option>
        {/each}
      </select>
    </div>
  {/if}

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">
      {error}
    </div>
  {:else if members.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message={$_('orgMembers.noMembers') || 'No members found'}>
        {#if hasPermission('org:member:manage')}
          <button
            on:click={() => showAddModal = true}
            class="mt-4 btn-primary px-5 py-2.5 rounded-lg font-semibold text-sm"
          >
            {$_('orgMembers.addFirstMember') || 'Add the first member'}
          </button>
        {/if}
      </EmptyState>
    </div>
  {:else}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card overflow-hidden">
      <table class="w-full">
        <thead class="bg-gray-50 border-b border-gray-200">
          <tr>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">
              {$_('orgMembers.member') || 'Member'}
            </th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">
              {$_('orgMembers.email') || 'Email'}
            </th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">
              {$_('orgMembers.role') || 'Role'}
            </th>
            <th class="px-6 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">
              {$_('orgMembers.joinedAt') || 'Joined'}
            </th>
            {#if hasPermission('org:member:manage')}
              <th class="px-6 py-3 text-right text-xs font-semibold text-gray-500 uppercase tracking-wider">
                {$_('common.actions') || 'Actions'}
              </th>
            {/if}
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each members as member (member.id)}
            <tr class="hover:bg-gray-50 transition-colors">
              <td class="px-6 py-4 whitespace-nowrap">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 rounded-full bg-blue-600 flex items-center justify-center text-white text-xs font-bold">
                    {(member.identity?.name || member.identity?.email || '?')[0]?.toUpperCase()}
                  </div>
                  <span class="text-sm font-medium text-gray-800">
                    {member.identity?.name || '-'}
                  </span>
                </div>
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                <span class="text-sm text-gray-500">{member.identity?.email || '-'}</span>
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                {#if hasPermission('org:member:manage') && member.role !== 'owner'}
                  <select
                    value={member.role}
                    on:change={(e) => handleUpdateRole(member.id, e.target.value)}
                    class="px-2 py-1 rounded text-xs font-medium {roleColor(member.role)} cursor-pointer focus:outline-none focus:ring-2 focus:ring-brand-500/30"
                  >
                    <option value="owner">owner</option>
                    <option value="admin">admin</option>
                    <option value="reviewer">reviewer</option>
                    <option value="developer">developer</option>
                    <option value="member">member</option>
                  </select>
                {:else}
                  <span class="px-2 py-1 rounded text-xs font-medium {roleColor(member.role)}">
                    {member.role}
                  </span>
                {/if}
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                <span class="text-sm text-gray-400">
                  {member.created_at ? new Date(member.created_at).toLocaleDateString() : '-'}
                </span>
              </td>
              {#if hasPermission('org:member:manage')}
                <td class="px-6 py-4 whitespace-nowrap text-right">
                  {#if member.role !== 'owner'}
                    <button
                      on:click={() => handleRemoveMember(member.id)}
                      class="p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all"
                      title={$_('orgMembers.remove') || 'Remove'}
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                      </svg>
                    </button>
                  {/if}
                </td>
              {/if}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- Add Member Modal -->
{#if showAddModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay" on:click={() => showAddModal = false} on:keydown={(e) => e.key === 'Escape' && (showAddModal = false)} role="dialog" aria-modal="true">
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content" on:click|stopPropagation role="document">
    <h2 class="text-lg font-bold text-gray-900 mb-5">{$_('orgMembers.addMember') || 'Add Member'}</h2>
    <div class="space-y-4">
      <div>
        <label for="member-email" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">
          {$_('orgMembers.email') || 'Email'}
        </label>
        <input
          id="member-email"
          type="email"
          bind:value={newMemberEmail}
          placeholder="user@example.com"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>
      <div>
        <label for="member-role" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">
          {$_('orgMembers.role') || 'Role'}
        </label>
        <select
          id="member-role"
          bind:value={newMemberRole}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        >
          <option value="member">member</option>
          <option value="developer">developer</option>
          <option value="reviewer">reviewer</option>
          <option value="admin">admin</option>
        </select>
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showAddModal = false; newMemberEmail = ''; newMemberRole = 'member'; }}
          class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
        >
          {$_('common.cancel') || 'Cancel'}
        </button>
        <button
          on:click={handleAddMember}
          disabled={adding || !newMemberEmail.trim()}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {adding ? ($_('common.loading') || 'Loading...') : ($_('common.add') || 'Add')}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}
