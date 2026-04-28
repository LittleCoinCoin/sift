<script lang="ts">
  import { progress } from './stores/log';
  import { job } from './stores/job.svelte';

  function formatEta(avg_ms: number, remaining: number): string {
    const ms = avg_ms * remaining;
    if (ms < 1000) return '<1s';
    if (ms < 60_000) return `${Math.round(ms / 1000)}s`;
    return `${Math.round(ms / 60_000)}m`;
  }

  type ProgressWithPhase = {
    done: number;
    total: number;
    avg_ms: number;
    current_file?: string;
    phase?: 'ocr' | 'extract';
  };

  const p = $derived($progress as ProgressWithPhase | null);
  const pct = $derived(p && p.total > 0 ? (p.done / p.total) * 100 : 0);
  const remaining = $derived(p ? p.total - p.done : 0);
  const currentFile = $derived(
    p?.current_file ? (p.current_file.split('/').pop() ?? p.current_file) : null,
  );
  const current_index = $derived(p ? p.done + 1 : 0);

  const canPause = $derived(job.status === 'running' || job.status === 'resuming');
  const canResume = $derived(job.status === 'paused');
  const isCancelling = $derived(job.status === 'cancelling');
  const canCancel = $derived(!isCancelling && job.status !== 'cancelled');
</script>

{#if job.isActive}
  <div class="job-overlay" role="status" aria-label="Job in progress">
    <div class="track" role="progressbar" aria-valuenow={p?.done ?? 0} aria-valuemax={p?.total ?? 0}>
      <div class="fill" style="width: {pct}%"></div>
    </div>

    <div class="body">
      <div class="info">
        <div class="phase-row">
          {#if p?.phase === 'ocr'}
            <span role="img" title="Content Extraction" aria-label="Content Extraction" class="phase-icon phase-icon--ocr">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                <path d="M3 7V5a2 2 0 0 1 2-2h2"/>
                <path d="M3 17v2a2 2 0 0 0 2 2h2"/>
                <path d="M21 7V5a2 2 0 0 0-2-2h-2"/>
                <path d="M21 17v2a2 2 0 0 1-2 2h-2"/>
                <line x1="7" y1="12" x2="17" y2="12"/>
              </svg>
            </span>
          {:else if p?.phase === 'extract'}
            <span role="img" title="Content Analysis" aria-label="Content Analysis" class="phase-icon phase-icon--extract">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                <path d="M12 3l2 6 6 2-6 2-2 6-2-6-6-2 6-2z"/>
                <path d="M5 3l1 2 2 1-2 1-1 2-1-2-2-1 2-1z"/>
                <path d="M19 15l1 2 2 1-2 1-1 2-1-2-2-1 2-1z"/>
              </svg>
            </span>
          {/if}
          <span class="file-name" title={p?.current_file ?? undefined}>
            {#if currentFile}
              {currentFile}
            {:else if isCancelling}
              Cancelling…
            {:else if job.status === 'paused'}
              Paused
            {:else}
              Processing…
            {/if}
          </span>
        </div>
        {#if p}
          <div class="counts-row">
            <span class="counts">{p.done} / {p.total} · file {current_index}</span>
          </div>
        {/if}
      </div>

      <div class="controls">
        {#if isCancelling}
          <span class="status-label">Cancelling…</span>
        {:else if canPause}
          <button class="ctrl-btn" onclick={() => job.pause()} title="Pause" aria-label="Pause job">
            ⏸
          </button>
        {:else if canResume}
          <button class="ctrl-btn" onclick={() => job.resume()} title="Resume" aria-label="Resume job">
            ▶
          </button>
        {/if}
        {#if canCancel}
          <button
            class="ctrl-btn ctrl-btn--cancel"
            onclick={() => job.requestCancel()}
            title="Cancel"
            aria-label="Cancel job"
          >✕</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .job-overlay {
    position: fixed;
    bottom: var(--space-5);
    left: 50%;
    transform: translateX(-50%);
    width: 320px;
    background: color-mix(in srgb, var(--color-surface) 92%, transparent);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    backdrop-filter: blur(8px);
    overflow: hidden;
    z-index: var(--z-overlay);
  }

  .track {
    height: 3px;
    background: var(--color-border);
  }

  .fill {
    height: 100%;
    background: var(--color-primary);
    transition: width var(--duration-fast) var(--easing-default);
  }

  .body {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .file-name {
    font-size: var(--font-size-sm);
    color: var(--color-text);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .counts {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    white-space: nowrap;
  }

  .phase-row {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
  }

  .counts-row {
    display: flex;
    align-items: center;
  }

  .phase-icon {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    transition: color var(--duration-fast) var(--easing-default);
  }

  .phase-icon--ocr {
    color: var(--color-text-muted);
  }

  .phase-icon--extract {
    color: var(--color-primary);
  }

  .controls {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex-shrink: 0;
  }

  .ctrl-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: none;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 0.7rem;
    transition:
      color var(--duration-fast) var(--easing-default),
      border-color var(--duration-fast) var(--easing-default),
      background var(--duration-fast) var(--easing-default);
  }

  .ctrl-btn:hover {
    color: var(--color-text);
    border-color: var(--color-text-muted);
    background: var(--color-surface-raised);
  }

  .ctrl-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  .ctrl-btn--cancel:hover {
    color: var(--color-error);
    border-color: var(--color-error);
  }

  .status-label {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    font-style: italic;
  }
</style>
