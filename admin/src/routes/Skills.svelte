<script>
  import { onMount } from 'svelte';
  import { Link } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { isAdmin } from '../stores/auth.js';

  $: skillLinkBase = $isAdmin ? '/skills' : '/user/skills';
  import Badge from '../components/Badge.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import SortHeader from '../components/SortHeader.svelte';
  import FileTreeNode from '../components/FileTreeNode.svelte';

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

  onMount(() => {
    loadSkills();
  });

  async function loadSkills() {
    loading = true;
    error = '';
    try {
      const params = { page, page_size: pageSize };
      if (keyword.trim()) params.keyword = keyword.trim();
      if (tagFilter) params.tag = tagFilter;

      const res = await api.listSkills(params);
      skills = res.data || [];
      total = res.total || skills.length;

      if (allTags.length === 0 && skills.length > 0) {
        const tagsSet = new Set();
        skills.forEach(s => (s.tags || []).forEach(t => tagsSet.add(t)));
        allTags = [...tagsSet].sort();
      }
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
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

  function goToPage(p) {
    page = p;
    loadSkills();
  }

  function handleKeydown(e) {
    if (e.key === 'Enter') handleSearch();
  }

  // --- Create Modal ---
  function openCreateModal() {
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
    showCreateModal = true;
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
      const res = await api.confirmSkillUpload(previewId);
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
      <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">Skills</h1>
      <p class="text-gray-500 text-sm mt-1.5 font-medium">Browse and manage all registered skills</p>
    </div>
    <div class="flex items-center gap-3">
      <button
        on:click={openCreateModal}
        class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        New Skill
      </button>
      <span class="inline-flex items-center gap-1.5 px-3.5 py-2 bg-white text-blue-700 rounded-xl text-sm font-semibold ring-1 ring-sky-600/20">
        <span class="w-1.5 h-1.5 rounded-full bg-white0"></span>
        {total} total
      </span>
    </div>
  </div>

  <div class="flex flex-wrap items-center gap-3 mb-6">
    <div class="relative flex-1 min-w-[280px] max-w-md">
      <svg class="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
      </svg>
      <input
        type="text"
        bind:value={keyword}
        on:keydown={handleKeydown}
        placeholder="Search skills by name or description..."
        class="w-full pl-10 pr-4 py-2.5 bg-white border border-gray-200 rounded-lg text-sm text-gray-900 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 transition-all"
      />
    </div>

    <select
      bind:value={tagFilter}
      on:change={() => handleTagFilter(tagFilter)}
      aria-label="Filter by tag"
      class="px-4 py-2.5 bg-white border border-gray-200 rounded-lg text-sm text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500/20 cursor-pointer"
    >
      <option value="" disabled selected hidden>Filter by tag</option>
      <option value="">All tags</option>
      {#each allTags as tag}
        <option value={tag}>{tag}</option>
      {/each}
    </select>

    <button
      on:click={handleSearch}
      class="px-5 py-2.5 bg-blue-600 text-white rounded-lg text-sm font-semibold hover:bg-blue-700 transition-colors shadow-sm"
    >
      Search
    </button>

    {#if keyword || tagFilter}
      <button
        on:click={handleClearFilters}
        class="px-4 py-2.5 text-gray-500 hover:text-gray-700 text-sm font-medium transition-colors"
      >
        Clear filters
      </button>
    {/if}
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-red-50 border border-red-100 text-red-600 px-5 py-4 rounded-xl text-sm font-medium">{error}</div>
  {:else if skills.length === 0}
    <div class="bg-white rounded-xl border border-gray-200 shadow-card">
      <EmptyState message="No skills found" />
    </div>
  {:else}
    <div class="bg-white rounded-xl border border-gray-200 overflow-hidden shadow-card">
      <table class="w-full">
        <thead>
          <tr class="border-b border-gray-100 bg-gray-50">
            <th class="px-6 py-4 text-left"><SortHeader label="Name" sortKey="name" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
            <th class="px-6 py-4 text-left"><SortHeader label="Version" sortKey="version" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
            <th class="px-6 py-4 text-left"><SortHeader label="Status" sortKey="status" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
            <th class="px-6 py-4 text-left"><SortHeader label="Tags" /></th>
            <th class="px-6 py-4 text-left"><SortHeader label="Visibility" sortKey="visibility" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
            <th class="px-6 py-4 text-left"><SortHeader label="Author" sortKey="author_agent_id" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
            <th class="px-6 py-4 text-left"><SortHeader label="Installs" sortKey="install_count" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
            <th class="px-6 py-4 text-left"><SortHeader label="Created" sortKey="created" currentSort="{{key: sortKey, dir: sortDir}}" onSort={handleSort} /></th>
          </tr>
        </thead>
        <tbody>
          {#each sortedSkills as skill (skill.id)}
            <tr class="table-row hover:bg-gray-50">
              <td class="px-6 py-4">
                <Link to="{skillLinkBase}/{skill.id}" class="text-blue-600 hover:text-blue-700 font-semibold text-sm transition-colors">
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
              <td class="px-6 py-4">
                <span class="text-gray-500 text-xs capitalize">{skill.visibility || 'org_visible'}</span>
              </td>
              <td class="px-6 py-4 text-gray-500 text-xs">{skill.author_name || skill.author_agent_id || 'N/A'}</td>
              <td class="px-6 py-4">
                <span class="text-gray-600 text-sm font-semibold">{skill.install_count || 0}</span>
              </td>
              <td class="px-6 py-4 text-gray-500 text-sm">{skill.created ? new Date(skill.created).toLocaleDateString() : 'N/A'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if totalPages > 1}
      <div class="flex items-center justify-between mt-5 px-2">
        <span class="text-gray-500 text-sm">
          Page {page} of {totalPages} ({total} total)
        </span>
        <div class="flex gap-1.5">
          <button
            on:click={() => goToPage(page - 1)}
            disabled={page <= 1}
            class="px-3.5 py-2 bg-white border border-gray-200 rounded-lg text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
          >
            Previous
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
            Next
          </button>
        </div>
      </div>
    {/if}
  {/if}
</div>

{#if showCreateModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  {#if createStep === 'upload'}
  <!-- Step 1: Upload ZIP -->
  <div class="bg-white rounded-xl p-6 w-full max-w-lg shadow-elevated-lg border border-gray-200 modal-content">
    <h2 class="text-lg font-bold text-gray-900 mb-5">Upload Skill Package</h2>
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
        <LoadingSpinner text="Uploading & analyzing..." />
      {:else if uploadedFileName}
        <div class="flex items-center justify-center gap-3">
          <svg class="w-8 h-8 text-emerald-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
          <div class="text-left">
            <p class="text-sm font-semibold text-gray-700">{uploadedFileName}</p>
            <p class="text-xs text-gray-400">Processing...</p>
          </div>
        </div>
      {:else}
        <svg class="w-12 h-12 mx-auto text-gray-300 mb-3 {isDragging ? 'text-blue-400' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
        </svg>
        <p class="text-sm text-gray-500 font-medium">
          {isDragging ? 'Drop your ZIP here' : 'Drag & drop a .zip file here, or click to browse'}
        </p>
        <p class="text-xs text-gray-400 mt-1">Upload a ZIP package containing SKILL.md + optional files</p>
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
        Cancel
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
          Cancel
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
            Confirm Upload
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

    <!-- Body: Sidebar + Content -->
    <div class="flex-1 flex overflow-hidden min-h-0">
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
          <span class="text-[11px] font-semibold text-gray-400 uppercase tracking-wider">Content</span>
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
              Select a file from the sidebar to preview
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
  {/if}
</div>
{/if}