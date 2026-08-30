<script>
  import { createEventDispatcher } from 'svelte';
  import { api } from "../lib/api.js";
  import { addToast } from "../stores/app.js";
  import { navigate } from "svelte-routing";
  import { hasPermission } from "../stores/permission.js";
  import { ACTIONS } from "../config/actions.js";
  import { canApproveReject } from "../lib/skillPerms.js";
  import RejectModal from "./RejectModal.svelte";

  const dispatch = createEventDispatcher();
  const ACT = ACTIONS.Review;

  export let skill;

  let loading = false;
  let showRejectModal = false;

  /* 审批按钮可见条件：
   * 1. RBAC: 组织 Reviewer+ → hasPermission('skill:approve_review')
   * 2. Skill 级: 个人 Skill 所有者可自审批 → canApproveReject()
   */
  $: canApprove = hasPermission(ACT.approve) || canApproveReject(skill);
  $: canReject = hasPermission(ACT.reject) || canApproveReject(skill);

  async function handleApprove() {
    loading = true;
    try {
      await api.approveSkill(skill.id);
      addToast(`${skill.name} approved`, "success");
      dispatch('action-complete', { action: 'approve' });
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  async function handleReject(reason) {
    loading = true;
    showRejectModal = false;
    try {
      await api.rejectSkill(skill.id, reason);
      addToast(`${skill.name} rejected`, "success");
      dispatch('action-complete', { action: 'reject' });
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      loading = false;
    }
  }
</script>

{#if skill.status === 'pending_review'}
  <div class="flex gap-2">
    {#if canApprove}
    <button
      on:click={handleApprove}
      disabled={loading}
      class="px-4 py-2 text-sm font-semibold bg-emerald-500 rounded-xl hover:bg-emerald-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-emerald-500/20 hover:shadow-md hover:shadow-emerald-500/30 active:scale-[0.97]"
    >
      Approve
    </button>
    {/if}
    {#if canReject}
    <button
      on:click={() => showRejectModal = true}
      disabled={loading}
      class="px-4 py-2 text-sm font-semibold text-rose-400 border border-rose-700 rounded-xl hover:bg-rose-900/30 hover:border-rose-600 disabled:opacity-50 transition-all duration-200 active:scale-[0.97]"
    >
      Reject
    </button>
    {/if}
  </div>

  <RejectModal
    show={showRejectModal}
    skillName={skill.name}
    on:submit={(e) => handleReject(e.detail)}
    on:cancel={() => showRejectModal = false}
  />
{/if}