<script>
  import { api } from "../lib/api.js";
  import { addToast } from "../stores/app.js";
  import { navigate } from "svelte-routing";
  import RejectModal from "./RejectModal.svelte";

  export let skill;

  let loading = false;
  let showRejectModal = false;

  async function handleApprove() {
    loading = true;
    try {
      await api.approveSkill(skill.id);
      addToast(`${skill.name} approved`, "success");
      navigate("/review", { replace: true });
    } catch (e) {
      addToast(e.message);
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
      navigate("/review", { replace: true });
    } catch (e) {
      addToast(e.message);
    } finally {
      loading = false;
    }
  }
</script>

{#if skill.status === 'pending_review'}
  <div class="flex gap-2">
    <button
      on:click={handleApprove}
      disabled={loading}
      class="px-4 py-2 text-sm font-semibold bg-emerald-500 rounded-xl hover:bg-emerald-600 disabled:opacity-50 transition-all duration-200 shadow-sm shadow-emerald-500/20 hover:shadow-md hover:shadow-emerald-500/30 active:scale-[0.97]"
    >
      Approve
    </button>
    <button
      on:click={() => showRejectModal = true}
      disabled={loading}
      class="px-4 py-2 text-sm font-semibold text-rose-400 border border-rose-700 rounded-xl hover:bg-rose-900/30 hover:border-rose-600 disabled:opacity-50 transition-all duration-200 active:scale-[0.97]"
    >
      Reject
    </button>
  </div>

  <RejectModal
    show={showRejectModal}
    skillName={skill.name}
    on:submit={(e) => handleReject(e.detail)}
    on:cancel={() => showRejectModal = false}
  />
{/if}