<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { receipts } from './stores/receipts.svelte';
  import type { ReceiptEntry } from './stores/receipts.svelte';
  import { showToast } from './stores/log';
  import { job, type JobConfig } from './stores/job.svelte';
  import FileSearchBar from './FileSearchBar.svelte';
  import ReceiptFileTree from './ReceiptFileTree.svelte';
  import ContextMenu from './ContextMenu.svelte';

  // Image display state
  let imageData = $state<string | null>(null);
  let imageLoading = $state(false);

  // Pan/zoom state
  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let isDragging = $state(false);
  let dragStartX = $state(0);
  let dragStartY = $state(0);
  let imagePane: HTMLDivElement;

  // Settings
  interface SystemPrompt {
    id: string;
    name: string;
    content: string;
  }
  interface AppSettings {
    url: string;
    ocr_model: string;
    extraction_url: string;
    extraction_model: string;
    receipt_dirs: string[];
    json_schema_keys: string[];
    system_prompts: SystemPrompt[];
    active_system_prompt_id: string;
  }

  let settings = $state<AppSettings>({
    url: '',
    ocr_model: '',
    extraction_url: '',
    extraction_model: '',
    receipt_dirs: [],
    json_schema_keys: [],
    system_prompts: [],
    active_system_prompt_id: '',
  });
  let apiKey = $state('');
  let extractionApiKey = $state('');

  // Load settings once
  $effect(() => {
    (async () => {
      try {
        const s = await invoke<AppSettings>('get_settings');
        settings = s;
      } catch {}
      try {
        apiKey = await invoke<string>('get_api_key');
      } catch {}
      try {
        extractionApiKey = await invoke<string>('get_extraction_api_key');
      } catch {}
    })();
  });

  const isPdf = (path: string) => path.toLowerCase().endsWith('.pdf');

  // Load image when selection changes; PDFs use receipt:// URI scheme directly
  $effect(() => {
    const file = receipts.selectedFile;
    zoom = 1;
    panX = 0;
    panY = 0;

    if (!file) {
      imageData = null;
      imageLoading = false;
      return;
    }

    if (isPdf(file.source_path)) {
      imageLoading = false;
      imageData = 'receipt://localhost' + file.source_path.split('/').map(encodeURIComponent).join('/');
      return;
    }

    imageLoading = true;
    imageData = null;
    invoke<string>('read_image_base64', { path: file.source_path })
      .then(data => { imageData = data; })
      .catch((e) => {
        imageData = null;
        showToast('error', `Preview failed: ${String(e)}`);
      })
      .finally(() => { imageLoading = false; });
  });

  async function refreshReceipts() {
    try {
      const files = await invoke<ReceiptEntry[]>('scan_all_receipt_dirs');
      receipts.setFiles(files);
    } catch (e: unknown) {
      showToast('error', `Scan failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  onMount(async () => {
    if (receipts.files.length === 0) {
      await refreshReceipts();
    }
  });

  async function processSelected() {
    const paths = [...receipts.selectedPaths];
    if (paths.length === 0) return;
    try {
      const activePrompt = settings.system_prompts.find(
        p => p.id === settings.active_system_prompt_id,
      );
      const config: JobConfig = {
        files: paths.map(p => ({
          path: p,
          file_type: isPdf(p) ? 'pdf' : 'image',
        })),
        api_url: settings.url,
        api_key: apiKey,
        ocr_model: settings.ocr_model,
        extraction_url: settings.extraction_url,
        extraction_api_key: extractionApiKey,
        extraction_model: settings.extraction_model,
        active_system_prompt: activePrompt?.content ?? '',
        json_schema_keys: settings.json_schema_keys,
      };
      await job.start(config);
    } catch (e: unknown) {
      showToast('error', `Failed to start job: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  let exporting = $state(false);

  async function exportCsv() {
    const processed = receipts.files.filter(f => f.status === 'Processed');
    if (processed.length === 0) {
      showToast('warn', 'No processed receipts to export.');
      return;
    }
    const outputPath = await save({
      defaultPath: 'receipts.csv',
      filters: [{ name: 'CSV', extensions: ['csv'] }],
    });
    if (!outputPath) return;
    exporting = true;
    try {
      const keys = settings.json_schema_keys?.length
        ? settings.json_schema_keys
        : ['date', 'category', 'entity', 'amount', 'payment_method', 'source_path'];
      await invoke('export_csv', { records: processed, keys, outputPath });
      showToast('success', `Exported ${processed.length} receipt(s) to CSV.`);
    } catch (e: unknown) {
      showToast('error', `Export failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      exporting = false;
    }
  }

  // Pan/zoom handlers
  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const rect = imagePane.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
    const newZoom = Math.max(0.1, Math.min(20, zoom * factor));
    const ratio = newZoom / zoom;
    panX = mx * (1 - ratio) + panX * ratio;
    panY = my * (1 - ratio) + panY * ratio;
    zoom = newZoom;
  }

  function onWindowMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    panX = e.clientX - dragStartX;
    panY = e.clientY - dragStartY;
  }

  function onWindowMouseUp() {
    isDragging = false;
    window.removeEventListener('mousemove', onWindowMouseMove);
    window.removeEventListener('mouseup', onWindowMouseUp);
  }

  function onMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    isDragging = true;
    dragStartX = e.clientX - panX;
    dragStartY = e.clientY - panY;
    e.preventDefault();
    window.addEventListener('mousemove', onWindowMouseMove);
    window.addEventListener('mouseup', onWindowMouseUp);
  }

  function resetView() {
    zoom = 1;
    panX = 0;
    panY = 0;
  }

  type ReceiptStatus = ReceiptEntry['status'];

  // Tree/filter state
  let viewMode = $state<'tree' | 'flat'>('tree');
  let filter = $state<{ text: string; statusFilters: ReceiptStatus[] }>({ text: '', statusFilters: [] });
  let treeSelectedPaths = $state(new SvelteSet<string>());

  // Sync tree selection → store
  $effect(() => {
    receipts.selectedPaths = new Set(treeSelectedPaths);
  });

  // Seed tree selection from store on first load
  $effect(() => {
    const storeFirst = [...receipts.selectedPaths][0];
    if (storeFirst && treeSelectedPaths.size === 0) {
      treeSelectedPaths = new SvelteSet(receipts.selectedPaths);
    }
  });

  const isTransformed = $derived(zoom !== 1 || panX !== 0 || panY !== 0);
  const isProcessing = $derived(job.isActive);
  const record = $derived(receipts.selectedFile);
  const processedCount = $derived(receipts.files.filter(f => f.status === 'Processed').length);

  // Context menu
  let contextMenu = $state<{ x: number; y: number; paths: string[] } | null>(null);

  function onTreeContextMenu(e: MouseEvent) {
    e.preventDefault();
    if (receipts.selectedPaths.size === 0) return;
    contextMenu = { x: e.clientX, y: e.clientY, paths: [...receipts.selectedPaths] };
  }

  async function deleteFiles(paths: string[]) {
    contextMenu = null;
    try {
      const deleted = await invoke<string[]>('delete_receipt_files', { paths });
      receipts.files = receipts.files.filter(f => !deleted.includes(f.source_path));
      for (const p of deleted) receipts.removeSelection(p);
      if (deleted.length > 0) showToast('success', `Deleted ${deleted.length} receipt(s).`);
    } catch (e: unknown) {
      showToast('error', `Delete failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
</script>

<div class="viewer">
  <!-- File list panel -->
  <aside class="file-panel">
    <div class="panel-header">
      <span class="panel-title">Receipts</span>
      <div class="panel-header-actions">
        <button
          class="icon-btn"
          onclick={() => { viewMode = viewMode === 'tree' ? 'flat' : 'tree'; }}
          title={viewMode === 'tree' ? 'Switch to flat list' : 'Switch to tree view'}
          aria-label={viewMode === 'tree' ? 'Switch to flat list' : 'Switch to tree view'}
          aria-pressed={viewMode === 'flat'}
        >{viewMode === 'tree' ? '☰' : '⊞'}</button>
        <button
          class="icon-btn"
          onclick={refreshReceipts}
          title="Refresh receipts"
          aria-label="Refresh receipts"
        >⟳</button>
      </div>
    </div>

    <div class="search-bar-wrap">
      <FileSearchBar onfilter={(f) => { filter = f; }} />
    </div>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="tree-wrap" oncontextmenu={onTreeContextMenu}>
      <ReceiptFileTree
        entries={receipts.files}
        {filter}
        {viewMode}
        bind:selectedPaths={treeSelectedPaths}
      />
    </div>

    <div class="panel-footer">
      <button
        class="btn-process"
        onclick={processSelected}
        disabled={receipts.selectedPaths.size === 0 || isProcessing}
      >
        {isProcessing ? 'Processing…' : 'Process'}
      </button>
    </div>
  </aside>

  <!-- Image pane -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
  <div
    class="image-pane"
    class:dragging={isDragging}
    bind:this={imagePane}
    onwheel={onWheel}
    onmousedown={onMouseDown}
    role="img"
    aria-label="Receipt image viewer — scroll to zoom, drag to pan"
  >
    {#if imageLoading}
      <div class="placeholder">Loading…</div>
    {:else if imageData}
      <img
        src={imageData}
        alt="Receipt"
        draggable="false"
        style="transform: translate({panX}px, {panY}px) scale({zoom}); transform-origin: 0 0;"
        onerror={() => showToast('error', `Failed to render PDF: ${receipts.selectedFile?.source_path ?? ''}`)}
      />
    {:else}
      <div class="placeholder muted">Select a receipt from the list</div>
    {/if}

    {#if isTransformed}
      <button class="reset-btn" onclick={resetView}>⟲ Reset view</button>
    {/if}
  </div>

  {#if contextMenu}
    <ContextMenu
      x={contextMenu.x}
      y={contextMenu.y}
      paths={contextMenu.paths}
      ondelete={deleteFiles}
      onclose={() => { contextMenu = null; }}
    />
  {/if}

  <!-- Fields pane -->
  <div class="fields-pane">
    <div class="fields-header">
      <h2>Receipt Details</h2>
      <div class="fields-actions">
        <button
          class="btn-export"
          onclick={exportCsv}
          disabled={exporting || processedCount === 0}
          title="Export all processed receipts to CSV"
        >
          {exporting ? 'Exporting…' : 'Export CSV'}
        </button>
      </div>
    </div>

    <div class="fields-body">
      {#if record && record.fields}
        {#each Object.entries(record.fields) as [key, value]}
          <div class="field-group">
            <label for="field-{key}">{key.replace(/_/g, ' ')}</label>
            <input
              id="field-{key}"
              type="text"
              value={value}
              oninput={(e) => { if (record && record.fields) record.fields[key] = e.currentTarget.value; }}
            />
          </div>
        {/each}
      {:else}
        <p class="fields-empty">
          {receipts.selectedFile
            ? 'Click "Process" to extract receipt data.'
            : 'Select a receipt from the list.'}
        </p>
      {/if}
    </div>
  </div>
</div>

<style>
  .viewer {
    display: flex;
    height: 100%;
    overflow: hidden;
  }

  /* ── File panel ─────────────────── */
  .file-panel {
    width: 220px;
    flex-shrink: 0;
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .panel-header-actions {
    display: flex;
    gap: var(--space-1);
    align-items: center;
  }

  .panel-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 1rem;
    transition: color var(--duration-fast) var(--easing-default),
                border-color var(--duration-fast) var(--easing-default),
                background var(--duration-fast) var(--easing-default);
  }

  .icon-btn:hover {
    color: var(--color-text);
    border-color: var(--color-border);
    background: var(--color-surface-raised);
  }

  .icon-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  .search-bar-wrap {
    padding: var(--space-2) var(--space-2);
    flex-shrink: 0;
    border-bottom: 1px solid var(--color-border);
  }

  .tree-wrap {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .panel-footer {
    flex-shrink: 0;
    padding: var(--space-2) var(--space-3);
    border-top: 1px solid var(--color-border);
  }

  /* ── Image pane ─────────────────── */
  .image-pane {
    flex: 1;
    position: relative;
    overflow: hidden;
    background: var(--color-bg);
    cursor: grab;
    user-select: none;
  }

  .image-pane.dragging { cursor: grabbing; }

  .image-pane img {
    position: absolute;
    top: 0;
    left: 0;
    max-width: none;
    display: block;
    box-shadow: var(--shadow-lg);
  }

  .placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    pointer-events: none;
  }

  .muted { color: var(--color-text-muted); }

  .reset-btn {
    position: absolute;
    bottom: var(--space-4);
    right: var(--space-4);
    padding: var(--space-1) var(--space-3);
    background: color-mix(in srgb, var(--color-surface) 85%, transparent);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    cursor: pointer;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    backdrop-filter: blur(6px);
    transition: color var(--duration-fast) var(--easing-default);
  }

  .reset-btn:hover { color: var(--color-text); }

  /* ── Fields pane ─────────────────── */
  .fields-pane {
    width: 280px;
    flex-shrink: 0;
    border-left: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
    overflow: hidden;
  }

  .fields-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .fields-header h2 {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--color-text);
    margin: 0;
  }

  .fields-actions {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .btn-export {
    padding: var(--space-2) var(--space-3);
    background: none;
    color: var(--color-text-muted);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: 500;
    transition: color var(--duration-fast), border-color var(--duration-fast);
    white-space: nowrap;
  }

  .btn-export:hover:not(:disabled) {
    color: var(--color-text);
    border-color: var(--color-text-muted);
  }

  .btn-export:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-process {
    width: 100%;
    padding: var(--space-2) var(--space-4);
    background: var(--color-primary);
    color: #fff;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: 500;
    transition: background var(--duration-fast) var(--easing-default);
    white-space: nowrap;
  }

  .btn-process:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .btn-process:hover:not(:disabled) { background: var(--color-primary-hover); }

  .btn-process:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .fields-body {
    flex: 1;
    overflow-y: auto;
  }

  .field-group {
    padding: var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }

  .field-group label {
    display: block;
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--color-text-muted);
    margin-bottom: var(--space-2);
  }

  .field-group input {
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface-raised);
    color: var(--color-text);
    font-size: var(--font-size-md);
    box-sizing: border-box;
    transition: border-color var(--duration-fast) var(--easing-default);
  }

  .field-group input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .fields-empty {
    padding: var(--space-6) var(--space-4);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    text-align: center;
    line-height: var(--line-height-normal);
  }
</style>
