<script>
  import { onDestroy } from 'svelte';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';

  export let open = false;
  export let onClose = () => {};

  let doc = null;
  let loading = false;
  let error = '';
  let objectUrl = '';
  let previewOpen = false;

  $: if (open && !doc && !loading) {
    loadDoc();
  }

  $: if (!open) {
    cleanup();
  }

  async function loadDoc() {
    loading = true;
    error = '';
    try {
      doc = await api.getSetupSkill();
    } catch (e) {
      error = e?.message || 'Failed to load setup guide';
    } finally {
      loading = false;
    }
  }

  function cleanup() {
    if (objectUrl) {
      URL.revokeObjectURL(objectUrl);
      objectUrl = '';
    }
    doc = null;
    error = '';
    previewOpen = false;
  }

  function close() {
    open = false;
    onClose();
  }

  function legacyCopy(value) {
    const textarea = document.createElement('textarea');
    textarea.value = value;
    textarea.setAttribute('readonly', '');
    textarea.style.position = 'fixed';
    textarea.style.top = '0';
    textarea.style.left = '0';
    textarea.style.opacity = '0';
    textarea.style.pointerEvents = 'none';
    document.body.appendChild(textarea);
    textarea.focus();
    textarea.select();
    let ok = false;
    try {
      ok = document.execCommand('copy');
    } catch {
      ok = false;
    }
    document.body.removeChild(textarea);
    return ok;
  }

  async function copy(value, label = '已复制') {
    if (!value) return;
    let ok = false;
    try {
      if (navigator?.clipboard?.writeText) {
        await navigator.clipboard.writeText(value);
        ok = true;
      }
    } catch {
      ok = false;
    }
    if (!ok) {
      ok = legacyCopy(value);
    }
    if (ok) {
      addToast(label, 'success');
    } else {
      addToast('复制失败，请手动复制', 'error');
    }
  }

  function download() {
    if (!doc) return;
    if (objectUrl) {
      URL.revokeObjectURL(objectUrl);
    }
    const blob = new Blob([doc.content], { type: doc.content_type || 'text/markdown; charset=utf-8' });
    objectUrl = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = objectUrl;
    a.download = doc.filename || 'skill-garden-setup.md';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
  }

  onDestroy(cleanup);
</script>

{#if open}
  <div
    class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay"
    on:click|self={close}
    on:keydown={(e) => { if (e.key === 'Escape') close(); }}
    role="presentation"
  >
    <div class="bg-white rounded-2xl w-full max-w-3xl max-h-[90vh] flex flex-col shadow-elevated-lg border border-gray-200 modal-content" role="dialog" aria-modal="true" aria-labelledby="setup-skill-title">
      <div class="px-6 py-4 border-b border-gray-100 flex items-center justify-between">
        <div>
          <h2 id="setup-skill-title" class="text-lg font-bold text-gray-800">Skill Garden 安装引导</h2>
          <p class="text-xs text-gray-500 mt-1">
            本文件不包含 API Key，请将服务地址与 API Key 单独提供给你的 Agent。
          </p>
        </div>
        <button
          class="text-gray-400 hover:text-gray-600 text-2xl leading-none"
          on:click={close}
          aria-label="关闭"
        >&times;</button>
      </div>

      <div class="px-6 py-4 overflow-y-auto flex-1 space-y-4">
        {#if loading}
          <div class="text-sm text-gray-500">加载中…</div>
        {:else if error}
          <div class="bg-rose-50 border border-rose-100 text-rose-600 px-4 py-3 rounded-xl text-sm">
            {error}
            <button
              class="ml-3 underline"
              on:click={loadDoc}
            >重试</button>
          </div>
        {:else if doc}
          <section>
            <div class="flex items-center justify-between mb-2">
              <h3 class="text-sm font-semibold text-gray-700">服务端点</h3>
            </div>
            <dl class="space-y-2 text-xs text-gray-600">
              <div class="flex items-center gap-3">
                <dt class="w-20 text-gray-400">服务地址</dt>
                <dd class="flex-1 font-mono break-all bg-gray-50 px-2 py-1 rounded">{doc.server_url}</dd>
                <button
                  class="px-2 py-1 text-[11px] font-semibold text-blue-600 hover:bg-blue-50 rounded"
                  on:click={() => copy(doc.server_url, '服务地址已复制')}
                >Copy</button>
              </div>
              <div class="flex items-center gap-3">
                <dt class="w-20 text-gray-400">MCP</dt>
                <dd class="flex-1 font-mono break-all bg-gray-50 px-2 py-1 rounded">{doc.mcp_url}</dd>
                <button
                  class="px-2 py-1 text-[11px] font-semibold text-blue-600 hover:bg-blue-50 rounded"
                  on:click={() => copy(doc.mcp_url, 'MCP 地址已复制')}
                >Copy</button>
              </div>
              <div class="flex items-center gap-3">
                <dt class="w-20 text-gray-400">SSE</dt>
                <dd class="flex-1 font-mono break-all bg-gray-50 px-2 py-1 rounded">{doc.sse_url}</dd>
                <button
                  class="px-2 py-1 text-[11px] font-semibold text-blue-600 hover:bg-blue-50 rounded"
                  on:click={() => copy(doc.sse_url, 'SSE 地址已复制')}
                >Copy</button>
              </div>
            </dl>
          </section>

          <section>
            <div class="flex items-center justify-between mb-2">
              <h3 class="text-sm font-semibold text-gray-700">Agent 提示词（不含 API Key）</h3>
              <button
                class="px-2 py-1 text-[11px] font-semibold text-blue-600 hover:bg-blue-50 rounded"
                on:click={() => copy(doc.agent_prompt, '提示词已复制')}
              >Copy Agent Prompt</button>
            </div>
            <pre class="bg-gray-50 border border-gray-100 rounded-lg p-3 text-xs font-mono whitespace-pre-wrap break-words text-gray-700">{doc.agent_prompt}</pre>
          </section>

          <section>
            <div class="flex items-center justify-between mb-2">
              <h3 class="text-sm font-semibold text-gray-700">安装引导文件预览</h3>
              <div class="flex gap-2">
                <button
                  class="px-2 py-1 text-[11px] font-semibold text-blue-600 hover:bg-blue-50 rounded"
                  on:click={() => previewOpen = !previewOpen}
                >{previewOpen ? '收起' : '展开'}</button>
                <button
                  class="px-2 py-1 text-[11px] font-semibold text-blue-600 hover:bg-blue-50 rounded"
                  on:click={() => copy(doc.content, 'Markdown 内容已复制')}
                >复制内容</button>
              </div>
            </div>
            {#if previewOpen}
              <pre class="bg-gray-900 text-gray-100 rounded-lg p-3 text-[11px] font-mono whitespace-pre-wrap break-words max-h-72 overflow-y-auto">{doc.content}</pre>
            {:else}
              <div class="text-xs text-gray-500 bg-gray-50 border border-gray-100 rounded-lg px-3 py-2">
                内容已折叠。点击“展开”查看完整 Markdown 文件，或使用“下载”获取文件。
              </div>
            {/if}
          </section>
        {/if}
      </div>

      <div class="px-6 py-4 border-t border-gray-100 flex items-center justify-end gap-2">
        <button
          class="px-4 py-2 text-sm font-semibold text-gray-600 hover:bg-gray-50 rounded-lg"
          on:click={close}
        >关闭</button>
        <button
          class="px-4 py-2 text-sm font-semibold text-white bg-blue-600 hover:bg-blue-700 rounded-lg disabled:opacity-50"
          on:click={download}
          disabled={!doc}
        >下载</button>
      </div>
    </div>
  </div>
{/if}
