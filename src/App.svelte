<script lang="ts">
  import { onMount } from 'svelte';
  import Toast from './lib/Toast.svelte';
  import JobProgressOverlay from './lib/JobProgressOverlay.svelte';
  import Caustics from './lib/Caustics.svelte';
  import ThemeToggle from './lib/ThemeToggle.svelte';
  import SettingsPanel from './lib/SettingsPanel.svelte';
  import ReceiptViewer from './lib/ReceiptViewer.svelte';
  import CancelSummaryModal from './lib/CancelSummaryModal.svelte';
  import { initLogStore } from './lib/stores/log';
  import { initJobStore } from './lib/stores/job.svelte';
  // @ts-ignore
  import './lib/tokens.css';
  // @ts-ignore
  import './lib/theme.css';

  let showSettings = $state(false);

  onMount(() => { initLogStore(); initJobStore(); });
</script>

<Caustics />
<JobProgressOverlay />
<Toast />
<CancelSummaryModal />

<header class="app-header">
  <span class="app-title">Receipts</span>
  <div class="header-actions">
    <button
      class="settings-btn"
      class:settings-btn--active={showSettings}
      onclick={() => (showSettings = !showSettings)}
      aria-label="Toggle settings"
      title="Settings"
    >⚙</button>
    <ThemeToggle />
  </div>
</header>

<main>
  {#if showSettings}
    <SettingsPanel />
  {:else}
    <ReceiptViewer />
  {/if}
</main>

<style>
  :global(body) {
    height: 100vh;
    overflow: hidden;
  }

  .app-header {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--space-4);
    background: color-mix(in srgb, var(--color-surface) 80%, transparent);
    backdrop-filter: blur(8px);
    border-bottom: 1px solid var(--color-border);
    z-index: var(--z-overlay);
  }

  .app-title {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--color-text);
    letter-spacing: -0.01em;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .settings-btn {
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 1.1rem;
    height: 32px;
    width: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color var(--duration-fast), border-color var(--duration-fast);
  }

  .settings-btn:hover { color: var(--color-text); border-color: var(--color-border); }
  .settings-btn--active { color: var(--color-primary); border-color: var(--color-primary); }

  main {
    height: 100vh;
    padding-top: 48px;
    overflow: hidden;
  }
</style>
