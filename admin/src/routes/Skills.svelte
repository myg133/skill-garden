<script>
  import { onMount } from 'svelte';
  import { Link } from 'svelte-routing';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { isAdmin } from '../stores/auth.js';
  import { hasPermission, permissionStore, isAnyAdmin } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import { selectedOrg, isPersonalSpace } from '../stores/org.js';

  $: skillLinkBase = ($isAdmin || ($permissionStore.loaded && (isAnyAdmin() || ($permissionStore.orgRoles || []).length > 0))) ? '/skills' : '/user/skills';
  const ACT = ACTIONS.Skills;
  import Badge from '../components/Badge.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import SortHeader from '../components/SortHeader.svelte';
  import FileTreeNode from '../components/FileTreeNode.svelte';

  // --- Role Detection ---
  $: systemRoles = $permissionStore.systemRoles || [];
  $: isSuperAdmin = systemRoles.includes('super_admin');
  $: isMarketplaceAdmin = systemRoles.includes('marketplace_admin');
  $: isMarketplaceReviewer = systemRoles.includes('marketplace_reviewer');
  $: isMarketplaceRole = isMarketplaceAdmin || isMarketplaceReviewer;

  // --- View Mode ---
  // Tabs only for marketplace roles
  let activeTab = 'marketplace-list'; // 'marketplace-stats' | 'marketplace-list' | 'personal'

  // --- Org Context ---
  $: currentOrgId = $selectedOrg?.id || null;
  $: currentOrgName = $selectedOrg?.name || '';
  $: currentOrgRole = $selectedOrg?.role || 'member';
  $: inPersonalSpace = !!$isPersonalSpace;

  let skills = [];
  let loading = true;
  let error = '';
  let keyword = '';
  let tagFilter = '';
  let page = 1;
  let total = 0;
  let pageSize = 20;
  let allTags = [];
  let currentSearch = '';
  let sortKey = '';
  let sortDir = 'asc';

  // Publish scope selection
  let showScopeModal = false;
  let selectedSkillForPublish = null;
  let selectedScope = 'organization';

  // Change visibility modal
  let showVisibilityModal = false;
  let selectedSkillForVisibility = null;
  let selectedVisibility = 'org_visible';

  const visibilityOptions = [
    { value: 'private', label: 'Private - Only you can see this' },
    { value: 'group_visible', label: 'Group - Visible to group members only' },
    { value: 'org_visible', label: 'Organization - Visible to all org members' },
    { value: 'tenant_visible', label: 'Tenant - Visible to all tenant members' },
  ];

  const scopeOptions = [
    { value: 'private', label: 'Private - Only you can see this' },
    { value: 'group', label: 'Group - Visible to group members only' },
    { value: 'organization', label: 'Organization - Visible to all org members' },
    { value: 'tenant', label: 'Tenant - Visible to all tenant members' },
  ];

  // Marketplace stats
  let marketStats = { listed: 0, pending: 0, newThisMonth: 0, downloads: 0 };

  $: sortedSkills = sortSkills(skills, sortKey, sortDir);

  function sortSkills(list, key, dir) {
    if (!key || !list.length) return list;
    return [...list].sort((a, b) => {
      let va = a[key] ?? '';
      let vb = b[key] ?? '';
      if (typeof va === 'string') va = va.toLowerCase();
      if (typeof vb === 'string') vb = vb.toLowerCase();
      if (va < vb) return dir === 'asc' ? -1 : 1;
      if (va > vb) return dir === 'asc' ? 1 : -1;
      return 0;
    });
  }

  function handleSort(key, dir) {
    sortKey = key;
    sortDir = dir;
  }

  function orgRoleLabel(role) {
    const labels = {
      owner: $_('skills.owner'),
      admin: $_('skills.admin'),
      reviewer: $_('skills.reviewer'),
      developer: $_('skills.developer'),
      member: $_('skills.member')
    };
    return labels[role] || role;
  }

  function orgRoleBadgeColor(role) {
    const colors = {
      owner: 'bg-amber-100 text-amber-700',
      admin: 'bg-blue-100 text-blue-700',
      reviewer: 'bg-purple-100 text-purple-700',
      developer: 'bg-emerald-100 text-emerald-700',
      member: 'bg-gray-100 text-gray-600',
    };
    return colors[role] || 'bg-gray-100 text-gray-600';
  }

  // --- Create Modal State ---
  let showCreateModal = false;
  // 'upload' | 'preview' | 'confirming'
  let createStep = 'upload';
  let uploading = false;
  let confirming = false;
  let isDragging = false;
  let uploadedFileName = '';
  // Preview data from server
  let previewId = '';
  let previewMeta = null;
  let previewFiles = [];
  let previewTotalSize = 0;
  // File viewer
  let selectedFilePath = '';
  let selectedFileContent = '';
  let selectedFileLoading = false;
  let fileFetchCache = {};
  // Organization selection for upload
  let userOrgs = [];
  let selectedOrgId = '';

  onMount(() => {
    loadSkills();
  });

  // Reload when org context changes
  $: $selectedOrg, (() => { page = 1; loadSkills(); })();

  // Reload when tab changes (for marketplace roles)
  $: activeTab, (() => { if (isMarketplaceRole) { page = 1; loadSkills(); } })();

  async function loadSkills() {
    loading = true;
    error = '';
    try {
      const params = { page, page_size: pageSize };
      if (keyword.trim()) params.keyword = keyword.trim();
      if (tagFilter) params.tag = tagFilter;

      // Phase 5-6: Apply context-aware filters
      const isAdminUser = $isAdmin || isSuperAdmin;
      const hasOrgRoles = ($permissionStore.orgRoles || []).length > 0;

      if (isSuperAdmin) {
        // super_admin: no extra filter (sees everything across all orgs)
      } else if (!isAdminUser && !isMarketplaceRole && !hasOrgRoles) {
        params.scope_personal = true;
      } else if (isMarketplaceRole) {
        if (activeTab === 'marketplace-list') {
          params.marketplace_status = 'listed';
        } else if (activeTab === 'personal') {
          params.scope_personal = true;
        }
      } else if (inPersonalSpace) {
        params.scope_personal = true;
      } else if (currentOrgId) {
        params.org_id = currentOrgId;
      }

      const res = await api.listSkills(params);
      skills = res.data || [];
      total = res.total || skills.length;

      if (allTags.length === 0 && skills.length > 0) {
        const tagsSet = new Set();
        skills.forEach(s => (s.tags || []).forEach(t => tagsSet.add(t)));
        allTags = [...tagsSet].sort();
      }

      // Load market stats for marketplace roles
      if (isMarketplaceRole) {
        loadMarketStats();
      }
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function loadMarketStats() {
    try {
      const stats = await api.marketplaceStats();
      marketStats = {
        listed: stats.listed || 0,
        pending: stats.pending_review || 0,
        pendingUpdate: stats.pending_update || 0,
        pendingDelist: stats.pending_delist || 0,
        newThisMonth: stats.new_this_month || 0,
        downloads: stats.total_installs || 0,
      };
    } catch {
      // silently fail for stats
    }
  }

  function handleSearch() {
    page = 1;
    currentSearch = keyword;
    loadSkills();
  }

  function handleTagFilter(tag) {
    tagFilter = tag;
    page = 1;
    loadSkills();
  }

  function handleClearFilters() {
    keyword = '';
    tagFilter = '';
    page = 1;
    currentSearch = '';
    loadSkills();
  }

  async function handleUnpublishSkill(skill) {
    const skillId = skill.id;
    const skillName = skill.name;
    try {
      await api.adminUnpublishSkill(skillId);
      addToast(`${skillName} delisted`, 'success');
      skills = skills.map(s => s.id === skillId ? { ...s, status: 'approved', visibility: 'private' } : s);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleDeleteSkill(skill) {
    // Listed marketplace skills must request delist before deletion
    if (skill.marketplace_status === 'listed' || skill.marketplace_status === 'pending_delist') {
      if (skill.marketplace_status === 'listed') {
        const reason = prompt(`"${skill.name}" is listed on marketplace. Request delist first.\nEnter delist reason (optional):`);
        if (reason === null) return;
        const skillId = skill.id;
        const skillName = skill.name;
        try {
          await api.requestMarketplaceDelist(skillId, reason || undefined);
          addToast(`${skillName} delist request submitted, can delete after approval`, 'success');
          skills = skills.map(s => s.id === skillId ? { ...s, marketplace_status: 'pending_delist' } : s);
        } catch (e) {
          addToast(`Delist request failed: ${e.message}`, 'error');
        }
      } else {
        addToast(`"${skill.name}" delist request pending, can delete after approval`, 'warning');
      }
      return;
    }

    let confirmMsg = '';
    if (skill.marketplace_status === 'pending_review') {
      confirmMsg = `"${skill.name}" is under marketplace review. Deleting will also cancel the review. Confirm?`;
    } else if (skill.marketplace_status === 'delisted') {
      confirmMsg = `"${skill.name}" has been delisted from marketplace. Delete permanently?`;
    } else {
      confirmMsg = `Permanently delete "${skill.name}"? This action cannot be undone.`;
    }
    if (!confirm(confirmMsg)) return;

    try {
      await api.deleteSkill(skill.id);
      addToast(`${skill.name} deleted`, 'success');
      loadSkills();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleAdminPublishSkill(skill) {
    const skillId = skill.id;
    const skillName = skill.name;
    try {
      await api.adminPublishSkill(skillId);
      addToast(`${skillName} listed`, 'success');
      skills = skills.map(s => s.id === skillId ? { ...s, status: 'published', visibility: 'marketplace' } : s);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleSubmitReview(skill) {
    const skillId = skill.id;
    const skillName = skill.name;
    try {
      await api.submitSkillForReview(skillId);
      addToast(`${skillName} submitted for review`, 'success');
      skills = skills.map(s => s.id === skillId ? { ...s, status: 'pending_review' } : s);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handlePublishSkill(skill) {
    // Open scope selection modal
    selectedSkillForPublish = skill;
    selectedScope = 'organization';
    showScopeModal = true;
  }

  async function confirmPublishSkill() {
    if (!selectedSkillForPublish) return;
    const skillId = selectedSkillForPublish.id;
    const skillName = selectedSkillForPublish.name;
    showScopeModal = false;
    selectedSkillForPublish = null;
    try {
      await api.publishSkill(skillId, selectedScope);
      addToast(`${skillName} published`, 'success');
      // 更新 skills 数组中的状态
      skills = skills.map(s => s.id === skillId ? { ...s, status: 'published' } : s);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  // --- Change Visibility ---
  function handleChangeVisibility(skill) {
    selectedSkillForVisibility = skill;
    selectedVisibility = skill.visibility || 'org_visible';
    showVisibilityModal = true;
  }

  async function confirmChangeVisibility() {
    if (!selectedSkillForVisibility) return;
    const skillId = selectedSkillForVisibility.id;
    const skillName = selectedSkillForVisibility.name;
    showVisibilityModal = false;
    selectedSkillForVisibility = null;
    try {
      await api.updateSkillVisibility(skillId, selectedVisibility);
      addToast(`${skillName} visibility changed`, 'success');
      skills = skills.map(s => s.id === skillId ? { ...s, visibility: selectedVisibility } : s);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  // --- Marketplace dual-track operations ---
  async function handleSubmitToMarketplace(skill) {
    if (!confirm(`Submit "${skill.name}" for marketplace review?`)) return;
    const skillId = skill.id;
    const skillName = skill.name;
    try {
      await api.submitToMarketplace(skillId);
      addToast(`${skillName} submitted for marketplace review`, 'success');
      skills = skills.map(s => s.id === skillId ? { ...s, marketplace_status: 'pending_review' } : s);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleApproveMarket(skill) {
    if (!confirm(`Approve "${skill.name}" for marketplace listing?`)) return;
    try {
      await api.marketplaceReviewApprove(skill.id);
      addToast(`${skill.name} approved and listed on marketplace`, 'success');
      skills = skills.map(s => s.id === skill.id ? { ...s, marketplace_status: 'listed' } : s);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleRejectMarket(skill) {
    if (!confirm(`Reject "${skill.name}" from marketplace?`)) return;
    try {
      await api.marketplaceReviewReject(skill.id);
      addToast(`${skill.name} rejected from marketplace`, 'success');
      skills = skills.map(s => s.id === skill.id ? { ...s, marketplace_status: 'rejected' } : s);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleMarketplaceDelist(skill) {
    if (!confirm(`Delist "${skill.name}" from marketplace? The skill will not be deleted.`)) return;
    const skillId = skill.id;
    const skillName = skill.name;
    try {
      await api.marketplaceDelist(skillId);
      addToast(`${skillName} delisted from marketplace`, 'success');
      skills = skills.map(s => s.id === skillId ? { ...s, marketplace_status: 'delisted' } : s);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleMarketplaceRelist(skill) {
    if (!confirm(`Relist "${skill.name}" on marketplace?`)) return;
    const skillId = skill.id;
    const skillName = skill.name;
    try {
      await api.marketplaceRelist(skillId);
      addToast(`${skillName} relisted`, 'success');
      skills = skills.map(s => s.id === skillId ? { ...s, marketplace_status: 'listed' } : s);
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  function goToPage(p) {
    page = p;
    loadSkills();
  }

  function handleKeydown(e) {
    if (e.key === 'Enter') handleSearch();
  }

  // --- Create Modal ---
  async function openCreateModal() {
    createStep = 'upload';
    uploading = false;
    confirming = false;
    uploadedFileName = '';
    previewId = '';
    previewMeta = null;
    previewFiles = [];
    previewTotalSize = 0;
    selectedFilePath = '';
    selectedFileContent = '';
    fileFetchCache = {};
    selectedOrgId = '';
    showCreateModal = true;

    // Load user's organizations and default to first one
    try {
      const res = await api.getUserOrgs();
      userOrgs = res.data || res || [];
      if (userOrgs.length > 0) {
        selectedOrgId = userOrgs[0].id;
      }
    } catch {
      userOrgs = [];
    }
  }

  function closeCreateModal() {
    showCreateModal = false;
  }

  // --- File Upload ---
  function handleFileUpload(e) {
    const file = e.target.files?.[0];
    if (!file) return;
    processZipFile(file);
    e.target.value = '';
  }

  function handleDragOver(e) {
    e.preventDefault();
    e.stopPropagation();
    isDragging = true;
  }

  function handleDragLeave(e) {
    e.preventDefault();
    e.stopPropagation();
    isDragging = false;
  }

  function handleDrop(e) {
    e.preventDefault();
    e.stopPropagation();
    isDragging = false;
    const file = e.dataTransfer?.files?.[0];
    if (!file) return;
    processZipFile(file);
  }

  function formatSize(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function fileIcon(path) {
    const ext = path.split('.').pop()?.toLowerCase();
    if (path === 'SKILL.md' || path.endsWith('.md')) return 'md';
    if (!ext) return 'file';
    switch (ext) {
      case 'js': case 'ts': case 'jsx': case 'tsx': return 'js';
      case 'py': return 'py';
      case 'rs': return 'rs';
      case 'json': case 'yaml': case 'yml': case 'toml': return 'config';
      case 'html': case 'css': case 'scss': return 'web';
      case 'sql': return 'db';
      case 'sh': case 'bash': return 'script';
      default: return 'file';
    }
  }

  function fileIconColor(path) {
    const icon = fileIcon(path);
    switch (icon) {
      case 'md': return 'text-blue-500';
      case 'js': return 'text-yellow-500';
      case 'py': return 'text-green-500';
      case 'rs': return 'text-orange-500';
      case 'config': return 'text-gray-500';
      case 'web': return 'text-purple-500';
      case 'db': return 'text-teal-500';
      default: return 'text-gray-400';
    }
  }

  async function processZipFile(file) {
    if (!file.name.endsWith('.zip') && !file.name.endsWith('.ZIP')) {
      addToast('Only .zip files are supported', 'warning');
      return;
    }
    uploadedFileName = file.name;
    uploading = true;
    try {
      const formData = new FormData();
      formData.append('file', file);

      const res = await api.previewSkillUpload(formData);
      previewId = res.preview_id;
      previewMeta = res.metadata;
      previewFiles = res.files.sort((a, b) => {
        // SKILL.md first, then dirs first
        if (a.path === 'SKILL.md') return -1;
        if (b.path === 'SKILL.md') return 1;
        const aDir = a.path.includes('/');
        const bDir = b.path.includes('/');
        if (aDir !== bDir) return aDir ? -1 : 1;
        return a.path.localeCompare(b.path);
      });
      previewTotalSize = res.total_size;
      createStep = 'preview';

      // Auto-select SKILL.md (may be at root or nested in dir)
      const skillMd = previewFiles.find(f => f.path === 'SKILL.md' || f.path.endsWith('/SKILL.md'));
      if (skillMd) {
        await selectFile(skillMd.path);
      }
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      uploading = false;
    }
  }

  async function selectFile(filePath) {
    if (selectedFilePath === filePath && fileFetchCache[filePath]) {
      selectedFileContent = fileFetchCache[filePath];
      return;
    }

    // Check cache
    if (fileFetchCache[filePath]) {
      selectedFilePath = filePath;
      selectedFileContent = fileFetchCache[filePath];
      return;
    }

    selectedFilePath = filePath;
    selectedFileLoading = true;
    try {
      const res = await api.getPreviewFile(previewId, filePath);
      selectedFileContent = res.content;
      fileFetchCache[filePath] = res.content;
    } catch (e) {
      selectedFileContent = `Error loading file: ${e.message}`;
    } finally {
      selectedFileLoading = false;
    }
  }

  async function handleConfirmUpload() {
    confirming = true;
    try {
      // 默认使用 organization 类型，使用当前选中的组织
      const data = {
        owner_type: 'organization',
      };
      if (selectedOrgId) {
        data.organization_id = selectedOrgId;
      } else if (userOrgs.length > 0) {
        data.organization_id = userOrgs[0].id;
      }
      const res = await api.confirmSkillUpload(previewId, data);
      addToast(res.message || 'Skill uploaded successfully', 'success');
      closeCreateModal();
      await loadSkills();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      confirming = false;
    }
  }

  // --- File tree helpers ---
  function buildFileTree(files) {
    const root = { children: {} };
    for (const f of files) {
      const parts = f.path.split('/');
      let current = root;
      for (let i = 0; i < parts.length; i++) {
        const name = parts[i];
        if (i === parts.length - 1) {
          current.children[name] = {
            name,
            type: 'file',
            path: f.path,
            size: f.size,
          };
        } else {
          if (!current.children[name]) {
            current.children[name] = {
              name,
              type: 'dir',
              path: parts.slice(0, i + 1).join('/'),
              children: {},
            };
          }
          current = current.children[name];
        }
      }
    }
    return sortTree(Object.values(root.children));
  }

  function sortTree(items) {
    return items
      .map(item => {
        if (item.type === 'dir' && item.children) {
          item.children = sortTree(Object.values(item.children));
        }
        return item;
      })
      .sort((a, b) => {
        if (a.type !== b.type) return a.type === 'dir' ? -1 : 1;
        return a.name.localeCompare(b.name);
      });
  }

  $: totalPages = Math.max(1, Math.ceil(total / pageSize));
  $: fileTreeNodes = buildFileTree(previewFiles);
</script>

<div class="p-8">
  <div class="page-header flex items-center justify-between">
    <div>
      <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">
        {#if isMarketplaceRole || $isAdmin || isSuperAdmin}
          {$_('skills.title')}
        {:else if inPersonalSpace}
          {$_('skills.mySkills')}
        {:else}
          {$_('skills.title')}
        {/if}
      </h1>
      <p class="text-gray-500 text-sm mt-1.5 font-medium">
        {#if isMarketplaceRole}
          {$_('skills.marketplaceDescription')}
        {:else if $isAdmin || isSuperAdmin}
          {$_('skills.adminDescription')}
        {:else if inPersonalSpace}
          {$_('skills.personalDescription')}
        {:else if currentOrgId}
          {$_('skills.orgDescription').replace('{org}', currentOrgName)}
          <span class="ml-2 inline-flex items-center px-2 py-0.5 text-xs rounded-full {orgRoleBadgeColor(currentOrgRole)}">{orgRoleLabel(currentOrgRole)}</span>
        {:else}
          Browse and manage all skills
        {/if}
      </p>
    </div>
    <div class="flex items-center gap-3">
      {#if hasPermission(ACT.create)}
      <button
        on:click={openCreateModal}
        class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        {$_('skills.newSkill')}
      </button>
      {/if}
      <span class="inline-flex items-center gap-1.5 px-3.5 py-2 bg-white text-blue-700 rounded-xl text-sm font-semibold ring-1 ring-sky-600/20">
        <span class="w-1.5 h-1.5 rounded-full bg-white0"></span>
        {$_('common.totalCount', { values: { total } })}
      </span>
    </div>
  </div>

  <!-- Tab bar for marketplace roles -->
  {#if isMarketplaceRole}
  <div class="flex items-center gap-1 mb-6 bg-gray-100 p-1 rounded-xl w-fit">
    <button
      on:click={() => activeTab = 'marketplace-stats'}
      class="px-4 py-2 rounded-lg text-sm font-semibold transition-all {activeTab === 'marketplace-stats' ? 'bg-white text-gray-800 shadow-sm' : 'text-gray-500 hover:text-gray-700'}"
    >{$_('skills.marketStats.title')}</button>
    <button
      on:click={() => activeTab = 'marketplace-list'}
      class="px-4 py-2 rounded-lg text-sm font-semibold transition-all {activeTab === 'marketplace-list' ? 'bg-white text-gray-800 shadow-sm' : 'text-gray-500 hover:text-gray-700'}"
    >{$_('skills.marketplaceSkills')}</button>
    <button
      on:click={() => activeTab = 'personal'}
      class="px-4 py-2 rounded-lg text-sm font-semibold transition-all {activeTab === 'personal' ? 'bg-white text-gray-800 shadow-sm' : 'text-gray-500 hover:text-gray-700'}"
    >{$_('skills.personal')}</button>
  </div>
  {/if}

  <!-- Marketplace Stats Tab -->
  {#if isMarketplaceRole && activeTab === 'marketplace-stats'}
  <div class="grid grid-cols-4 gap-4 mb-6">
    <div class="bg-white rounded-xl border border-gray-200 p-5 shadow-card">
      <p class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('skills.marketStats.listed')}</p>
      <p class="text-2xl font-bold text-emerald-600">{marketStats.listed}</p>
    </div>
    <div class="bg-white rounded-xl border border-gray-200 p-5 shadow-card">
      <p class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('skills.marketStats.pending')}</p>
      <p class="text-2xl font-bold text-amber-600">{marketStats.pending}</p>
    </div>
    <div class="bg-white rounded-xl border border-gray-200 p-5 shadow-card">
      <p class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('skills.marketStats.newThisMonth')}</p>
      <p class="text-2xl font-bold text-blue-600">{marketStats.newThisMonth}</p>
    </div>
    <div class="bg-white rounded-xl border border-gray-200 p-5 shadow-card">
      <p class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">{$_('skills.marketStats.downloads')}</p>
      <p class="text-2xl font-bold text-purple-600">{marketStats.downloads}</p>
    </div>
  </div>
  {/if}

  <!-- 搜索/筛选栏：市场角色只在非 stats tab 时显示 -->
  {#if !(isMarketplaceRole && activeTab === 'marketplace-stats')}
  <div class="flex flex-wrap items-center gap-3 mb-6">
    <div class="relative flex-1 min-w-[280px] max-w-md">
      <svg class="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
      </svg>
      <input
        type="text"
        bind:value={keyword}
        on:keydown={handleKeydown}
        placeholder={$_('skills.searchPlaceholder')}
        class="w-full pl-10 pr-4 py-2.5 bg-white border border-gray-200 rounded-lg text-sm text-gray-900 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 transition-all"
      />
    </div>

    <select
      bind:value={tagFilter}
      on:change={() => handleTagFilter(tagFilter)}
      aria-label={$_('skills.table.tags')}
      class="px-4 py-2.5 bg-white border border-gray-200 rounded-lg text-sm text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500/20 cursor-pointer"
    >
      <option value="" disabled selected hidden>{$_('common.filter')}</option>
      <option value="">{$_('common.all')}</option>
      {#each allTags as tag}
        <option value={tag}>{tag}</option>
      {/each}
    </select>

    <button
      on:click={handleSearch}
      class="px-5 py-2.5 bg-blue-600 text-white rounded-lg text-sm font-semibold hover:bg-blue-700 transition-colors shadow-sm"
    >
      {$_('common.search')}
    </button>

    {#if keyword || tagFilter}
      <button
        on:click={handleClearFilters}
        class="px-4 py-2.5 text-gray-500 hover:text-gray-700 text-sm font-medium transition-colors"
      >
        {$_('common.clearFilter')}
      </button>
    {/if}
  </div>
  {/if}

  <!-- 列表表格：市场角色只在非 stats tab 时显示 -->
  {#if !(isMarketplaceRole && activeTab === 'marketplace-stats')}
    {#if loading}
      <LoadingSpinner />
    {:else if error}
      <div class="bg-red-50 border border-red-100 text-red-600 px-5 py-4 rounded-xl text-sm font-medium">{error}</div>
    {:else if skills.length === 0}
      <div class="bg-white rounded-xl border border-gray-200 shadow-card">
        <EmptyState message={$_('skills.noSkillsFound')} />
      </div>
    {:else}
      <div class="bg-white rounded-xl border border-gray-200 overflow-hidden shadow-card">
        <table class="w-full">
          <thead>
            <tr class="border-b border-gray-100 bg-gray-50">
              <th class="px-6 py-4 text-left"><SortHeader label={$_('skills.table.name')} sortKey="name" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
              <th class="px-6 py-4 text-left"><SortHeader label={$_('skills.table.version')} sortKey="version" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
              <th class="px-6 py-4 text-left"><SortHeader label={$_('skills.table.status')} sortKey="status" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
              {#if isMarketplaceRole}
              <th class="px-6 py-4 text-left"><SortHeader label={$_('skills.table.mktStatus')} sortKey="marketplace_status" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
              {/if}
              {#if !isMarketplaceRole}
              <th class="px-6 py-4 text-left"><SortHeader label={$_('skills.table.tags')} /></th>
              {/if}
              {#if !isMarketplaceRole && !inPersonalSpace}
              <th class="px-6 py-4 text-left"><SortHeader label={$_('skills.table.visibility')} sortKey="visibility" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
              {/if}
              {#if isMarketplaceRole}
              <th class="px-6 py-4 text-left"><SortHeader label={$_('skills.table.source')} /></th>
              {:else if !inPersonalSpace}
              <th class="px-6 py-4 text-left"><SortHeader label={$_('skills.table.author')} sortKey="author_agent_id" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
              {/if}
              {#if !isMarketplaceRole}
              <th class="px-6 py-4 text-left"><SortHeader label={$_('skills.table.installs')} sortKey="install_count" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
              <th class="px-6 py-4 text-left"><SortHeader label={$_('skills.table.created')} sortKey="created" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
              {/if}
              <th class="px-6 py-4 text-left"><SortHeader label={$_('skills.table.actions')} /></th>
            </tr>
          </thead>
          <tbody>
            {#each sortedSkills as skill (skill.id)}
              <tr class="table-row hover:bg-gray-50">
                <td class="px-6 py-4">
                  <Link to="{skillLinkBase}/{skill.id}?from=skills" class="text-blue-600 hover:text-blue-700 font-semibold text-sm transition-colors">
                    {skill.name}
                  </Link>
                  {#if skill.description}
                    <p class="text-gray-500 text-xs mt-0.5 max-w-[240px] truncate">{skill.description}</p>
                  {/if}
                </td>
                <td class="px-6 py-4">
                  <span class="text-gray-500 text-xs font-mono">{skill.version || '1.0.0'}</span>
                </td>
                <td class="px-6 py-4">
                  <Badge status={skill.status || 'draft'} />
                </td>
                {#if isMarketplaceRole}
                <td class="px-6 py-4">
                  {#if skill.marketplace_status === 'listed'}
                    <span class="px-2 py-0.5 bg-emerald-100 text-emerald-700 text-[11px] font-medium rounded">listed</span>
                  {:else if skill.marketplace_status === 'pending_review'}
                    <span class="px-2 py-0.5 bg-amber-100 text-amber-700 text-[11px] font-medium rounded">pending</span>
                  {:else if skill.marketplace_status === 'rejected'}
                    <span class="px-2 py-0.5 bg-red-100 text-red-700 text-[11px] font-medium rounded">rejected</span>
                  {:else if skill.marketplace_status === 'delisted'}
                    <span class="px-2 py-0.5 bg-gray-100 text-gray-500 text-[11px] font-medium rounded">delisted</span>
                  {:else}
                    <span class="text-xs text-gray-400">—</span>
                  {/if}
                </td>
                {/if}
                {#if !isMarketplaceRole}
                <td class="px-6 py-4">
                  <div class="flex gap-1.5 flex-wrap">
                    {#each (skill.tags || []).slice(0, 2) as tag}
                      <span class="px-2 py-0.5 bg-gray-100 text-gray-500 text-[11px] font-medium rounded">{tag}</span>
                    {/each}
                    {#if (skill.tags || []).length > 2}
                      <span class="px-2 py-0.5 bg-gray-100 text-gray-500 text-[11px] font-medium rounded">+{skill.tags.length - 2}</span>
                    {/if}
                  </div>
                </td>
                {/if}
                {#if !isMarketplaceRole && !inPersonalSpace}
                <td class="px-6 py-4">
                  <span class="text-gray-500 text-xs capitalize">{skill.visibility || 'org_visible'}</span>
                </td>
                {/if}
                {#if isMarketplaceRole}
                <td class="px-6 py-4 text-gray-500 text-xs">
                  {skill.owner_name || (skill.owner_type === 'user' ? 'Personal · ' + (skill.author_name || 'N/A') : skill.owner_type || 'N/A')}
                </td>
                {:else if !inPersonalSpace}
                <td class="px-6 py-4 text-gray-500 text-xs">{skill.author_name || skill.author_agent_id || 'N/A'}</td>
                {/if}
                {#if !isMarketplaceRole}
                <td class="px-6 py-4">
                  <span class="text-gray-600 text-sm font-semibold">{skill.install_count || 0}</span>
                </td>
                <td class="px-6 py-4 text-gray-500 text-sm">{(skill.created || skill.created_at) ? new Date(skill.created || skill.created_at).toLocaleDateString() : 'N/A'}</td>
                {/if}
                  <td class="px-6 py-4">
                    <div class="flex items-center gap-1.5">
                      <!-- Submit internal review -->
                      {#if (skill.status === 'draft' || skill.status === 'rejected') && hasPermission(ACT.submitReview)}
                        <button
                          on:click={() => handleSubmitReview(skill)}
                          class="px-2.5 py-1 text-[11px] font-semibold bg-amber-50 text-amber-700 border border-amber-200 rounded-lg hover:bg-amber-100 transition-colors"
                        >Submit Review</button>
                      {/if}

                      <!-- Publish (approved -> published) -->
                      {#if skill.status === 'approved' && hasPermission(ACT.publishInternal)}
                        <button
                          on:click={() => handlePublishSkill(skill)}
                          class="px-2.5 py-1 text-[11px] font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200 rounded-lg hover:bg-emerald-100 transition-colors"
                        >Publish</button>
                      {/if}

                      <!-- Marketplace operations (marketplace_admin / marketplace_reviewer) -->
                      {#if isMarketplaceRole}
                        {#if skill.marketplace_status === 'listed'}
                          {#if hasPermission(ACT.marketFeature) && !skill.is_featured}
                            <button
                              on:click={() => handleAdminPublishSkill(skill)}
                              class="px-2.5 py-1 text-[11px] font-semibold bg-amber-50 text-amber-700 border border-amber-200 rounded-lg hover:bg-amber-100 transition-colors"
                            >Feature</button>
                          {/if}
                          {#if hasPermission(ACT.marketDelist)}
                            <button
                              on:click={() => handleMarketplaceDelist(skill)}
                              class="px-2.5 py-1 text-[11px] font-semibold bg-rose-50 text-rose-600 border border-rose-200 rounded-lg hover:bg-rose-100 transition-colors"
                            >Delist</button>
                          {/if}
                        {:else if skill.marketplace_status === 'delisted' && hasPermission(ACT.marketRelist)}
                          <button
                            on:click={() => handleMarketplaceRelist(skill)}
                            class="px-2.5 py-1 text-[11px] font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200 rounded-lg hover:bg-emerald-100 transition-colors"
                          >Relist</button>
                        {/if}
                      {:else if skill.status === 'published'}
                        <!-- Submit to marketplace (org owner/admin) -->
                        {#if (!skill.marketplace_status || skill.marketplace_status === 'rejected' || skill.marketplace_status === 'delisted') && hasPermission(ACT.submitToMarketplace)}
                          <button
                            on:click={() => handleSubmitToMarketplace(skill)}
                            class="px-2.5 py-1 text-[11px] font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200 rounded-lg hover:bg-emerald-100 transition-colors"
                          >List on Market</button>
                        {:else if skill.marketplace_status === 'pending_review'}
                          {#if hasPermission(ACT.marketApprove)}
                            <button
                              on:click={() => handleApproveMarket(skill)}
                              class="px-2.5 py-1 text-[11px] font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200 rounded-lg hover:bg-emerald-100 transition-colors"
                            >Approve</button>
                            <button
                              on:click={() => handleRejectMarket(skill)}
                              class="px-2.5 py-1 text-[11px] font-semibold bg-red-50 text-red-600 border border-red-200 rounded-lg hover:bg-red-100 transition-colors"
                            >Reject</button>
                          {:else}
                            <span class="text-[11px] text-amber-500 font-medium">Market Review</span>
                          {/if}
                        {:else if skill.marketplace_status === 'listed'}
                          <span class="text-[11px] text-emerald-500 font-medium">Listed</span>
                        {/if}
                      {/if}

                      <!-- Delete (all roles with permission) -->
                      {#if hasPermission(ACT.delete)}
                      <button
                        on:click={() => handleDeleteSkill(skill)}
                        class="px-2.5 py-1 text-[11px] font-semibold bg-red-50 text-red-600 border border-red-200 rounded-lg hover:bg-red-100 transition-colors"
                      >Delete</button>
                      {/if}

                      <!-- Change Visibility (for published skills) -->
                      {#if skill.status === 'published'}
                      <button
                        on:click={() => handleChangeVisibility(skill)}
                        class="px-2.5 py-1 text-[11px] font-semibold bg-blue-50 text-blue-700 border border-blue-200 rounded-lg hover:bg-blue-100 transition-colors"
                      >Visibility</button>
                      {/if}
                    </div>
                  </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      {#if totalPages > 1 && !isMarketplaceRole}
        <div class="flex items-center justify-between mt-5 px-2">
          <span class="text-gray-500 text-sm">
            {$_('skills.pagination', { values: { page, totalPages, total } })}
          </span>
          <div class="flex gap-1.5">
            <button
              on:click={() => goToPage(page - 1)}
              disabled={page <= 1}
              class="px-3.5 py-2 bg-white border border-gray-200 rounded-lg text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              {$_('common.previous')}
            </button>
            {#each Array(totalPages) as _, i}
              {@const pageNum = i + 1}
              {#if pageNum === 1 || pageNum === totalPages || (pageNum >= page - 2 && pageNum <= page + 2)}
                <button
                  on:click={() => goToPage(pageNum)}
                  class="w-9 h-9 rounded-lg text-sm font-semibold transition-colors {pageNum === page ? 'bg-blue-600 text-white shadow-sm' : 'bg-white border border-gray-200 text-gray-600 hover:bg-gray-50'}"
                >
                  {pageNum}
                </button>
              {:else if pageNum === page - 3 || pageNum === page + 3}
                <span class="w-9 h-9 flex items-center justify-center text-gray-400 text-sm">...</span>
              {/if}
            {/each}
            <button
              on:click={() => goToPage(page + 1)}
              disabled={page >= totalPages}
              class="px-3.5 py-2 bg-white border border-gray-200 rounded-lg text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              {$_('common.next')}
            </button>
          </div>
        </div>
      {/if}
    {/if}
  {/if}
</div>

{#if showCreateModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  {#if createStep === 'upload'}
  <!-- Step 1: Upload ZIP -->
  <div class="bg-white rounded-xl p-6 w-full max-w-lg shadow-elevated-lg border border-gray-200 modal-content">
    <h2 class="text-lg font-bold text-gray-900 mb-5">{$_('skills.uploadSkillPackage')}</h2>
    <div
      role="button"
      tabindex="0"
      on:keydown={(e) => e.key === 'Enter' && document.getElementById('skill-file-input')?.click()}
      class="relative border-2 border-dashed rounded-xl p-10 text-center transition-all cursor-pointer {isDragging ? 'border-blue-400 bg-blue-50/50 scale-[1.01]' : 'border-gray-200 hover:border-blue-300 hover:bg-blue-50'}"
      on:dragover={handleDragOver}
      on:dragleave={handleDragLeave}
      on:drop={handleDrop}
      on:click={() => document.getElementById('skill-file-input')?.click()}
    >
      {#if uploading}
        <LoadingSpinner text={$_('skills.uploadingAnalyzing')} />
      {:else if uploadedFileName}
        <div class="flex items-center justify-center gap-3">
          <svg class="w-8 h-8 text-emerald-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
          <div class="text-left">
            <p class="text-sm font-semibold text-gray-700">{uploadedFileName}</p>
            <p class="text-xs text-gray-400">{$_('skills.uploadProcessing')}</p>
          </div>
        </div>
      {:else}
        <svg class="w-12 h-12 mx-auto text-gray-300 mb-3 {isDragging ? 'text-blue-400' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
        </svg>
        <p class="text-sm text-gray-500 font-medium">
          {isDragging ? $_('skills.dropzone') : $_('skills.dragDropZip')}
        </p>
        <p class="text-xs text-gray-400 mt-1">{$_('skills.skillFileInput')}</p>
      {/if}
      <input
        id="skill-file-input"
        type="file"
        accept=".zip"
        on:change={handleFileUpload}
        class="hidden"
      />
    </div>
    <div class="flex justify-end pt-4">
      <button
        on:click={closeCreateModal}
        class="px-5 py-2.5 border border-gray-200 rounded-xl text-sm font-semibold text-gray-600 hover:bg-gray-100 transition-colors"
      >
        {$_('common.cancel')}
      </button>
    </div>
  </div>

  {:else if createStep === 'preview'}
  <!-- Step 2: Preview -->
  <div class="bg-white rounded-xl shadow-elevated-lg border border-gray-200 modal-content flex flex-col" style="width:90vw;max-width:900px;height:85vh;max-height:700px;">
    <!-- Header -->
    <div class="flex-shrink-0 px-6 py-4 border-b border-gray-200 flex items-center justify-between">
      <div class="flex items-center gap-4 min-w-0">
        <h2 class="text-lg font-bold text-gray-900 truncate">Preview: {previewMeta?.name || 'Skill'}</h2>
        <span class="text-xs text-gray-500 font-mono">
          {#if previewMeta?.version}
            v{previewMeta.version}
          {:else}
            <span class="text-orange-500">v? (auto)</span>
          {/if}
        </span>
        <span class="text-xs text-gray-400">{previewFiles.length} files · {formatSize(previewTotalSize)}</span>
      </div>
      <div class="flex items-center gap-3 flex-shrink-0">
        <button
          on:click={closeCreateModal}
          class="px-4 py-2 border border-gray-200 rounded-lg text-sm font-semibold text-gray-600 hover:bg-gray-100 transition-colors"
        >
          {$_('common.cancel')}
        </button>
        <button
          on:click={handleConfirmUpload}
          disabled={confirming}
          class="px-5 py-2 bg-blue-600 text-white rounded-lg text-sm font-semibold hover:bg-blue-700 transition-colors shadow-sm disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
        >
          {#if confirming}
            <svg class="animate-spin w-4 h-4" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>
            Uploading...
          {:else}
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/></svg>
            {$_('skills.confirmUpload')}
          {/if}
        </button>
      </div>
    </div>

    <!-- Meta tags -->
    {#if previewMeta}
    <div class="flex-shrink-0 px-6 py-2.5 border-b border-gray-100 bg-gray-50 flex flex-wrap items-center gap-2">
      {#if previewMeta.description}
        <span class="text-xs text-gray-600 max-w-[400px] truncate">{previewMeta.description}</span>
      {/if}
      {#each (previewMeta.tags || []) as tag}
        <span class="px-2 py-0.5 bg-white text-gray-500 text-[11px] font-medium rounded border border-gray-200">{tag}</span>
      {/each}
      {#if (previewMeta.dependencies || []).length > 0}
        <span class="text-[11px] text-gray-400 ml-2">deps: {previewMeta.dependencies.join(', ')}</span>
      {/if}
    </div>
    {/if}

    <!-- Organization selector for upload -->
    {#if userOrgs.length > 0}
    <div class="flex-shrink-0 px-6 py-3 border-b border-gray-100 bg-white flex items-center gap-4">
      <span class="text-xs font-semibold text-gray-500 uppercase tracking-wider">{$_('skills.uploadTo') || 'Upload to'}</span>
      <select bind:value={selectedOrgId} class="px-3 py-1.5 border border-gray-200 rounded-lg text-sm text-gray-700 bg-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500">
        {#each userOrgs as org (org.id)}
          <option value={org.id}>
            {org.name}{org.slug ? ` (@${org.slug})` : ''}{org.role ? ` · ${org.role}` : ''}
          </option>
        {/each}
      </select>
    </div>
    {/if}

    <!-- Body: Sidebar + Content -->
    <div class="flex-1 flex overflow-hidden min-h-0">
      <!-- File tree sidebar -->
      <div class="w-56 flex-shrink-0 border-r border-gray-200 overflow-y-auto bg-gray-50">
        <div class="px-3 py-2 text-[11px] font-semibold text-gray-400 uppercase tracking-wider">{$_('skills.files')}</div>
        <div class="px-1 pb-2">
          {#each fileTreeNodes as node (node.path || node.name)}
            <FileTreeNode
              {node}
              {selectedFilePath}
              {selectFile}
              {formatSize}
              {fileIconColor}
              depth={0}
            />
          {/each}
        </div>
      </div>

      <!-- Content viewer -->
      <div class="flex-1 overflow-hidden flex flex-col min-w-0">
        <div class="flex-shrink-0 px-4 py-2 border-b border-gray-100 bg-white flex items-center gap-2">
          <span class="text-[11px] font-semibold text-gray-400 uppercase tracking-wider">{$_('skills.content')}</span>
          {#if selectedFilePath}
            <span class="text-xs text-gray-600 font-mono truncate flex-1">{selectedFilePath}</span>
          {/if}
          {#if selectedFileLoading}
            <span class="text-xs text-gray-400">Loading...</span>
          {/if}
        </div>
        <div class="flex-1 overflow-y-auto p-4">
          {#if selectedFileLoading}
            <div class="flex items-center justify-center h-full">
              <LoadingSpinner text="Loading file..." />
            </div>
          {:else if selectedFilePath}
            <!-- Markdown preview for .md files -->
            {#if selectedFilePath.endsWith('.md') || selectedFilePath === 'SKILL.md'}
              <div class="prose prose-sm max-w-none">
                <!-- Strip frontmatter for preview display -->
                <pre class="whitespace-pre-wrap text-sm text-gray-700 bg-white p-0 font-mono text-[13px] leading-relaxed">{selectedFileContent}</pre>
              </div>
            {:else}
              <pre class="whitespace-pre-wrap text-sm text-gray-700 bg-gray-50 p-5 rounded-xl font-mono text-[13px] leading-relaxed border border-gray-200">{selectedFileContent}</pre>
            {/if}
          {:else}
            <div class="flex items-center justify-center h-full text-gray-400 text-sm">
              {$_('skills.selectFileToPreview')}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
  {/if}
</div>
{/if}

<!-- Publish Scope Selection Modal -->
{#if showScopeModal}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-[9999]" on:click={() => showScopeModal = false}>
    <div class="bg-white rounded-2xl shadow-xl max-w-md w-full mx-4 overflow-hidden" on:click|stopPropagation>
      <div class="px-6 py-4 border-b border-gray-100">
        <h3 class="text-lg font-semibold text-gray-900">Select Publish Scope</h3>
        <p class="text-sm text-gray-500 mt-1">Choose who can see this skill</p>
      </div>
      <div class="p-6">
        <div class="space-y-3">
          {#each scopeOptions as option}
            <label class="flex items-start gap-3 p-3 rounded-xl border cursor-pointer transition-all duration-200 hover:border-indigo-300 hover:bg-indigo-50/50 {selectedScope === option.value ? 'border-indigo-400 bg-indigo-50' : 'border-gray-200'}">
              <input
                type="radio"
                name="scope"
                value={option.value}
                bind:group={selectedScope}
                class="mt-0.5 w-4 h-4 text-indigo-600 border-gray-300 focus:ring-indigo-500"
              />
              <div class="flex-1">
                <span class="text-sm font-medium text-gray-900 capitalize">{option.value}</span>
                <p class="text-xs text-gray-500 mt-0.5">{option.label}</p>
              </div>
            </label>
          {/each}
        </div>
      </div>
      <div class="px-6 py-4 border-t border-gray-100 flex justify-end gap-3">
        <button
          on:click={() => showScopeModal = false}
          class="px-4 py-2 text-sm font-medium text-gray-600 hover:text-gray-800 transition-colors"
        >
          Cancel
        </button>
        <button
          on:click={confirmPublishSkill}
          class="px-4 py-2 text-sm font-medium bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
        >
          Publish
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Change Visibility Modal -->
{#if showVisibilityModal}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-[9999]" on:click={() => showVisibilityModal = false}>
    <div class="bg-white rounded-2xl shadow-xl max-w-md w-full mx-4 overflow-hidden" on:click|stopPropagation>
      <div class="px-6 py-4 border-b border-gray-100">
        <h3 class="text-lg font-semibold text-gray-900">Change Visibility</h3>
        <p class="text-sm text-gray-500 mt-1">Choose who can see this skill</p>
      </div>
      <div class="p-6">
        <div class="space-y-3">
          {#each visibilityOptions as option}
            <label class="flex items-start gap-3 p-3 rounded-xl border cursor-pointer transition-all duration-200 hover:border-indigo-300 hover:bg-indigo-50/50 {selectedVisibility === option.value ? 'border-indigo-400 bg-indigo-50' : 'border-gray-200'}">
              <input
                type="radio"
                name="visibility"
                value={option.value}
                bind:group={selectedVisibility}
                class="mt-0.5 w-4 h-4 text-indigo-600 border-gray-300 focus:ring-indigo-500"
              />
              <div class="flex-1">
                <span class="text-sm font-medium text-gray-900 capitalize">{option.value.replace('_', ' ')}</span>
                <p class="text-xs text-gray-500 mt-0.5">{option.label}</p>
              </div>
            </label>
          {/each}
        </div>
      </div>
      <div class="px-6 py-4 border-t border-gray-100 flex justify-end gap-3">
        <button
          on:click={() => showVisibilityModal = false}
          class="px-4 py-2 text-sm font-medium text-gray-600 hover:text-gray-800 transition-colors"
        >
          Cancel
        </button>
        <button
          on:click={confirmChangeVisibility}
          class="px-4 py-2 text-sm font-medium bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
        >
          Save
        </button>
      </div>
    </div>
  </div>
{/if}