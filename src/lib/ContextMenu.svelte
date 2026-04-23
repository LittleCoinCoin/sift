<script lang="ts">
  let {
    x,
    y,
    paths,
    ondelete,
    onclose,
  }: {
    x: number;
    y: number;
    paths: string[];
    ondelete: (paths: string[]) => void;
    onclose: () => void;
  } = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.context-menu')) onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} onclick={handleClickOutside} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
  class="context-menu"
  style="left: {x}px; top: {y}px;"
  role="menu"
>
  <button
    class="menu-item menu-item-danger"
    role="menuitem"
    onclick={() => ondelete(paths)}
  >
    Delete {paths.length > 1 ? `${paths.length} files` : 'file'}
  </button>
</div>

<style>
  .context-menu {
    position: fixed;
    z-index: 9999;
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    min-width: 160px;
    padding: var(--space-1) 0;
    font-size: var(--font-size-sm);
  }

  .menu-item {
    display: block;
    width: 100%;
    padding: var(--space-2) var(--space-4);
    background: none;
    border: none;
    text-align: left;
    cursor: pointer;
    color: var(--color-text);
    white-space: nowrap;
    transition: background var(--duration-fast) var(--easing-default);
  }

  .menu-item:hover {
    background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  }

  .menu-item-danger {
    color: var(--color-error, #e53e3e);
  }

  .menu-item-danger:hover {
    background: color-mix(in srgb, var(--color-error, #e53e3e) 10%, transparent);
  }
</style>
