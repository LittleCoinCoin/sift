// Self-driving update-UX harness. Runs the REAL updater store and the REAL
// Toast.svelte; only the Tauri plugin modules are replaced (see vite.e2e.config.ts).
//
// One scenario per page load (`e2e.html?scenario=<name>`): initUpdaterStore() is
// once-only and the store keeps module state. The harness
//   - observes what a user would see: the `.toast` elements Toast.svelte renders;
//   - acts as a user would: it clicks the real `.toast-action` / `.toast-dismiss` buttons;
//   - quotes copy only through INSTALL_MESSAGES (never a duplicated string);
//   - publishes `window.__e2e` and the same JSON in `<pre id="e2e-result">`.
import { mount } from 'svelte';
import { get } from 'svelte/store';
import Toast from '../Toast.svelte';
import '../tokens.css';
import '../theme.css';
import { initUpdaterStore } from '../stores/updater.svelte';
import { INSTALL_MESSAGES as M } from '../stores/updater-errors';
import { logEntries } from '../stores/log';
import { configureUpdater, updaterStats, type DownloadScript, type UpdaterScript } from './fakes/updater';
import { relaunchCalls } from './fakes/process';
import { emitForTest } from './fakes/event';

// ---------------------------------------------------------------- observation

export type ToastLevel = 'info' | 'warn' | 'error' | 'success' | 'unknown';
export type ToastSnap = { level: ToastLevel; text: string; action: string | null };

export type Result = {
  scenario: string;
  description: string;
  pass: boolean;
  failures: string[];
  /** Every distinct DOM state of the toast stack, in order (each entry is the whole stack). */
  toasts: ToastSnap[][];
  observed: { checkCalls: number; downloadCalls: number; relaunchCalls: number; logEntries: string[] };
  /** Number of expectations evaluated; 0 would mean a vacuous pass. */
  checks: number;
  done: true;
};

class Abort extends Error {}

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

function readStack(): ToastSnap[] {
  return [...document.querySelectorAll<HTMLElement>('.toast')].map((el) => ({
    level: (/toast--(info|warn|error|success)/.exec(el.className)?.[1] ?? 'unknown') as ToastLevel,
    text: el.querySelector('.toast-message')?.textContent?.trim() ?? '',
    action: el.querySelector('.toast-action')?.textContent?.trim() ?? null,
  }));
}

class Harness {
  readonly snapshots: ToastSnap[][] = [];
  readonly failures: string[] = [];
  checks = 0;
  private lastKey = '[]';
  private readonly observer: MutationObserver;

  constructor() {
    this.observer = new MutationObserver(() => this.snapshot());
    this.observer.observe(document.body, { childList: true, subtree: true, characterData: true });
  }

  snapshot(): void {
    const stack = readStack();
    const key = JSON.stringify(stack);
    if (key === this.lastKey) return;
    this.lastKey = key;
    this.snapshots.push(stack);
  }

  stop(): void {
    this.snapshot();
    this.observer.disconnect();
  }

  // --- reading the screen
  now(): ToastSnap[] {
    return readStack();
  }
  showing(text: string): boolean {
    return this.now().some((t) => t.text === text);
  }
  showingLevel(level: ToastLevel): boolean {
    return this.now().some((t) => t.level === level);
  }
  everShown(text: string): boolean {
    return this.snapshots.some((s) => s.some((t) => t.text === text));
  }
  /** Snapshots (distinct DOM states) in which `text` is on screen. */
  statesWith(text: string): ToastSnap[][] {
    return this.snapshots.filter((s) => s.some((t) => t.text === text));
  }
  /** Texts in the order they first appeared, consecutive repeats collapsed. */
  textStream(): string[] {
    const out: string[] = [];
    for (const snap of this.snapshots) {
      for (const t of snap) if (out[out.length - 1] !== t.text) out.push(t.text);
    }
    return out;
  }
  last(): ToastSnap[] {
    return this.snapshots[this.snapshots.length - 1] ?? [];
  }
  nonEmptyStates(): ToastSnap[][] {
    return this.snapshots.filter((s) => s.length > 0);
  }

  // --- acting like a user
  click(selector: '.toast-action' | '.toast-dismiss', label?: string): HTMLButtonElement {
    const buttons = [...document.querySelectorAll<HTMLButtonElement>(selector)];
    const button = label === undefined ? buttons[0] : buttons.find((b) => b.textContent?.trim() === label);
    if (!button) throw new Abort(`no ${selector}${label ? ` "${label}"` : ''} button on screen to click`);
    button.click();
    return button;
  }

  // --- waiting
  async waitFor(label: string, pred: () => boolean, timeoutMs: number): Promise<number> {
    const start = performance.now();
    for (;;) {
      if (pred()) return performance.now() - start;
      if (performance.now() - start > timeoutMs) throw new Abort(`timed out after ${timeoutMs} ms waiting for ${label}`);
      await sleep(25);
    }
  }
  sleep = sleep;

  // --- expectations (recorded, never thrown, so every check runs)
  expect(ok: boolean, message: string): void {
    this.checks += 1;
    if (!ok && !this.failures.includes(message)) this.failures.push(message);
  }
  expectOrder(label: string, wanted: string[]): void {
    const stream = this.textStream();
    let at = 0;
    for (const text of wanted) {
      const found = stream.indexOf(text, at);
      if (found < 0) {
        this.expect(false, `${label}: "${text}" missing or out of order; saw ${JSON.stringify(stream)}`);
        return;
      }
      at = found + 1;
    }
    this.expect(true, label);
  }
}

// ----------------------------------------------------------------- scenarios

const VERSION = '9.9.9';
const MB = 1_000_000;

type Scenario = {
  description: string;
  script: UpdaterScript;
  drive: (h: Harness) => Promise<void>;
  check: (h: Harness) => void;
};

const isProgress = (text: string) => text.startsWith(M.downloading) || text === M.installing;

function downloadOf(
  events: DownloadScript['events'],
  outcome: DownloadScript['outcome'],
  stepMs = 50,
  lateEvents?: DownloadScript['events'],
): DownloadScript {
  return { events, stepMs, outcome, lateEvents };
}

const withContentLength: DownloadScript['events'] = [
  { event: 'Started', data: { contentLength: MB } },
  { event: 'Progress', data: { chunkLength: MB / 4 } },
  { event: 'Progress', data: { chunkLength: MB / 4 } },
  { event: 'Progress', data: { chunkLength: MB / 4 } },
  { event: 'Progress', data: { chunkLength: MB / 4 } },
  { event: 'Finished' },
];

const updateStep = (download: DownloadScript) => ({ kind: 'update', version: VERSION, download }) as const;

/** The Install flow up to the click, shared by every scenario that downloads. */
async function openAvailableAndInstall(h: Harness): Promise<void> {
  await h.waitFor(`the "${M.available(VERSION)}" toast (background check, 4 s)`, () => h.showing(M.available(VERSION)), 8000);
  h.expect(h.now().some((t) => t.action === M.installLabel), `available toast offers "${M.installLabel}"`);
  h.click('.toast-action', M.installLabel);
}

function installFailure(description: string, error: string, expectedCopy: string): Scenario {
  return {
    description,
    script: {
      checkLatencyMs: 20,
      checks: [
        updateStep(
          downloadOf(
            [
              { event: 'Started', data: { contentLength: MB } },
              { event: 'Progress', data: { chunkLength: MB / 2 } },
              { event: 'Progress', data: { chunkLength: MB / 2 } },
              { event: 'Finished' }, // download done; the INSTALL step is what fails
            ],
            { reject: error },
          ),
        ),
      ],
    },
    async drive(h) {
      await openAvailableAndInstall(h);
      await h.waitFor('the install to settle (error or ready toast)', () => h.showingLevel('error') || h.showing(M.ready), 5000);
      await h.sleep(500); // anything wrongly scheduled after the failure would land here
    },
    check(h) {
      h.expect(!h.everShown(M.ready), 'ready shown after failure');
      h.expect(h.everShown(M.installing), 'failure arrived in the install phase (Installing toast seen first)');
      const last = h.last();
      h.expect(
        last.length === 1 && last[0].level === 'error' && last[0].text === expectedCopy,
        `final screen is exactly one error toast with the expected copy; got ${JSON.stringify(last)}`,
      );
      h.expect(!last.some((t) => isProgress(t.text)), 'no progress toast left behind after the failure');
      h.expect(updaterStats().downloadCalls === 1, 'exactly one downloadAndInstall call');
      h.expect(relaunchCalls() === 0, 'no relaunch after a failed install');
    },
  };
}

function upToDate(description: string, step: UpdaterScript['checks'][number]): Scenario {
  return {
    description,
    script: { checkLatencyMs: 20, checks: [step] },
    async drive(h) {
      await h.waitFor('the background check', () => updaterStats().checkCalls >= 1, 8000);
      await h.sleep(500);
      h.expect(h.nonEmptyStates().length === 0, 'background check with nothing newer shows no toast');
      emitForTest('check-for-updates');
      const appeared = performance.now();
      await h.waitFor(`the "${M.upToDate}" toast`, () => h.showing(M.upToDate), 3000);
      await h.waitFor('the up-to-date toast to auto-dismiss', () => h.now().length === 0, 7000);
      const shownFor = performance.now() - appeared;
      h.expect(shownFor >= 3500 && shownFor <= 5500, `info toast auto-dismisses after ~4 s (took ${Math.round(shownFor)} ms)`);
    },
    check(h) {
      const states = h.nonEmptyStates();
      h.expect(
        states.length === 1 && states[0].length === 1 && states[0][0].level === 'info' && states[0][0].text === M.upToDate,
        `the only toast ever shown is the info "${M.upToDate}"; got ${JSON.stringify(states)}`,
      );
      h.expect(updaterStats().checkCalls === 2, 'one background check plus one manual check');
    },
  };
}

export const SCENARIOS: Record<string, Scenario> = {
  success: {
    description: 'Content-Length known: Install shows percentages, then installing, then ready; late Finished ignored; Restart relaunches',
    script: {
      checkLatencyMs: 20,
      checks: [updateStep(downloadOf(withContentLength, { resolve: true }, 50, [{ event: 'Finished' }]))],
    },
    async drive(h) {
      await openAvailableAndInstall(h);
      await h.waitFor(`the "${M.ready}" toast`, () => h.showing(M.ready), 8000);
      h.expect(h.now().some((t) => t.action === M.restartLabel), `ready toast offers "${M.restartLabel}"`);
      h.click('.toast-action', M.restartLabel);
      await h.waitFor('relaunch() to be called', () => relaunchCalls() === 1, 2000);
      await h.sleep(500); // the scripted late Finished arrives in here
    },
    check(h) {
      h.expectOrder('toast sequence', [
        M.available(VERSION),
        M.downloading,
        M.downloadingPercent(25),
        M.downloadingPercent(50),
        M.downloadingPercent(75),
        M.downloadingPercent(100),
        M.installing,
        M.ready,
      ]);
      const afterReady = h.snapshots.slice(h.snapshots.findIndex((s) => s.some((t) => t.text === M.ready)));
      h.expect(
        afterReady.length > 0 && afterReady.every((s) => s.length === 1 && s[0].text === M.ready),
        'once ready, the screen stays exactly one ready toast (late Finished and later events are ignored)',
      );
      h.expect(h.last().length === 1 && h.last()[0].level === 'success', 'final toast is the success-level ready toast');
      h.expect(!h.snapshots.some((s) => s.some((t) => t.level === 'error')), 'no error toast');
      h.expect(updaterStats().downloadCalls === 1, 'exactly one downloadAndInstall call');
      h.expect(relaunchCalls() === 1, 'Restart Now called relaunch() exactly once');
    },
  },

  'no-content-length': {
    description: 'Server sends no Content-Length: progress is shown in KB, never as a percentage',
    script: {
      checkLatencyMs: 20,
      checks: [
        updateStep(
          downloadOf(
            [
              { event: 'Started', data: {} },
              { event: 'Progress', data: { chunkLength: 524288 } },
              { event: 'Progress', data: { chunkLength: 524288 } },
              { event: 'Finished' },
            ],
            { resolve: true },
          ),
        ),
      ],
    },
    async drive(h) {
      await openAvailableAndInstall(h);
      await h.waitFor(`the "${M.ready}" toast`, () => h.showing(M.ready), 8000);
      await h.sleep(300);
    },
    check(h) {
      h.expectOrder('toast sequence', [
        M.available(VERSION),
        M.downloading,
        M.downloadingKb(512),
        M.downloadingKb(1024),
        M.installing,
        M.ready,
      ]);
      h.expect(!h.textStream().some((t) => t.includes('%')), 'no percentage is ever shown without a Content-Length');
      h.expect(h.last().length === 1 && h.last()[0].text === M.ready, 'final screen is exactly the ready toast');
    },
  },

  'readonly-volume': installFailure(
    'Install fails because the app sits on a read-only volume (a mounted DMG): permission copy, never ready',
    'Read-only file system (os error 30)',
    M.permission,
  ),
  'permission-denied': installFailure(
    'Install fails with a permission error: permission copy, never ready',
    'Permission denied (os error 13)',
    M.permission,
  ),
  'admin-cancel': installFailure(
    'User cancels the administrator prompt: permission copy, never ready',
    'Failed to move the new app into place',
    M.permission,
  ),
  'signature-fail': installFailure(
    'Signature verification fails: verification copy, never ready',
    'The signature verification failed: signature was created with a different key than the one provided',
    M.verification,
  ),

  'check-fail-background': {
    description: 'Background check fails (offline): no toast at all, one log entry',
    script: {
      checkLatencyMs: 20,
      checks: [{ kind: 'reject', message: 'error sending request for url (https://example.invalid/latest.json): dns error' }],
    },
    async drive(h) {
      await h.waitFor('the background check', () => updaterStats().checkCalls >= 1, 8000);
      await h.waitFor('the failure to be logged', () => get(logEntries).length > 0, 2000);
      await h.sleep(1000); // a wrongly produced toast would render in here
    },
    check(h) {
      h.expect(h.nonEmptyStates().length === 0, `no toast is ever shown; got ${JSON.stringify(h.nonEmptyStates())}`);
      const entries = get(logEntries);
      h.expect(
        entries.length === 1 && entries[0].level === 'warn' && entries[0].message.startsWith(M.backgroundCheckFailedPrefix),
        `exactly one warn log entry starting with the background-failure prefix; got ${JSON.stringify(entries.map((e) => e.message))}`,
      );
      h.expect(updaterStats().checkCalls === 1, 'exactly the one background check ran');
    },
  },

  'check-fail-manual': {
    description: 'Manual check fails while an Install toast is open: the stale Install toast is replaced by a warn toast that auto-dismisses',
    script: {
      checkLatencyMs: 20,
      checks: [
        updateStep(downloadOf([], { resolve: true })),
        { kind: 'reject', message: 'request timed out' },
      ],
    },
    async drive(h) {
      await h.waitFor(`the "${M.available(VERSION)}" toast`, () => h.showing(M.available(VERSION)), 8000);
      emitForTest('check-for-updates');
      const warning = `${M.checkFailedPrefix}request timed out`;
      await h.waitFor('the manual-check failure toast', () => h.showing(warning), 3000);
      const appeared = performance.now();
      h.expect(
        h.now().length === 1 && h.now()[0].level === 'warn' && h.now()[0].action === null,
        `stale Install toast is gone; only the warn toast remains; got ${JSON.stringify(h.now())}`,
      );
      await h.waitFor('the warn toast to auto-dismiss', () => h.now().length === 0, 12000);
      const shownFor = performance.now() - appeared;
      h.expect(shownFor >= 7500 && shownFor <= 9500, `warn toast auto-dismisses after ~8 s (took ${Math.round(shownFor)} ms)`);
    },
    check(h) {
      const warning = `${M.checkFailedPrefix}request timed out`;
      h.expectOrder('toast sequence', [M.available(VERSION), warning]);
      h.expect(h.statesWith(M.available(VERSION)).every((s) => !s.some((t) => t.text === warning)), 'available and warning toasts never coexist');
      h.expect(updaterStats().downloadCalls === 0, 'nothing was downloaded');
    },
  },

  'up-to-date': upToDate('Check finds nothing: "up to date" is shown only for the manual check', { kind: 'none' }),
  'latest-older': upToDate('Server latest is older than the running version: treated as up to date, shown only on a manual check', {
    kind: 'older',
    version: '0.0.1',
  }),

  'double-install-click': {
    description: 'Install clicked repeatedly (same tick and later, on the stale button): exactly one download',
    script: {
      checkLatencyMs: 20,
      checks: [updateStep(downloadOf(withContentLength, { resolve: true }, 100))],
    },
    async drive(h) {
      await h.waitFor(`the "${M.available(VERSION)}" toast`, () => h.showing(M.available(VERSION)), 8000);
      const button = h.click('.toast-action', M.installLabel);
      button.click(); // double-click: second click lands before Svelte removes the button
      await h.sleep(10);
      button.click(); // a third click on the now-detached button
      await h.waitFor(`the "${M.ready}" toast`, () => h.showing(M.ready), 8000);
      await h.sleep(300);
    },
    check(h) {
      h.expect(updaterStats().downloadCalls === 1, `exactly one downloadAndInstall call; got ${updaterStats().downloadCalls}`);
      h.expect(h.statesWith(M.ready).length === 1, 'the ready toast appears exactly once');
      h.expect(h.last().length === 1 && h.last()[0].text === M.ready, 'final screen is exactly the ready toast');
      h.expect(!h.snapshots.some((s) => s.some((t) => t.level === 'error')), 'no error toast');
    },
  },

  'recheck-while-toast-open': {
    description: 'Manual re-checks with an Install toast open, mid-download and after ready keep one updater toast; a dismissed busy toast is re-shown',
    script: {
      checkLatencyMs: 50,
      checks: [updateStep(downloadOf(withContentLength, { resolve: true }, 300))],
    },
    async drive(h) {
      const only = (text: string, why: string) =>
        h.expect(h.now().length === 1 && h.now()[0].text === text, `${why}; got ${JSON.stringify(h.now().map((t) => t.text))}`);

      await h.waitFor(`the "${M.available(VERSION)}" toast`, () => h.showing(M.available(VERSION)), 8000);

      // 1. re-check with the Install toast open (twice, the second while the first is in flight)
      emitForTest('check-for-updates');
      emitForTest('check-for-updates');
      await h.waitFor('both manual checks to finish', () => updaterStats().checkCalls >= 3, 2000);
      await h.sleep(300);
      only(M.available(VERSION), 're-check with the Install toast open leaves one Install toast');
      h.expect(h.now()[0].action === M.installLabel, 'the surviving toast still offers Install');

      // 2. re-check while downloading, then dismiss the progress toast and re-check again
      h.click('.toast-action', M.installLabel);
      await h.waitFor('the download to start', () => h.now().some((t) => t.text.startsWith(M.downloading)), 2000);
      emitForTest('check-for-updates');
      await h.sleep(100);
      h.expect(h.now().length === 1 && isProgress(h.now()[0].text), 're-check mid-download leaves the single progress toast');
      h.click('.toast-dismiss');
      await h.sleep(50);
      h.expect(h.now().length === 0, 'the user can dismiss the progress toast');
      emitForTest('check-for-updates');
      await h.sleep(100);
      h.expect(h.now().length === 1 && isProgress(h.now()[0].text), 're-check re-shows the in-flight progress toast the user dismissed');

      // 3. re-check after ready, then dismiss ready and re-check again
      await h.waitFor(`the "${M.ready}" toast`, () => h.showing(M.ready), 8000);
      emitForTest('check-for-updates');
      await h.sleep(200);
      only(M.ready, 're-check after ready does not clobber the ready toast');
      h.click('.toast-dismiss');
      await h.sleep(50);
      h.expect(h.now().length === 0, 'the user can dismiss the ready toast');
      emitForTest('check-for-updates');
      await h.sleep(200);
      only(M.ready, 're-check re-shows the ready toast the user dismissed');
      h.expect(h.now()[0].action === M.restartLabel, 're-shown ready toast offers Restart Now again');
    },
    check(h) {
      h.expect(updaterStats().downloadCalls === 1, `exactly one downloadAndInstall call; got ${updaterStats().downloadCalls}`);
      h.expect(!h.snapshots.some((s) => s.some((t) => t.level === 'error')), 'no error toast');
    },
  },
};

// -------------------------------------------------------------------- runner

export async function runScenario(name: string): Promise<Result> {
  const scenario = SCENARIOS[name];
  const h = new Harness();
  const uncaught: string[] = [];
  window.addEventListener('error', (e) => uncaught.push(String(e.message)));
  window.addEventListener('unhandledrejection', (e) => uncaught.push(`unhandled rejection: ${String(e.reason)}`));

  if (!scenario) {
    h.stop();
    return finish(name, '(unknown scenario)', h, [`unknown scenario "${name}"; known: ${Object.keys(SCENARIOS).join(', ')}`]);
  }

  configureUpdater(scenario.script);
  mount(Toast, { target: document.body });
  await initUpdaterStore();

  const aborted: string[] = [];
  try {
    await scenario.drive(h);
  } catch (err) {
    aborted.push(err instanceof Abort ? err.message : `harness error: ${String(err)}`);
  }
  await h.sleep(50);
  h.stop();
  scenario.check(h);
  // Invariants that hold in every scenario.
  const widest = Math.max(0, ...h.snapshots.map((s) => s.length));
  h.expect(widest <= 1, `at most one updater toast on screen at a time; saw ${widest}`);
  h.expect(uncaught.length === 0, `no uncaught errors; got ${JSON.stringify(uncaught)}`);
  return finish(name, scenario.description, h, [...aborted, ...h.failures]);
}

function finish(name: string, description: string, h: Harness, failures: string[]): Result {
  if (h.checks === 0) failures.push('vacuous run: no expectation was evaluated');
  return {
    scenario: name,
    description,
    pass: failures.length === 0,
    failures,
    toasts: h.snapshots,
    observed: {
      checkCalls: updaterStats().checkCalls,
      downloadCalls: updaterStats().downloadCalls,
      relaunchCalls: relaunchCalls(),
      logEntries: get(logEntries).map((e) => `${e.level}: ${e.message}`),
    },
    checks: h.checks,
    done: true,
  };
}

// ---------------------------------------------------------------------- page

function el<K extends keyof HTMLElementTagNameMap>(tag: K, text?: string, id?: string): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (id !== undefined) node.id = id;
  return node;
}

async function main(): Promise<void> {
  const root = document.getElementById('e2e-root')!;
  root.style.cssText = 'font: 14px/1.5 var(--font-mono, monospace); padding: 16px; max-width: 640px';
  const name = new URLSearchParams(location.search).get('scenario');

  if (name === null) {
    root.append(el('h1', 'Sift updater UX harness'), el('p', 'Pick a scenario (each runs the real store and Toast once per page load):'));
    const list = el('ul');
    for (const [scenario, def] of Object.entries(SCENARIOS)) {
      const li = el('li');
      const a = el('a', scenario);
      a.href = `?scenario=${encodeURIComponent(scenario)}`;
      li.append(a, ` — ${def.description}`);
      list.append(li);
    }
    root.append(list);
    return;
  }

  const status = el('p', `Running "${name}"… (the real store waits 4 s before its background check)`, 'e2e-status');
  const pre = el('pre', '', 'e2e-result');
  pre.style.cssText = 'white-space: pre-wrap; word-break: break-word';
  root.append(el('h1', `Scenario: ${name}`), status, pre);

  const result = await runScenario(name);
  (window as unknown as { __e2e: Result }).__e2e = result;
  status.textContent = `${result.pass ? 'PASS' : 'FAIL'}: ${name}`;
  status.style.color = result.pass ? 'green' : 'crimson';
  pre.textContent = JSON.stringify(result, null, 2);
}

void main();
