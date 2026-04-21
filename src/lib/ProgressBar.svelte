<script lang="ts">
  import { progress } from './stores/log';

  function formatEta(avg_ms: number, remaining: number): string {
    const ms = avg_ms * remaining;
    if (ms < 1000) return '<1s';
    if (ms < 60_000) return `${Math.round(ms / 1000)}s`;
    return `${Math.round(ms / 60_000)}m`;
  }
</script>

{#if $progress !== null}
  {@const p = $progress}
  {@const pct = p.total > 0 ? (p.done / p.total) * 100 : 0}
  {@const remaining = p.total - p.done}
  <div class="progress-wrap" role="progressbar" aria-valuenow={p.done} aria-valuemax={p.total}>
    <div class="progress-bar" style="width: {pct}%"></div>
    <span class="progress-label">
      {p.done}/{p.total} · ETA {formatEta(p.avg_ms, remaining)}
    </span>
  </div>
{/if}

<style>
  .progress-wrap {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 4px;
    background: var(--color-border);
    z-index: var(--z-overlay);
  }

  .progress-bar {
    height: 100%;
    background: var(--color-primary);
    transition: width var(--duration-fast) var(--easing-default);
  }

  .progress-label {
    position: absolute;
    top: 6px;
    right: var(--space-4);
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    white-space: nowrap;
  }
</style>
