<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity';
  import type { ReceiptEntry } from './stores/receipts.svelte';
  import ReceiptFileNode from './ReceiptFileNode.svelte';
  import ReceiptDirectoryNode from './ReceiptDirectoryNode.svelte';

  type ReceiptStatus = ReceiptEntry['status'];

  /** Browser-safe glob matcher: * = within segment, ** = cross-segment, ? = single char. */
  function matchGlob(paths: string[], pattern: string): string[] {
    const re = new RegExp(
      '^' +
      pattern
        .replace(/[.+^${}()|[\]\\]/g, '\\$&')
        .replace(/\*\*/g, '\u0000')
        .replace(/\*/g, '[^/]*')
        .replace(/\?/g, '[^/]')
        .replace(/\u0000/g, '.*') +
      '$'
    );
    return paths.filter((p) => re.test(p));
  }

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------

  let {
    entries,
    filter = { text: '', statusFilters: [] as ReceiptStatus[] },
    viewMode = 'tree' as 'tree' | 'flat',
    selectedPaths = $bindable(new SvelteSet<string>()),
    filteredCount = $bindable(0),
    filteredPaths = $bindable([] as string[]),
    ontabopen,
  }: {
    entries: ReceiptEntry[];
    filter?: { text: string; statusFilters: ReceiptStatus[] };
    viewMode?: 'tree' | 'flat';
    selectedPaths?: SvelteSet<string>;
    filteredCount?: number;
    filteredPaths?: string[];
    ontabopen?: (path: string, permanent: boolean) => void;
  } = $props();

  // ---------------------------------------------------------------------------
  // Relative path computation
  // ---------------------------------------------------------------------------

  /** Strip the longest common directory prefix so paths are relative to the
   *  deepest shared ancestor. Falls back to basename if no common prefix. */
  function computeRelPaths(es: ReceiptEntry[]): Map<string, string> {
    const result = new Map<string, string>();
    if (es.length === 0) return result;

    const split = es.map((e) => e.source_path.split('/'));
    const minLen = Math.min(...split.map((p) => p.length));

    let commonLen = 0;
    for (let i = 0; i < minLen - 1; i++) {
      if (split.every((p) => p[i] === split[0][i])) commonLen = i + 1;
      else break;
    }

    for (const e of es) {
      const parts = e.source_path.split('/');
      result.set(e.source_path, parts.slice(commonLen).join('/'));
    }
    return result;
  }

  // ---------------------------------------------------------------------------
  // Filter logic
  // ---------------------------------------------------------------------------

  const relPaths = $derived(computeRelPaths(entries));

  const filteredEntries = $derived.by((): ReceiptEntry[] => {
    let result: ReceiptEntry[];

    if (filter.text) {
      const paths = entries.map((e) => relPaths.get(e.source_path) ?? e.source_path);
      const matched = new Set(matchGlob(paths, filter.text));
      result = entries.filter((e) => matched.has(relPaths.get(e.source_path) ?? e.source_path));
    } else {
      result = entries;
    }

    if (filter.statusFilters.length > 0) {
      result = result.filter((e) => filter.statusFilters.includes(e.status));
    }

    return result;
  });

  $effect(() => {
    filteredCount = filteredEntries.length;
    filteredPaths = filteredEntries.map((e) => e.source_path);
  });

  // ---------------------------------------------------------------------------
  // Tree render list
  // ---------------------------------------------------------------------------

  interface RenderItem {
    kind: 'dir' | 'file';
    depth: number;
    dirPath?: string;
    dirName?: string;
    dirEntries?: ReceiptEntry[];
    entry?: ReceiptEntry;
  }

  function buildRenderList(
    es: ReceiptEntry[],
    dirs: SvelteSet<string>,
  ): RenderItem[] {
    const items: RenderItem[] = [];
    const getRel = (e: ReceiptEntry) => relPaths.get(e.source_path) ?? e.source_path;

    const rootFiles = es.filter((e) => !getRel(e).includes('/'));
    const topDirs = [
      ...new Set(
        es
          .filter((e) => getRel(e).includes('/'))
          .map((e) => getRel(e).split('/')[0]),
      ),
    ].sort();

    function addDir(dirPath: string, depth: number): void {
      const dirName = dirPath.split('/').at(-1)!;
      const allUnder = es.filter((e) => getRel(e).startsWith(dirPath + '/'));
      items.push({ kind: 'dir', depth, dirPath, dirName, dirEntries: allUnder });
      if (!dirs.has(dirPath)) return;
      for (const e of allUnder) {
        const rel = getRel(e).slice(dirPath.length + 1);
        if (!rel.includes('/')) items.push({ kind: 'file', depth: depth + 1, entry: e });
      }
      const subDirs = [
        ...new Set(
          allUnder
            .map((e) => getRel(e).slice(dirPath.length + 1))
            .filter((r) => r.includes('/'))
            .map((r) => dirPath + '/' + r.split('/')[0]),
        ),
      ];
      for (const sub of subDirs) addDir(sub, depth + 1);
    }

    for (const e of rootFiles) items.push({ kind: 'file', depth: 0, entry: e });
    for (const d of topDirs) addDir(d, 0);
    return items;
  }

  // ---------------------------------------------------------------------------
  // Expanded directory state
  // ---------------------------------------------------------------------------

  const expandedDirs = new SvelteSet<string>();

  function toggleDir(dir: string): void {
    if (expandedDirs.has(dir)) expandedDirs.delete(dir);
    else expandedDirs.add(dir);
  }

  // ---------------------------------------------------------------------------
  // Selection
  // ---------------------------------------------------------------------------

  let lastClickedPath = $state<string | null>(null);

  const visiblePaths = $derived.by((): string[] => {
    if (viewMode === 'flat') {
      return filteredEntries.map((e) => e.source_path);
    }
    return buildRenderList(filteredEntries, expandedDirs)
      .filter((item) => item.kind === 'file')
      .map((item) => item.entry!.source_path);
  });

  function handleSelect(path: string, e: MouseEvent): void {
    if (e.metaKey || e.ctrlKey) {
      if (selectedPaths.has(path)) selectedPaths.delete(path);
      else selectedPaths.add(path);
    } else if (e.shiftKey && lastClickedPath !== null) {
      const ids = visiblePaths;
      const fromIdx = ids.indexOf(lastClickedPath);
      const toIdx = ids.indexOf(path);
      if (fromIdx !== -1 && toIdx !== -1) {
        const lo = Math.min(fromIdx, toIdx);
        const hi = Math.max(fromIdx, toIdx);
        for (let i = lo; i <= hi; i++) selectedPaths.add(ids[i]);
      } else {
        selectedPaths.add(path);
      }
    } else {
      selectedPaths = new SvelteSet([path]);
    }
    if (!e.metaKey && !e.ctrlKey && !e.shiftKey) {
      ontabopen?.(path, false);
    }
    lastClickedPath = path;
  }

  function handleDblclick(path: string): void {
    ontabopen?.(path, true);
  }

  function handleKeydown(e: KeyboardEvent): void {
    if ((e.metaKey || e.ctrlKey) && e.key === 'a') {
      e.preventDefault();
      selectedPaths = new SvelteSet(filteredEntries.map((en) => en.source_path));
    } else if (e.key === 'Escape') {
      selectedPaths = new SvelteSet();
    }
  }

  function handleDirSelect(dirEntries: ReceiptEntry[], e: MouseEvent): void {
    e.stopPropagation();
    const dirPaths = dirEntries.map((en) => en.source_path);
    const allSelected = dirPaths.every((p) => selectedPaths.has(p));

    if (e.metaKey || e.ctrlKey) {
      if (allSelected) {
        for (const p of dirPaths) selectedPaths.delete(p);
      } else {
        for (const p of dirPaths) selectedPaths.add(p);
      }
    } else if (e.shiftKey && lastClickedPath !== null) {
      const ids = visiblePaths;
      const fromIdx = ids.indexOf(lastClickedPath);
      const dirVisible = dirPaths.filter((p) => ids.includes(p));
      if (fromIdx !== -1 && dirVisible.length > 0) {
        const dirIdxs = dirVisible.map((p) => ids.indexOf(p));
        const lo = Math.min(fromIdx, Math.min(...dirIdxs));
        const hi = Math.max(fromIdx, Math.max(...dirIdxs));
        for (let i = lo; i <= hi; i++) selectedPaths.add(ids[i]);
      } else {
        for (const p of dirPaths) selectedPaths.add(p);
      }
    } else {
      selectedPaths = new SvelteSet(dirPaths);
    }

    const lastVisible = dirPaths.filter((p) => visiblePaths.includes(p)).at(-1);
    if (lastVisible) lastClickedPath = lastVisible;
  }

  // ---------------------------------------------------------------------------
  // Empty state
  // ---------------------------------------------------------------------------

  const isEmpty = $derived(filteredEntries.length === 0);
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="receipt-file-tree"
  role="tree"
  tabindex="0"
  onkeydown={handleKeydown}
  onclick={() => { selectedPaths = new SvelteSet(); }}
>
  {#if isEmpty}
    <div class="tree-state tree-empty" aria-live="polite">
      {#if filter.text || filter.statusFilters.length > 0}
        No receipts match the current filter.
      {:else}
        No receipts loaded yet.
      {/if}
    </div>

  {:else if viewMode === 'tree'}
    <div class="tree-list" role="group">
      {#each buildRenderList(filteredEntries, expandedDirs) as item (item.kind === 'file' ? item.entry!.source_path : item.dirPath)}
        <div style="padding-left: calc({item.depth} * 1.25rem)">
          {#if item.kind === 'dir'}
            <ReceiptDirectoryNode
              name={item.dirName!}
              entries={item.dirEntries!}
              expanded={expandedDirs.has(item.dirPath!)}
              selectionState={
                item.dirEntries!.every((e) => selectedPaths.has(e.source_path)) ? 'all' :
                item.dirEntries!.some((e) => selectedPaths.has(e.source_path)) ? 'indeterminate' : 'none'
              }
              ontoggle={() => toggleDir(item.dirPath!)}
              onselect={(e: MouseEvent) => handleDirSelect(item.dirEntries!, e)}
            />
          {:else}
            <ReceiptFileNode
              entry={item.entry!}
              displayName={item.entry!.source_path.split('/').at(-1)}
              selected={selectedPaths.has(item.entry!.source_path)}
              onselect={(e: MouseEvent) => handleSelect(item.entry!.source_path, e)}
              ondblclick={() => handleDblclick(item.entry!.source_path)}
            />
          {/if}
        </div>
      {/each}
    </div>

  {:else}
    <div class="flat-list" role="group">
      {#each filteredEntries as entry (entry.source_path)}
        {@const rel = relPaths.get(entry.source_path) ?? entry.source_path}
        {@const parts = rel.split('/')}
        {@const fname = parts.at(-1) ?? rel}
        {@const dir = parts.length > 1 ? parts.slice(0, -1).join('/') + '/' : ''}
        <ReceiptFileNode
          {entry}
          displayName={fname}
          dirPrefix={dir}
          selected={selectedPaths.has(entry.source_path)}
          onselect={(e: MouseEvent) => handleSelect(entry.source_path, e)}
          ondblclick={() => handleDblclick(entry.source_path)}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .receipt-file-tree {
    display: flex;
    flex-direction: column;
    height: 100%;
    outline: none;
    overflow: hidden;
    background: var(--color-surface);
    color: var(--color-text);
    font-size: var(--font-size-sm);
  }

  .receipt-file-tree:focus-visible {
    box-shadow: inset 0 0 0 1px var(--color-primary);
  }

  .tree-list,
  .flat-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: var(--space-1) 0;
  }

  .tree-state {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    padding: var(--space-8);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    text-align: center;
    line-height: var(--line-height-normal);
  }
</style>
