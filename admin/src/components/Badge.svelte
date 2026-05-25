<script>
  export let status = 'draft';

  const colors = {
    pending_review: 'bg-yellow-100 text-yellow-800',
    published: 'bg-green-100 text-green-800',
    rejected: 'bg-red-100 text-red-800',
    draft: 'bg-gray-100 text-gray-800'
  };

  const labels = {
    pending_review: 'Pending',
    published: 'Published',
    rejected: 'Rejected',
    draft: 'Draft'
  };
</script>

<span class="px-2 py-1 text-xs font-medium rounded-full {colors[status] || colors.draft}">
  {labels[status] || status}
</span>
