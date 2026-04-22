<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import { receipts } from './stores/receipts.svelte';
  import type { ReceiptFile, ReceiptRecord, EditableField } from './stores/receipts.svelte';
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

  // Directory input
  let dirInput = $state(receipts.currentDir);

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
    receipt_dir: string;
    json_schema_keys: string[];
    system_prompts: SystemPrompt[];
    active_system_prompt_id: string;
  }

  let settings = $state<AppSettings>({
    url: '',
    ocr_model: '',
    extraction_url: '',
    extraction_model: '',
    receipt_dir: '',
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
        if (s.receipt_dir && !dirInput) dirInput = s.receipt_dir;
      } catch {}
      try {
        apiKey = await invoke<string>('get_api_key');
      } catch {}
      try {
        extractionApiKey = await invoke<string>('get_extraction_api_key');
      } catch {}
    })();
  });

  // Load image (or PDF rendered to PNG) when selection changes
  $effect(() => {
    const file = receipts.selectedFile;
    zoom = 1;
    panX = 0;
    panY = 0;

    if (!file) {
      imageData = null;
      return;
    }

    imageLoading = true;
    imageData = null;
    const cmd = file.type === 'pdf' ? 'render_pdf_preview' : 'read_image_base64';
    invoke<string>(cmd, { path: file.path })
      .then(data => { imageData = data; })
      .catch((e) => {
        imageData = null;
        showToast('error', `Preview failed: ${String(e)}`);
      })
      .finally(() => { imageLoading = false; });
  });

  async function scanDir() {
    const dir = dirInput.trim();
    if (!dir) return;
    receipts.setDir(dir);
    try {
      const files = await invoke<ReceiptFile[]>('scan_receipts', { dir });
      receipts.setFiles(files);
    } catch (e: unknown) {
      showToast('error', `Scan failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function processSelected() {
    const file = receipts.selectedFile;
    if (!file) return;
    receipts.setProcessing(file.path, true);
    try {
      const activePrompt = settings.system_prompts.find(
        (p) => p.id === settings.active_system_prompt_id,
      );
      const record = await invoke<ReceiptRecord>('process_receipt', {
        path: file.path,
        fileType: file.type,
        apiUrl: settings.url,
        ocrModel: settings.ocr_model,
        extractionUrl: settings.extraction_url,
        extractionModel: settings.extraction_model,
        extractionApiKey: extractionApiKey,
        activeSystemPrompt: activePrompt?.content ?? '',
        jsonSchemaKeys: settings.json_schema_keys,
        apiKey: apiKey,
      });
      receipts.setRecord(file.path, record);
    } catch (e: unknown) {
      showToast('error', `OCR failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      receipts.setProcessing(file.path, false);
    }
  }

  let exporting = $state(false);

  async function exportCsv() {
    const records = [...receipts.records.values()];
    if (records.length === 0) {
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
      await invoke('export_csv', { records, keys, outputPath });
      showToast('success', `Exported ${records.length} receipt(s) to CSV.`);
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
  const record = $derived(receipts.selectedRecord);

</script>

<div class="viewer">
  <!-- File list panel -->
  <aside class="file-panel">
    <div class="dir-row">
      <input
        class="dir-input"
        type="text"
        placeholder="Directory path…"
        bind:value={dirInput}
        onkeydown={(e) => e.key === 'Enter' && scanDir()}
      />
      <button class="btn-scan" onclick={scanDir}>Scan</button>
    </div>

    <ul class="file-list" role="listbox" aria-label="Receipt files">
      {#each receipts.files as file (file.path)}
        {@const name = file.path.split('/').at(-1) ?? file.path}
        {@const done = receipts.records.has(file.path)}
        <li
          class="file-item"
          class:selected={receipts.selectedPath === file.path}
          class:done
          onclick={() => receipts.selectFile(file.path)}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && receipts.selectFile(file.path)}
          role="option"
          aria-selected={receipts.selectedPath === file.path}
          tabindex="0"
          title={file.path}
        >
          <span class="badge">{file.type === 'pdf' ? 'PDF' : 'IMG'}</span>
          <span class="filename">{name}</span>
          {#if done}
            <span class="check" aria-label="processed">✓</span>
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
          disabled={exporting || receipts.records.size === 0}
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
      {#if record}
        {#each Object.entries(record.fields) as [key, value]}
          <div class="field-group">
            <label for="field-{key}">{key.replace(/_/g, ' ')}</label>
            <input
              id="field-{key}"
              type="text"
              value={value}
              oninput={(e) => selectedPath && receipts.updateField(selectedPath, key, e.currentTarget.value)}
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

  .dir-row {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-3);
    border-bottom: 1px solid var(--color-border);
  }

  .dir-input {
    flex: 1;
    min-width: 0;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface-raised);
    color: var(--color-text);
    font-size: var(--font-size-sm);
    font-family: var(--font-mono);
  }

  .dir-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .btn-scan {
    padding: var(--space-1) var(--space-3);
    background: var(--color-primary);
    color: #fff;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: 500;
    white-space: nowrap;
    transition: background var(--duration-fast) var(--easing-default);
  }

  .btn-scan:hover { background: var(--color-primary-hover); }

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
