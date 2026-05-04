<script module lang="ts">
  // Survives mount/unmount so the refresh-on-job-completion effect only fires
  // on actual transitions, not on every remount of this component.
  let lastSeenCompletedAt = 0;
</script>

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
  let fitScale = $state(1);
  let zoomPercent = $state(100);
  const zoom = $derived(fitScale * zoomPercent / 100);
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

  // ── Tab state ─────────────────────────────────────────────
  interface TabEntry {
    path: string;
    temporary: boolean;
    fieldsVisible: boolean;
  }

  let tabs = $state<TabEntry[]>([]);
  let activeTabPath = $state<string | null>(null);

  function openTab(path: string, asTemporary: boolean): void {
    const existingIdx = tabs.findIndex(t => t.path === path);
    if (existingIdx !== -1) {
      // Already open: activate. Promote to permanent if not asTemporary.
      if (!asTemporary && tabs[existingIdx].temporary) {
        tabs[existingIdx].temporary = false;
      }
      activeTabPath = path;
      return;
    }
    if (asTemporary) {
      const tempIdx = tabs.findIndex(t => t.temporary);
      if (tempIdx !== -1) {
        tabs[tempIdx] = { path, temporary: true, fieldsVisible: true };
        activeTabPath = path;
        return;
      }
    }
    tabs.push({ path, temporary: asTemporary, fieldsVisible: true });
    activeTabPath = path;
  }

  function setFieldsVisible(path: string | null, value: boolean): void {
    if (!path) return;
    const idx = tabs.findIndex(t => t.path === path);
    if (idx === -1) return;
    tabs[idx].fieldsVisible = value;
  }

  function closeTab(path: string, e?: MouseEvent): void {
    e?.stopPropagation();
    const idx = tabs.findIndex(t => t.path === path);
    if (idx === -1) return;
    tabs.splice(idx, 1);
    if (activeTabPath === path) {
      if (tabs.length === 0) {
        activeTabPath = null;
      } else {
        const nextIdx = idx < tabs.length ? idx : idx - 1;
        activeTabPath = tabs[nextIdx].path;
      }
    }
  }

  const activeFile = $derived(
    activeTabPath ? receipts.files.find(f => f.source_path === activeTabPath) ?? null : null
  );
  const activeTab = $derived(tabs.find(t => t.path === activeTabPath) ?? null);

  // Load image when active tab changes; PDFs use receipt:// URI scheme directly
  $effect(() => {
    const file = activeFile;
    zoomPercent = 100;
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

  $effect(() => {
    const completedAt = job.completedAt;
    if (completedAt && completedAt !== lastSeenCompletedAt) {
      lastSeenCompletedAt = completedAt;
      receipts.flushPendingCommits().then(() => refreshReceipts());
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
  function onImageLoad(e: Event) {
    const img = e.currentTarget as HTMLImageElement;
    const rect = imagePane.getBoundingClientRect();
    const paneW = rect.width;
    const paneH = rect.height;
    const nW = img.naturalWidth || 0;
    const nH = img.naturalHeight || 0;
    if (nW === 0 || nH === 0 || paneW === 0 || paneH === 0) {
      fitScale = 1;
    } else {
      fitScale = Math.min(paneH / nH, paneW / nW);
    }
    zoomPercent = 100;
    panX = Math.max(0, (paneW - nW * fitScale) / 2);
    panY = 0;
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const rect = imagePane.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
    const oldZoom = zoom;
    const newPercent = Math.max(100, Math.min(2000, zoomPercent * factor));
    const newZoom = fitScale * newPercent / 100;
    if (oldZoom === 0) return;
    const ratio = newZoom / oldZoom;
    panX = mx * (1 - ratio) + panX * ratio;
    panY = my * (1 - ratio) + panY * ratio;
    zoomPercent = newPercent;
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

  const ZOOM_PRESETS = [100, 125, 150, 200, 300, 400, 500];
  let zoomDropdownOpen = $state(false);

  function zoomIn() {
    const next = ZOOM_PRESETS.find(p => p > zoomPercent);
    if (next !== undefined) zoomPercent = next;
  }

  function zoomOut() {
    let prev = 100;
    for (const p of ZOOM_PRESETS) {
      if (p < zoomPercent) prev = p;
      else break;
    }
    zoomPercent = prev;
  }

  function applyZoomInput(raw: string) {
    const n = parseInt(raw.replace(/[^0-9]/g, ''), 10);
    if (Number.isNaN(n)) {
      zoomPercent = zoomPercent; // no-op redisplay
      return;
    }
    zoomPercent = Math.max(100, Math.min(500, n));
  }

  function resetView() {
    zoomPercent = 100;
    if (imagePane) {
      const rect = imagePane.getBoundingClientRect();
      const img = imagePane.querySelector('img') as HTMLImageElement | null;
      const nW = img?.naturalWidth ?? 0;
      panX = Math.max(0, (rect.width - nW * fitScale) / 2);
    } else {
      panX = 0;
    }
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

  const isProcessing = $derived(job.isActive);
  const record = $derived(activeFile);

  // AC #3/#4: no selection → counts across all files
  // AC #5: selection active → Process = all selected; Export = selected ∩ Processed
  const processCount = $derived(
    receipts.selectedPaths.size > 0
      ? receipts.selectedPaths.size
      : receipts.files.filter(f => f.status !== 'Processed').length
  );
  const exportCount = $derived(
    receipts.selectedPaths.size > 0
      ? receipts.files.filter(f => receipts.selectedPaths.has(f.source_path) && f.status === 'Processed').length
      : receipts.files.filter(f => f.status === 'Processed').length
  );

  // Sidebar resize/toggle state
  let sidebarWidth = $state(220);
  let sidebarVisible = $state(true);
  let resizeStartX = 0;
  let resizeStartWidth = 0;
  let resizeMinReached = false;

  function onResizeMove(e: MouseEvent) {
    const delta = e.clientX - resizeStartX;
    const next = resizeStartWidth + delta;
    if (next < 160) {
      resizeMinReached = true;
    } else {
      resizeMinReached = false;
    }
    sidebarWidth = Math.max(180, Math.min(480, next));
  }

  function stopResize() {
    window.removeEventListener('mousemove', onResizeMove);
    window.removeEventListener('mouseup', stopResize);
    if (resizeMinReached) {
      sidebarVisible = false;
      resizeMinReached = false;
    }
  }

  function toggleSidebar() {
    sidebarVisible = !sidebarVisible;
  }

  function onWindowKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'b') {
      e.preventDefault();
      toggleSidebar();
    }
  }

  function startResize(e: MouseEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    resizeStartX = e.clientX;
    resizeStartWidth = sidebarWidth;
    resizeMinReached = false;
    window.addEventListener('mousemove', onResizeMove);
    window.addEventListener('mouseup', stopResize);
  }

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

<svelte:window onkeydown={onWindowKeydown} />

<div class="viewer">
  <!-- File list panel -->
  <aside class="file-panel" style="width: {sidebarVisible ? sidebarWidth : 0}px">
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
        ontabopen={(path, permanent) => openTab(path, !permanent)}
      />
    </div>

    <div class="panel-footer">
      <button
        class="btn-process"
        onclick={processSelected}
        disabled={processCount === 0 || isProcessing}
        title={isProcessing
          ? undefined
          : processCount === 0
            ? (receipts.selectedPaths.size > 0 ? 'No files selected' : 'No unprocessed files')
            : undefined}
        aria-label={isProcessing ? 'Processing…' : `Process ${processCount} file${processCount !== 1 ? 's' : ''}`}
      >
        {isProcessing ? 'Processing…' : `Process (${processCount})`}
      </button>
      <button
        class="btn-export"
        onclick={exportCsv}
        disabled={exporting || exportCount === 0}
        title={exportCount === 0
          ? (receipts.selectedPaths.size > 0 ? 'None of the selected files are processed' : 'No processed files to export')
          : undefined}
        aria-label={exporting ? 'Exporting…' : `Export ${exportCount} processed file${exportCount !== 1 ? 's' : ''} to CSV`}
      >
        {exporting ? 'Exporting…' : `Export CSV (${exportCount})`}
      </button>
    </div>
  </aside>

  <!-- svelte-ignore a11y_no_static_element_interactions a11y_no_noninteractive_element_interactions -->
  <div
    class="resize-handle"
    onmousedown={startResize}
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize sidebar"
  >
    <button
      class="drawer-toggle"
      onclick={toggleSidebar}
      onmousedown={(e) => e.stopPropagation()}
      title={sidebarVisible ? 'Hide sidebar (⌘B)' : 'Show sidebar (⌘B)'}
      aria-label={sidebarVisible ? 'Hide sidebar' : 'Show sidebar'}
    >{sidebarVisible ? '‹' : '›'}</button>
  </div>

  <!-- Main area: tab bar + content -->
  <div class="main-area">
    <div class="tab-bar" role="tablist">
      {#each tabs as tab (tab.path)}
        {@const fname = tab.path.split('/').at(-1) ?? tab.path}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="tab"
          class:tab-active={tab.path === activeTabPath}
          class:tab-temporary={tab.temporary}
          role="tab"
          tabindex="0"
          aria-selected={tab.path === activeTabPath}
          title={tab.path}
          onclick={() => { activeTabPath = tab.path; }}
          ondblclick={() => openTab(tab.path, false)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') { activeTabPath = tab.path; }
          }}
        >
          <span class="tab-label">{fname}</span>
          <button
            type="button"
            class="tab-close"
            aria-label="Close tab"
            onclick={(e) => closeTab(tab.path, e)}
          >×</button>
        </div>
      {/each}
    </div>

    {#if tabs.length === 0}
      <div class="no-tab-placeholder">Select a receipt from the list</div>
    {:else}
      <div class="tab-content">
        <!-- Image area: image pane + zoom toolbar -->
        <div class="image-area">
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
                onload={onImageLoad}
                onerror={() => showToast('error', `Failed to render PDF: ${activeFile?.source_path ?? ''}`)}
              />
            {:else}
              <div class="placeholder muted">Select a receipt from the list</div>
            {/if}
          </div>

          <div class="zoom-toolbar">
            <button
              class="zt-btn"
              onclick={zoomOut}
              disabled={!imageData || zoomPercent === 100}
              aria-label="Zoom out"
              title="Zoom out"
            >−</button>
            <button
              class="zt-btn"
              onclick={zoomIn}
              disabled={!imageData}
              aria-label="Zoom in"
              title="Zoom in"
            >+</button>
            <span class="zt-sep" aria-hidden="true"></span>
            <div class="zt-input-group">
              <input
                class="zt-input"
                type="text"
                value="{zoomPercent}%"
                disabled={!imageData}
                aria-label="Zoom level"
                onfocus={(e) => e.currentTarget.select()}
                onblur={(e) => applyZoomInput(e.currentTarget.value)}
                onkeydown={(e) => {
                  if (e.key === 'Enter') {
                    applyZoomInput(e.currentTarget.value);
                    e.currentTarget.blur();
                  }
                }}
              />
              <button
                class="zt-dropdown-btn"
                onclick={() => { zoomDropdownOpen = !zoomDropdownOpen; }}
                disabled={!imageData}
                aria-label="Zoom presets"
                aria-expanded={zoomDropdownOpen}
                title="Zoom presets"
              >▾</button>
              {#if zoomDropdownOpen}
                <div class="zt-dropdown" role="listbox">
                  {#each ZOOM_PRESETS as preset (preset)}
                    <button
                      class="zt-dropdown-item"
                      role="option"
                      aria-selected={zoomPercent === preset}
                      onclick={() => { zoomPercent = preset; zoomDropdownOpen = false; }}
                    >{preset}%</button>
                  {/each}
                </div>
              {/if}
            </div>
            <span class="zt-sep" aria-hidden="true"></span>
            <button
              class="zt-btn"
              onclick={resetView}
              disabled={!imageData}
              aria-label="Reset view"
              title="Reset view"
            >⟲</button>
          </div>
        </div>

        <!-- Fields pane / reveal strip -->
        {#if activeTab?.fieldsVisible && activeFile}
          <div class="fields-pane">
            <div class="fields-header">
              <h2>Receipt Details</h2>
              <button
                type="button"
                class="fields-collapse-btn"
                onclick={() => setFieldsVisible(activeTabPath, false)}
                aria-label="Collapse fields panel"
                title="Collapse fields panel"
              >›</button>
            </div>

            <div class="fields-body">
              {#if record && record.fields}
                {#each Object.entries(record.fields).sort(([a], [b]) => a.localeCompare(b)) as [key, value] (key)}
                  <div class="field-group">
                    <label for="field-{key}">{key.replace(/_/g, ' ')}</label>
                    <input
                      id="field-{key}"
                      type="text"
                      value={value}
                      oninput={(e) => record && receipts.updateField(record.source_path, key, e.currentTarget.value)}
                      onblur={() => record && receipts.commitField(record.source_path)}
                    />
                  </div>
                {/each}
              {:else}
                <p class="fields-empty">
                  {activeFile
                    ? 'Click "Process" to extract receipt data.'
                    : 'Select a receipt from the list.'}
                </p>
              {/if}
            </div>
          </div>
        {:else if activeFile}
          <button
            type="button"
            class="fields-reveal-strip"
            onclick={() => setFieldsVisible(activeTabPath, true)}
            aria-label="Show fields panel"
            title="Show fields panel"
          >‹</button>
        {/if}
      </div>
    {/if}
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
    flex-shrink: 0;
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
    overflow: hidden;
    transition: width var(--duration-normal) var(--easing-default);
  }

  .resize-handle {
    flex-shrink: 0;
    width: 4px;
    cursor: col-resize;
    background: transparent;
    transition: background var(--duration-fast) var(--easing-default);
    position: relative;
  }

  .resize-handle:hover {
    background: var(--color-primary);
  }

  .drawer-toggle {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 16px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: var(--font-size-sm);
    line-height: 1;
    pointer-events: all;
    z-index: 1;
    transition: color var(--duration-fast) var(--easing-default),
                background var(--duration-fast) var(--easing-default);
  }

  .drawer-toggle:hover {
    color: var(--color-text);
    background: var(--color-surface-raised);
  }

  .drawer-toggle:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
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
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-top: 1px solid var(--color-border);
  }

  /* ── Main area / tab bar ──────────── */
  .main-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  .tab-bar {
    flex-shrink: 0;
    display: flex;
    align-items: stretch;
    overflow-x: auto;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
    min-height: 32px;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    background: transparent;
    border: none;
    border-right: 1px solid var(--color-border);
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: var(--font-size-sm);
    white-space: nowrap;
    max-width: 220px;
    transition: background var(--duration-fast) var(--easing-default),
                color var(--duration-fast) var(--easing-default);
  }

  .tab:hover {
    background: var(--color-surface-raised);
    color: var(--color-text);
  }

  .tab-active {
    background: var(--color-bg);
    color: var(--color-text);
  }

  .tab-temporary .tab-label {
    font-style: italic;
  }

  .tab-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 180px;
  }

  .tab-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    padding: 0;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
    flex-shrink: 0;
  }

  .tab-close:hover {
    background: var(--color-surface-raised);
    color: var(--color-text);
  }

  .tab-content {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }

  .no-tab-placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    background: var(--color-bg);
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

  /* ── Image area / zoom toolbar ──── */
  .image-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .zoom-toolbar {
    flex-shrink: 0;
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-2);
    background: var(--color-surface);
    border-top: 1px solid var(--color-border);
  }

  .zt-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: var(--font-size-md);
    line-height: 1;
    transition: color var(--duration-fast) var(--easing-default),
                background var(--duration-fast) var(--easing-default),
                border-color var(--duration-fast) var(--easing-default);
  }

  .zt-btn:hover:not(:disabled) {
    color: var(--color-text);
    background: var(--color-surface-raised);
    border-color: var(--color-border);
  }

  .zt-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  .zt-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .zt-sep {
    width: 1px;
    height: 18px;
    background: var(--color-border);
    margin: 0 var(--space-1);
  }

  .zt-input-group {
    position: relative;
    display: inline-flex;
    align-items: stretch;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface-raised);
    overflow: visible;
  }

  .zt-input {
    width: 56px;
    padding: 0 var(--space-2);
    background: transparent;
    border: none;
    color: var(--color-text);
    font-size: var(--font-size-sm);
    text-align: right;
    outline: none;
  }

  .zt-input:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .zt-dropdown-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    padding: 0;
    background: none;
    border: none;
    border-left: 1px solid var(--color-border);
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: var(--font-size-sm);
    line-height: 1;
  }

  .zt-dropdown-btn:hover:not(:disabled) {
    color: var(--color-text);
    background: var(--color-surface);
  }

  .zt-dropdown-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .zt-dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    margin-bottom: var(--space-1);
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-lg);
    z-index: 10;
    overflow: hidden;
  }

  .zt-dropdown-item {
    padding: var(--space-1) var(--space-2);
    background: none;
    border: none;
    color: var(--color-text);
    cursor: pointer;
    font-size: var(--font-size-sm);
    text-align: right;
  }

  .zt-dropdown-item:hover {
    background: var(--color-surface-raised);
  }

  .zt-dropdown-item[aria-selected="true"] {
    color: var(--color-primary);
    font-weight: 600;
  }

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
    gap: var(--space-2);
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

  .fields-collapse-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: var(--font-size-md);
    line-height: 1;
    flex-shrink: 0;
    transition: color var(--duration-fast) var(--easing-default),
                border-color var(--duration-fast) var(--easing-default),
                background var(--duration-fast) var(--easing-default);
  }

  .fields-collapse-btn:hover {
    color: var(--color-text);
    border-color: var(--color-border);
    background: var(--color-surface-raised);
  }

  .fields-collapse-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  .fields-reveal-strip {
    flex-shrink: 0;
    width: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    background: var(--color-surface);
    border: none;
    border-left: 1px solid var(--color-border);
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: var(--font-size-sm);
    line-height: 1;
    transition: color var(--duration-fast) var(--easing-default),
                background var(--duration-fast) var(--easing-default);
  }

  .fields-reveal-strip:hover {
    color: var(--color-text);
    background: var(--color-surface-raised);
  }

  .fields-reveal-strip:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }

  .btn-export {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: var(--space-2) var(--space-4);
    background: none;
    color: var(--color-text-muted);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: 500;
    white-space: nowrap;
    transition: background var(--duration-fast) var(--easing-default),
                color var(--duration-fast) var(--easing-default),
                border-color var(--duration-fast) var(--easing-default);
  }

  .btn-export:hover:not(:disabled) {
    background: var(--color-surface-raised);
    color: var(--color-text);
    border-color: var(--color-text-muted);
  }

  .btn-export:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .btn-export:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-process {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: var(--space-2) var(--space-4);
    background: var(--color-primary);
    color: #fff; /* gap: --color-on-primary */
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: 500;
    white-space: nowrap;
    transition: background var(--duration-fast) var(--easing-default),
                color var(--duration-fast) var(--easing-default),
                border-color var(--duration-fast) var(--easing-default);
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
