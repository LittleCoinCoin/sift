import { listen } from '@tauri-apps/api/event';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { get } from 'svelte/store';
import {
  showToast,
  showToastReturningId,
  updateToastMessage,
  dismissToast,
  logEntry,
  toasts,
  type LogLevel,
} from './log';
import { classifyInstallError, INSTALL_MESSAGES } from './updater-errors';

// idle        nothing in flight
// available   an "Update X available" toast is open
// installing  download + install running (blocks Install clicks and re-checks)
// ready       install succeeded, waiting for the user to restart (blocks re-checks:
//             check() would return the same update and clobber the ready toast)
type Phase = 'idle' | 'available' | 'installing' | 'ready';

let _initialized = false;
let _phase: Phase = 'idle';
let _toastId: number | null = null; // the single updater toast
let _attempt = 0; // identifies the current install attempt
let _progressMessage: string = INSTALL_MESSAGES.downloading; // last in-flight copy

type ToastOpts = { action?: { label: string; onClick: () => void }; persistent?: boolean };

// Show an updater toast, replacing whichever updater toast is currently open
// so at most one is ever on screen.
function showUpdaterToast(level: LogLevel, message: string, opts?: ToastOpts): void {
  if (_toastId !== null) dismissToast(_toastId);
  _toastId = showToastReturningId(level, message, opts);
}

function isUpdaterToastOpen(): boolean {
  const id = _toastId;
  return id !== null && get(toasts).some((t) => t.id === id);
}

// A boolean helper (not an inline comparison) so TypeScript does not narrow
// _phase across the awaits below.
function isBusy(): boolean {
  return _phase === 'installing' || _phase === 'ready';
}

function setProgress(message: string): void {
  _progressMessage = message;
  if (_toastId !== null) updateToastMessage(_toastId, message);
}

function showReadyToast(): void {
  showUpdaterToast('success', INSTALL_MESSAGES.ready, {
    persistent: true,
    action: {
      label: INSTALL_MESSAGES.restartLabel,
      onClick: async () => {
        try {
          await relaunch();
        } catch (err) {
          showToast('error', `${INSTALL_MESSAGES.restartFailedPrefix}${String(err)}`);
        }
      },
    },
  });
}

// An explicit check while an install is running or finished must not be silent
// (the user may have dismissed the toast): bring the current state back.
function resurfaceBusyState(): void {
  if (isUpdaterToastOpen()) return;
  if (_phase === 'ready') {
    showReadyToast();
  } else if (_phase === 'installing') {
    showUpdaterToast('info', _progressMessage, { persistent: true });
  }
}

async function startDownload(update: Update): Promise<void> {
  if (isBusy()) return;
  _phase = 'installing';
  const attempt = ++_attempt;
  // Callbacks travel over a channel separate from the invoke response, so one
  // can arrive after the promise settled; `settled` makes those no-ops.
  let settled = false;
  const live = () => !settled && attempt === _attempt && _phase === 'installing';

  _progressMessage = INSTALL_MESSAGES.downloading;
  showUpdaterToast('info', INSTALL_MESSAGES.downloading, { persistent: true });
  let downloaded = 0;
  let contentLength: number | undefined;
  let installing = false;

  try {
    await update.downloadAndInstall((event) => {
      if (!live()) return;
      if (event.event === 'Started') {
        contentLength = event.data.contentLength;
      } else if (event.event === 'Progress') {
        if (installing) return;
        downloaded += event.data.chunkLength;
        if (contentLength !== undefined) {
          setProgress(INSTALL_MESSAGES.downloadingPercent(Math.round((downloaded / contentLength) * 100)));
        } else {
          // server sent no Content-Length — show bytes instead of percentage
          setProgress(INSTALL_MESSAGES.downloadingKb(Math.round(downloaded / 1024)));
        }
      } else if (event.event === 'Finished') {
        // Finished marks the end of the DOWNLOAD only; install() runs after it.
        installing = true;
        _progressMessage = INSTALL_MESSAGES.installing;
        showUpdaterToast('info', INSTALL_MESSAGES.installing, { persistent: true });
      }
    });

    // downloadAndInstall resolved: the new bundle is in place.
    settled = true;
    _phase = 'ready';
    showReadyToast();
  } catch (err) {
    settled = true;
    console.error('[updater] downloadAndInstall() threw:', err);
    _phase = 'idle';
    showUpdaterToast('error', classifyInstallError(err));
  }
}

async function checkForUpdates(explicit: boolean): Promise<void> {
  // Never let a re-check clobber a running install or the "ready" toast.
  if (isBusy()) {
    if (explicit) resurfaceBusyState();
    return;
  }
  try {
    const update = await check();
    // The phase may have moved on while check() was awaiting.
    if (isBusy()) {
      if (explicit) resurfaceBusyState();
      return;
    }
    if (update !== null) {
      _phase = 'available';
      showUpdaterToast('info', INSTALL_MESSAGES.available(update.version), {
        persistent: true,
        action: {
          label: INSTALL_MESSAGES.installLabel,
          onClick: () => {
            if (_phase !== 'available') return;
            // Dismiss the "available" toast before the progress toast takes over.
            if (_toastId !== null) dismissToast(_toastId);
            _toastId = null;
            void startDownload(update);
          },
        },
      });
    } else if (explicit) {
      // an "available" toast, if any, is stale now and gets replaced
      _phase = 'idle';
      showUpdaterToast('info', INSTALL_MESSAGES.upToDate);
    }
  } catch (err) {
    console.error('[updater] check() threw:', err);
    if (explicit) {
      // replaces any open updater toast, so the "available" Install is gone
      _phase = 'idle';
      showUpdaterToast('warn', `${INSTALL_MESSAGES.checkFailedPrefix}${String(err)}`);
    } else {
      // background check: no toast, but keep a record in the log store
      logEntry('warn', `${INSTALL_MESSAGES.backgroundCheckFailedPrefix}${String(err)}`);
    }
  }
}

export async function initUpdaterStore(): Promise<void> {
  if (_initialized) return;
  _initialized = true;

  await listen('check-for-updates', () => checkForUpdates(true));

  setTimeout(() => checkForUpdates(false), 4000);
}
