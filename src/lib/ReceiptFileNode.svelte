<script lang="ts">
  import type { ReceiptEntry } from './stores/receipts.svelte';

  let {
    entry,
    selected = false,
    displayName = undefined,
    dirPrefix = undefined,
    onselect,
    ondblclick,
  }: {
    entry: ReceiptEntry;
    selected?: boolean;
    displayName?: string;
    dirPrefix?: string;
    onselect: (e: MouseEvent) => void;
    ondblclick?: () => void;
  } = $props();

  const filenameColor = $derived(
    entry.status === 'Processed'   ? 'var(--color-success)'    :
    entry.status === 'Processing'  ? 'var(--color-primary)'    :
    /* Unprocessed */                'var(--color-text-muted)'
  );

  function relativeTime(ms: number): string {
    const diff = Date.now() - ms * 1000;
    const hours = Math.floor(diff / 3_600_000);
    if (hours < 1) return 'just now';
    if (hours < 24) return `${hours}h ago`;
    return `${Math.floor(hours / 24)}d ago`;
  }

  const secondaryLine = $derived(relativeTime(entry.source_mtime));
  const name = $derived(displayName ?? entry.source_path.split('/').at(-1) ?? entry.source_path);
</script>

<div
  class="file-row"
  class:selected
  class:processing={entry.status === 'Processing'}
  role="treeitem"
  aria-selected={selected}
  tabindex="0"
  onclick={(e) => { e.stopPropagation(); onselect(e); }}
  ondblclick={(e) => { e.stopPropagation(); ondblclick?.(); }}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') onselect(e as unknown as MouseEvent);
  }}
>
  {#if dirPrefix !== undefined}
    <!-- Flat list layout -->
    {#if dirPrefix}
      <span class="dir-prefix" title={entry.source_path}>{dirPrefix}</span>
    {/if}
    <span class="filename" style:color={filenameColor}>{name}</span>
    <span class="flat-meta">{secondaryLine}</span>
    <span class="flat-spacer"></span>
  {:else}
    <!-- Tree layout -->
    <div class="row-body">
      <span class="filename" style:color={filenameColor}>{name}</span>
      <span class="secondary">{secondaryLine}</span>
    </div>
  {/if}
</div>

<style>
  .file-row {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    cursor: default;
    user-select: none;
    transition: background var(--duration-fast) var(--easing-default);
  }

  .file-row:hover {
    background: color-mix(in srgb, var(--color-primary) 8%, transparent);
  }

  .file-row.selected {
    background: color-mix(in srgb, var(--color-primary) 15%, transparent);
  }

  @keyframes processing-pulse {
    0%, 100% { opacity: 0; }
    50%       { opacity: 0.12; }
  }

  .file-row.processing::before {
    content: '';
    position: absolute;
    inset: 0;
    background: var(--color-primary);
    opacity: 0;
    animation: processing-pulse 1.6s ease-in-out infinite;
    pointer-events: none;
  }

  .file-row:focus { outline: none; }

  .file-row:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }

  .row-body {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    flex: 1;
  }

  .filename {
    font-size: var(--font-size-sm);
    font-weight: 300;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }

  .secondary {
    font-size: 0.65rem;
    color: var(--color-text-muted);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .dir-prefix {
    font-size: 0.65rem;
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 180px;
    flex-shrink: 1;
    min-width: 0;
  }

  .flat-meta {
    font-size: 0.65rem;
    color: var(--color-text-muted);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .flat-spacer {
    flex: 1;
  }
</style>
