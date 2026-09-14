<script lang="ts">
  export let percent: number | null;
  export let label: string;
  export let valueText: string | null;
  export let indeterminate = false;
  export let speed: string | null = null;
  export let remaining: string | null = null;
  $: safePercent = percent !== null && Number.isFinite(percent) ? Math.max(0, Math.min(100, percent)) : null;
</script>

<div class="update-progress">
  <div class="progress-values"><span>{label}</span><strong>{safePercent !== null ? `${Math.floor(safePercent)}%` : ""}</strong></div>
  <div class:indeterminate class="update-progress-track" role="progressbar" aria-label={label}
    aria-valuemin="0" aria-valuemax="100" aria-valuenow={indeterminate ? undefined : safePercent ?? undefined}
    aria-valuetext={valueText ?? label}>
    <span class="progress-fill" style:transform={indeterminate ? undefined : `scaleX(${(safePercent ?? 0) / 100})`}></span>
  </div>
  <div class="progress-values progress-detail"><span>{valueText ?? ""}</span><span>{[speed, remaining].filter(Boolean).join(" · ")}</span></div>
</div>

<style>
  .update-progress { display: grid; gap: 9px; padding-block: 12px; }
  .progress-values { display: flex; justify-content: space-between; flex-wrap: wrap; gap: 6px 16px; font-size: 13px; font-variant-numeric: tabular-nums; }
  .progress-detail { color: var(--text-secondary); }
  .update-progress-track { height: 7px; overflow: hidden; border-radius: 4px; background: var(--bg-control-active); }
  .progress-fill { display: block; width: 100%; height: 100%; background: var(--accent); border-radius: inherit; transform-origin: left; transition: transform 160ms cubic-bezier(0.16, 1, 0.3, 1); }
  .indeterminate .progress-fill { width: 35%; animation: download-pending 1.4s ease-in-out infinite; }
  @keyframes download-pending { from { transform: translateX(-100%); } to { transform: translateX(386%); } }
  @media (prefers-reduced-motion: reduce) {
    .progress-fill { transition: none; }
    .indeterminate .progress-fill { animation: none; transform: translateX(90%); }
  }
</style>
