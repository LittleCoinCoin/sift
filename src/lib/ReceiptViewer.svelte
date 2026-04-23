<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { receipts } from './stores/receipts.svelte';
  import type { ReceiptEntry } from './stores/receipts.svelte';
  import { showToast } from './stores/log';

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
    const file = receipts.selectedFile;
    if (!file) return;
    receipts.setProcessing(file.source_path, true);
    try {
      const activePrompt = settings.system_prompts.find(
        (p) => p.id === settings.active_system_prompt_id,
      );
      const result = await invoke<{ fields: Record<string, string> }>('process_receipt', {
        path: file.source_path,
        fileType: isPdf(file.source_path) ? 'pdf' : 'image',
        apiUrl: settings.url,
        ocrModel: settings.ocr_model,
        extractionUrl: settings.extraction_url,
        extractionModel: settings.extraction_model,
        extractionApiKey: extractionApiKey,
        activeSystemPrompt: activePrompt?.content ?? '',
        jsonSchemaKeys: settings.json_schema_keys,
        apiKey: apiKey,
      });
      const entry = receipts.files.find(f => f.source_path === file.source_path);
      if (entry) {
        entry.fields = result.fields;
        entry.status = 'Processed';
      }
      const indexMap: Record<string, unknown> = {};
      for (const f of receipts.files) {
        indexMap[f.source_path] = f;
      }
      await invoke('save_receipt_index', { index: indexMap });
    } catch {
      // backend emits log event with error details
    } finally {
      receipts.setProcessing(file.source_path, false);
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

  const isTransformed = $derived(zoom !== 1 || panX !== 0 || panY !== 0);
  const selectedPath = $derived(receipts.selectedPath);
  const isProcessing = $derived(selectedPath ? receipts.isProcessing(selectedPath) : false);
  const record = $derived(receipts.selectedFile);
  const processedCount = $derived(receipts.files.filter(f => f.status === 'Processed').length);
</script>

<div class="viewer">
  <!-- File list panel -->
  <aside class="file-panel">
    <div class="panel-header">
      <span class="panel-title">Receipts</span>
      <button
        class="icon-btn"
        onclick={refreshReceipts}
        title="Refresh receipts"
        aria-label="Refresh receipts"
      >⟳</button>
    </div>

    <ul class="file-list" role="listbox" aria-label="Receipt files">
      {#each receipts.files as file (file.source_path)}
        {@const name = file.source_path.split('/').at(-1) ?? file.source_path}
        {@const done = file.status === 'Processed'}
        <li
          class="file-item"
          class:selected={receipts.selectedPath === file.source_path}
          class:done
          onclick={() => receipts.selectFile(file.source_path)}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && receipts.selectFile(file.source_path)}
          role="option"
          aria-selected={receipts.selectedPath === file.source_path}
          tabindex="0"
          title={file.source_path}
        >
          <span class="badge">{isPdf(file.source_path) ? 'PDF' : 'IMG'}</span>
          <span class="filename">{name}</span>
          {#if done}
            <span class="check" aria-label="processed">✓</span>
          {/if}
          {#if file.status === 'Unprocessed'}
            <span class="unprocessed" aria-label="unprocessed">○</span>
          {/if}
        </li>
      {/each}
      {#if receipts.files.length === 0}
        <li class="empty">No receipts loaded</li>
      {/if}
    </ul>
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
        <button
          class="btn-process"
          onclick={processSelected}
          disabled={!receipts.selectedFile || isProcessing}
        >
          {isProcessing ? 'Processing…' : 'Process'}
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

  .file-list {
    flex: 1;
    overflow-y: auto;
    list-style: none;
    padding: var(--space-2) 0;
    margin: 0;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    font-size: var(--font-size-sm);
    color: var(--color-text);
    transition: background var(--duration-fast) var(--easing-default);
    user-select: none;
  }

  .file-item:hover { background: var(--color-surface-raised); }

  .file-item.selected {
    background: color-mix(in srgb, var(--color-primary) 15%, transparent);
  }

  .file-item:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }

  .badge {
    font-size: 0.65rem;
    padding: 1px 4px;
    border-radius: var(--radius-sm);
    background: var(--color-surface-raised);
    color: var(--color-text-muted);
    border: 1px solid var(--color-border);
    flex-shrink: 0;
    font-family: var(--font-mono);
    letter-spacing: 0.02em;
  }

  .filename {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-item.done .filename { color: var(--color-success); }

  .check {
    color: var(--color-success);
    font-size: 0.75rem;
    flex-shrink: 0;
  }

  .unprocessed {
    color: var(--color-text-muted);
    font-size: 0.75rem;
    flex-shrink: 0;
  }

  .empty {
    padding: var(--space-6) var(--space-4);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    text-align: center;
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
