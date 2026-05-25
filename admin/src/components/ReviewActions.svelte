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

<div class="flex gap-2">
  <button
    on:click={handleApprove}
    disabled={loading}
    class="px-3 py-1 text-sm font-medium text-white bg-green-600 rounded hover:bg-green-700 disabled:opacity-50">
    Approve
  </button>
  <button
    on:click={() => showRejectModal = true}
    disabled={loading}
    class="px-3 py-1 text-sm font-medium text-red-600 border border-red-600 rounded hover:bg-red-50 disabled:opacity-50">
    Reject
  </button>
</div>

<RejectModal 
  show={showRejectModal} 
  skillName={skill.name} 
  on:submit={(e) => handleReject(e.detail)}
  on:cancel={() => showRejectModal = false}
/>