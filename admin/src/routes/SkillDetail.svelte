<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import Badge from '../components/Badge.svelte';
  import ReviewActions from '../components/ReviewActions.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import FileTreeNode from '../components/FileTreeNode.svelte';

  export let id;

  let skill = null;
  let stats = null;
  let loading = true;
  let error = '';

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
      const files = (res.files || []).sort((a, b) => {
        if (a === 'SKILL.md' || a.endsWith('/SKILL.md')) return -1;
        if (b === 'SKILL.md' || b.endsWith('/SKILL.md')) return 1;
        const aDir = a.includes('/');
        const bDir = b.includes('/');
        if (aDir !== bDir) return aDir ? -1 : 1;
        return a.localeCompare(b);
      });
      fileList = files.map(f => ({ path: f, size: 0 }));
      fileListLoaded = true;

      // Auto-select SKILL.md
      const skillMd = fileList.find(f => f.path === 'SKILL.md' || f.path.endsWith('/SKILL.md'));
      if (skillMd) {
        await selectFile(skillMd.path);
      }
    } catch (e) {
      // Fallback: show SKILL.md only
      fileList = [{ path: 'SKILL.md', size: 0 }];
      fileListLoaded = true;
      selectedFilePath = 'SKILL.md';
      selectedFileContent = skill?.content || '';
    }
  }

  async function selectFile(filePath) {
    if (selectedFilePath === filePath && fileFetchCache[filePath]) {
      selectedFileContent = fileFetchCache[filePath];
      return;
    }

    if (fileFetchCache[filePath]) {
      selectedFilePath = filePath;
      selectedFileContent = fileFetchCache[filePath];
      return;
    }

    selectedFilePath = filePath;
    selectedFileLoading = true;
    try {
      const res = await api.getSkillFile(id, filePath);
      selectedFileContent = res.content;
      fileFetchCache[filePath] = res.content;
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

  $: fileTreeNodes = buildFileTree(fileList);
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
            </div>
            <p class="text-gray-400 text-sm font-medium">
              v{skill.version || '1.0.0'} · {fileList.length} files · Skill details and statistics
            </p>
          </div>
        </div>
        {#if skill.status === 'pending_review'}
          <ReviewActions {skill} />
        {/if}
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
          {#if !editingTags}
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
              <button
                on:click={startEditTags}
                class="text-xs font-medium text-blue-600 hover:text-blue-700 transition-colors"
              >
                + Add tags
              </button>
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
