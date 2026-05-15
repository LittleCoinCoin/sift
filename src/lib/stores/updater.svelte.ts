import { listen } from '@tauri-apps/api/event';

let _initialized = false;

// eslint-disable-next-line @typescript-eslint/no-unused-vars
async function checkForUpdates(_explicit: boolean): Promise<void> {
  // stub — implemented in Step 2
}

export async function initUpdaterStore(): Promise<void> {
  if (_initialized) return;
  _initialized = true;

  await listen('check-for-updates', () => checkForUpdates(true));

  setTimeout(() => checkForUpdates(false), 4000);
}
