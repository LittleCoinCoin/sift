// E2E driver for the native update test (scripts/e2e-updater.sh).
// Loaded only when the app is built with VITE_SIFT_E2E=1 (see src/main.ts), so
// it never reaches a normal bundle. It reports what the real webview shows to
// the mock server and clicks the real toast buttons.
import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { INSTALL_MESSAGES } from '../stores/updater-errors';

const LOG_URL = 'http://127.0.0.1:1430/e2e-log';
const TARGET_VERSION = '0.2.0';
const PROBE_KEY = 'e2e-probe';
// How long the keychain read is left pending before the IPC liveness check.
const IPC_PROBE_DELAY_MS = 1000;
// A sync command answering later than this means the main thread was held, so the
// answer after the dialog is dismissed does not count as 'alive'.
const IPC_ALIVE_MAX_MS = 2000;
// Gives the orchestrator time to screenshot each stage before the click moves on.
const CLICK_DELAY_MS = 2000;

let queue: Promise<void> = Promise.resolve();

// text/plain no-cors POST: a CORS "simple request", so no preflight. Posts are
// chained so the server log keeps the order in which the webview saw things.
export function post(line: string): Promise<void> {
  queue = queue.then(() =>
    fetch(LOG_URL, {
      method: 'POST',
      mode: 'no-cors',
      headers: { 'Content-Type': 'text/plain' },
      body: line,
    }).then(
      () => undefined,
      () => undefined,
    ),
  );
  return queue;
}

function compareVersions(a: string, b: string): number {
  const pa = a.split('.').map((n) => Number.parseInt(n, 10) || 0);
  const pb = b.split('.').map((n) => Number.parseInt(n, 10) || 0);
  for (let i = 0; i < 3; i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d < 0 ? -1 : 1;
  }
  return 0;
}

// "Update @@ available" -> prefix "Update ", suffix " available"
const [AVAILABLE_PREFIX, AVAILABLE_SUFFIX] = INSTALL_MESSAGES.available('\u0000').split('\u0000');

function isAvailableCopy(text: string): boolean {
  return (
    text.startsWith(AVAILABLE_PREFIX) &&
    text.endsWith(AVAILABLE_SUFFIX) &&
    text.length > AVAILABLE_PREFIX.length + AVAILABLE_SUFFIX.length
  );
}

async function probeKeychain(version: string): Promise<void> {
  try {
    if (compareVersions(version, TARGET_VERSION) < 0) {
      await invoke('set_api_key', { key: PROBE_KEY });
      await post('keychain set ok');
      // Same-process roundtrip: tells a real store from a mock one.
      const back = await invoke<string>('get_api_key');
      await post(back === PROBE_KEY ? 'keychain roundtrip ok' : 'keychain roundtrip mismatch');
    } else {
      // The read may block in a login-keychain dialog after an update. Start it without
      // awaiting, then prove the IPC still answers: get_settings is a sync command, so it
      // runs on the main thread, and a pending read that held that thread would keep
      // 'ipc alive' from being posted in time.
      const pending = invoke<string>('get_api_key');
      pending.catch(() => undefined); // handled below; this only keeps an early rejection quiet
      await new Promise<void>((resolve) => setTimeout(resolve, IPC_PROBE_DELAY_MS));
      const askedAt = performance.now();
      try {
        await invoke('get_settings');
      } catch {
        // an error reply is still an answer
      } finally {
        // Timed here, not by the script: a frozen thread answers as soon as the dialog is
        // dismissed, and only the elapsed time tells that apart from a live one.
        const elapsed = Math.round(performance.now() - askedAt);
        await post(elapsed <= IPC_ALIVE_MAX_MS ? 'ipc alive' : `ipc slow|${elapsed}`);
      }
      const value = await pending;
      await post(value === PROBE_KEY ? 'keychain ok' : 'keychain error|unexpected value');
    }
  } catch (err) {
    await post(`keychain error|${String(err)}`);
  }
}

function observeToasts(): void {
  const lastText = new WeakMap<Element, string>();
  const clicked = { available: false, ready: false };

  const scheduleClick = (kind: 'available' | 'ready', toast: Element, text: string): void => {
    if (clicked[kind]) return;
    clicked[kind] = true;
    setTimeout(async () => {
      const button = toast.querySelector<HTMLButtonElement>('.toast-action');
      const stillThere =
        toast.isConnected && toast.querySelector('.toast-message')?.textContent?.trim() === text;
      if (button && stillThere) {
        const label = button.textContent?.trim() ?? '';
        // Post first: Restart Now ends this process, so a post after the click is lost.
        await post(`click ${kind} (${label})`);
        button.click();
      } else {
        await post(`click ${kind} skipped: toast or button gone`);
      }
    }, CLICK_DELAY_MS);
  };

  const scan = (): void => {
    for (const toast of document.querySelectorAll('.toast')) {
      const text = toast.querySelector('.toast-message')?.textContent?.trim() ?? '';
      if (lastText.get(toast) === text) continue;
      lastText.set(toast, text);
      void post(`toast ${text}`);
      if (toast.querySelector('.toast-action')) {
        if (isAvailableCopy(text)) scheduleClick('available', toast, text);
        else if (text === INSTALL_MESSAGES.ready) scheduleClick('ready', toast, text);
      }
    }
  };

  new MutationObserver(scan).observe(document.body, {
    subtree: true,
    childList: true,
    characterData: true,
  });
  scan();
}

export async function startDriver(): Promise<void> {
  const version = await getVersion();
  await post(`booted ${version}`);
  observeToasts();
  void probeKeychain(version);
}

void startDriver();
