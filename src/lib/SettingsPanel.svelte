<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';

  const DEFAULT_COLUMNS = ['date', 'category', 'entity', 'amount', 'payment_method'];

  // Persisted settings
  let url = $state('');
  let model = $state('');
  let receiptDir = $state('');
  let columns: string[] = $state([...DEFAULT_COLUMNS]);

  // Write-only API key field — cleared after save, never stored reactively
  let keyInput = $state('');
  let keyStored = $state(false);

  // Ping state
  type PingStatus = 'idle' | 'loading' | 'ok' | 'fail';
  let pingStatus: PingStatus = $state('idle');

  // Models dropdown
  let models: string[] = $state([]);
  let modelsLoading = $state(false);
  let modelsError = $state('');

  // Save feedback
  let saveMsg = $state('');

  // CSV columns drag state
  let draggingIdx: number | null = $state(null);
  let dropTargetIdx: number | null = $state(null);

  // New column input
  let newCol = $state('');

  onMount(async () => {
    try {
      const s = await invoke<{ url: string; model: string; receipt_dir: string; csv_columns: string[] }>('get_settings');
      url = s.url ?? '';
      model = s.model ?? '';
      receiptDir = s.receipt_dir ?? '';
      columns = s.csv_columns?.length ? s.csv_columns : [...DEFAULT_COLUMNS];
    } catch {
      // no saved settings yet
    }
    // Check if a key is stored (do not read its value)
    try {
      const k = await invoke<string>('get_api_key');
      keyStored = k.length > 0;
    } catch {
      keyStored = false;
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

  async function pickDirectory() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === 'string') receiptDir = selected;
  }

  async function saveSettings() {
    await invoke('save_settings', {
      settings: { url, model, receipt_dir: receiptDir, csv_columns: columns },
    });
    saveMsg = 'Saved';
    setTimeout(() => (saveMsg = ''), 2000);
  }

  // Drag-and-drop helpers
  // Reorder happens on ondrop, not ondragover — mutating columns mid-drag
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
      const next = [...columns];
      const [moved] = next.splice(draggingIdx, 1);
      next.splice(i, 0, moved);
      columns = next;
    }
    draggingIdx = null;
    dropTargetIdx = null;
  }

  function onDragEnd() {
    draggingIdx = null;
    dropTargetIdx = null;
  }

  function removeColumn(i: number) {
    columns = columns.filter((_, idx) => idx !== i);
  }

  function addColumn() {
    const trimmed = newCol.trim();
    if (!trimmed || columns.includes(trimmed)) return;
    columns = [...columns, trimmed];
    newCol = '';
  }
</script>

<div class="settings-panel">
  <h2 class="settings-title">Settings</h2>

  <!-- ── Endpoint ── -->
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
        {#if pingStatus === 'loading'}Pinging…{:else if pingStatus === 'ok'}✓ Online{:else if pingStatus === 'fail'}✗ Offline{:else}Ping{/if}
      </button>
    </div>
  </section>

  <!-- ── API Key ── -->
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
          placeholder="sk-…"
          bind:value={keyInput}
          aria-label="API key"
          autocomplete="new-password"
        />
        <button class="btn btn--primary" onclick={saveApiKey} disabled={!keyInput}>Save key</button>
      </div>
    {/if}
  </section>

  <!-- ── Model ── -->
  <section class="settings-section">
    <h3 class="section-heading">Model</h3>
    <div class="field-row">
      <select class="input" bind:value={model} aria-label="Model" disabled={modelsLoading}>
        {#if models.length === 0}
          <option value={model}>{model || 'No models loaded'}</option>
        {:else}
          {#each models as m}
            <option value={m}>{m}</option>
          {/each}
        {/if}
      </select>
      <button class="btn" onclick={loadModels} disabled={!url || modelsLoading}>
        {modelsLoading ? 'Loading…' : 'Load models'}
      </button>
    </div>
    {#if modelsError}
      <p class="field-error">{modelsError}</p>
    {/if}
  </section>

  <!-- ── Receipt directory ── -->
  <section class="settings-section">
    <h3 class="section-heading">Receipt Directory</h3>
    <div class="field-row">
      <input
        class="input input--readonly"
        type="text"
        readonly
        value={receiptDir}
        placeholder="(not set)"
        aria-label="Receipt directory"
      />
      <button class="btn" onclick={pickDirectory}>Browse…</button>
    </div>
  </section>

  <!-- ── CSV columns ── -->
  <section class="settings-section">
    <h3 class="section-heading">CSV Columns</h3>
    <p class="section-hint">Drag to reorder. These become the CSV header row.</p>
    <ul class="columns-list" role="list">
      {#each columns as col, i (col)}
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
          <span class="drag-handle" aria-hidden="true">⠿</span>
          <span class="col-name">{col}</span>
          <button
            class="btn-icon btn--remove"
            onclick={() => removeColumn(i)}
            ondragover={(e) => onDragOver(e, i)}
            ondrop={(e) => onDrop(e, i)}
            aria-label={`Remove ${col}`}
          >×</button>
        </li>
      {/each}
    </ul>
    <div class="field-row field-row--tight">
      <input
        class="input"
        type="text"
        placeholder="custom_column"
        bind:value={newCol}
        onkeydown={(e) => e.key === 'Enter' && addColumn()}
        aria-label="New column name"
      />
      <button class="btn btn--primary" onclick={addColumn} disabled={!newCol.trim()}>Add</button>
    </div>
  </section>

  <!-- ── Save ── -->
  <div class="settings-footer">
    <button class="btn btn--primary btn--wide" onclick={saveSettings}>Save settings</button>
    {#if saveMsg}
      <span class="save-msg">{saveMsg}</span>
    {/if}
  </div>
</div>

<style>
  .settings-panel {
    padding: var(--space-6);
    max-width: 560px;
    font-family: var(--font-body);
    color: var(--color-text);
  }

  .settings-title {
    font-size: var(--font-size-xl);
    font-weight: 600;
    margin-bottom: var(--space-6);
  }

  .settings-section {
    margin-bottom: var(--space-6);
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

  .input--readonly {
    background: var(--color-surface-raised);
    cursor: default;
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

  .settings-footer {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding-top: var(--space-4);
    border-top: 1px solid var(--color-border);
  }

  .save-msg {
    font-size: var(--font-size-sm);
    color: var(--color-success);
  }
</style>
