<script>
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import { hasPermission } from '../stores/permission.js';
  import { ACTIONS } from '../config/actions.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  const ACT = ACTIONS.OrgTools;

  let tools = [];
  let organizations = [];
  let loading = true;
  let error = '';
  let showRegisterModal = false;
  let selectedOrgId = '';
  let newTool = { name: '', tool_id: '', description: '', schema: '', implementation: '' };
  let registering = false;

  onMount(async () => {
    await Promise.all([loadTools(), loadOrganizations()]);
  });

  async function loadTools() {
    loading = true;
    error = '';
    try {
      const res = await api.listOrgTools({ limit: 100 });
      tools = res.data || [];
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
      if (organizations.length > 0) {
        selectedOrgId = organizations[0].id;
      }
    } catch (e) {
      addToast('组织列表加载失败', 'warning');
    }
  }

  async function handleRegister() {
    if (!selectedOrgId || !newTool.name.trim() || !newTool.tool_id.trim()) return;
    registering = true;
    try {
      let schema, implementation;
      try {
        schema = newTool.schema.trim() ? JSON.parse(newTool.schema) : {};
        implementation = newTool.implementation.trim() ? JSON.parse(newTool.implementation) : {};
      } catch {
        addToast('Invalid JSON in schema or implementation', 'error');
        registering = false;
        return;
      }
      await api.registerOrgTool({
        org_id: selectedOrgId,
        tool_id: newTool.tool_id,
        name: newTool.name,
        description: newTool.description,
        schema,
        implementation
      });
      newTool = { name: '', tool_id: '', description: '', schema: '{}', implementation: '{}' };
      showRegisterModal = false;
      addToast('Tool registered', 'success');
      await loadTools();
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      registering = false;
    }
  }

  async function handleDelete(id) {
    if (!confirm('Delete this tool?')) return;
    try {
      await api.deleteOrgTool(id);
      addToast('Tool deleted', 'success');
      await loadTools();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleApprove(id) {
    try {
      await api.approveOrgTool(id);
      addToast('Tool approved', 'success');
      await loadTools();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  async function handleReject(id) {
    try {
      await api.rejectOrgTool(id);
      addToast('Tool rejected', 'success');
      await loadTools();
    } catch (e) {
      addToast(e.message, 'error');
    }
  }

  function statusBadgeClass(status) {
    switch (status) {
      case 'approved': return 'bg-emerald-50 text-emerald-600 ring-1 ring-emerald-600/20';
      case 'rejected': return 'bg-rose-50 text-rose-600 ring-1 ring-rose-600/20';
      case 'pending':
      default: return 'bg-amber-50 text-amber-600 ring-1 ring-amber-600/20';
    }
  }
</script>

<div class="p-8">
  <div class="page-header flex items-center justify-between">
    <div>
      <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">Org Tools</h1>
      <p class="text-gray-500 text-sm mt-1.5 font-medium">Manage organization tools and schemas</p>
    </div>
    {#if hasPermission(ACT.create)}
      <button
        on:click={() => showRegisterModal = true}
        class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        Register Tool
      </button>
    {/if}
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if tools.length === 0}
    <div class="bg-white rounded-2xl border border-gray-200 shadow-card">
      <EmptyState message="No tools registered yet" />
    </div>
  {:else}
    <div class="bg-white rounded-2xl border border-gray-200 overflow-hidden shadow-card">
      <table class="w-full">
        <thead>
          <tr class="border-b border-gray-100 bg-gray-50">
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Name</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Tool ID</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Description</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Status</th>
            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Registered</th>
            <th class="px-6 py-4 text-right text-xs font-semibold text-gray-400 uppercase tracking-wider">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-50">
          {#each tools as tool (tool.id)}
            <tr class="table-row">
              <td class="px-6 py-4 text-gray-800 font-semibold text-sm">{tool.name}</td>
              <td class="px-6 py-4 text-gray-500 text-sm font-mono text-xs">{tool.tool_id}</td>
              <td class="px-6 py-4 text-gray-500 text-sm truncate max-w-[200px]">{tool.description}</td>
              <td class="px-6 py-4">
                <span class="inline-flex px-2.5 py-1 text-xs font-semibold rounded-full {statusBadgeClass(tool.status)}">
                  {tool.status}
                </span>
              </td>
              <td class="px-6 py-4 text-gray-400 text-sm">
                {new Date(tool.created_at).toLocaleDateString()}
              </td>
              <td class="px-6 py-4 text-right">
                <div class="flex items-center justify-end gap-3">
                  {#if tool.status === 'pending' && hasPermission(ACT.approve)}
                    <button
                      on:click={() => handleApprove(tool.id)}
                      class="text-emerald-600 hover:text-emerald-700 text-sm font-semibold transition-colors"
                    >
                      Approve
                    </button>
                    <button
                      on:click={() => handleReject(tool.id)}
                      class="text-rose-500 hover:text-rose-600 text-sm font-semibold transition-colors"
                    >
                      Reject
                    </button>
                  {/if}
                  {#if hasPermission(ACT.delete)}
                    <button
                      on:click={() => handleDelete(tool.id)}
                      class="text-gray-400 hover:text-rose-500 text-sm font-semibold transition-colors"
                    >
                      Delete
                    </button>
                  {/if}
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- Register Modal -->
  {#if showRegisterModal}
    <button type="button" class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 w-full border-0 cursor-default" aria-label="Close modal" on:click={() => showRegisterModal = false}>
      <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
      <div class="bg-white rounded-2xl p-6 w-full max-w-lg shadow-elevated border border-gray-200 max-h-[85vh] overflow-y-auto" on:click|stopPropagation>
        <div class="flex items-center justify-between mb-5">
          <h2 class="text-lg font-semibold text-gray-800">Register Tool</h2>
          <button
            on:click={() => showRegisterModal = false}
            class="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-xl transition-all duration-200"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M6 18L18 6M6 6l12 12"/></svg>
          </button>
        </div>
        <div class="space-y-4">
      <div>
        <label for="org-select" class="block text-sm font-semibold text-gray-500 mb-2">Organization</label>
        <select
          id="org-select"
          bind:value={selectedOrgId}
          class="w-full px-4 py-2.5 border border-gray-200 rounded-xl text-sm input-focus outline-none bg-white text-gray-900 transition-all"
        >
          {#each organizations as org (org.id)}
            <option value={org.id}>{org.name}</option>
          {/each}
        </select>
      </div>

      <div>
        <label for="tool-name" class="block text-sm font-semibold text-gray-500 mb-2">Tool Name</label>
        <input
          id="tool-name"
          type="text"
          bind:value={newTool.name}
          placeholder="e.g., github-cli, docker-tool"
          class="w-full px-4 py-2.5 border border-gray-200 rounded-xl text-sm input-focus outline-none bg-white text-gray-900 transition-all"
        />
      </div>

      <div>
        <label for="tool-id" class="block text-sm font-semibold text-gray-500 mb-2">Tool ID</label>
        <input
          id="tool-id"
          type="text"
          bind:value={newTool.tool_id}
          placeholder="e.g., github_issue_lister"
          class="w-full px-4 py-2.5 border border-gray-200 rounded-xl text-sm input-focus outline-none bg-white text-gray-900 transition-all"
        />
      </div>

      <div>
        <label for="tool-desc" class="block text-sm font-semibold text-gray-500 mb-2">Description</label>
        <input
          id="tool-desc"
          type="text"
          bind:value={newTool.description}
          placeholder="Describe what this tool does"
          class="w-full px-4 py-2.5 border border-gray-200 rounded-xl text-sm input-focus outline-none bg-white text-gray-900 transition-all"
        />
      </div>

      <div>
        <label for="tool-schema" class="block text-sm font-semibold text-gray-500 mb-2">Schema (JSON) <span class="text-gray-400 font-normal text-xs">— 定义工具接受的输入参数</span></label>
        <textarea
          id="tool-schema"
          bind:value={newTool.schema}
          rows="5"
          placeholder={`{
  "type": "object",
  "properties": {
    "repo": { "type": "string", "description": "仓库名" },
    "limit": { "type": "number", "description": "返回条数" }
  },
  "required": ["repo"]
}`}
          class="w-full px-4 py-2.5 border border-gray-200 rounded-xl text-sm input-focus outline-none font-mono bg-white text-gray-900 transition-all"
        ></textarea>
      </div>

      <div>
        <label for="tool-impl" class="block text-sm font-semibold text-gray-500 mb-2">Implementation (JSON) <span class="text-gray-400 font-normal text-xs">— 指定镜像、容器内执行的命令和超时</span></label>
        <textarea
          id="tool-impl"
          bind:value={newTool.implementation}
          rows="5"
          placeholder={`{
  "docker_image": "ghcr.io/myorg/shared-image:v1",
  "cmd": ["python", "/app/tools/issue_lister.py"],
  "timeout_seconds": 60
}`}
          class="w-full px-4 py-2.5 border border-gray-200 rounded-xl text-sm input-focus outline-none font-mono bg-white text-gray-900 transition-all"
        ></textarea>
      </div>
    </div>

    <div class="flex gap-3 justify-end mt-6">
      <button
            on:click={() => showRegisterModal = false}
            class="btn-secondary px-5 py-2.5 rounded-xl text-sm font-medium"
          >
            Cancel
          </button>
          <button
            on:click={handleRegister}
            disabled={!newTool.name || !newTool.tool_id || registering}
            class="btn-primary px-6 py-2.5 rounded-xl text-sm font-semibold disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {registering ? 'Registering...' : 'Register'}
          </button>
    </div>
  </div>
</button>
{/if}