<script lang="ts">
  import type { ReceiptEntry } from './stores/receipts.svelte';

  interface Props {
    name: string;
    entries: ReceiptEntry[];
    expanded?: boolean;
    selectionState?: 'all' | 'indeterminate' | 'none';
    ontoggle: () => void;
    onselect: (e: MouseEvent) => void;
  }

  let {
    name,
    entries,
    expanded = false,
    selectionState = 'none',
    ontoggle,
    onselect,
  }: Props = $props();

  const nameColor = $derived.by(() => {
    const statuses = entries.map((e) => e.status);
    if (statuses.includes('Processing'))  return 'var(--color-primary)';
    if (statuses.includes('Unprocessed')) return 'var(--color-text-muted)';
    return 'var(--color-success)';
  });

  const countUnprocessed = $derived(entries.filter((e) => e.status === 'Unprocessed').length);
  const countProcessing  = $derived(entries.filter((e) => e.status === 'Processing').length);
  const countProcessed   = $derived(entries.filter((e) => e.status === 'Processed').length);
</script>

<div
  class="rdn-row"
  class:rdn-selected={selectionState !== 'none'}
  onclick={(e) => onselect(e)}
  onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onselect(e as unknown as MouseEvent)}
  role="row"
  tabindex="0"
>
  <button
    type="button"
    class="rdn-chevron"
    class:rdn-chevron--expanded={expanded}
    onclick={(e) => { e.stopPropagation(); ontoggle(); }}
    aria-label={expanded ? 'Collapse folder' : 'Expand folder'}
  >&#9654;</button>

  <span class="rdn-name" style:color={nameColor}>{name}</span>

  <span class="rdn-pills">
    {#if countUnprocessed > 0}
      <span class="rdn-pill rdn-pill--muted">{countUnprocessed}</span>
    {/if}
    {#if countProcessing > 0}
      <span class="rdn-pill rdn-pill--info">{countProcessing}</span>
    {/if}
    {#if countProcessed > 0}
      <span class="rdn-pill rdn-pill--success">{countProcessed}</span>
    {/if}
  </span>
</div>

<style>
  .rdn-row {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    background: transparent;
    cursor: default;
    position: relative;
    min-height: 28px;
  }

  .rdn-row:hover { background: var(--color-surface-raised); }

  .rdn-row:focus { outline: none; }

  .rdn-row:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }

  .rdn-row.rdn-selected {
    background: color-mix(in srgb, var(--color-primary) 15%, transparent);
  }

  .rdn-chevron {
    flex-shrink: 0;
    background: transparent;
    border: none;
    padding: 0;
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 0.65rem;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    transition: transform 150ms ease, color var(--duration-fast) var(--easing-default);
  }

  .rdn-chevron:hover { color: var(--color-text); }

  .rdn-chevron--expanded { transform: rotate(90deg); }

  .rdn-chevron:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
    border-radius: 2px;
  }

  .rdn-name {
    flex: 1;
    font-size: var(--font-size-sm);
    font-weight: 300;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: color 150ms ease;
  }

  .rdn-pills {
    display: flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
  }

  .rdn-pill {
    display: inline-flex;
    align-items: center;
    padding: 0 5px;
    height: 16px;
    border-radius: var(--radius-pill);
    font-size: 0.65rem;
    font-weight: 600;
    line-height: 1;
    font-family: var(--font-mono);
  }

  .rdn-pill--muted {
    background: var(--color-surface-raised);
    color: var(--color-text-muted);
    border: 1px solid var(--color-border);
  }

  .rdn-pill--info {
    background: color-mix(in srgb, var(--color-primary) 15%, transparent);
    color: var(--color-primary);
  }

  .rdn-pill--success {
    background: color-mix(in srgb, var(--color-success) 15%, transparent);
    color: var(--color-success);
  }
</style>
