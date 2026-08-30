<script>
  import { onMount } from 'svelte';
  import { Link, navigate } from 'svelte-routing';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission, isSuperAdmin } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  const ACT = ACTIONS.Tenants;
  let tenants = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newTenant = { name: '', slug: '', admin_email: '' };
  let creating = false;

  // 企业模式检测（通过检查 URL 参数或 API）
  let isEnterpriseMode = false;
  // 搜索用户时的临时变量
  let userSearchQuery = '';
  let searchResults = [];
  let searching = false;

  // Detail view state
  let selectedTenant = null;
  let tenantOrgs = [];
  let tenantAdmins = [];
  let loadingDetail = false;

  // ===== 自助申请模式状态 =====
  let isSaasApprovalMode = false;
  let showRequestModal = false;
  let showRequestsListModal = false;
  let newRequest = { tenant_name: '', message: '' };
  let submittingRequest = false;
  let tenantRequests = [];
  let loadingRequests = false;
  let selectedRequest = null;
  let reviewNote = '';
  let reviewing = false;

  onMount(async () => {
    await loadTenants();
    await checkApprovalMode();
  });

  async function checkApprovalMode() {
    // SaaS 模式且需要审批时启用申请功能
    // 这里简化检测，实际应该从服务端获取配置
    try {
      const status = await api.getAdminStatus();
      // 如果是 SaaS 模式且需要审批，显示申请按钮
      // 后端会验证实际的配置
      isSaasApprovalMode = true;
    } catch (e) {
      console.log('Could not check approval mode:', e);
      isSaasApprovalMode = false;
    }
  }

  async function loadTenants() {
    loading = true;
    error = '';
    try {
      const res = await api.listTenants({ limit: 100 });
      tenants = res.data || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function loadTenantDetail(tenant) {
    selectedTenant = tenant;
    loadingDetail = true;
    try {
      // Load organizations for this tenant
      const orgsRes = await api.listOrganizations({ tenant_id: tenant.id, limit: 100 });
      tenantOrgs = orgsRes.data || [];
      
      // Load tenant role assignments (admins)
      const rolesRes = await api.listTenantRoleAssignments({ tenant_id: tenant.id });
      tenantAdmins = rolesRes.data || [];
    } catch (e) {
      console.error('Failed to load tenant detail:', e);
      tenantOrgs = [];
      tenantAdmins = [];
    } finally {
      loadingDetail = false;
    }
  }

  function closeDetail() {
    selectedTenant = null;
    tenantOrgs = [];
    tenantAdmins = [];
  }

  async function handleCreate() {
    if (!newTenant.name.trim() || !newTenant.slug.trim()) return;
    // 企业模式需要 admin_email
    if (isEnterpriseMode && !newTenant.admin_email.trim()) return;
    creating = true;
    try {
      const payload = {
        name: newTenant.name,
        slug: newTenant.slug,
        ...(isEnterpriseMode && newTenant.admin_email && { admin_email: newTenant.admin_email })
      };
      await api.createTenant(payload);
      newTenant = { name: '', slug: '', admin_email: '' };
      showCreateModal = false;
      addToast($_('tenants.tenantCreated'), 'success');
      await loadTenants();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      creating = false;
    }
  }

  // 搜索用户（通过邮箱）
  async function searchUsers(query) {
    if (!query.trim()) {
      searchResults = [];
      return;
    }
    searching = true;
    try {
      // 使用 identity search API
      const results = await api.searchIdentities(query, 5);
      searchResults = results || [];
    } catch (e) {
      searchResults = [];
    } finally {
      searching = false;
    }
  }

  function selectUser(user) {
    newTenant.admin_email = user.email || user.username || '';
    userSearchQuery = '';
    searchResults = [];
  }

  function handleUserSearchInput(e) {
    userSearchQuery = e.target.value;
    newTenant.admin_email = userSearchQuery;
    // 防抖搜索
    clearTimeout(window._userSearchTimer);
    window._userSearchTimer = setTimeout(() => {
      searchUsers(userSearchQuery);
    }, 300);
  }

  async function handleDelete(id) {
    if (!confirm('Delete this tenant?')) return;
    try {
      await api.deleteTenant(id);
      addToast('Tenant deleted', 'success');
      await loadTenants();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  // ===== 租户申请相关函数 =====
  async function handleSubmitRequest() {
    if (!newRequest.tenant_name.trim()) return;
    submittingRequest = true;
    try {
      await api.createTenantRequest({
        tenant_name: newRequest.tenant_name,
        message: newRequest.message || null
      });
      newRequest = { tenant_name: '', message: '' };
      showRequestModal = false;
      addToast($_('tenants.requestSubmitted'), 'success');
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      submittingRequest = false;
    }
  }

  async function loadTenantRequests() {
    loadingRequests = true;
    try {
      const res = await api.listTenantRequests({ limit: 100 });
      tenantRequests = res.data || [];
    } catch (e) {
      addToast(e.message, 'error');
      tenantRequests = [];
    } finally {
      loadingRequests = false;
    }
  }

  function openRequestsList() {
    showRequestsListModal = true;
    loadTenantRequests();
  }

  function closeRequestsList() {
    showRequestsListModal = false;
    selectedRequest = null;
    reviewNote = '';
  }

  function selectRequestForReview(request) {
    selectedRequest = request;
    reviewNote = '';
  }

  async function handleReview(action) {
    if (!selectedRequest) return;
    reviewing = true;
    try {
      const res = await api.reviewTenantRequest(selectedRequest.id, {
        action,
        note: reviewNote || null
      });
      addToast(
        action === 'approve' 
          ? $_('tenants.requestApproved') 
          : $_('tenants.requestRejected'),
        'success'
      );
      selectedRequest = null;
      reviewNote = '';
      await loadTenantRequests();
      await loadTenants(); // Refresh tenant list in case approved
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      reviewing = false;
    }
  }
</script>

<div class="p-8">
  <div class="page-header">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">{$_('tenants.title')}</h1>
        <p class="text-gray-500 text-sm mt-1.5 font-medium">{$_('tenants.description')}</p>
      </div>
      <div class="flex items-center gap-3">
        {#if isSaasApprovalMode && hasPermission(ACT.create)}
        <button
          on:click={() => showRequestModal = true}
          class="btn-secondary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2 border border-indigo-200 text-indigo-600 hover:bg-indigo-50"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/></svg>
          {$_('tenants.requestCreateTenant')}
        </button>
        {/if}
        {#if isSuperAdmin() && isSaasApprovalMode}
        <button
          on:click={openRequestsList}
          class="btn-secondary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2 border border-amber-200 text-amber-600 hover:bg-amber-50"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"/></svg>
          {$_('tenants.tenantRequests')}
        </button>
        {/if}
        {#if hasPermission(ACT.create)}
        <button
          on:click={() => showCreateModal = true}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
          {$_('tenants.newTenant')}
        </button>
        {/if}
      </div>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if tenants.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message={$_('tenants.noTenants')}>
        {#if hasPermission(ACT.create)}
        <button
          on:click={() => showCreateModal = true}
          class="mt-4 btn-primary px-5 py-2.5 rounded-lg font-semibold text-sm"
        >
          {$_('tenants.createFirst')}
        </button>
        {/if}
      </EmptyState>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {#each tenants as tenant (tenant.id)}
        <div class="group bg-white rounded-xl border border-gray-200 p-5 card card-interactive">
          <div class="flex items-start gap-4">
            <div class="w-10 h-10 rounded-lg bg-blue-600 flex items-center justify-center font-bold text-white text-sm flex-shrink-0 group-hover:scale-105 transition-transform duration-300">
              {tenant.name[0]?.toUpperCase() || '?'}
            </div>
            <div class="flex-1 min-w-0">
              <h3 class="text-gray-900 font-semibold text-[15px] truncate mb-0.5">{tenant.name}</h3>
              <p class="text-gray-400 text-xs font-mono truncate">{tenant.slug}</p>
            </div>
            <span class="px-2 py-1 rounded text-xs font-medium {tenant.status === 'active' ? 'bg-emerald-50 text-emerald-600' : 'bg-amber-50 text-amber-600'}">
              {tenant.status}
            </span>
          </div>
          <div class="mt-4 pt-4 border-t border-gray-100 flex items-center justify-between">
            <p class="text-gray-400 text-xs">
              Created {new Date(tenant.created_at).toLocaleDateString()}
            </p>
            <div class="flex items-center gap-1">
              <button
                on:click={() => loadTenantDetail(tenant)}
                class="px-3 py-1.5 rounded-lg text-xs font-medium text-blue-600 hover:bg-blue-50 transition-all"
                title="View details"
              >
                {$_('common.view')}
              </button>
              {#if hasPermission(ACT.delete)}
              <button
                on:click={() => handleDelete(tenant.id)}
                class="p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all"
                title="Delete"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
              </button>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Tenant Detail Modal -->
{#if selectedTenant}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay" on:click={closeDetail} on:keydown={(e) => e.key === 'Escape' && closeDetail()} role="dialog" aria-modal="true">
  <div class="bg-white rounded-2xl p-6 w-full max-w-2xl shadow-elevated-lg border border-gray-200 modal-content max-h-[90vh] overflow-y-auto" on:click|stopPropagation role="document">
    <div class="flex items-center justify-between mb-6">
      <div class="flex items-center gap-3">
        <div class="w-12 h-12 rounded-xl bg-blue-600 flex items-center justify-center font-bold text-white text-lg">
          {selectedTenant.name[0]?.toUpperCase() || '?'}
        </div>
        <div>
          <h2 class="text-xl font-bold text-gray-800">{selectedTenant.name}</h2>
          <p class="text-gray-400 text-sm">{selectedTenant.slug}</p>
        </div>
      </div>
      <button on:click={closeDetail} class="p-2 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-all">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
      </button>
    </div>

    {#if loadingDetail}
      <LoadingSpinner />
    {:else}
      <!-- Related Organizations -->
      <div class="mb-6">
        <h3 class="text-sm font-semibold text-gray-700 uppercase tracking-wider mb-3">{$_('tenants.organizations')}</h3>
        {#if tenantOrgs.length === 0}
          <div class="bg-gray-50 rounded-xl p-4 text-center text-gray-400 text-sm">
            {$_('organizations.noOrganizations')}
          </div>
        {:else}
          <div class="space-y-2">
            {#each tenantOrgs as org (org.id)}
              <Link to="/organizations/{org.id}" class="block bg-gray-50 rounded-xl p-4 hover:bg-gray-100 transition-colors">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-3">
                    <div class="w-8 h-8 rounded-lg bg-emerald-500 flex items-center justify-center text-white text-xs font-bold">
                      {org.name[0]?.toUpperCase() || '?'}
                    </div>
                    <div>
                      <p class="text-sm font-medium text-gray-800">{org.name}</p>
                      <p class="text-xs text-gray-400">{org.slug || ''}</p>
                    </div>
                  </div>
                  <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/></svg>
                </div>
              </Link>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Tenant Admins -->
      <div>
        <h3 class="text-sm font-semibold text-gray-700 uppercase tracking-wider mb-3">{$_('tenants.tenantAdmins')}</h3>
        {#if tenantAdmins.length === 0}
          <div class="bg-gray-50 rounded-xl p-4 text-center text-gray-400 text-sm">
            {$_('tenants.noTenantAdmins')}
          </div>
        {:else}
          <div class="space-y-2">
            {#each tenantAdmins as assignment (assignment.id)}
              <div class="bg-gray-50 rounded-xl p-4 flex items-center gap-3">
                <div class="w-8 h-8 rounded-full bg-purple-600 flex items-center justify-center text-white text-xs font-bold">
                  {(assignment.identity?.name || assignment.identity?.email || '?')[0]?.toUpperCase()}
                </div>
                <div class="flex-1">
                  <p class="text-sm font-medium text-gray-800">{assignment.identity?.name || assignment.identity?.email || 'Unknown'}</p>
                  <p class="text-xs text-gray-400">{assignment.identity?.email || ''}</p>
                </div>
                <span class="px-2 py-1 rounded text-xs font-medium bg-amber-50 text-amber-600">
                  {assignment.role_name}
                </span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
{/if}

<!-- Create Tenant Modal -->
{#if showCreateModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay" on:click={() => showCreateModal = false} on:keydown={(e) => e.key === 'Escape' && (showCreateModal = false)} role="dialog" aria-modal="true">
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content" on:click|stopPropagation role="document">
    <h2 class="text-lg font-bold text-gray-900 mb-5">{$_('tenants.createTenant')}</h2>
    <div class="space-y-4">
      <div>
        <label for="tenant-name" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('tenants.name')}</label>
        <input
          id="tenant-name"
          type="text"
          bind:value={newTenant.name}
          placeholder={$_('tenants.companyName')}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>
      <div>
        <label for="tenant-slug" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('tenants.slug')}</label>
        <input
          id="tenant-slug"
          type="text"
          bind:value={newTenant.slug}
          placeholder={$_('tenants.companySlug')}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>
      <!-- 企业模式：管理员邮箱 -->
      <div>
        <label for="admin-email" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">
          {$_('tenants.adminEmail') || 'Administrator Email'}
          <span class="text-rose-500">*</span>
        </label>
        <input
          id="admin-email"
          type="email"
          value={newTenant.admin_email}
          on:input={handleUserSearchInput}
          placeholder="admin@example.com"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
        <!-- 搜索结果下拉 -->
        {#if searchResults.length > 0}
          <div class="mt-1 bg-white border border-gray-200 rounded-xl shadow-lg overflow-hidden">
            {#each searchResults as user (user.id)}
              <button
                type="button"
                class="w-full text-left px-4 py-2.5 hover:bg-blue-50 transition-colors border-b border-gray-100 last:border-b-0"
                on:click={() => selectUser(user)}
              >
                <p class="text-sm font-medium text-gray-800">{user.name || user.username || 'Unknown'}</p>
                <p class="text-xs text-gray-400">{user.email || user.username || ''}</p>
              </button>
            {/each}
          </div>
        {/if}
        {#if searching}
          <p class="mt-1 text-xs text-gray-400">{$_('common.loading') || 'Searching...'}</p>
        {/if}
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showCreateModal = false; newTenant = { name: '', slug: '', admin_email: '' }; userSearchQuery = ''; searchResults = []; }}
          class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
        >
          {$_('common.cancel')}
        </button>
        <button
          on:click={handleCreate}
          disabled={creating || !newTenant.name.trim() || !newTenant.slug.trim() || (isEnterpriseMode && !newTenant.admin_email.trim())}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {creating ? $_('common.loading') : $_('common.create')}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}

<!-- Request Tenant Creation Modal -->
{#if showRequestModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay" on:click={() => showRequestModal = false} on:keydown={(e) => e.key === 'Escape' && (showRequestModal = false)} role="dialog" aria-modal="true">
  <div class="bg-white rounded-xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content" on:click|stopPropagation role="document">
    <h2 class="text-lg font-bold text-gray-900 mb-5">{$_('tenants.requestCreateTenant')}</h2>
    <p class="text-sm text-gray-500 mb-4">{$_('tenants.selfServiceNotice')}</p>
    <div class="space-y-4">
      <div>
        <label for="request-tenant-name" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('tenants.tenantName')} <span class="text-rose-500">*</span></label>
        <input
          id="request-tenant-name"
          type="text"
          bind:value={newRequest.tenant_name}
          placeholder={$_('tenants.companyName')}
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900"
        />
      </div>
      <div>
        <label for="request-message" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('tenants.applyMessage')}</label>
        <textarea
          id="request-message"
          bind:value={newRequest.message}
          placeholder={$_('tenants.applyMessagePlaceholder')}
          rows="3"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white text-gray-900 resize-none"
        ></textarea>
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showRequestModal = false; newRequest = { tenant_name: '', message: '' }; }}
          class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
        >
          {$_('common.cancel')}
        </button>
        <button
          on:click={handleSubmitRequest}
          disabled={submittingRequest || !newRequest.tenant_name.trim()}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {submittingRequest ? $_('common.loading') : $_('common.submit')}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}

<!-- Tenant Requests List Modal (for super_admin) -->
{#if showRequestsListModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay" on:click={closeRequestsList} on:keydown={(e) => e.key === 'Escape' && closeRequestsList()} role="dialog" aria-modal="true">
  <div class="bg-white rounded-xl p-6 w-full max-w-4xl shadow-elevated-lg border border-gray-200 modal-content max-h-[90vh] overflow-y-auto" on:click|stopPropagation role="document">
    <div class="flex items-center justify-between mb-5">
      <h2 class="text-lg font-bold text-gray-900">{$_('tenants.tenantRequests')}</h2>
      <button on:click={closeRequestsList} class="p-2 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-all">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
      </button>
    </div>

    {#if loadingRequests}
      <LoadingSpinner />
    {:else if tenantRequests.length === 0}
      <div class="text-center py-12 text-gray-400">
        <svg class="w-12 h-12 mx-auto mb-3 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/></svg>
        <p>{$_('tenants.noRequests')}</p>
      </div>
    {:else}
      <div class="flex gap-6">
        <!-- Requests List -->
        <div class="flex-1 space-y-3">
          {#each tenantRequests as request (request.id)}
            <button
              on:click={() => selectRequestForReview(request)}
              class="w-full text-left p-4 rounded-xl border transition-all {selectedRequest?.id === request.id ? 'border-blue-400 bg-blue-50' : 'border-gray-200 hover:border-gray-300 hover:bg-gray-50'}"
            >
              <div class="flex items-start justify-between">
                <div class="flex-1">
                  <h4 class="font-semibold text-gray-900">{request.tenant_name}</h4>
                  <p class="text-sm text-gray-500 mt-1">
                    {$_('tenants.applicant')}: {request.applicant_name} ({request.applicant_email})
                  </p>
                  <p class="text-xs text-gray-400 mt-1">
                    {new Date(request.created_at).toLocaleString()}
                  </p>
                  {#if request.message}
                    <p class="text-sm text-gray-600 mt-2 italic">"{request.message}"</p>
                  {/if}
                </div>
                <span class="px-2 py-1 rounded text-xs font-medium {request.status === 'pending' ? 'bg-amber-50 text-amber-600' : request.status === 'approved' ? 'bg-emerald-50 text-emerald-600' : 'bg-rose-50 text-rose-600'}">
                  {request.status}
                </span>
              </div>
            </button>
          {/each}
        </div>

        <!-- Review Panel -->
        {#if selectedRequest}
          <div class="w-80 border-l border-gray-200 pl-6">
            <h3 class="font-semibold text-gray-800 mb-4">{$_('tenants.reviewRequest')}</h3>
            <div class="mb-4">
              <p class="text-sm text-gray-600"><strong>{$_('tenants.requestedTenant')}:</strong></p>
              <p class="text-lg font-medium text-gray-900">{selectedRequest.tenant_name}</p>
            </div>
            <div class="mb-4">
              <p class="text-sm text-gray-600"><strong>{$_('tenants.applicant')}:</strong></p>
              <p class="text-sm text-gray-800">{selectedRequest.applicant_name}</p>
              <p class="text-xs text-gray-500">{selectedRequest.applicant_email}</p>
            </div>
            {#if selectedRequest.message}
              <div class="mb-4">
                <p class="text-sm text-gray-600"><strong>{$_('tenants.applyMessage')}:</strong></p>
                <p class="text-sm text-gray-700 italic">"{selectedRequest.message}"</p>
              </div>
            {/if}
            <div class="mb-4">
              <label for="review-note" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('tenants.reviewNote')}</label>
              <textarea
                id="review-note"
                bind:value={reviewNote}
                placeholder={$_('tenants.reviewNotePlaceholder')}
                rows="3"
                class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm resize-none"
              ></textarea>
            </div>
            <div class="flex gap-3">
              <button
                on:click={() => handleReview('approve')}
                disabled={reviewing || selectedRequest.status !== 'pending'}
                class="flex-1 px-4 py-2 bg-emerald-500 hover:bg-emerald-600 text-white rounded-lg font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {$_('tenants.approveRequest')}
              </button>
              <button
                on:click={() => handleReview('reject')}
                disabled={reviewing || selectedRequest.status !== 'pending'}
                class="flex-1 px-4 py-2 bg-rose-500 hover:bg-rose-600 text-white rounded-lg font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {$_('tenants.rejectRequest')}
              </button>
            </div>
            {#if selectedRequest.status !== 'pending'}
              <p class="text-xs text-gray-500 mt-2 text-center">
                This request has been {selectedRequest.status}
              </p>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
{/if}

<style>
  .card-interactive {
    transition: all 0.2s ease;
  }
  .card-interactive:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    transform: translateY(-2px);
  }
</style>
