<script lang="ts">
  import { toasts, dismissToast } from './stores/log';

  let copiedId = $state<number | null>(null);

  async function copyMessage(id: number, message: string) {
    await navigator.clipboard.writeText(message);
    copiedId = id;
    setTimeout(() => { copiedId = null; }, 1500);
  }
</script>

{#if $toasts.length > 0}
  <div class="toast-stack">
    {#each $toasts as toast (toast.id)}
      <div class="toast toast--{toast.level}" role="alert">
        <span class="toast-message">{toast.message}</span>
        <button
          class="toast-copy"
          class:copied={copiedId === toast.id}
          onclick={() => copyMessage(toast.id, toast.message)}
          aria-label="Copy message"
        >
          {#if copiedId === toast.id}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>
          {:else}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
          {/if}
        </button>
        <button class="toast-dismiss" onclick={() => dismissToast(toast.id)} aria-label="Dismiss">×</button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .toast-stack {
    position: fixed;
    top: var(--space-4);
    right: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    z-index: var(--z-toast);
    max-width: 360px;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    font-size: var(--font-size-sm);
    font-family: var(--font-mono);
    color: #fff;
    pointer-events: auto;
    animation: slide-in var(--duration-normal) var(--easing-default);
  }

  .toast--info    { background: var(--color-primary); }
  .toast--success { background: var(--color-success); }
  .toast--warn    { background: var(--color-warning); }
  .toast--error   { background: var(--color-error); }

  .toast-message {
    flex: 1;
    min-width: 0;
    line-height: var(--line-height-tight);
    word-break: break-word;
    overflow-wrap: anywhere;
  }

  .toast-copy, .toast-dismiss {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    line-height: 1;
    opacity: 0.75;
    padding: 0 var(--space-1);
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }
  .toast-copy  { font-size: 0; }
  .toast-copy:hover, .toast-dismiss:hover { opacity: 1; }
  .toast-copy.copied { color: #fff; opacity: 1; }
  .toast-dismiss { font-size: 1.1rem; }

  @keyframes slide-in {
    from { transform: translateX(calc(100% + var(--space-4))); opacity: 0; }
    to   { transform: translateX(0); opacity: 1; }
  }
</style>
