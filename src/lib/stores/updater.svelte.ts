import { listen } from '@tauri-apps/api/event';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { showToast, showToastReturningId, updateToastMessage, dismissToast } from './log';

let _initialized = false;
let _downloadInProgress = false; // guard against concurrent Install clicks

async function startDownload(update: Update): Promise<void> {
  if (_downloadInProgress) return;
  _downloadInProgress = true;

  const id = showToastReturningId('info', 'Downloading update…', { persistent: true });
  let downloaded = 0;
  let contentLength: number | undefined;
  let isFinished = false;

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
        } else {
          // server sent no Content-Length — show bytes instead of percentage
          updateToastMessage(id, `Downloading update… ${Math.round(downloaded / 1024)} KB`);
        }
      } else if (event.event === 'Finished') {
        isFinished = true;
        dismissToast(id);
        showToastReturningId('success', 'Update ready — restart to apply', {
          persistent: true,
          action: {
            label: 'Restart Now',
            onClick: async () => {
              try {
                await relaunch();
              } catch (err) {
                showToast('error', `Restart failed: ${String(err)}`);
              }
            },
          },
        });
      }
    });
  } catch (err) {
    // Only dismiss/error if Finished never fired — avoids clobbering the
    // success toast in the unlikely case the promise rejects after Finished.
    if (!isFinished) {
      dismissToast(id);
      showToast('error', String(err));
    }
  } finally {
    _downloadInProgress = false;
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
  } catch (err) {
    console.error('[updater] check() threw:', err);
    if (explicit) {
      showToast('warn', String(err));
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
