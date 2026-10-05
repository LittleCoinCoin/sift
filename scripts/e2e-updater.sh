#!/usr/bin/env bash
# Native update E2E: real signed binaries, real webview, real toast buttons.
#
#   Build A  release, version 0.2.0   -> its .app.tar.gz + .sig are the "update"
#   Build B  --debug, version 0.0.1   -> the "old" installed app (plain-http
#                                        endpoints are only accepted by debug builds)
#
# Happy path: B detects 0.2.0, downloads, verifies, installs over itself and
# restarts as A. Read-only case: B launched from a read-only disk image must
# fail the install with the INSTALL_MESSAGES.permission copy and never reach
# the ready toast. The in-app driver (src/lib/e2e/updater-driver.ts) clicks the
# buttons and reports to the mock server's /e2e-log.
#
# Safety: everything lives under $TMPDIR/sift-e2e; nothing is installed to
# /Applications; the keychain service is compiled to "sift-e2e" (never "sift");
# the signing key is a throwaway generated here and never committed; every
# process and the disk image are cleaned up in a trap.
#
# Usage: bash scripts/e2e-updater.sh          (exit 0 only if every assertion passed)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
W="${TMPDIR:-/tmp}"; W="${W%/}/sift-e2e"
PORT=1430
BASE="http://127.0.0.1:$PORT"
IDENTIFIER="dev.eliottjacopin.sift.e2e"
OLD_VERSION="0.0.1"
NEW_VERSION="0.2.0"
KEYCHAIN_SERVICE="sift-e2e"
RO_VOLNAME="SiftE2E"
RO_MOUNT="/Volumes/$RO_VOLNAME"
CHUNK_DELAY_MS=15
export PYTHONDONTWRITEBYTECODE=1

APP="$W/Sift.app"            # the installed "old" app of the happy path
ART_DIR="$W/artifact"
B_PRISTINE="$W/B/Sift.app"   # untouched copy of build B (the happy path overwrites $APP)
LOG_DIR="$W/logs"
SHOT_DIR="$W/shots"

FAILS=0
SERVER_LAUNCH_PID=""
SERVER_PID=""
RO_DEVICE=""
REAL_KEYCHAIN_BEFORE=""
SHOTS_OK=1

# ---------------------------------------------------------------- helpers

say()  { printf '[e2e %s] %s\n' "$(date +%H:%M:%S)" "$*"; }
pass() { printf '  PASS  %s\n' "$*"; }
fail() { printf '  FAIL  %s\n' "$*"; FAILS=$((FAILS + 1)); }
die()  { printf '[e2e] fatal: %s\n' "$*" >&2; exit 2; }

# Presence of a generic-password item (attributes only, so no keychain prompt).
keychain_presence() {
  if security find-generic-password -s "$1" >/dev/null 2>&1; then echo present; else echo absent; fi
}

# Toast copy comes from the app's own source of truth, never duplicated here.
copy() {
  node --no-warnings --input-type=module -e \
    'const { INSTALL_MESSAGES: M } = await import(process.argv[1]); process.stdout.write(String(eval(process.argv[2])));' \
    "file://$ROOT/src/lib/stores/updater-errors.ts" "$1"
}

fetch_log() { curl -s --max-time 2 "$BASE/e2e-log" 2>/dev/null || true; }

# wait_for_log <regex> <timeout-seconds>: poll the server's /e2e-log, bounded.
wait_for_log() {
  local pattern="$1" limit="$2" deadline=$((SECONDS + $2))
  while (( SECONDS < deadline )); do
    if fetch_log | grep -qE -e "$pattern"; then return 0; fi
    sleep 0.25
  done
  return 1
}

# wait_for_log_after <anchor-regex> <regex> <timeout>: a line matching <regex> that
# comes after the first line matching <anchor-regex>.
wait_for_log_after() {
  local anchor="$1" pattern="$2" deadline=$((SECONDS + $3))
  while (( SECONDS < deadline )); do
    if fetch_log | awk -v a="$anchor" -v b="$pattern" '$0 ~ a { seen = 1; next } seen && $0 ~ b { found = 1 } END { exit !found }'; then
      return 0
    fi
    sleep 0.25
  done
  return 1
}

# Escape a literal for use inside an ERE.
re_escape() { printf '%s' "$1" | sed 's/[][\.*^$()+?{|]/\\&/g'; }

log_has() { grep -qE -e "$2" "$1"; }          # <file> <regex>
first_line() { grep -nE -e "$2" "$1" | head -1 | cut -d: -f1; }   # <file> <regex> -> line number

shot() {
  local out="$SHOT_DIR/$1.png"
  if ! screencapture -x "$out" 2>>"$LOG_DIR/screencapture.err" || [ ! -s "$out" ]; then
    SHOTS_OK=0
    say "screencapture produced no image for $1 (see $LOG_DIR/screencapture.err)"
  fi
}

# PID of the app whose executable lives under the given .app bundle (0 if none).
app_pid() { pgrep -f "^(/private)?$1/Contents/MacOS/" | head -1 || true; }

kill_app() {
  pkill -f "^(/private)?$1/Contents/MacOS/" 2>/dev/null || true
  local deadline=$((SECONDS + 10))
  while (( SECONDS < deadline )) && [ -n "$(app_pid "$1")" ]; do sleep 0.25; done
  pkill -9 -f "^(/private)?$1/Contents/MacOS/" 2>/dev/null || true
}

start_server() {  # <logfile>
  uv run python "$ROOT/scripts/serve-mock-update.py" \
    --artifact "$ART_DIR/Sift.app.tar.gz" --version "$NEW_VERSION" --port "$PORT" \
    --chunk-delay-ms "$CHUNK_DELAY_MS" >"$1" 2>&1 &
  SERVER_LAUNCH_PID=$!
  local deadline=$((SECONDS + 30))
  until curl -sf --max-time 1 "$BASE/latest.json" >/dev/null 2>&1; do
    (( SECONDS < deadline )) || die "mock server did not come up on port $PORT (see $1)"
    sleep 0.25
  done
  SERVER_PID="$(lsof -nP -tiTCP:$PORT -sTCP:LISTEN | head -1)"
}

stop_server() {
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null
  [ -n "$SERVER_LAUNCH_PID" ] && kill "$SERVER_LAUNCH_PID" 2>/dev/null
  local deadline=$((SECONDS + 10))
  while (( SECONDS < deadline )) && lsof -nP -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1; do sleep 0.25; done
  SERVER_PID=""; SERVER_LAUNCH_PID=""
}

plist_version() { defaults read "$1/Contents/Info" CFBundleShortVersionString 2>/dev/null; }
app_binary() { echo "$1/Contents/MacOS/$(defaults read "$1/Contents/Info" CFBundleExecutable)"; }
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

# ---------------------------------------------------------------- cleanup

cleanup() {
  local rc=$?
  trap - EXIT INT TERM
  say "cleanup"
  kill_app "$APP"
  kill_app "$RO_MOUNT/Sift.app"
  stop_server
  if [ -n "$RO_DEVICE" ]; then
    hdiutil detach -quiet "$RO_MOUNT" 2>/dev/null || hdiutil detach -quiet -force "$RO_MOUNT" 2>/dev/null || true
  fi
  # The e2e service only; never "sift".
  local i
  for i in 1 2 3 4; do security delete-generic-password -s "$KEYCHAIN_SERVICE" >/dev/null 2>&1 || break; done
  # State of the throwaway identifier only.
  rm -rf "$HOME/Library/Application Support/$IDENTIFIER" "$HOME/Library/WebKit/$IDENTIFIER" \
         "$HOME/Library/Caches/$IDENTIFIER" "$HOME/Library/HTTPStorages/$IDENTIFIER" \
         "$HOME/Library/Saved Application State/$IDENTIFIER.savedState"
  if [ -n "$REAL_KEYCHAIN_BEFORE" ]; then
    local after; after="$(keychain_presence sift)"
    if [ "$after" = "$REAL_KEYCHAIN_BEFORE" ]; then
      pass "real keychain item 'sift' unchanged ($REAL_KEYCHAIN_BEFORE before and after)"
    else
      fail "real keychain item 'sift' changed: $REAL_KEYCHAIN_BEFORE -> $after"
    fi
    if [ "$(keychain_presence "$KEYCHAIN_SERVICE")" = absent ]; then
      pass "no '$KEYCHAIN_SERVICE' keychain item remains"
    else
      fail "a '$KEYCHAIN_SERVICE' keychain item remains"
    fi
  fi
  if [ "$rc" -eq 0 ] && [ "$FAILS" -gt 0 ]; then rc=1; fi
  say "exit $rc ($FAILS failed assertion(s)); evidence kept in $W"
  exit "$rc"
}

# ---------------------------------------------------------------- builds

CSP="default-src 'self' 'unsafe-inline' 'unsafe-eval' data: blob:; img-src 'self' data: blob: receipt://*; connect-src 'self' ipc: http://ipc.localhost http://127.0.0.1:$PORT"

# write_config <version> <outfile> <extra-updater-json>
write_config() {
  node -e '
    const [version, pubkey, csp, id, endpoint, extra, out] = process.argv.slice(1);
    const updater = { endpoints: [endpoint], pubkey, ...JSON.parse(extra) };
    require("fs").writeFileSync(out, JSON.stringify({
      identifier: id, version, app: { security: { csp } }, plugins: { updater },
    }, null, 2));
  ' "$1" "$(tr -d '\n' <"$W/key.pub")" "$CSP" "$IDENTIFIER" "$BASE/latest.json" "$3" "$2"
}

# Hard bound for the long builds (macOS has no coreutils timeout).
bounded() { local secs="$1"; shift; perl -e 'alarm shift; exec @ARGV' "$secs" "$@"; }

build_a() {
  say "build A (release, $NEW_VERSION) -- this takes minutes"
  # Release builds reject http endpoints unless explicitly allowed; A needs it so its
  # own post-update check can reach the mock server.
  write_config "$NEW_VERSION" "$W/config-a.json" '{"dangerousInsecureTransportProtocol": true}'
  (cd "$ROOT" && bounded 3000 pnpm tauri build --bundles app --config "$W/config-a.json") >"$LOG_DIR/build-a.log" 2>&1 \
    || { tail -30 "$LOG_DIR/build-a.log"; die "build A failed (see $LOG_DIR/build-a.log)"; }
  local out="$ROOT/src-tauri/target/release/bundle/macos"
  [ -s "$out/Sift.app.tar.gz" ] && [ -s "$out/Sift.app.tar.gz.sig" ] || die "build A produced no .app.tar.gz/.sig in $out"
  mkdir -p "$ART_DIR"
  cp "$out/Sift.app.tar.gz" "$out/Sift.app.tar.gz.sig" "$ART_DIR/"
  rm -rf "$W/A" && mkdir -p "$W/A" && ditto "$out/Sift.app" "$W/A/Sift.app"
  [ "$(plist_version "$W/A/Sift.app")" = "$NEW_VERSION" ] || die "build A has version $(plist_version "$W/A/Sift.app")"
}

build_b() {
  say "build B (--debug, $OLD_VERSION) -- this takes minutes"
  write_config "$OLD_VERSION" "$W/config-b.json" '{}'
  (cd "$ROOT" && bounded 3000 pnpm tauri build --debug --bundles app --config "$W/config-b.json") >"$LOG_DIR/build-b.log" 2>&1 \
    || { tail -30 "$LOG_DIR/build-b.log"; die "build B failed (see $LOG_DIR/build-b.log)"; }
  local out="$ROOT/src-tauri/target/debug/bundle/macos/Sift.app"
  [ -d "$out" ] || die "build B produced no $out"
  rm -rf "$W/B" && mkdir -p "$W/B" && ditto "$out" "$B_PRISTINE"
  [ "$(plist_version "$B_PRISTINE")" = "$OLD_VERSION" ] || die "build B has version $(plist_version "$B_PRISTINE")"
  # Both builds embed the driver; leave a driver-free dist behind for later builds.
  (cd "$ROOT" && VITE_SIFT_E2E= pnpm build >/dev/null 2>&1) || true
}

# The compile-time keychain override must be in both binaries, or the run could touch 'sift'.
verify_keychain_isolation() {
  local label bundle
  for label in A B; do
    if [ "$label" = A ]; then bundle="$W/A/Sift.app"; else bundle="$B_PRISTINE"; fi
    if LC_ALL=C grep -aq "$KEYCHAIN_SERVICE" "$(app_binary "$bundle")"; then
      pass "build $label embeds keychain service '$KEYCHAIN_SERVICE'"
    else
      die "build $label does not embed '$KEYCHAIN_SERVICE': refusing to launch (it could use the real keychain)"
    fi
  done
}

# launch_app <bundle> <logprefix> [VAR=value ...]: start the app through LaunchServices
# and wait, bounded, for its first 'booted' line; retried up to 3 times.
# Why `open -n` and not the executable: exec'ing Contents/MacOS/* from a script left the
# webview unloaded (process alive, no JS) or aborted in the setup hook ("unknown path"),
# in 9 of 9 standalone trials; a LaunchServices launch booted in ~3 s. See the report.
LAUNCH_ATTEMPTS=0
launch_app() {
  local bundle="$1" prefix="$2" attempt kv deadline
  shift 2
  local env_args=()
  for kv in "$@"; do env_args+=(--env "$kv"); done
  for attempt in 1 2 3; do
    LAUNCH_ATTEMPTS=$attempt
    open -n --stdout "$prefix.$attempt.log" --stderr "$prefix.$attempt.log" ${env_args[@]+"${env_args[@]}"} "$bundle"
    deadline=$((SECONDS + 30))
    while (( SECONDS < deadline )); do
      if fetch_log | grep -qE -e "booted $OLD_VERSION\$"; then return 0; fi
      sleep 0.25
    done
    say "launch attempt $attempt: no 'booted' within 30s, killing"
    kill_app "$bundle"
  done
  return 1
}

# ---------------------------------------------------------------- happy path

run_happy_path() {
  say "happy path: $OLD_VERSION -> $NEW_VERSION"
  rm -rf "$APP" && ditto "$B_PRISTINE" "$APP"
  start_server "$LOG_DIR/server-happy.log"

  local t0=$SECONDS
  launch_app "$APP" "$LOG_DIR/app-happy" || { fail "app never reported 'booted $OLD_VERSION'"; fetch_log >"$LOG_DIR/happy.log"; return; }
  local pid1; pid1="$(app_pid "$APP")"
  say "booted $OLD_VERSION as pid $pid1 after $((SECONDS - t0))s (launch attempts: $LAUNCH_ATTEMPTS)"
  shot 01-booted

  local available percent_re
  available="$(copy "M.available('$NEW_VERSION')")"
  wait_for_log "toast $available\$" 40 || fail "no '$available' toast within 40s"
  shot 02-available
  wait_for_log "click available" 15 || fail "driver never clicked Install"

  percent_re="toast $(copy "M.downloadingPercent('@')" | sed 's/@%$//')([1-9]|[1-9][0-9])%\$"
  wait_for_log "$percent_re" 60 && shot 03-downloading
  wait_for_log "toast $(copy M.installing)\$" 90 && shot 04-installing
  wait_for_log "toast $(copy M.ready)\$" 90 || fail "never reached the ready toast within 90s"
  shot 05-ready
  wait_for_log "click ready" 15 || fail "driver never clicked Restart Now"

  # The relaunch is a new process; wait for it to announce itself.
  if wait_for_log "booted $NEW_VERSION\$" 60; then
    say "booted $NEW_VERSION after $((SECONDS - t0))s since launch"
    shot 06-restarted
  else
    fail "restarted app never reported 'booted $NEW_VERSION'"
  fi
  # A new (old) toast would mean an update loop; the background check fires ~4 s after boot.
  # keychain probe result also arrives in this window (a keychain dialog would block it).
  wait_for_log_after "booted $NEW_VERSION\$" "keychain (ok|error)" 20 || { say "no keychain probe result within 20s: probe blocked"; shot 07-keychain-blocked; }
  sleep 6
  local pid2; pid2="$(app_pid "$APP")"
  fetch_log >"$LOG_DIR/happy.log"

  say "happy-path /e2e-log:"; sed 's/^/    /' "$LOG_DIR/happy.log"
  local log="$LOG_DIR/happy.log"

  # 1. ordered sequence (line numbers must increase)
  local n_av n_click1 n_pct n_inst n_ready n_click2 n_boot ok=1 prev=0 name
  n_av="$(first_line "$log" "toast $available\$")"
  n_click1="$(first_line "$log" "click available")"
  n_pct="$(first_line "$log" "$percent_re")"
  n_inst="$(first_line "$log" "toast $(copy M.installing)\$")"
  n_ready="$(first_line "$log" "toast $(copy M.ready)\$")"
  n_click2="$(first_line "$log" "click ready")"
  n_boot="$(first_line "$log" "booted $NEW_VERSION\$")"
  for name in n_av n_click1 n_pct n_inst n_ready n_click2 n_boot; do
    local v="${!name}"
    if [ -z "$v" ] || (( v <= prev )); then ok=0; fi
    prev="${v:-$prev}"
  done
  if [ "$ok" = 1 ]; then
    pass "log order: available($n_av) < click($n_click1) < downloading N%($n_pct) < installing($n_inst) < ready($n_ready) < click($n_click2) < booted $NEW_VERSION($n_boot)"
  else
    fail "log order broken: available=$n_av click=$n_click1 pct=$n_pct installing=$n_inst ready=$n_ready click=$n_click2 booted=$n_boot"
  fi

  # 2. bundle replaced
  if [ "$(plist_version "$APP")" = "$NEW_VERSION" ]; then
    pass "CFBundleShortVersionString of $APP is $NEW_VERSION"
  else
    fail "CFBundleShortVersionString of $APP is '$(plist_version "$APP")'"
  fi
  if [ "$(sha "$(app_binary "$APP")")" = "$(sha "$(app_binary "$W/A/Sift.app")")" ] \
     && [ "$(sha "$(app_binary "$APP")")" != "$(sha "$(app_binary "$B_PRISTINE")")" ]; then
    pass "installed executable is build A's, not build B's (sha256)"
  else
    fail "installed executable does not match build A"
  fi

  # 3. restart = different process
  if [ -n "$pid1" ] && [ -n "$pid2" ] && [ "$pid1" != "$pid2" ] && ! kill -0 "$pid1" 2>/dev/null; then
    pass "PID before restart $pid1, after restart $pid2; the old process is gone"
  else
    fail "PID before='$pid1' after='$pid2' (old alive: $(kill -0 "$pid1" 2>/dev/null && echo yes || echo no))"
  fi

  # 4. no update loop after restart
  if awk -v n="$n_boot" 'NR > n && /toast .*available/ { bad = 1 } END { exit bad }' "$log"; then
    pass "no update toast after the restart (no loop)"
  else
    fail "an update toast reappeared after the restart"
  fi

  # 5. the artifact really came over HTTP from the server (not faked by a log line)
  if grep -q 'GET /artifact.tar.gz' "$LOG_DIR/server-happy.log"; then
    pass "mock server served GET /artifact.tar.gz"
  else
    fail "mock server never served the artifact"
  fi
  say "GET /latest.json hits: $(grep -c 'GET /latest.json' "$LOG_DIR/server-happy.log") (A re-checks after restart if > 1)"

  # 6. positive control for the 'never ready' assertion of the read-only case
  if log_has "$log" "toast $(copy M.ready)\$"; then
    pass "control: the 'ready' pattern matches in the happy-path log (so its absence below is meaningful)"
  else
    fail "control: the 'ready' pattern did not match the happy-path log"
  fi

  say "keychain probe: $(grep -E ' keychain ' "$log" | sed 's/^[^ ]* //' | tr '\n' ';')"
  kill_app "$APP"
  stop_server
}

# ---------------------------------------------------------------- read-only case

run_readonly_case() {
  say "read-only case: $OLD_VERSION launched from a read-only image"
  [ -e "$RO_MOUNT" ] && die "$RO_MOUNT already exists; detach it first"
  rm -rf "$W/ro-src" "$W/ro.dmg" && mkdir -p "$W/ro-src/tmp"
  ditto "$B_PRISTINE" "$W/ro-src/Sift.app"
  hdiutil create -quiet -format UDRO -fs HFS+ -volname "$RO_VOLNAME" -srcfolder "$W/ro-src" "$W/ro.dmg" \
    || die "hdiutil create failed"
  RO_DEVICE="$(hdiutil attach -nobrowse -noautoopen -readonly "$W/ro.dmg" | awk '/\/Volumes\// { print $1; exit }')"
  RO_DEVICE="${RO_DEVICE:-unknown}"   # non-empty marks the mount as ours, for cleanup
  [ -d "$RO_MOUNT/Sift.app" ] || die "image did not mount at $RO_MOUNT"
  if touch "$RO_MOUNT/.probe" 2>/dev/null; then rm -f "$RO_MOUNT/.probe"; die "$RO_MOUNT is writable"; fi
  pass "$RO_MOUNT is read-only ($RO_DEVICE)"

  # (a) What a user running Sift from a mounted disk image gets: the temp dir is on
  #     the writable system volume, so renaming the bundle fails with EXDEV, not EROFS.
  readonly_attempt image-realistic
  # (b) The EROFS path (INSTALL_MESSAGES.permission): only reachable when the temp dir
  #     itself is on the read-only volume, so point the app's TMPDIR there.
  readonly_attempt tmpdir-on-volume "TMPDIR=$RO_MOUNT/tmp/"

  hdiutil detach -quiet "$RO_MOUNT" 2>/dev/null || hdiutil detach -quiet -force "$RO_MOUNT"
  RO_DEVICE=""
}

# readonly_attempt <label> [VAR=value ...]
readonly_attempt() {
  local label="$1"; shift
  local ro_app="$RO_MOUNT/Sift.app" log="$LOG_DIR/readonly-$label.log"
  say "read-only attempt '$label' $*"
  start_server "$LOG_DIR/server-readonly-$label.log"
  launch_app "$ro_app" "$LOG_DIR/app-readonly-$label" "$@" || { fail "[$label] app never reported 'booted $OLD_VERSION'"; fetch_log >"$log"; stop_server; return; }
  local pid1; pid1="$(app_pid "$ro_app")"
  local available permission permission_re ready_re
  available="$(copy "M.available('$NEW_VERSION')")"
  permission="$(copy M.permission)"
  permission_re="toast $(re_escape "$permission")\$"
  ready_re="toast $(copy M.ready)\$"
  wait_for_log "toast $available\$" 40 || fail "[$label] no '$available' toast within 40s"
  wait_for_log "click available" 15 || fail "[$label] driver never clicked Install"
  # Any install error ends up as a toast: the permission copy or "Update failed: ...".
  if wait_for_log "$permission_re|toast $(re_escape "$(copy M.failedPrefix)")" 90; then
    shot "08-readonly-$label"
  else
    fail "[$label] no install error toast within 90s"
  fi
  sleep 8   # long enough that a ready toast would have shown up
  local pid2; pid2="$(app_pid "$ro_app")"
  fetch_log >"$log"
  say "[$label] /e2e-log (tail):"; tail -n 6 "$log" | sed 's/^/    /'

  local shown; shown="$(grep -E "toast ($(re_escape "$(copy M.failedPrefix)")|$(re_escape "$permission"))" "$log" | tail -1 | sed 's/^[^ ]* toast //')"
  if log_has "$log" "$permission_re"; then
    pass "[$label] the INSTALL_MESSAGES.permission copy was shown"
  elif [ -n "$shown" ] && [ "$label" = image-realistic ]; then
    # Honest outcome, not a pass of the permission-copy gate: see the report.
    pass "[$label] the install failed with an error toast, but not the permission copy: '$shown'"
  else
    fail "[$label] no install error toast was shown"
  fi
  if log_has "$log" "$ready_re" || log_has "$log" "click ready"; then
    fail "[$label] a ready toast appeared although the volume is read-only"
  else
    pass "[$label] no ready toast and no Restart Now click"
  fi
  if [ "$(plist_version "$ro_app")" = "$OLD_VERSION" ] && [ "$pid1" = "$pid2" ] && [ -n "$pid2" ]; then
    pass "[$label] bundle still $OLD_VERSION and the same process ($pid2) is still running"
  else
    fail "[$label] bundle '$(plist_version "$ro_app")', pid $pid1 -> '$pid2'"
  fi
  kill_app "$ro_app"
  stop_server
}

# ---------------------------------------------------------------- main

main() {
  mkdir -p "$W" "$LOG_DIR" "$SHOT_DIR"
  rm -f "$LOG_DIR"/* "$SHOT_DIR"/*
  trap cleanup EXIT INT TERM

  lsof -nP -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1 && die "port $PORT is already in use"
  [ -e "$RO_MOUNT" ] && die "$RO_MOUNT already exists"
  [ "$(uname -s)" = Darwin ] || die "macOS only"

  # E2E_SKIP_BUILD=1 reuses the builds and key of the previous run (iterating on the
  # test itself); a normal run always rebuilds with a fresh key.
  local skip_build="${E2E_SKIP_BUILD:-0}"
  if [ "$skip_build" = 1 ]; then
    [ -s "$W/key" ] && [ -d "$W/A/Sift.app" ] && [ -d "$B_PRISTINE" ] && [ -s "$ART_DIR/Sift.app.tar.gz.sig" ] \
      || die "E2E_SKIP_BUILD=1 needs the artifacts of a previous run in $W"
    say "reusing the builds and key of the previous run"
  else
    say "throwaway signing key"
    rm -f "$W/key" "$W/key.pub"
    (cd "$ROOT" && pnpm tauri signer generate --ci -p '' -w "$W/key") >"$LOG_DIR/signer.log" 2>&1 \
      || die "signer generate failed (see $LOG_DIR/signer.log)"
    [ -s "$W/key" ] && [ -s "$W/key.pub" ] || die "signer wrote no key pair"
  fi
  export TAURI_SIGNING_PRIVATE_KEY="$(cat "$W/key")"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=''
  export SIFT_KEYRING_SERVICE="$KEYCHAIN_SERVICE"
  export VITE_SIFT_E2E=1

  REAL_KEYCHAIN_BEFORE="$(keychain_presence sift)"
  say "real keychain item 'sift': $REAL_KEYCHAIN_BEFORE (must be the same at the end)"

  if [ "$skip_build" != 1 ]; then
    build_a
    build_b
  fi
  verify_keychain_isolation
  run_happy_path
  run_readonly_case

  if [ "$SHOTS_OK" = 1 ]; then say "screenshots: $SHOT_DIR"; else say "screenshots unavailable (Screen Recording permission?)"; fi
  [ "$FAILS" -eq 0 ] && say "all assertions passed" || say "$FAILS assertion(s) failed"
}

main "$@"
