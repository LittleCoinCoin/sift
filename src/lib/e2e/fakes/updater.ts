// Scripted stand-in for `@tauri-apps/plugin-updater`, aliased in vite.e2e.config.ts.
// Behaves like the real plugin where the store can observe it:
//   - check() resolves to `Update | null` (null when the server's version is not newer);
//   - downloadAndInstall() streams Started/Progress/Finished and then resolves or
//     REJECTS WITH A PLAIN STRING, as Tauri does (never an Error instance);
//   - Finished marks the end of the download only; the install happens after it, so
//     an install failure arrives after Finished.
import type { check as realCheck, DownloadEvent, Update } from '@tauri-apps/plugin-updater';

export const CURRENT_VERSION = '0.1.4';

export type DownloadScript = {
  /** Emitted in order, `stepMs` apart, before the outcome. */
  events: DownloadEvent[];
  stepMs: number;
  outcome: { resolve: true } | { reject: string };
  /** Delivered after the promise settled (Tauri channels are separate from the invoke response). */
  lateEvents?: DownloadEvent[];
};

export type CheckStep =
  | { kind: 'update'; version: string; download: DownloadScript }
  /** The server's latest is older than (or equal to) the running version: check() yields null. */
  | { kind: 'older'; version: string }
  | { kind: 'none' }
  | { kind: 'reject'; message: string };

export type UpdaterScript = {
  /** One entry per check() call; the last entry repeats. */
  checks: CheckStep[];
  checkLatencyMs: number;
};

const NEVER_DOWNLOADED: DownloadScript = { events: [], stepMs: 0, outcome: { resolve: true } };

const stats = { checkCalls: 0, downloadCalls: 0 };
let script: UpdaterScript = { checks: [{ kind: 'none' }], checkLatencyMs: 0 };

export function configureUpdater(next: UpdaterScript): void {
  script = next;
  stats.checkCalls = 0;
  stats.downloadCalls = 0;
}

export function updaterStats(): Readonly<typeof stats> {
  return stats;
}

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

function parse(version: string): number[] {
  return version.split('.').map((part) => Number.parseInt(part, 10) || 0);
}

function isNewer(candidate: string, current: string): boolean {
  const a = parse(candidate);
  const b = parse(current);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const diff = (a[i] ?? 0) - (b[i] ?? 0);
    if (diff !== 0) return diff > 0;
  }
  return false;
}

function fakeUpdate(version: string, download: DownloadScript): Update {
  const downloadAndInstall: Update['downloadAndInstall'] = async (onEvent) => {
    stats.downloadCalls += 1;
    for (const event of download.events) {
      await sleep(download.stepMs);
      onEvent?.(event);
    }
    await sleep(download.stepMs);
    if ('reject' in download.outcome) throw download.outcome.reject; // plain string, like Tauri
    if (download.lateEvents) {
      const late = download.lateEvents;
      setTimeout(() => late.forEach((event) => onEvent?.(event)), download.stepMs * 2);
    }
  };
  const update = {
    available: true,
    currentVersion: CURRENT_VERSION,
    version,
    rawJson: {},
    downloadAndInstall,
    download: async () => {},
    install: async () => {},
    close: async () => {},
  } satisfies Pick<
    Update,
    | 'available'
    | 'currentVersion'
    | 'version'
    | 'rawJson'
    | 'downloadAndInstall'
    | 'download'
    | 'install'
    | 'close'
  >;
  // The real Update extends Tauri's Resource (private fields); the store uses none of that.
  return update as unknown as Update;
}

export const check: typeof realCheck = async () => {
  const index = stats.checkCalls++;
  if (script.checkLatencyMs > 0) await sleep(script.checkLatencyMs);
  const step = script.checks[Math.min(index, script.checks.length - 1)];
  switch (step.kind) {
    case 'update':
      return fakeUpdate(step.version, step.download);
    case 'older':
      return isNewer(step.version, CURRENT_VERSION) ? fakeUpdate(step.version, NEVER_DOWNLOADED) : null;
    case 'none':
      return null;
    case 'reject':
      throw step.message; // plain string, like Tauri
  }
};
