import { listen } from '@tauri-apps/api/event';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { showToast, showToastReturningId, updateToastMessage, dismissToast } from './log';

let _initialized = false;

async function startDownload(update: Update): Promise<void> {
  const id = showToastReturningId('info', 'Downloading update…', { persistent: true });
  let downloaded = 0;
  let contentLength: number | undefined;

  try {
    await update.downloadAndInstall((event) => {
      if (event.event === 'Started') {
        contentLength = event.data.contentLength;
      } else if (event.event === 'Progress') {
        downloaded += event.data.chunkLength;
        if (contentLength !== undefined) {
          updateToastMessage(
            id,
            `Downloading update… ${Math.round((downloaded / contentLength) * 100)}%`,
          );
        }
      } else if (event.event === 'Finished') {
        dismissToast(id);
        showToastReturningId('success', 'Update ready — restart to apply', {
          persistent: true,
          action: { label: 'Restart Now', onClick: () => relaunch() },
        });
      }
    });
  } catch (err) {
    dismissToast(id);
    showToast('error', String(err));
  }
}

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
