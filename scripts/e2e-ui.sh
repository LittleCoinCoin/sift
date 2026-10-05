#!/usr/bin/env bash
# Runs every update-UX scenario headless against the REAL updater store and the
# REAL Toast.svelte (only the Tauri plugins are faked; see vite.e2e.config.ts).
#
#   bash scripts/e2e-ui.sh                    all scenarios; exit 0 only if all PASS
#   bash scripts/e2e-ui.sh --negative-control swap in the pre-campaign store
#                                             (git show bfef3c5:...) and prove the
#                                             failure scenarios catch its defect
#   options: --verbose (print each scenario's toast sequence)
#            --only <scenario>[,<scenario>...]
#   env:     E2E_PORT (4174)  E2E_BUDGET virtual-time ms (20000)  CHROME (path)
#
# Exit codes: 0 all PASS | 1 a scenario FAILed (negative control: failure
# scenarios FAILed as intended, i.e. the defect was detected) | 2 harness or
# environment error | 3 negative control did NOT detect the old defect.
set -euo pipefail

cd "$(dirname "$0")/.."

CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
PORT="${E2E_PORT:-4174}"
BUDGET="${E2E_BUDGET:-20000}"
STORE="src/lib/stores/updater.svelte.ts"
PRE_CAMPAIGN_REV="bfef3c5"
SCENARIOS=(
  success no-content-length readonly-volume permission-denied admin-cancel
  signature-fail check-fail-background check-fail-manual up-to-date latest-older
  double-install-click recheck-while-toast-open
)
# Scenarios that inject an install failure; the pre-campaign store must fail them.
FAILURE_SCENARIOS="readonly-volume,permission-denied,admin-cancel,signature-fail"

NEGATIVE=0
ONLY=0
VERBOSE=0
while [ $# -gt 0 ]; do
  case "$1" in
    --negative-control) NEGATIVE=1 ;;
    --verbose) VERBOSE=1 ;;
    --only) ONLY=1; shift; IFS=, read -r -a SCENARIOS <<< "${1:?--only needs a scenario list}" ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

[ -x "$CHROME" ] || { echo "Chrome not found at: $CHROME (set CHROME=...)" >&2; exit 2; }

WORK="$(mktemp -d "${TMPDIR:-/tmp}/sift-e2e-ui.XXXXXX")"
SERVER_PID=""
STORE_BACKUP=""

kill_tree() {
  local pid="$1" child
  for child in $(pgrep -P "$pid" 2>/dev/null || true); do kill_tree "$child"; done
  kill "$pid" 2>/dev/null || true
}

cleanup() {
  local status=$?
  [ -n "$SERVER_PID" ] && kill_tree "$SERVER_PID"
  if [ -n "$STORE_BACKUP" ] && [ -f "$STORE_BACKUP" ]; then
    cp "$STORE_BACKUP" "$STORE"
  fi
  rm -rf "$WORK"
  exit "$status"
}
trap cleanup EXIT INT TERM

if curl -fs -o /dev/null "http://127.0.0.1:$PORT/" 2>/dev/null; then
  echo "port $PORT is already serving something; refusing to test a stale server (set E2E_PORT)" >&2
  exit 2
fi

OUT="dist-e2e"
if [ "$NEGATIVE" -eq 1 ]; then
  OUT="dist-e2e/negative"
  STORE_BACKUP="$WORK/updater.svelte.ts.current"
  cp "$STORE" "$STORE_BACKUP"
  git show "$PRE_CAMPAIGN_REV:$STORE" > "$STORE"
  echo "negative control: $STORE replaced by $PRE_CAMPAIGN_REV:$STORE"
fi

echo "building $OUT ..."
pnpm exec vite build --config vite.e2e.config.ts --outDir "$OUT" > "$WORK/build.log" 2>&1 \
  || { cat "$WORK/build.log" >&2; exit 2; }

if [ "$NEGATIVE" -eq 1 ]; then
  # The bundle holds the old store now; the working tree no longer needs it.
  cp "$STORE_BACKUP" "$STORE"
  STORE_BACKUP=""
  echo "working tree restored: $STORE"
fi

pnpm exec vite preview --config vite.e2e.config.ts --outDir "$OUT" --port "$PORT" --strictPort \
  > "$WORK/preview.log" 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 60); do
  curl -fs -o /dev/null "http://127.0.0.1:$PORT/e2e.html" 2>/dev/null && break
  kill -0 "$SERVER_PID" 2>/dev/null || { cat "$WORK/preview.log" >&2; echo "preview server died" >&2; exit 2; }
  sleep 0.5
done
curl -fs -o /dev/null "http://127.0.0.1:$PORT/e2e.html" || { echo "preview server never became ready" >&2; exit 2; }

# One headless page load. Output goes to a file, not a pipe: Chrome's helper
# processes inherit stdout and would keep a pipe open after Chrome exits.
dump_dom() { # <url> <budget-ms> <outfile> <tag>
  "$CHROME" --headless=new --disable-gpu --no-first-run --disable-background-networking \
    --disable-component-update --disable-sync --user-data-dir="$WORK/profile-$4" \
    --virtual-time-budget="$2" --dump-dom "$1" > "$3" 2> "$WORK/chrome-$4.err" &
  local chrome_pid=$! polls=0
  # Measured: with --headless=new --dump-dom this Chrome prints the DOM but then
  # never exits, so the DOM's closing tag is the "page is done" signal. Cap: 60 s.
  while kill -0 "$chrome_pid" 2>/dev/null && ! grep -q '</html>' "$3" 2>/dev/null; do
    polls=$((polls + 1))
    [ "$polls" -gt 300 ] && break
    sleep 0.2
  done
  sleep 0.3
  kill "$chrome_pid" 2>/dev/null || true
  pkill -f "user-data-dir=$WORK/profile-$4" 2>/dev/null || true
  wait "$chrome_pid" 2>/dev/null || true
}

# The runner's scenario list must match the harness's, or a scenario goes unrun.
dump_dom "http://127.0.0.1:$PORT/e2e.html" 5000 "$WORK/index.html" index
KNOWN="$(grep -o 'href="?scenario=[^"]*"' "$WORK/index.html" | sed 's/.*scenario=//; s/"//' | sort | tr '\n' ' ')"
if [ "$ONLY" -eq 0 ]; then
  WANT="$(printf '%s\n' "${SCENARIOS[@]}" | sort | tr '\n' ' ')"
  [ "$KNOWN" = "$WANT" ] || { echo "scenario list drift: harness has [$KNOWN] but runner has [$WANT]" >&2; exit 2; }
fi

for s in "${SCENARIOS[@]}"; do
  echo "running $s ... (t=${SECONDS}s)"
  dump_dom "http://127.0.0.1:$PORT/e2e.html?scenario=$s" "$BUDGET" "$WORK/$s.html" "$s"
done

set +e
E2E_WORK="$WORK" E2E_SCENARIOS="${SCENARIOS[*]}" E2E_NEGATIVE="$NEGATIVE" E2E_VERBOSE="$VERBOSE" \
  E2E_FAILURE_SCENARIOS="$FAILURE_SCENARIOS" node --input-type=module - <<'NODE'
import { readFileSync, existsSync } from 'node:fs';

const work = process.env.E2E_WORK;
const names = process.env.E2E_SCENARIOS.split(' ');
const negative = process.env.E2E_NEGATIVE === '1';
const verbose = process.env.E2E_VERBOSE === '1';
const failureScenarios = new Set(process.env.E2E_FAILURE_SCENARIOS.split(','));

const decode = (s) =>
  s.replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#0*39;/g, "'").replace(/&amp;/g, '&');

function load(name) {
  const file = `${work}/${name}.html`;
  if (!existsSync(file)) return { pass: false, failures: ['no DOM dump produced'] };
  const html = readFileSync(file, 'utf8');
  const m = /<pre id="e2e-result"[^>]*>([\s\S]*?)<\/pre>/.exec(html);
  if (!m || m[1].trim() === '') return { pass: false, failures: ['harness published no result (timed out or crashed)'] };
  try {
    const r = JSON.parse(decode(m[1]));
    if (r.scenario !== name) return { pass: false, failures: [`result is for "${r.scenario}", not "${name}"`] };
    if (!r.checks) r.failures = [...r.failures, 'vacuous run: no expectation was evaluated'], (r.pass = false);
    return r;
  } catch (err) {
    return { pass: false, failures: [`unparseable result: ${err.message}`] };
  }
}

const results = new Map(names.map((n) => [n, load(n)]));
const width = Math.max(...names.map((n) => n.length), 8);
const line = (a, b, c) => `${a.padEnd(width)}  ${b.padEnd(6)}  ${c}`;

console.log('');
console.log(line('scenario', 'result', 'detail'));
console.log(line('-'.repeat(width), '------', '------'));
for (const [name, r] of results) {
  const detail = r.pass ? `${r.checks} checks` : r.failures.join(' | ');
  console.log(line(name, r.pass ? 'PASS' : 'FAIL', detail));
}
const passed = [...results.values()].filter((r) => r.pass).length;
console.log(`\n${passed}/${results.size} scenarios PASS`);

if (verbose) {
  const fmt = (t) => `[${t.level}] ${t.text}${t.action ? ` {${t.action}}` : ''}`;
  for (const [name, r] of results) {
    console.log(`\n--- ${name}: toast sequence`);
    const states = r.toasts ?? [];
    if (states.length === 0) console.log('  (no toast ever shown)');
    states.forEach((s, i) => console.log(`  ${String(i + 1).padStart(2)}. ${s.length ? s.map(fmt).join('  +  ') : '(empty)'}`));
    if (r.observed) console.log(`  observed: ${JSON.stringify(r.observed)}`);
  }
}

if (!negative) process.exit(passed === results.size ? 0 : 1);

console.log('\nnegative control (pre-campaign store):');
let detected = 0;
let expected = 0;
for (const [name, r] of results) {
  if (!failureScenarios.has(name)) continue;
  expected += 1;
  const hit = !r.pass && r.failures.includes('ready shown after failure');
  if (hit) detected += 1;
  console.log(`  ${name.padEnd(width)}  ${hit ? 'DETECTED  (FAIL: ready shown after failure)' : 'MISSED    (' + (r.pass ? 'PASS' : r.failures.join(' | ')) + ')'}`);
}
if (expected === 0) {
  console.log('  no failure scenarios were selected');
  process.exit(3);
}
console.log(detected === expected ? `\nVERDICT: control DETECTED the old defect (${detected}/${expected}); exiting non-zero as intended` : `\nVERDICT: control did NOT detect the old defect (${detected}/${expected}); the harness is vacuous`);
process.exit(detected === expected ? 1 : 3);
NODE
STATUS=$?
set -e
exit "$STATUS"
