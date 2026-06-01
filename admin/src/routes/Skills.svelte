<script>
  import { onMount } from 'svelte';
  import { Link, navigate } from 'svelte-routing';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import Badge from '../components/Badge.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

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

  let showCreateModal = false;
  let createForm = { name: '', description: '', tags: '', version: '1.0.0', visibility: 'org_visible', content: '' };
  let creating = false;
  let uploadedFileName = '';

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

  function openCreateModal() {
    createForm = { name: '', description: '', tags: '', version: '1.0.0', visibility: 'org_visible', content: '' };
    uploadedFileName = '';
    showCreateModal = true;
  }

  function closeCreateModal() {
    showCreateModal = false;
  }

  function parseFrontmatter(md) {
    const fmMatch = md.match(/^---\s*\n([\s\S]*?)\n---\s*\n([\s\S]*)$/);
    if (!fmMatch) return { metadata: {}, content: md };

    const fmText = fmMatch[1];
    const content = fmMatch[2];
    const metadata = {};

    const lines = fmText.split('\n');
    let currentKey = '';
    let inList = false;
    let listValues = [];

    for (const line of lines) {
      if (/^\s*-\s+/.test(line)) {
        inList = true;
        listValues.push(line.replace(/^\s*-\s+/, '').trim());
        continue;
      }
      if (inList && currentKey) {
        metadata[currentKey] = listValues;
        inList = false;
        listValues = [];
      }
      const kvMatch = line.match(/^(\w[\w-]*)\s*:\s*(.*)/);
      if (kvMatch) {
        currentKey = kvMatch[1];
        const val = kvMatch[2].trim();
        if (val !== '') {
          metadata[currentKey] = val.replace(/^["']|["']$/g, '');
          inList = false;
        } else {
          inList = true;
          listValues = [];
        }
      }
    }
    if (inList && currentKey) {
      metadata[currentKey] = listValues;
    }

    return { metadata, content };
  }

  function handleFileUpload(e) {
    const file = e.target.files?.[0];
    if (!file) return;

    uploadedFileName = file.name;
    const reader = new FileReader();
    reader.onload = (ev) => {
      const text = ev.target.result;
      const { metadata, content } = parseFrontmatter(text);
      createForm.name = metadata.name || '';
      createForm.description = metadata.description || '';
      createForm.version = metadata.version || '1.0.0';
      createForm.visibility = metadata.visibility || 'org_visible';
      createForm.tags = Array.isArray(metadata.tags) ? metadata.tags.join(', ') : (metadata.tags || '');
      createForm.content = content.trim();
    };
    reader.readAsText(file);
    e.target.value = '';
  }

  function handleClearFile() {
    uploadedFileName = '';
    createForm.content = '';
  }

  async function handleCreate() {
    if (!createForm.name.trim()) {
      addToast('Name is required', 'error');
      return;
    }
    if (!createForm.content.trim()) {
      addToast('Content is required', 'error');
      return;
    }

    creating = true;
    try {
      const body = {
        name: createForm.name.trim(),
        description: createForm.description.trim(),
        tags: createForm.tags.split(',').map(t => t.trim()).filter(Boolean),
        content: createForm.content.trim(),
        version: createForm.version.trim() || '1.0.0',
        visibility: createForm.visibility
      };
      const res = await api.createSkill(body);
      addToast(res.message || 'Skill created successfully', 'success');
      closeCreateModal();
      await loadSkills();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      creating = false;
    }
  }

  $: totalPages = Math.max(1, Math.ceil(total / pageSize));
</script>

<div class="p-8">
  <div class="page-header flex items-center justify-between">
    <div>
      <h1 class="text-[28px] font-extrabold text-surface-800 tracking-tight">Skills</h1>
      <p class="text-surface-500 text-sm mt-1.5 font-medium">Browse and manage all registered skills</p>
    </div>
    <div class="flex items-center gap-3">
      <button
        on:click={openCreateModal}
        class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        New Skill
      </button>
      <span class="inline-flex items-center gap-1.5 px-3.5 py-2 bg-sky-50 text-sky-700 rounded-xl text-sm font-semibold ring-1 ring-sky-600/20">
        <span class="w-1.5 h-1.5 rounded-full bg-sky-500"></span>
        {total} total
      </span>
    </div>
  </div>

  <div class="flex flex-wrap items-center gap-3 mb-6">
    <div class="relative flex-1 min-w-[280px] max-w-md">
      <svg class="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
      </svg>
      <input
        type="text"
        bind:value={keyword}
        on:keydown={handleKeydown}
        placeholder="Search skills by name or description..."
        class="w-full pl-10 pr-4 py-2.5 bg-sky-50 border border-indigo-200 rounded-xl text-sm text-surface-800 placeholder-surface-400 focus:outline-none focus:ring-2 focus:ring-brand-500/30 focus:border-brand-400 transition-all"
      />
    </div>

    <select
      bind:value={tagFilter}
      on:change={() => handleTagFilter(tagFilter)}
      class="px-4 py-2.5 bg-sky-50 border border-indigo-200 rounded-xl text-sm text-surface-700 focus:outline-none focus:ring-2 focus:ring-brand-500/30 cursor-pointer"
    >
      <option value="" disabled selected hidden>Filter by tag</option>
      <option value="">All tags</option>
      {#each allTags as tag}
        <option value={tag}>{tag}</option>
      {/each}
    </select>

    <button
      on:click={handleSearch}
      class="px-5 py-2.5 bg-brand-500 rounded-xl text-sm font-semibold hover:bg-brand-600 transition-colors shadow-sm"
    >
      Search
    </button>

    {#if keyword || tagFilter}
      <button
        on:click={handleClearFilters}
        class="px-4 py-2.5 text-surface-500 hover:text-surface-700 text-sm font-medium transition-colors"
      >
        Clear filters
      </button>
    {/if}
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if skills.length === 0}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 shadow-card">
      <EmptyState message="No skills found" />
    </div>
  {:else}
    <div class="bg-sky-50 rounded-2xl border border-indigo-200 overflow-hidden shadow-card">
      <table class="w-full">
        <thead>
          <tr class="border-b border-surface-100 bg-gradient-to-r from-surface-50/80 to-transparent">
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Name</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Version</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Status</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Tags</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Visibility</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Author</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Installs</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-surface-400 uppercase tracking-wider">Created</th>
          </tr>
        </thead>
        <tbody>
          {#each skills as skill (skill.id)}
            <tr class="table-row hover:bg-surface-800/50">
              <td class="px-6 py-4">
                <Link to="/skills/{skill.id}" class="text-brand-400 hover:text-brand-300 font-semibold text-sm transition-colors">
                  {skill.name}
                </Link>
                {#if skill.description}
                  <p class="text-surface-500 text-xs mt-0.5 max-w-[240px] truncate">{skill.description}</p>
                {/if}
              </td>
              <td class="px-6 py-4">
                <span class="text-surface-500 text-xs font-mono">{skill.version || '1.0.0'}</span>
              </td>
              <td class="px-6 py-4">
                <Badge status={skill.status || 'draft'} />
              </td>
              <td class="px-6 py-4">
                <div class="flex gap-1.5 flex-wrap">
                  {#each (skill.tags || []).slice(0, 2) as tag}
                    <span class="px-2 py-0.5 bg-surface-800 text-surface-400 text-[11px] font-medium rounded-lg">{tag}</span>
                  {/each}
                  {#if (skill.tags || []).length > 2}
                    <span class="px-2 py-0.5 bg-surface-800 text-surface-500 text-[11px] font-medium rounded-lg">+{skill.tags.length - 2}</span>
                  {/if}
                </div>
              </td>
              <td class="px-6 py-4">
                <span class="text-surface-500 text-xs capitalize">{skill.visibility || 'org_visible'}</span>
              </td>
              <td class="px-6 py-4 text-surface-400 text-xs font-mono">{skill.author_agent_id || 'N/A'}</td>
              <td class="px-6 py-4">
                <span class="text-surface-600 text-sm font-semibold">{skill.install_count || 0}</span>
              </td>
              <td class="px-6 py-4 text-surface-400 text-sm">{skill.created ? new Date(skill.created).toLocaleDateString() : 'N/A'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if totalPages > 1}
      <div class="flex items-center justify-between mt-5 px-2">
        <span class="text-surface-500 text-sm">
          Page {page} of {totalPages} ({total} total)
        </span>
        <div class="flex gap-1.5">
          <button
            on:click={() => goToPage(page - 1)}
            disabled={page <= 1}
            class="px-3.5 py-2 bg-sky-50 border border-indigo-200 rounded-xl text-sm font-medium text-surface-700 hover:bg-indigo-50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
          >
            Previous
          </button>
          {#each Array(totalPages) as _, i}
            {@const pageNum = i + 1}
            {#if pageNum === 1 || pageNum === totalPages || (pageNum >= page - 2 && pageNum <= page + 2)}
              <button
                on:click={() => goToPage(pageNum)}
                class="w-9 h-9 rounded-xl text-sm font-semibold transition-colors {pageNum === page ? 'bg-brand-500 shadow-sm' : 'bg-sky-50 border border-indigo-200 text-surface-600 hover:bg-indigo-50'}"
              >
                {pageNum}
              </button>
            {:else if pageNum === page - 3 || pageNum === page + 3}
              <span class="w-9 h-9 flex items-center justify-center text-surface-400 text-sm">...</span>
            {/if}
          {/each}
          <button
            on:click={() => goToPage(page + 1)}
            disabled={page >= totalPages}
            class="px-3.5 py-2 bg-sky-50 border border-indigo-200 rounded-xl text-sm font-medium text-surface-700 hover:bg-indigo-50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
          >
            Next
          </button>
        </div>
      </div>
    {/if}
  {/if}
</div>

{#if showCreateModal}
<div class="fixed inset-0 bg-surface-900/50 backdrop-blur-sm flex items-center justify-center z-50 modal-overlay">
  <div class="bg-sky-50 rounded-2xl p-6 w-full max-w-xl shadow-elevated-lg border border-indigo-200 modal-content max-h-[90vh] overflow-y-auto">
    <h2 class="text-lg font-bold text-surface-800 mb-5">Create Skill</h2>
    <div class="space-y-4">
      <div>
        <span class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Upload SKILL.md</span>
        <div class="flex items-center gap-3">
          <label class="px-4 py-2.5 bg-brand-500 rounded-xl text-sm font-semibold hover:bg-brand-600 transition-colors cursor-pointer shadow-sm inline-flex items-center gap-2">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/></svg>
            Choose File
            <input type="file" accept=".md" on:change={handleFileUpload} class="hidden" />
          </label>
          {#if uploadedFileName}
            <span class="text-sm text-surface-600 font-medium truncate max-w-[200px]">{uploadedFileName}</span>
            <button
              on:click={handleClearFile}
              class="text-surface-400 hover:text-rose-500 transition-colors text-sm font-medium"
            >
              Remove
            </button>
          {/if}
        </div>
        <p class="text-surface-400 text-xs mt-1.5">Upload a SKILL.md file to auto-fill fields below</p>
      </div>

      <div>
        <label for="skill-name" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Name <span class="text-rose-500">*</span></label>
        <input
          id="skill-name"
          type="text"
          bind:value={createForm.name}
          placeholder="Skill name"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>

      <div>
        <label for="skill-desc" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Description</label>
        <textarea
          id="skill-desc"
          bind:value={createForm.description}
          placeholder="Brief description of the skill"
          rows="2"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium resize-none"
        ></textarea>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label for="skill-version" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Version</label>
          <input
            id="skill-version"
            type="text"
            bind:value={createForm.version}
            placeholder="1.0.0"
            class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium"
          />
        </div>
        <div>
          <label for="skill-visibility" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Visibility</label>
          <select
            id="skill-visibility"
            bind:value={createForm.visibility}
            class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
          >
            <option value="private">Private</option>
            <option value="org_visible">Org Visible</option>
            <option value="marketplace">Marketplace</option>
            <option value="shared">Shared</option>
          </select>
        </div>
      </div>

      <div>
        <label for="skill-tags" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Tags</label>
        <input
          id="skill-tags"
          type="text"
          bind:value={createForm.tags}
          placeholder="e.g. python, web, automation (comma separated)"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium"
        />
      </div>

      <div>
        <label for="skill-content" class="block text-xs font-semibold text-surface-500 uppercase tracking-wider mb-2">Content <span class="text-rose-500">*</span></label>
        <textarea
          id="skill-content"
          bind:value={createForm.content}
          placeholder="Skill markdown content..."
          rows="8"
          class="w-full px-4 py-3 border border-surface-200 rounded-xl text-sm input-focus outline-none font-medium font-mono resize-y"
        ></textarea>
      </div>

      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={closeCreateModal}
          class="px-5 py-2.5 border border-surface-200 rounded-xl text-sm font-semibold text-surface-600 hover:bg-surface-100 transition-colors"
        >
          Cancel
        </button>
        <button
          on:click={handleCreate}
          disabled={creating}
          class="px-5 py-2.5 bg-brand-500 rounded-xl text-sm font-semibold hover:bg-brand-600 transition-colors shadow-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {creating ? 'Creating...' : 'Create Skill'}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}