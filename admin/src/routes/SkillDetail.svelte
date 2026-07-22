<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { auth, isAdmin } from '../stores/auth.js';
  import { hasPermission, permissionStore, isAnyAdmin } from '../stores/permission.js';
  import { useLocation } from 'svelte-routing';
  import Badge from '../components/Badge.svelte';
  import ReviewActions from '../components/ReviewActions.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import FileTreeNode from '../components/FileTreeNode.svelte';

  export let id;

  let skill = null;
  let stats = null;
  let loading = true;
  let error = '';
  let publishLoading = false;
  let unpublishLoading = false;
  let submitLoading = false;
  let marketplaceLoading = false;
  let requestDelistLoading = false;

  const location = useLocation();

  // 从市场进入的详情页强制只读，不允许编辑
  $: isMarketplaceView = $location.state?.readonly === true;

  // ========== 角色判断 ==========
  $: isSuperAdmin = ($permissionStore.systemRoles || []).includes('super_admin');
  $: isMarketAdmin = ($permissionStore.systemRoles || []).some(r => r === 'marketplace_admin' || r === 'marketplace_reviewer');
  $: isMarketAdminOnly = ($permissionStore.systemRoles || []).includes('marketplace_admin');
  $: isMarketReviewer = ($permissionStore.systemRoles || []).includes('marketplace_reviewer');
  // 管理员（super_admin / tenant_admin / marketplace_admin / marketplace_reviewer）
  $: isAnyAdminUser = $isAdmin || isAnyAdmin();

  // 当前用户是否能编辑此 skill（tags / description / content）
  // - 从市场进入 → 始终不可编辑
  // - super_admin / tenant_admin → 可编辑任意 Skill
  // - marketplace_admin / marketplace_reviewer → 可编辑任意已提交市场的 Skill
  // - 个人 Skill owner → 可编辑
  // - 组织 owner/admin/developer → 对该组织的 skill 可编辑
  $: canEdit = !isMarketplaceView && ($isAdmin || isSuperAdmin || isMarketAdmin || (skill && (
    skill.owner_type === 'user' ||
    skill.author_name === $auth.username ||
    skill.author_agent_id === $auth.username ||
    (skill.owner_type === 'organization' && skill.owner_id &&
      ($permissionStore.orgRoles || []).some(r => r.org_id === skill.owner_id && ['owner', 'admin', 'developer'].includes(r.role)))
  )));

  // Skill 作者（owner）判断 — 用于作者专属操作（上传新版本、申请下架等）
  $: isOwner = skill && (
    skill.owner_type === 'user' ||
    skill.author_name === $auth.username ||
    skill.author_agent_id === $auth.username ||
    (skill.owner_type === 'organization' && skill.owner_id &&
      ($permissionStore.orgRoles || []).some(r => r.org_id === skill.owner_id && ['owner', 'admin', 'developer'].includes(r.role)))
  );

  // Tag editing state
  let editingTags = false;
  let editTags = [];
  let tagInput = '';
  let tagSaveLoading = false;
  let tagSaveError = '';

  // File tree state
  let fileList = [];
  let selectedFilePath = '';
  let selectedFileContent = '';
  let selectedFileLoading = false;
  let fileFetchCache = {};
  let fileListLoaded = false;

  onMount(async () => {
    try {
      const [skillRes, statsRes] = await Promise.all([
        api.getSkill(id),
        api.getSkillStats(id)
      ]);
      const detail = skillRes;
      skill = { ...detail.metadata, content: detail.content };
      stats = detail.stats || (statsRes.data || statsRes);

      // Load file list
      loadFileList();
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  });

  async function loadFileList() {
    try {
      const res = await api.getSkillFiles(id);
      const rawFiles = res.files || [];
      // 后端偶尔会返回带转义引号的路径，先归一化，保留原始路径用于获取文件内容
      const normalized = rawFiles.map(f => ({
        original: f,
        display: normalizeFilePath(f)
      }));
      let files = normalized.map(n => ({
        path: n.display,
        originalPath: n.original,
        size: 0
      }));
      files = files.sort((a, b) => {
        if (a.path === 'SKILL.md' || a.path.endsWith('/SKILL.md')) return -1;
        if (b.path === 'SKILL.md' || b.path.endsWith('/SKILL.md')) return 1;
        const aDir = a.path.includes('/');
        const bDir = b.path.includes('/');
        if (aDir !== bDir) return aDir ? -1 : 1;
        return a.path.localeCompare(b.path);
      });
      fileList = files;
      fileListLoaded = true;

      // Auto-select SKILL.md
      const skillMd = fileList.find(f => f.path === 'SKILL.md' || f.path.endsWith('/SKILL.md'));
      if (skillMd) {
        await selectFile(skillMd.path, skillMd.originalPath);
      }
    } catch (e) {
      // Fallback: show SKILL.md only
      fileList = [{ path: 'SKILL.md', originalPath: 'SKILL.md', size: 0 }];
      fileListLoaded = true;
      selectedFilePath = 'SKILL.md';
      selectedFileContent = skill?.content || '';
    }
  }

  function normalizeFilePath(p) {
    if (!p) return p;
    let s = p.trim();
    // 后端返回的路径有时被多余的双引号包裹，移除它们
    if (s.length >= 2 && s.startsWith('"') && s.endsWith('"')) {
      s = s.slice(1, -1);
    }
    return s;
  }


  async function selectFile(displayPath, originalPath) {
    const fetchPath = originalPath || displayPath;
    if (selectedFilePath === displayPath && fileFetchCache[fetchPath]) {
      selectedFileContent = fileFetchCache[fetchPath];
      return;
    }

    if (fileFetchCache[fetchPath]) {
      selectedFilePath = displayPath;
      selectedFileContent = fileFetchCache[fetchPath];
      return;
    }

    selectedFilePath = displayPath;
    selectedFileLoading = true;
    try {
      const res = await api.getSkillFile(id, fetchPath);
      selectedFileContent = res.content;
      fileFetchCache[fetchPath] = res.content;
    } catch (e) {
      selectedFileContent = `Error loading file: ${e.message}`;
    } finally {
      selectedFileLoading = false;
    }
  }

  function formatSize(bytes) {
    if (!bytes || bytes === 0) return '';
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

  function startEditTags() {
    editTags = [...(skill.tags || [])];
    tagInput = '';
    tagSaveError = '';
    editingTags = true;
  }

  function cancelEditTags() {
    editingTags = false;
    tagInput = '';
    tagSaveError = '';
  }

  function addTag() {
    const raw = tagInput.trim();
    if (!raw) return;
    // Validate: 1-50 chars, alphanumeric + hyphens/underscores
    if (raw.length > 50) {
      tagSaveError = 'Tag must be 50 characters or less';
      return;
    }
    if (!/^[\p{L}\p{N}_-]+$/u.test(raw)) {
      tagSaveError = 'Tags only allow letters, numbers, hyphens and underscores';
      return;
    }
    if (editTags.length >= 10) {
      tagSaveError = 'Maximum 10 tags';
      return;
    }
    if (editTags.includes(raw)) {
      tagSaveError = 'Duplicate tag';
      return;
    }
    editTags = [...editTags, raw];
    tagInput = '';
    tagSaveError = '';
  }

  function handleTagKeydown(e) {
    if (e.key === 'Enter') {
      e.preventDefault();
      addTag();
    }
    if (e.key === 'Backspace' && !tagInput && editTags.length > 0) {
      editTags = editTags.slice(0, -1);
    }
  }

  function removeTag(index) {
    editTags = editTags.filter((_, i) => i !== index);
  }

  async function saveTags() {
    // 若输入框有未提交的标签文本，先尝试自动添加到列表
    if (tagInput.trim()) {
      addTag();
      if (tagSaveError) return; // addTag 校验失败则终止
    }
    if (editTags.length === 0) {
      tagSaveError = 'At least one tag is required';
      return;
    }
    tagSaveLoading = true;
    tagSaveError = '';
    try {
      await api.updateSkill(skill.id, { tags: editTags });
      skill.tags = [...editTags];
      editingTags = false;
    } catch (e) {
      tagSaveError = e.message;
    } finally {
      tagSaveLoading = false;
    }
  }

  async function handlePublish() {
    publishLoading = true;
    try {
      if ($isAdmin) {
        // Admin: force publish to marketplace (bypasses review)
        await api.adminPublishSkill(id);
        addToast(`${skill.name} listed on marketplace`, 'success');
        skill.marketplace_status = 'listed';
        skill.visibility = 'marketplace';
      } else {
        // User: internal publish only, doesn't affect marketplace
        await api.publishSkill(id);
        addToast(`${skill.name} published`, 'success');
        skill.status = 'published';
      }
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      publishLoading = false;
    }
  }

  async function handleUnpublish() {
    unpublishLoading = true;
    try {
      await api.adminUnpublishSkill(id);
      addToast(`${skill.name} delisted from marketplace`, 'success');
      skill.marketplace_status = 'delisted';
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      unpublishLoading = false;
    }
  }

  async function handleRequestDelist() {
    const reason = prompt('请输入申请下架的原因（可选）：');
    if (reason === null) return; // user cancelled
    requestDelistLoading = true;
    try {
      await api.requestMarketplaceDelist(id, reason || undefined);
      addToast(`${skill.name} 下架申请已提交，等待审核`, 'success');
      skill.marketplace_status = 'pending_delist';
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      requestDelistLoading = false;
    }
  }

  async function handleSubmitToMarketplace() {
    marketplaceLoading = true;
    try {
      await api.submitToMarketplace(id);
      addToast(`${skill.name} submitted to marketplace review`, 'success');
      skill.marketplace_status = 'pending_review';
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      marketplaceLoading = false;
    }
  }

  async function handleMarketplaceApprove() {
    marketplaceLoading = true;
    try {
      await api.marketplaceReviewApprove(id);
      addToast(`${skill.name} approved for marketplace`, 'success');
      skill.marketplace_status = 'listed';
      skill.visibility = 'marketplace';
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      marketplaceLoading = false;
    }
  }

  async function handleMarketplaceReject() {
    marketplaceLoading = true;
    try {
      await api.marketplaceReviewReject(id);
      addToast(`${skill.name} rejected from marketplace`, 'success');
      skill.marketplace_status = 'rejected';
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      marketplaceLoading = false;
    }
  }

  async function handleCancelUpdate() {
    if (!confirm('确定要取消此次更新吗？草稿内容将被丢弃。')) return;
    marketplaceLoading = true;
    try {
      await api.cancelUpdate(id);
      addToast(`${skill.name} 更新已取消`, 'success');
      skill.marketplace_status = 'listed';
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      marketplaceLoading = false;
    }
  }

  // --- Upload New Version ---
  let uploadVersionLoading = false;

  async function handleUploadVersion() {
    // 创建隐藏的 file input
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.zip';
    input.onchange = async (e) => {
      const file = e.target.files?.[0];
      if (!file) return;
      if (!file.name.endsWith('.zip')) {
        addToast('请选择 .zip 文件', 'error');
        return;
      }
      uploadVersionLoading = true;
      try {
        const res = await api.uploadSkill(file);
        addToast(res.message || '新版本上传成功，已提交审核', 'success');
        // 刷新页面
        window.location.reload();
      } catch (e) {
        addToast(e.message, 'error');
      } finally {
        uploadVersionLoading = false;
      }
    };
    input.click();
  }

  async function handleRelist() {
    marketplaceLoading = true;
    try {
      await api.marketplaceRelist(id);
      addToast(`${skill.name} relisted on marketplace`, 'success');
      skill.marketplace_status = 'listed';
      skill.visibility = 'marketplace';
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      marketplaceLoading = false;
    }
  }

  async function handleSubmitReview() {
    submitLoading = true;
    try {
      await api.submitSkillForReview(id);
      addToast(`${skill.name} 已提交审核`, 'success');
      skill.status = 'pending_review';
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      submitLoading = false;
    }
  }

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
            originalPath: f.originalPath,
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

  $: fileTreeNodes = buildFileTree(fileList);

  function marketplaceLabel(status) {
    const labels = {
      pending_review: 'Market Review',
      pending_delist: 'Delist Review',
      listed: 'Listed',
      rejected: 'Rejected',
      delisted: 'Delisted',
    };
    return labels[status] || status;
  }

  function marketplaceColor(status) {
    const colors = {
      pending_review: 'bg-amber-50 text-amber-600 ring-1 ring-amber-600/20',
      pending_delist: 'bg-orange-50 text-orange-600 ring-1 ring-orange-600/20',
      listed: 'bg-blue-50 text-blue-600 ring-1 ring-blue-600/20',
      rejected: 'bg-rose-50 text-rose-600 ring-1 ring-rose-600/20',
      delisted: 'bg-gray-100 text-gray-500 ring-1 ring-gray-600/20',
    };
    return colors[status] || '';
  }

  function marketplaceDotColor(status) {
    return status === 'listed' ? 'bg-blue-500 pulse-dot' : status === 'pending_review' || status === 'pending_delist' ? 'bg-amber-500 pulse-dot' : 'bg-gray-400';
  }
</script>

<div class="p-8">
  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if skill}
    <div class="page-header">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
          <div class="w-12 h-12 rounded-2xl gradient-brand flex items-center justify-center font-bold text-lg shadow-glow">
            {skill.name[0]?.toUpperCase() || 'S'}
          </div>
          <div>
            <div class="flex items-center gap-3 mb-1">
              <h1 class="text-[28px] font-extrabold text-gray-900 tracking-tight">{skill.name}</h1>
              <Badge status={skill.status} />
              {#if skill.marketplace_status}
                <span class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-semibold rounded-full {marketplaceColor(skill.marketplace_status)}">
                  <span class="w-1.5 h-1.5 rounded-full {marketplaceDotColor(skill.marketplace_status)}"></span>
                  {marketplaceLabel(skill.marketplace_status)}
                </span>
              {/if}
            </div>
            <p class="text-gray-400 text-sm font-medium">
              v{skill.version || '1.0.0'} · {fileList.length} files · Skill details and statistics
            </p>
          </div>
        </div>
        {#if canEdit && skill.status === 'pending_review'}
          <ReviewActions {skill} />
        {/if}

        <!-- ========== 操作按钮区（右对齐） ========== -->
        <div class="flex items-center gap-2 ml-auto flex-shrink-0">

        <!-- 1. 作者专属操作（非管理员，非市场管理员） -->
        {#if isOwner && !isAnyAdminUser}
          {#if skill.status === 'draft' || skill.status === 'rejected'}
            <button on:click={handleSubmitReview} disabled={submitLoading}
              class="px-4 py-2 text-sm font-semibold bg-amber-500 text-white rounded-xl hover:bg-amber-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-amber-500/20 hover:shadow-md hover:shadow-amber-500/30 active:scale-[0.97]">
              {#if submitLoading}<svg class="w-4 h-4 animate-spin mr-1 inline" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>{/if}
              Submit for Review
            </button>
          {/if}
          {#if skill.status === 'approved'}
            <button on:click={handlePublish} disabled={publishLoading}
              class="px-4 py-2 text-sm font-semibold bg-emerald-500 text-white rounded-xl hover:bg-emerald-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-emerald-500/20 hover:shadow-md hover:shadow-emerald-500/30 active:scale-[0.97]">
              {#if publishLoading}<svg class="w-4 h-4 animate-spin mr-1 inline" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>{/if}
              Publish
            </button>
          {/if}
          {#if skill.status === 'published' && (!skill.marketplace_status || skill.marketplace_status === 'rejected' || skill.marketplace_status === 'delisted')}
            <button on:click={handleSubmitToMarketplace} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-blue-500 text-white rounded-xl hover:bg-blue-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-blue-500/20 hover:shadow-md hover:shadow-blue-500/30 active:scale-[0.97]">
              {#if marketplaceLoading}<svg class="w-4 h-4 animate-spin mr-1 inline" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>{/if}
              Submit to Marketplace
            </button>
          {/if}
          {#if skill.marketplace_status === 'pending_review'}
            <span class="px-4 py-2 text-sm font-semibold bg-amber-50 text-amber-600 rounded-xl ring-1 ring-amber-600/20">Awaiting Market Review</span>
          {/if}
          {#if skill.marketplace_status === 'listed'}
            <button on:click={handleRequestDelist} disabled={requestDelistLoading}
              class="px-4 py-2 text-sm font-semibold bg-rose-500 text-white rounded-xl hover:bg-rose-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-rose-500/20 hover:shadow-md hover:shadow-rose-500/30 active:scale-[0.97]">
              {#if requestDelistLoading}<svg class="w-4 h-4 animate-spin mr-1 inline" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>{/if}
              申请下架
            </button>
          {/if}
          {#if skill.status !== 'draft'}
            <button on:click={handleUploadVersion} disabled={uploadVersionLoading}
              class="px-4 py-2 text-sm font-semibold bg-indigo-500 text-white rounded-xl hover:bg-indigo-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-indigo-500/20 hover:shadow-md hover:shadow-indigo-500/30 active:scale-[0.97]">
              {#if uploadVersionLoading}<svg class="w-4 h-4 animate-spin mr-1 inline" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>{/if}
              上传新版本
            </button>
          {/if}
          {#if skill.marketplace_status === 'pending_update'}
            <span class="px-4 py-2 text-sm font-semibold bg-purple-50 text-purple-600 rounded-xl ring-1 ring-purple-600/20">Update Pending Review</span>
            <button on:click={handleCancelUpdate} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold text-gray-500 border border-gray-300 rounded-xl hover:bg-gray-100 disabled:opacity-50 transition-all duration-200 active:scale-[0.97]">取消更新</button>
          {/if}
          {#if skill.marketplace_status === 'pending_delist'}
            <span class="px-4 py-2 text-sm font-semibold bg-orange-50 text-orange-600 rounded-xl ring-1 ring-orange-600/20">Delist Request Pending</span>
          {/if}
        {/if}

        <!-- 2. 市场管理员/审核员操作（marketplace_admin / marketplace_reviewer） -->
        {#if isMarketAdmin && !isSuperAdmin && !$isAdmin}
          {#if skill.marketplace_status === 'pending_review'}
            <button on:click={handleMarketplaceApprove} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-emerald-500 text-white rounded-xl hover:bg-emerald-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-emerald-500/20 hover:shadow-md hover:shadow-emerald-500/30 active:scale-[0.97]">通过</button>
            <button on:click={handleMarketplaceReject} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-rose-500 text-white rounded-xl hover:bg-rose-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-rose-500/20 hover:shadow-md hover:shadow-rose-500/30 active:scale-[0.97]">驳回</button>
          {/if}
          {#if skill.marketplace_status === 'pending_update'}
            <button on:click={() => { api.marketplaceApproveUpdate(id).then(() => { addToast('更新已批准', 'success'); skill.marketplace_status = 'listed'; }); }} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-emerald-500 text-white rounded-xl hover:bg-emerald-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-emerald-500/20 hover:shadow-md hover:shadow-emerald-500/30 active:scale-[0.97]">批准更新</button>
            <button on:click={() => { api.marketplaceRejectUpdate(id).then(() => { addToast('更新已驳回', 'success'); skill.marketplace_status = 'listed'; }); }} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-rose-500 text-white rounded-xl hover:bg-rose-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-rose-500/20 hover:shadow-md hover:shadow-rose-500/30 active:scale-[0.97]">驳回</button>
          {/if}
          {#if skill.marketplace_status === 'pending_delist'}
            <button on:click={() => { api.marketplaceApproveDelist(id).then(() => { addToast('下架已批准', 'success'); skill.marketplace_status = 'delisted'; }); }} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-emerald-500 text-white rounded-xl hover:bg-emerald-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-emerald-500/20 hover:shadow-md hover:shadow-emerald-500/30 active:scale-[0.97]">批准下架</button>
            <button on:click={() => { api.marketplaceRejectDelist(id).then(() => { addToast('下架已驳回', 'success'); skill.marketplace_status = 'listed'; }); }} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-rose-500 text-white rounded-xl hover:bg-rose-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-rose-500/20 hover:shadow-md hover:shadow-rose-500/30 active:scale-[0.97]">驳回</button>
          {/if}
          {#if skill.marketplace_status === 'listed'}
            <button on:click={handleUnpublish} disabled={unpublishLoading}
              class="px-4 py-2 text-sm font-semibold bg-amber-500 text-white rounded-xl hover:bg-amber-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-amber-500/20 hover:shadow-md hover:shadow-amber-500/30 active:scale-[0.97]">下架</button>
          {/if}
        {/if}

        <!-- 3. super_admin / tenant_admin 操作（全部管理权限） -->
        {#if $isAdmin || isSuperAdmin}
          {#if skill.marketplace_status === 'pending_review'}
            <button on:click={handleMarketplaceApprove} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-emerald-500 text-white rounded-xl hover:bg-emerald-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-emerald-500/20 hover:shadow-md hover:shadow-emerald-500/30 active:scale-[0.97]">通过</button>
            <button on:click={handleMarketplaceReject} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-rose-500 text-white rounded-xl hover:bg-rose-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-rose-500/20 hover:shadow-md hover:shadow-rose-500/30 active:scale-[0.97]">驳回</button>
          {/if}
          {#if skill.marketplace_status === 'listed'}
            <button on:click={handleUnpublish} disabled={unpublishLoading}
              class="px-4 py-2 text-sm font-semibold bg-amber-500 text-white rounded-xl hover:bg-amber-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-amber-500/20 hover:shadow-md hover:shadow-amber-500/30 active:scale-[0.97]">下架</button>
          {/if}
          {#if skill.marketplace_status === 'delisted'}
            <button on:click={handleRelist} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-emerald-500 text-white rounded-xl hover:bg-emerald-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-emerald-500/20 hover:shadow-md hover:shadow-emerald-500/30 active:scale-[0.97]">重新上架</button>
          {/if}
          {#if (!skill.marketplace_status || skill.marketplace_status === 'rejected') && skill.status !== 'pending_review'}
            <button on:click={handlePublish} disabled={publishLoading}
              class="px-4 py-2 text-sm font-semibold bg-emerald-500 text-white rounded-xl hover:bg-emerald-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-emerald-500/20 hover:shadow-md hover:shadow-emerald-500/30 active:scale-[0.97]">上架</button>
          {/if}
          {#if skill.marketplace_status === 'pending_update'}
            <button on:click={() => { api.marketplaceApproveUpdate(id).then(() => { addToast('更新已批准', 'success'); skill.marketplace_status = 'listed'; }); }} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-emerald-500 text-white rounded-xl hover:bg-emerald-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-emerald-500/20 hover:shadow-md hover:shadow-emerald-500/30 active:scale-[0.97]">批准更新</button>
            <button on:click={() => { api.marketplaceRejectUpdate(id).then(() => { addToast('更新已驳回', 'success'); skill.marketplace_status = 'listed'; }); }} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-rose-500 text-white rounded-xl hover:bg-rose-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-rose-500/20 hover:shadow-md hover:shadow-rose-500/30 active:scale-[0.97]">驳回</button>
          {/if}
          {#if skill.marketplace_status === 'pending_delist'}
            <button on:click={() => { api.marketplaceApproveDelist(id).then(() => { addToast('下架已批准', 'success'); skill.marketplace_status = 'delisted'; }); }} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-emerald-500 text-white rounded-xl hover:bg-emerald-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-emerald-500/20 hover:shadow-md hover:shadow-emerald-500/30 active:scale-[0.97]">批准下架</button>
            <button on:click={() => { api.marketplaceRejectDelist(id).then(() => { addToast('下架已驳回', 'success'); skill.marketplace_status = 'listed'; }); }} disabled={marketplaceLoading}
              class="px-4 py-2 text-sm font-semibold bg-rose-500 text-white rounded-xl hover:bg-rose-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-rose-500/20 hover:shadow-md hover:shadow-rose-500/30 active:scale-[0.97]">驳回</button>
          {/if}
        {/if}

        </div>
      </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
      <div class="bg-white rounded-2xl border border-gray-200 p-5 card">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 rounded-xl bg-blue-100 flex items-center justify-center">
            <svg class="w-4 h-4 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
            </svg>
          </div>
          <span class="text-gray-500 text-[11px] font-semibold uppercase tracking-wider">Installs</span>
        </div>
        <p class="text-[28px] font-extrabold text-blue-600 stat-number">{skill?.install_count || 0}</p>
      </div>

      <div class="bg-white rounded-2xl border border-gray-200 p-5 card">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 rounded-xl bg-purple-100 flex items-center justify-center">
            <svg class="w-4 h-4 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/>
            </svg>
          </div>
          <span class="text-gray-500 text-[11px] font-semibold uppercase tracking-wider">Evaluations</span>
        </div>
        <p class="text-[28px] font-extrabold text-purple-600 stat-number">{stats?.total_evaluations || 0}</p>
      </div>

      <div class="bg-white rounded-2xl border border-gray-200 p-5 card">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 rounded-xl bg-emerald-100 flex items-center justify-center">
            <svg class="w-4 h-4 text-emerald-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
            </svg>
          </div>
          <span class="text-gray-500 text-[11px] font-semibold uppercase tracking-wider">Success Rate</span>
        </div>
        <p class="text-[28px] font-extrabold text-emerald-600 stat-number">{((stats?.success_rate || 0) * 100).toFixed(1)}%</p>
      </div>

      <div class="bg-white rounded-2xl border border-gray-200 p-5 card">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 rounded-xl bg-amber-100 flex items-center justify-center">
            <svg class="w-4 h-4 text-amber-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"/>
            </svg>
          </div>
          <span class="text-gray-500 text-[11px] font-semibold uppercase tracking-wider">Confidence</span>
        </div>
        <p class="text-[28px] font-extrabold text-amber-600 stat-number">{(stats?.confidence || 0).toFixed(2)}</p>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-5 mb-5">
      <div class="bg-white rounded-2xl border border-gray-200 shadow-card">
        <div class="px-6 py-4 border-b border-gray-200">
          <h2 class="font-semibold text-gray-800 text-sm">Description</h2>
        </div>
        <div class="p-6">
          <p class="text-gray-600 text-sm leading-relaxed">{skill.description || 'No description'}</p>
        </div>
      </div>

      <div class="bg-white rounded-2xl border border-gray-200 shadow-card">
        <div class="px-6 py-4 border-b border-gray-200 flex items-center justify-between">
          <h2 class="font-semibold text-gray-800 text-sm">Tags</h2>
          {#if !editingTags && canEdit}
            <button
              on:click={startEditTags}
              class="text-xs font-medium text-blue-600 hover:text-blue-700 transition-colors"
            >
              Edit
            </button>
          {/if}
        </div>
        <div class="p-6">
          {#if editingTags}
            <div class="space-y-3">
              <div class="flex flex-wrap gap-2">
                {#each editTags as tag, i}
                  <span class="inline-flex items-center gap-1 px-2.5 py-1 bg-blue-50 text-blue-700 text-xs font-medium rounded-lg border border-blue-200">
                    {tag}
                    <button
                      on:click={() => removeTag(i)}
                      class="w-4 h-4 inline-flex items-center justify-center rounded-full hover:bg-blue-200 transition-colors text-blue-500 hover:text-blue-700"
                      type="button"
                    >&times;</button>
                  </span>
                {/each}
              </div>
              <div class="flex items-center gap-2">
                <input
                  type="text"
                  bind:value={tagInput}
                  on:keydown={handleTagKeydown}
                  placeholder="Type tag and press Enter..."
                  maxlength="50"
                  class="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm text-gray-700 placeholder-gray-400 input-focus outline-none"
                />
                <button
                  on:click={addTag}
                  disabled={!tagInput.trim()}
                  class="px-3 py-2 text-xs font-semibold text-blue-600 bg-blue-50 hover:bg-blue-100 rounded-lg transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                  type="button"
                >Add</button>
              </div>
              {#if tagSaveError}
                <p class="text-rose-500 text-xs">{tagSaveError}</p>
              {/if}
              <div class="flex items-center gap-2 pt-1">
                <button
                  on:click={saveTags}
                  disabled={tagSaveLoading}
                  class="px-4 py-2 text-xs font-semibold text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors disabled:opacity-50"
                  type="button"
                >{tagSaveLoading ? 'Saving...' : 'Save'}</button>
                <button
                  on:click={cancelEditTags}
                  disabled={tagSaveLoading}
                  class="px-4 py-2 text-xs font-semibold text-gray-600 bg-gray-100 hover:bg-gray-200 rounded-lg transition-colors"
                  type="button"
                >Cancel</button>
              </div>
            </div>
          {:else if (skill.tags || []).length > 0}
            <div class="flex gap-2 flex-wrap">
              {#each skill.tags as tag}
                <span class="px-3 py-1.5 bg-slate-100 text-gray-600 text-xs font-medium rounded-lg border border-gray-200">
                  {tag}
                </span>
              {/each}
            </div>
          {:else}
            <div class="text-center">
              <p class="text-gray-400 text-sm mb-3">No tags</p>
              {#if canEdit}
              <button
                on:click={startEditTags}
                class="text-xs font-medium text-blue-600 hover:text-blue-700 transition-colors"
              >
                + Add tags
              </button>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    </div>

    <!-- File Browser -->
    <div class="bg-white rounded-2xl border border-gray-200 shadow-card overflow-hidden">
      <div class="px-6 py-4 border-b border-gray-200 flex items-center justify-between">
        <h2 class="font-semibold text-gray-800 text-sm">Files</h2>
        <span class="text-xs text-gray-400">{fileList.length} files</span>
      </div>
      <div class="flex" style="height: 480px;">
        <!-- File tree sidebar -->
        <div class="w-56 flex-shrink-0 border-r border-gray-200 overflow-y-auto bg-gray-50">
          <div class="px-3 py-2 text-[11px] font-semibold text-gray-400 uppercase tracking-wider">Files</div>
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
            <span class="text-[11px] font-semibold text-gray-400 uppercase tracking-wider">Preview</span>
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
              <pre class="whitespace-pre-wrap text-sm text-gray-700 bg-gray-50 p-5 rounded-xl font-mono text-[13px] leading-relaxed border border-gray-200">{selectedFileContent}</pre>
            {:else}
              <div class="flex items-center justify-center h-full text-gray-400 text-sm">
                Select a file from the sidebar to preview
              </div>
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
