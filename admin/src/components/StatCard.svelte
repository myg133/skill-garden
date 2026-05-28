<script>
  export let title = '';
  export let value = 0;
  export let subtitle = '';
  export let variant = 'default';
  export let icon = '';

  const gradients = {
    brand: 'gradient-card-brand-light',
    purple: 'gradient-card-purple-light',
    green: 'gradient-card-green-light',
    amber: 'gradient-card-amber-light',
    rose: 'gradient-card-rose-light',
    sky: 'gradient-card-sky-light',
    default: 'bg-slate-50',
  };

  const borders = {
    brand: 'border-brand-200/60',
    purple: 'border-purple-200/60',
    green: 'border-emerald-200/60',
    amber: 'border-amber-200/60',
    rose: 'border-rose-200/60',
    sky: 'border-sky-200/60',
    default: 'border-surface-200',
  };

  const accentBg = {
    brand: 'from-brand-500/10 to-brand-500/0',
    purple: 'from-purple-500/10 to-purple-500/0',
    green: 'from-emerald-500/10 to-emerald-500/0',
    amber: 'from-amber-500/10 to-amber-500/0',
    rose: 'from-rose-500/10 to-rose-500/0',
    sky: 'from-sky-500/10 to-sky-500/0',
    default: 'from-brand-500/10 to-brand-500/0',
  };

  const valueColors = {
    brand: 'text-brand-600',
    purple: 'text-purple-600',
    green: 'text-emerald-600',
    amber: 'text-amber-600',
    rose: 'text-rose-600',
    sky: 'text-sky-600',
    default: 'text-surface-800',
  };

  const iconBg = {
    brand: 'bg-brand-50 text-brand-600',
    purple: 'bg-purple-50 text-purple-600',
    green: 'bg-emerald-50 text-emerald-600',
    amber: 'bg-amber-50 text-amber-600',
    rose: 'bg-rose-50 text-rose-600',
    sky: 'bg-sky-50 text-sky-600',
    default: 'bg-surface-100 text-surface-600',
  };
</script>

<div
  class="relative overflow-hidden rounded-2xl p-6 border card shadow-card {gradients[variant] || gradients.default} {borders[variant] || borders.default}"
>
  <div class="absolute top-0 right-0 w-32 h-32 rounded-bl-[48px] bg-gradient-to-bl {accentBg[variant] || accentBg.default}"></div>

  <div class="absolute -bottom-6 -right-6 w-20 h-20 rounded-full bg-gradient-to-br {accentBg[variant] || accentBg.default} blur-xl"></div>

  <div class="flex items-start justify-between mb-4 relative">
    <div>
      <p class="text-[11px] font-semibold text-surface-500 uppercase tracking-widest mb-0.5">{title}</p>
    </div>
    {#if icon}
      <span class="w-9 h-9 rounded-xl {iconBg[variant] || iconBg.default} flex items-center justify-center shadow-sm">
        <svg class="w-[18px] h-[18px]" fill="none" stroke="currentColor" viewBox="0 0 24 24">{@html icon}</svg>
      </span>
    {/if}
  </div>

  <p class="text-[32px] font-extrabold relative tracking-tight stat-number {valueColors[variant] || valueColors.default}">
    {typeof value === 'number' ? value.toLocaleString() : value}
  </p>

  {#if subtitle}
    <p class="text-xs text-surface-400 mt-2 relative font-medium">{subtitle}</p>
  {/if}
</div>