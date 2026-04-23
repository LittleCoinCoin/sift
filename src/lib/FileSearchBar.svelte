<script lang="ts">
  import { untrack } from 'svelte';
  import type { ReceiptEntry } from './stores/receipts.svelte';

  type ReceiptStatus = ReceiptEntry['status'];

  const STATUS_TOKENS: ReceiptStatus[] = ['Unprocessed', 'Processing', 'Processed'];
  const TOKEN_RE = /\bstatus:(unprocessed|processing|processed)\b/gi;

  let {
    onfilter,
    initialStatusFilters = [],
  }: {
    onfilter: (f: { text: string; statusFilters: ReceiptStatus[] }) => void;
    initialStatusFilters?: ReceiptStatus[];
  } = $props();

  let text = $state('');
  let statusFilters = $state<ReceiptStatus[]>([]);

  $effect(() => {
    const incoming = initialStatusFilters;
    const current = untrack(() => statusFilters);
    if (
      incoming.length !== current.length ||
      incoming.some((s, i) => s !== current[i])
    ) {
      statusFilters = incoming.slice();
    }
  });

  $effect(() => {
    onfilter({ text, statusFilters: statusFilters.slice() });
  });

  function parseStatusTokens(raw: string): { cleaned: string; extracted: ReceiptStatus[] } {
    const extracted: ReceiptStatus[] = [];
    const cleaned = raw
      .replace(TOKEN_RE, (_match, capture: string) => {
        const canonical = (capture.charAt(0).toUpperCase() +
          capture.slice(1).toLowerCase()) as ReceiptStatus;
        if (STATUS_TOKENS.includes(canonical) && !extracted.includes(canonical)) {
          extracted.push(canonical);
        }
        return '';
      })
      .replace(/\s{2,}/g, ' ')
      .trim();
    return { cleaned, extracted };
  }

  function handleInput() {
    const { cleaned, extracted } = parseStatusTokens(text);
    if (extracted.length > 0) {
      text = cleaned;
      for (const s of extracted) addStatusFilter(s);
    }
  }

  export function addStatusFilter(s: ReceiptStatus) {
    if (!statusFilters.includes(s)) statusFilters = [...statusFilters, s];
  }

  export function removeStatusFilter(s: ReceiptStatus) {
    statusFilters = statusFilters.filter((f) => f !== s);
  }

  function clearAll() {
    text = '';
    statusFilters = [];
  }

  let hasContent = $derived(text !== '' || statusFilters.length > 0);

  function chipColor(s: ReceiptStatus): string {
    switch (s) {
      case 'Processed':   return 'var(--color-success)';
      case 'Processing':  return 'var(--color-primary)';
      case 'Unprocessed': return 'var(--color-text-muted)';
      default:            return 'var(--color-text-muted)';
    }
  }
</script>

<div class="fsb-root">
  <div class="fsb-bar" class:fsb-bar--active={hasContent}>
    <svg class="fsb-icon" width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
      <circle cx="5" cy="5" r="3.5" stroke="currentColor" stroke-width="1.25"/>
      <line x1="7.8" y1="7.8" x2="11" y2="11" stroke="currentColor" stroke-width="1.25" stroke-linecap="round"/>
    </svg>

    {#each statusFilters as s (s)}
      <span class="fsb-chip" style="--chip-color: {chipColor(s)}">
        {s}
        <button
          class="fsb-chip-remove"
          tabindex="-1"
          onmousedown={(e) => { e.preventDefault(); removeStatusFilter(s); }}
          aria-label="Remove {s} filter"
        >&#xD7;</button>
      </span>
    {/each}

    <input
      class="fsb-input"
      type="text"
      placeholder="filter receipts…"
      bind:value={text}
      oninput={handleInput}
      onblur={handleInput}
      autocomplete="off"
      spellcheck={false}
      aria-label="Filter receipts by glob pattern or status:token"
    />

    {#if hasContent}
      <button
        class="fsb-clear"
        onclick={clearAll}
        aria-label="Clear filter"
        tabindex="-1"
      >&#xD7;</button>
    {/if}
  </div>
</div>

<style>
  .fsb-root {
    width: 100%;
  }

  .fsb-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1);
    min-height: 28px;
    padding: 3px var(--space-2) 3px 6px;
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    transition: border-color var(--duration-fast) var(--easing-default);
  }

  .fsb-bar:focus-within,
  .fsb-bar--active {
    border-color: var(--color-primary);
  }

  .fsb-icon {
    flex-shrink: 0;
    color: var(--color-text-muted);
    pointer-events: none;
    user-select: none;
  }

  .fsb-chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 4px 1px 7px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--chip-color);
    background: color-mix(in srgb, var(--chip-color) 15%, transparent);
    color: var(--color-text);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    white-space: nowrap;
    line-height: 1.4;
  }

  .fsb-chip-remove {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 13px;
    height: 13px;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1;
    border-radius: 2px;
    flex-shrink: 0;
  }

  .fsb-chip-remove:hover {
    color: var(--color-text);
    background: color-mix(in srgb, var(--chip-color) 25%, transparent);
  }

  .fsb-chip-remove:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  .fsb-input {
    flex: 1;
    min-width: 80px;
    background: none;
    border: none;
    outline: none;
    color: var(--color-text);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    padding: 0;
    caret-color: var(--color-primary);
  }

  .fsb-input::placeholder {
    color: var(--color-text-muted);
    font-style: italic;
  }

  .fsb-clear {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-text-muted);
    font-size: 13px;
    line-height: 1;
    border-radius: var(--radius-sm);
    margin-left: auto;
  }

  .fsb-clear:hover {
    color: var(--color-text);
    background: var(--color-surface-raised);
  }

  .fsb-clear:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
</style>
