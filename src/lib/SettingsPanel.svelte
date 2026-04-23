<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';

  const DEFAULT_SCHEMA_KEYS = ['date', 'category', 'entity', 'amount', 'payment_method'];

  type SystemPrompt = { id: string; name: string; content: string };

  // Persisted settings
  let url = $state('');
  let model = $state('');
  let extractionUrl = $state('');
  let extractionModel = $state('');
  let receiptDirs: string[] = $state([]);
  let jsonSchemaKeys: string[] = $state([...DEFAULT_SCHEMA_KEYS]);
  let systemPrompts: SystemPrompt[] = $state([]);
  let activePromptId = $state('');
  let promptContent = $state('');

  // Write-only API key fields — cleared after save, never stored reactively
  let keyInput = $state('');
  let keyStored = $state(false);
  let extractionKeyInput = $state('');
  let extractionKeyStored = $state(false);

  // Ping state
  type PingStatus = 'idle' | 'loading' | 'ok' | 'fail';
  let pingStatus: PingStatus = $state('idle');

  // OCR models dropdown
  let models: string[] = $state([]);
  let modelsLoading = $state(false);
  let modelsError = $state('');

  // Text Processing models dropdown
  let extractionModels: string[] = $state([]);
  let extractionModelsLoading = $state(false);
  let extractionModelsError = $state('');

  // Save feedback
  let saveMsg = $state('');

  // Schema keys drag state
  let draggingIdx: number | null = $state(null);
  let dropTargetIdx: number | null = $state(null);

  // New schema key input
  let newKey = $state('');

  // New system prompt version name
  let newVersionName = $state('');

  // Tabbed navigation
  type TabId = 'ocr' | 'text' | 'prompt' | 'output';
  const tabs: { id: TabId; label: string }[] = [
    { id: 'ocr', label: 'OCR' },
    { id: 'text', label: 'Text Processing' },
    { id: 'prompt', label: 'Prompt' },
    { id: 'output', label: 'Output' },
  ];
  let activeTab: TabId = $state('ocr');

  function onTabKey(e: KeyboardEvent, i: number) {
    const last = tabs.length - 1;
    let next = i;
    if (e.key === 'ArrowRight') next = i === last ? 0 : i + 1;
    else if (e.key === 'ArrowLeft') next = i === 0 ? last : i - 1;
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = last;
    else return;
    e.preventDefault();
    activeTab = tabs[next].id;
    // Move focus to the newly active tab (WAI-ARIA roving tabindex pattern)
    queueMicrotask(() => {
      const btn = document.getElementById(`settings-tab-${tabs[next].id}`);
      btn?.focus();
    });
  }

  // Keep promptContent in sync with the currently selected prompt.
  $effect(() => {
    const p = systemPrompts.find((sp) => sp.id === activePromptId);
    if (p) promptContent = p.content;
  });

  onMount(async () => {
    try {
      const s = await invoke<{
        url: string;
        ocr_model: string;
        extraction_url: string;
        extraction_model: string;
        receipt_dirs: string[];
        json_schema_keys: string[];
        system_prompts: SystemPrompt[];
        active_system_prompt_id: string;
      }>('get_settings');
      url = s.url ?? '';
      model = s.ocr_model ?? '';
      extractionUrl = s.extraction_url ?? '';
      extractionModel = s.extraction_model ?? '';
      receiptDirs = s.receipt_dirs ?? [];
      jsonSchemaKeys = s.json_schema_keys?.length ? s.json_schema_keys : [...DEFAULT_SCHEMA_KEYS];
      systemPrompts = s.system_prompts ?? [];
      activePromptId = s.active_system_prompt_id ?? systemPrompts[0]?.id ?? '';
    } catch {
      // no saved settings yet
    }
    // Check if keys are stored (do not read their values)
    try {
      const k = await invoke<string>('get_api_key');
      keyStored = k.length > 0;
    } catch {
      keyStored = false;
    }
    try {
      const k = await invoke<string>('get_extraction_api_key');
      extractionKeyStored = k.length > 0;
    } catch {
      extractionKeyStored = false;
    }
  });

  async function ping() {
    if (!url) return;
    pingStatus = 'loading';
    try {
      const ok = await invoke<boolean>('ping_endpoint', { url });
      pingStatus = ok ? 'ok' : 'fail';
    } catch {
      pingStatus = 'fail';
    }
  }

  async function loadModels() {
    modelsLoading = true;
    modelsError = '';
    try {
      // Retrieve key transiently — not stored in reactive state
      const key = await invoke<string>('get_api_key').catch(() => '');
      const list = await invoke<{ id: string }[]>('list_models', { url, key });
      models = list.map((m) => m.id);
      if (!model && models.length) model = models[0];
    } catch (e: unknown) {
      modelsError = e instanceof Error ? e.message : String(e);
    } finally {
      modelsLoading = false;
    }
  }

  async function loadExtractionModels() {
    extractionModelsLoading = true;
    extractionModelsError = '';
    try {
      const key = await invoke<string>('get_extraction_api_key').catch(() => '');
      const list = await invoke<{ id: string }[]>('list_models', { url: extractionUrl, key });
      extractionModels = list.map((m) => m.id);
      if (!extractionModel && extractionModels.length) extractionModel = extractionModels[0];
    } catch (e: unknown) {
      extractionModelsError = e instanceof Error ? e.message : String(e);
    } finally {
      extractionModelsLoading = false;
    }
  }

  async function saveApiKey() {
    if (!keyInput) return;
    await invoke('set_api_key', { key: keyInput });
    keyInput = '';
    keyStored = true;
  }

  async function clearApiKey() {
    await invoke('delete_api_key');
    keyStored = false;
    models = [];
  }

  async function saveExtractionApiKey() {
    if (!extractionKeyInput) return;
    await invoke('set_extraction_api_key', { key: extractionKeyInput });
    extractionKeyInput = '';
    extractionKeyStored = true;
  }

  async function clearExtractionApiKey() {
    await invoke('delete_extraction_api_key');
    extractionKeyStored = false;
    extractionModels = [];
  }

  async function addReceiptDir() {
    const selected = await invoke<string | null>('open_directory_picker');
    if (selected && !receiptDirs.includes(selected)) {
      receiptDirs = [...receiptDirs, selected];
      await saveSettings();
    }
  }

  function removeReceiptDir(dir: string) {
    receiptDirs = receiptDirs.filter((d) => d !== dir);
    saveSettings();
  }

  function updateCurrentVersion() {
    const idx = systemPrompts.findIndex((p) => p.id === activePromptId);
    if (idx === -1) return;
    const next = [...systemPrompts];
    next[idx] = { ...next[idx], content: promptContent };
    systemPrompts = next;
  }

  function saveAsNewVersion() {
    const name = newVersionName.trim();
    if (!name) return;
    const id = (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function')
      ? crypto.randomUUID()
      : `prompt-${Date.now()}`;
    systemPrompts = [...systemPrompts, { id, name, content: promptContent }];
    activePromptId = id;
    newVersionName = '';
  }

  async function saveSettings() {
    await invoke('save_settings', {
      settings: {
        url,
        ocr_model: model,
        extraction_url: extractionUrl,
        extraction_model: extractionModel,
        receipt_dirs: receiptDirs,
        json_schema_keys: jsonSchemaKeys,
        system_prompts: systemPrompts,
        active_system_prompt_id: activePromptId,
      },
    });
    saveMsg = 'Saved';
    setTimeout(() => (saveMsg = ''), 2000);
  }

  // Drag-and-drop helpers
  // Reorder happens on ondrop, not ondragover — mutating the list mid-drag
  // destroys the dragged DOM node in WebKit's keyed list re-render.
  function onDragStart(e: DragEvent, i: number) {
    e.dataTransfer?.setData('text/plain', String(i));
    draggingIdx = i;
  }

  function onDragOver(e: DragEvent, i: number) {
    e.preventDefault();
    dropTargetIdx = i;
  }

  function onDrop(e: DragEvent, i: number) {
    e.preventDefault();
    if (draggingIdx !== null && draggingIdx !== i) {
      const next = [...jsonSchemaKeys];
      const [moved] = next.splice(draggingIdx, 1);
      next.splice(i, 0, moved);
      jsonSchemaKeys = next;
    }
    draggingIdx = null;
    dropTargetIdx = null;
  }

  function onDragEnd() {
    draggingIdx = null;
    dropTargetIdx = null;
  }

  function removeKey(i: number) {
    jsonSchemaKeys = jsonSchemaKeys.filter((_, idx) => idx !== i);
  }

  function addKey() {
    const trimmed = newKey.trim();
    if (!trimmed || jsonSchemaKeys.includes(trimmed)) return;
    jsonSchemaKeys = [...jsonSchemaKeys, trimmed];
    newKey = '';
  }
</script>

<div class="settings-panel">
  <header class="settings-header">
    <h2 class="settings-title">Settings</h2>
    <div class="tablist" role="tablist" aria-label="Settings sections">
      {#each tabs as t, i (t.id)}
        <button
          id={`settings-tab-${t.id}`}
          class="tab"
          class:tab--active={activeTab === t.id}
          role="tab"
          type="button"
          aria-selected={activeTab === t.id}
          aria-controls={`settings-tabpanel-${t.id}`}
          tabindex={activeTab === t.id ? 0 : -1}
          onclick={() => (activeTab = t.id)}
          onkeydown={(e) => onTabKey(e, i)}
        >{t.label}</button>
      {/each}
    </div>
  </header>

  <div
    class="settings-body"
    id={`settings-tabpanel-${activeTab}`}
    role="tabpanel"
    aria-labelledby={`settings-tab-${activeTab}`}
    tabindex="0"
  >
  {#if activeTab === 'ocr'}
  <!-- \u2500\u2500 Endpoint \u2500\u2500 -->
  <section class="settings-section">
    <h3 class="section-heading">API Endpoint</h3>
    <div class="field-row">
      <input
        class="input"
        type="url"
        placeholder="http://localhost:11434"
        bind:value={url}
        aria-label="API URL"
      />
      <button
        class="btn btn--ping"
        class:btn--ok={pingStatus === 'ok'}
        class:btn--fail={pingStatus === 'fail'}
        class:btn--loading={pingStatus === 'loading'}
        onclick={ping}
        disabled={!url || pingStatus === 'loading'}
      >
        {#if pingStatus === 'loading'}Pinging\u2026{:else if pingStatus === 'ok'}\u2713 Online{:else if pingStatus === 'fail'}\u2717 Offline{:else}Ping{/if}
      </button>
    </div>
  </section>

  <!-- \u2500\u2500 API Key \u2500\u2500 -->
  <section class="settings-section">
    <h3 class="section-heading">API Key</h3>
    {#if keyStored}
      <div class="field-row">
        <span class="key-stored-hint">Key stored in system keychain</span>
        <button class="btn btn--danger" onclick={clearApiKey}>Clear</button>
      </div>
    {:else}
      <div class="field-row">
        <input
          class="input"
          type="password"
          placeholder="sk-\u2026"
          bind:value={keyInput}
          aria-label="API key"
          autocomplete="new-password"
        />
        <button class="btn btn--primary" onclick={saveApiKey} disabled={!keyInput}>Save key</button>
      </div>
    {/if}
  </section>

  <!-- \u2500\u2500 OCR Model \u2500\u2500 -->
  <section class="settings-section">
    <h3 class="section-heading">OCR Model</h3>
    <div class="field-row">
      <select class="input" bind:value={model} aria-label="OCR model" disabled={modelsLoading}>
        {#if models.length === 0}
          <option value={model}>{model || 'No models loaded'}</option>
        {:else}
          {#each models as m}
            <option value={m}>{m}</option>
          {/each}
        {/if}
      </select>
      <button class="btn" onclick={loadModels} disabled={!url || modelsLoading}>
        {modelsLoading ? 'Loading\u2026' : 'Load models'}
      </button>
    </div>
    {#if modelsError}
      <p class="field-error">{modelsError}</p>
    {/if}
  </section>
  {:else if activeTab === 'text'}
  <!-- \u2500\u2500 Text Processing Endpoint \u2500\u2500 -->
  <section class="settings-section">
    <h3 class="section-heading">Text Processing Endpoint</h3>
    <div class="field-row">
      <input
        class="input"
        type="url"
        placeholder="http://localhost:11434"
        bind:value={extractionUrl}
        aria-label="Text Processing URL"
      />
    </div>
  </section>

  <!-- \u2500\u2500 Text Processing API Key \u2500\u2500 -->
  <section class="settings-section">
    <h3 class="section-heading">Text Processing API Key</h3>
    {#if extractionKeyStored}
      <div class="field-row">
        <span class="key-stored-hint">Key stored in system keychain</span>
        <button class="btn btn--danger" onclick={clearExtractionApiKey}>Clear</button>
      </div>
    {:else}
      <div class="field-row">
        <input
          class="input"
          type="password"
          placeholder="sk-\u2026"
          bind:value={extractionKeyInput}
          aria-label="Text Processing API key"
          autocomplete="new-password"
        />
        <button class="btn btn--primary" onclick={saveExtractionApiKey} disabled={!extractionKeyInput}>Save key</button>
      </div>
    {/if}
  </section>

  <!-- \u2500\u2500 Text Processing Model \u2500\u2500 -->
  <section class="settings-section">
    <h3 class="section-heading">Text Processing Model</h3>
    <div class="field-row">
      <select class="input" bind:value={extractionModel} aria-label="Text Processing model" disabled={extractionModelsLoading}>
        {#if extractionModels.length === 0}
          <option value={extractionModel}>{extractionModel || 'No models loaded'}</option>
        {:else}
          {#each extractionModels as m}
            <option value={m}>{m}</option>
          {/each}
        {/if}
      </select>
      <button class="btn" onclick={loadExtractionModels} disabled={!extractionUrl || extractionModelsLoading}>
        {extractionModelsLoading ? 'Loading\u2026' : 'Load models'}
      </button>
    </div>
    {#if extractionModelsError}
      <p class="field-error">{extractionModelsError}</p>
    {/if}
  </section>
  {:else if activeTab === 'prompt'}
  <!-- \u2500\u2500 System Prompt \u2500\u2500 -->
  <section class="settings-section">
    <h3 class="section-heading">System Prompt</h3>
    <p class="section-hint">Use <code>{'{schema_keys}'}</code> to inject the JSON schema keys into the prompt.</p>
    <div class="field-row">
      <select class="input" bind:value={activePromptId} aria-label="Active system prompt">
        {#if systemPrompts.length === 0}
          <option value="">(none)</option>
        {:else}
          {#each systemPrompts as p (p.id)}
            <option value={p.id}>{p.name}</option>
          {/each}
        {/if}
      </select>
      <button class="btn" onclick={updateCurrentVersion} disabled={!activePromptId}>Update Current Version</button>
    </div>
    <textarea
      class="input prompt-textarea"
      bind:value={promptContent}
      aria-label="System prompt content"
      rows="8"
    ></textarea>
    <div class="field-row field-row--tight">
      <input
        class="input"
        type="text"
        placeholder="New version name"
        bind:value={newVersionName}
        aria-label="New version name"
      />
      <button class="btn btn--primary" onclick={saveAsNewVersion} disabled={!newVersionName.trim()}>Save as New Version</button>
    </div>
  </section>
  {:else if activeTab === 'output'}
  <!-- ── Receipt directories ── -->
  <section class="settings-section">
    <h3 class="section-heading">Receipt Directories</h3>
    {#if receiptDirs.length > 0}
      <ul class="file-list" role="list" aria-label="Receipt directories">
        {#each receiptDirs as dir (dir)}
          <li class="file-item">
            <span class="file-path">{dir}</span>
            <button
              class="btn-icon btn--remove"
              onclick={() => removeReceiptDir(dir)}
              aria-label={`Remove ${dir}`}
            >&times;</button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty-hint">No directories added yet.</p>
    {/if}
    <button class="btn btn--secondary" onclick={addReceiptDir}>+ Add Directory</button>
  </section>

  <!-- \u2500\u2500 Structured Output JSON Schema Keys \u2500\u2500 -->
  <section class="settings-section">
    <h3 class="section-heading">Structured Output JSON Schema Keys</h3>
    <p class="section-hint">
      These keys define the JSON schema returned by the text processing model and map to CSV export columns.
      Drag to reorder.
    </p>
    <ul class="columns-list" role="list">
      {#each jsonSchemaKeys as k, i (k)}
        <li
          class="column-item"
          class:dragging={draggingIdx === i}
          class:drop-target={dropTargetIdx === i && draggingIdx !== i}
          draggable="true"
          ondragstart={(e) => onDragStart(e, i)}
          ondragover={(e) => onDragOver(e, i)}
          ondrop={(e) => onDrop(e, i)}
          ondragend={onDragEnd}
          role="listitem"
        >
          <span class="drag-handle" aria-hidden="true">\u2837</span>
          <span class="col-name">{k}</span>
          <button
            class="btn-icon btn--remove"
            onclick={() => removeKey(i)}
            ondragover={(e) => onDragOver(e, i)}
            ondrop={(e) => onDrop(e, i)}
            aria-label={`Remove ${k}`}
          >\u00d7</button>
        </li>
      {/each}
    </ul>
    <div class="field-row field-row--tight">
      <input
        class="input"
        type="text"
        placeholder="custom_key"
        bind:value={newKey}
        onkeydown={(e) => e.key === 'Enter' && addKey()}
        aria-label="New schema key"
      />
      <button class="btn btn--primary" onclick={addKey} disabled={!newKey.trim()}>Add</button>
    </div>
  </section>
  {/if}
  </div>

  <!-- \u2500\u2500 Sticky Save footer \u2500\u2500 -->
  <footer class="settings-footer">
    <button class="btn btn--primary btn--wide" onclick={saveSettings}>Save settings</button>
    <span class="save-msg" role="status" aria-live="polite">{saveMsg}</span>
  </footer>
</div>

<style>
  /* Three-zone flex column that fits the viewport below the 48px app header. */
  .settings-panel {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 48px);
    max-width: 640px;
    margin: 0 auto;
    font-family: var(--font-body);
    color: var(--color-text);
  }

  .settings-header {
    flex-shrink: 0;
    padding: var(--space-6) var(--space-6) 0;
    background: var(--color-bg);
  }

  .settings-title {
    font-size: var(--font-size-xl);
    font-weight: 600;
    margin-bottom: var(--space-4);
  }

  .tablist {
    display: flex;
    gap: var(--space-1);
    border-bottom: 1px solid var(--color-border);
  }

  .tab {
    position: relative;
    appearance: none;
    background: transparent;
    border: none;
    padding: var(--space-3) var(--space-4);
    margin-bottom: -1px;
    font-family: var(--font-body);
    font-size: var(--font-size-md);
    font-weight: 500;
    color: var(--color-text-muted);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    border-top-left-radius: var(--radius-sm);
    border-top-right-radius: var(--radius-sm);
    transition: color var(--duration-fast), border-color var(--duration-fast),
      background var(--duration-fast);
  }

  .tab:hover {
    color: var(--color-text);
    background: var(--color-surface-raised);
  }

  /* Active tab uses readable --color-text (AA) + bold weight + primary underline.
     Selection is also conveyed by aria-selected, so it never depends on color alone. */
  .tab--active {
    color: var(--color-text);
    font-weight: 600;
    border-bottom-color: var(--color-primary);
  }

  .tab--active:hover {
    color: var(--color-text);
    background: transparent;
  }

  .tab:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }

  /* Scrollable middle zone: keyboard-focusable so arrow-key scroll works. */
  .settings-body {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-6);
  }

  .settings-body:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -4px;
    border-radius: var(--radius-sm);
  }

  .settings-section {
    margin-bottom: var(--space-6);
  }

  .settings-section:last-child {
    margin-bottom: 0;
  }

  .section-heading {
    font-size: var(--font-size-md);
    font-weight: 600;
    margin-bottom: var(--space-3);
    color: var(--color-text);
  }

  .section-hint {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    margin-bottom: var(--space-2);
  }

  .field-row {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .field-row--tight {
    margin-top: var(--space-2);
  }

  .input {
    flex: 1;
    height: 36px;
    padding: 0 var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: var(--font-size-md);
    font-family: var(--font-body);
    outline: none;
    transition: border-color var(--duration-fast);
  }

  .input:focus {
    border-color: var(--color-primary);
  }

  .input:focus-visible {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-primary) 35%, transparent);
  }

  .prompt-textarea {
    height: auto;
    min-height: 140px;
    padding: var(--space-2) var(--space-3);
    resize: vertical;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    line-height: 1.4;
    margin-top: var(--space-2);
    width: 100%;
    box-sizing: border-box;
  }

  .file-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-bottom: var(--space-2);
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  .file-path {
    flex: 1;
    font-size: var(--font-size-sm);
    font-family: var(--font-mono);
    color: var(--color-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty-hint {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    margin-bottom: var(--space-2);
  }

  .btn--secondary {
    background: var(--color-surface);
    border-color: var(--color-border);
    color: var(--color-text);
  }

  .btn--secondary:hover:not(:disabled) {
    background: var(--color-surface-raised);
  }

  select.input {
    cursor: pointer;
  }

  .btn {
    height: 36px;
    padding: 0 var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: var(--font-size-md);
    font-family: var(--font-body);
    cursor: pointer;
    white-space: nowrap;
    transition: background var(--duration-fast), color var(--duration-fast),
      border-color var(--duration-fast);
  }

  .btn:hover:not(:disabled) {
    background: var(--color-surface-raised);
  }

  .btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn--primary {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: #fff;
  }

  .btn--primary:hover:not(:disabled) {
    background: var(--color-primary-hover);
    border-color: var(--color-primary-hover);
  }

  .btn--danger {
    background: var(--color-error);
    border-color: var(--color-error);
    color: #fff;
  }

  .btn--danger:hover:not(:disabled) {
    filter: brightness(0.9);
  }

  .btn--ping {
    min-width: 90px;
  }

  .btn--ok {
    background: var(--color-success);
    border-color: var(--color-success);
    color: #fff;
  }

  .btn--fail {
    background: var(--color-error);
    border-color: var(--color-error);
    color: #fff;
  }

  .btn--loading {
    opacity: 0.7;
    cursor: wait;
  }

  .btn--wide {
    min-width: 160px;
  }

  .key-stored-hint {
    flex: 1;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    font-style: italic;
  }

  .field-error {
    margin-top: var(--space-2);
    font-size: var(--font-size-sm);
    color: var(--color-error);
  }

  .columns-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-bottom: var(--space-2);
  }

  .column-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    cursor: grab;
    user-select: none;
    transition: background var(--duration-fast), opacity var(--duration-fast);
  }

  .column-item:active {
    cursor: grabbing;
  }

  .column-item.dragging {
    opacity: 0.4;
    background: var(--color-surface-raised);
  }

  .column-item.drop-target {
    border-color: var(--color-primary);
    background: color-mix(in srgb, var(--color-primary) 10%, var(--color-surface));
  }

  .drag-handle {
    color: var(--color-text-muted);
    font-size: 1rem;
    flex-shrink: 0;
    pointer-events: none;
  }

  .col-name {
    flex: 1;
    font-size: var(--font-size-md);
    font-family: var(--font-mono);
    pointer-events: none;
  }

  .btn-icon {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-text-muted);
    font-size: 1.1rem;
    line-height: 1;
    padding: 0 var(--space-1);
    flex-shrink: 0;
  }

  .btn-icon:hover {
    color: var(--color-error);
  }

  .btn-icon:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
    border-radius: var(--radius-sm);
  }

  /* Sticky save footer, always visible regardless of body scroll position. */
  .settings-footer {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-6);
    background: var(--color-surface);
    border-top: 1px solid var(--color-border);
    box-shadow: 0 -1px 2px rgba(0, 0, 0, 0.04);
  }

  .save-msg {
    min-height: 1rem;
    font-size: var(--font-size-sm);
    color: var(--color-success);
  }

  @media (prefers-reduced-motion: reduce) {
    .tab,
    .btn,
    .input,
    .column-item {
      transition: none;
    }
  }
</style>
