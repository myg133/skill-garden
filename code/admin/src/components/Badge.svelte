<script>
  export let status = 'draft';

  const colors = {
    pending_review: 'bg-amber-50 text-amber-600 ring-1 ring-amber-600/20',
    published: 'bg-emerald-50 text-emerald-600 ring-1 ring-emerald-600/20',
    approved: 'bg-emerald-50 text-emerald-600 ring-1 ring-emerald-600/20',
    rejected: 'bg-rose-50 text-rose-600 ring-1 ring-rose-600/20',
    active: 'bg-emerald-50 text-emerald-600 ring-1 ring-emerald-600/20',
    ended: 'bg-gray-100 text-gray-500 ring-1 ring-surface-600/10',
    draft: 'bg-gray-100 text-gray-500 ring-1 ring-surface-600/10',
    pending: 'bg-amber-50 text-amber-600 ring-1 ring-amber-600/20',
  };

  const labels = {
    pending_review: 'Pending Review',
    published: 'Published',
    approved: 'Approved',
    rejected: 'Rejected',
    active: 'Active',
    ended: 'Ended',
    draft: 'Draft',
    pending: 'Pending',
  };
</script>

<span class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-semibold rounded-full {colors[status] || colors.draft}">
  {#if status === 'active' || status === 'published' || status === 'approved'}
    <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 pulse-dot"></span>
  {:else if status === 'pending_review' || status === 'pending'}
    <span class="w-1.5 h-1.5 rounded-full bg-amber-500 pulse-dot"></span>
  {/if}
  {labels[status] || status}
</span>