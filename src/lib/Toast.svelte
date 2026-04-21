<script lang="ts">
  import { toasts, dismissToast } from './stores/log';
</script>

{#if $toasts.length > 0}
  <div class="toast-stack">
    {#each $toasts as toast (toast.id)}
      <div class="toast toast--{toast.level}" role="alert">
        <span class="toast-message">{toast.message}</span>
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
    font-family: var(--font-body);
    color: #fff;
    pointer-events: auto;
    animation: slide-in var(--duration-normal) var(--easing-default);
  }

  .toast--info    { background: var(--color-primary); }
  .toast--success { background: var(--color-success); }
  .toast--warn    { background: var(--color-warning); }
  .toast--error   { background: var(--color-error); }

  .toast-message { flex: 1; line-height: var(--line-height-tight); }

  .toast-dismiss {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: 1.1rem;
    line-height: 1;
    opacity: 0.75;
    padding: 0 var(--space-1);
    flex-shrink: 0;
  }
  .toast-dismiss:hover { opacity: 1; }

  @keyframes slide-in {
    from { transform: translateX(calc(100% + var(--space-4))); opacity: 0; }
    to   { transform: translateX(0); opacity: 1; }
  }
</style>
