import { listen } from '@tauri-apps/api/event';
import { check } from '@tauri-apps/plugin-updater';
import { showToast, showToastReturningId } from './log';

let _initialized = false;

// stub — startDownload implemented in Step 3
// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function startDownload(_update: any): Promise<void> {}

async function checkForUpdates(explicit: boolean): Promise<void> {
  try {
    const update = await check();
    if (update !== null) {
      showToastReturningId('info', `Update ${update.version} available`, {
        persistent: true,
        action: { label: 'Install', onClick: () => startDownload(update) },
      });
    } else if (explicit) {
      showToast('info', 'Sift is up to date.');
    }
  } catch {
    if (explicit) {
      showToast('warn', 'Could not reach update server.');
    }
    // background check failure silently swallowed
  }
}

export async function initUpdaterStore(): Promise<void> {
  if (_initialized) return;
  _initialized = true;

  await listen('check-for-updates', () => checkForUpdates(true));

  setTimeout(() => checkForUpdates(false), 4000);
}
