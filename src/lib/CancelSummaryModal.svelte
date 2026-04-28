<script lang="ts">
  import { job } from './stores/job.svelte';
  import type { FileOutcomeStatus } from './stores/job.svelte';

  function statusLabel(s: FileOutcomeStatus): string {
    return s === 'processed' ? 'done' : s;
  }

  function basename(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  let modalEl: HTMLElement | null = $state(null);

  $effect(() => {
    if (!job.cancelOpen || !modalEl) return;

    const previousFocus = document.activeElement as HTMLElement | null;

    const focusables = (): HTMLElement[] =>
      Array.from(
        modalEl!.querySelectorAll<HTMLElement>(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        )
      ).filter((el) => !el.hasAttribute('disabled'));

    focusables()[0]?.focus();

    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        job.dismissCancelConfirm();
        return;
      }
      if (e.key === 'Tab') {
        const els = focusables();
        if (els.length === 0) return;
        const first = els[0];
        const last = els[els.length - 1];
        if (e.shiftKey) {
          if (document.activeElement === first) {
            e.preventDefault();
            last.focus();
          }
        } else {
          if (document.activeElement === last) {
            e.preventDefault();
            first.focus();
          }
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      previousFocus?.focus();
    };
  });
</script>

{#if job.cancelOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={() => job.dismissCancelConfirm()}>
    <div
      bind:this={modalEl}
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-label="Cancel job confirmation"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="modal-header">
        <span class="modal-title">Cancel Job?</span>
        <button class="close-btn" onclick={() => job.dismissCancelConfirm()} aria-label="Close">×</button>
      </div>

      {#if job.cancelSummary}
        {@const s = job.cancelSummary}
        <div class="stats-row">
          <span class="stat stat--processed">{s.processed} processed</span>
          <span class="stat-sep">·</span>
          <span class="stat stat--abandoned">{s.abandoned} abandoned</span>
          {#if s.failed > 0}
            <span class="stat-sep">·</span>
            <span class="stat stat--failed">{s.failed} failed</span>
          {/if}
          <span class="stat-total">of {s.total}</span>
        </div>

        {#if s.files.length > 0}
          <ul class="file-list" aria-label="Predicted file outcomes">
            {#each s.files as f (f.path)}
              <li class="file-row file-row--{f.status}">
                <span class="file-badge">{statusLabel(f.status)}</span>
                <span class="file-path" title={f.path}>{basename(f.path)}</span>
                {#if f.in_flight}
                  <span class="in-flight-tag">mid-flight</span>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      {/if}

      <div class="modal-footer modal-footer--split">
        <button class="dismiss-btn dismiss-btn--secondary" onclick={() => job.dismissCancelConfirm()}>Dismiss</button>
        <button class="dismiss-btn dismiss-btn--danger" onclick={() => job.confirmCancel()}>Confirm Cancel</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-modal);
    animation: fade-in var(--duration-normal) var(--easing-default);
  }

  .modal {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    width: min(480px, calc(100vw - var(--space-8)));
    max-height: min(600px, calc(100vh - var(--space-8)));
    display: flex;
    flex-direction: column;
    animation: slide-up var(--duration-normal) var(--easing-default);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-5) var(--space-6);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .modal-title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--color-text);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 1.4rem;
    line-height: 1;
    padding: 0 var(--space-1);
    transition: color var(--duration-fast);
  }
  .close-btn:hover { color: var(--color-text); }

  .stats-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-4) var(--space-6);
    flex-shrink: 0;
    flex-wrap: wrap;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }

  .stat--processed { color: var(--color-success); }
  .stat--abandoned  { color: var(--color-warning); }
  .stat--failed     { color: var(--color-error); }
  .stat-sep  { color: var(--color-text-muted); }
  .stat-total {
    margin-left: auto;
    color: var(--color-text-muted);
  }

  .file-list {
    list-style: none;
    margin: 0;
    padding: 0 var(--space-6) var(--space-4);
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }

  .file-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--color-border);
    font-size: var(--font-size-sm);
  }
  .file-row:last-child { border-bottom: none; }

  .file-badge {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    flex-shrink: 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .file-row--processed .file-badge {
    background: color-mix(in srgb, var(--color-success) 15%, transparent);
    color: var(--color-success);
  }
  .file-row--abandoned .file-badge {
    background: color-mix(in srgb, var(--color-warning) 15%, transparent);
    color: var(--color-warning);
  }
  .file-row--failed .file-badge {
    background: color-mix(in srgb, var(--color-error) 15%, transparent);
    color: var(--color-error);
  }

  .file-path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }

  .in-flight-tag {
    font-size: 0.65rem;
    font-family: var(--font-mono);
    color: var(--color-text-muted);
    flex-shrink: 0;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    padding: var(--space-4) var(--space-6);
    border-top: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .modal-footer--split {
    justify-content: space-between;
  }

  .dismiss-btn {
    background: var(--color-primary);
    border: none;
    border-radius: var(--radius-md);
    color: #fff;
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: 600;
    padding: var(--space-2) var(--space-5);
    transition: background var(--duration-fast);
  }
  .dismiss-btn:hover { background: var(--color-primary-hover); }

  .dismiss-btn--secondary {
    background: none;
    border: 1px solid var(--color-border);
    color: var(--color-text);
  }
  .dismiss-btn--secondary:hover {
    background: var(--color-surface-raised);
    border-color: var(--color-text-muted);
  }

  .dismiss-btn--danger {
    background: var(--color-error);
  }
  .dismiss-btn--danger:hover {
    background: color-mix(in srgb, var(--color-error) 85%, #000);
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  @keyframes slide-up {
    from { transform: translateY(12px); opacity: 0; }
    to   { transform: translateY(0);    opacity: 1; }
  }
</style>
