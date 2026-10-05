#!/usr/bin/env bash
# Probes the commitizen validator with subjects that must be accepted and
# subjects that must be rejected, via `cz check --message`. `cz check
# --rev-range` only ever sees commits that already passed, so it cannot show a
# rejection; this probe exercises the reject path. Config lives in .cz.toml.
# A leading space is not probed here: cz strips the message before matching, so
# only .githooks/commit-msg can reject it.
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    cat <<'USAGE'
Usage: scripts/probe_cz_check.sh

Runs `uvx --from commitizen==4.19.1 cz check --message "<subject>"` from the
repository root for every subject in ACCEPT and REJECT, prints a PASS/FAIL
table, and exits non-zero if any subject lands on the wrong side.
USAGE
    exit 0
fi

command -v uvx >/dev/null 2>&1 || {
    echo "probe_cz_check: uvx not found; install uv: https://docs.astral.sh/uv/" >&2
    exit 1
}

cd "$(git rev-parse --show-toplevel)"

# 101 characters: `chore(x): ` (10) plus 91 lowercase letters.
LONG="chore(x): $(printf 'a%.0s' $(seq 1 91))"

ACCEPT=(
    "rdm(update-flow-hardening): author roadmap"
    "report(updater-audit): add e2e results"
    "release(sift): v0.2.0"
    "ci(release): pin actions"
    "feat(updater)!: drop the legacy endpoint"
    "fix(toast): keep persistent toasts on top"
    "Merge pull request #5 from a/b"
    "Merge task/a into milestone/b"
    "Merge pull request #12 from LittleCoinCoin/task/x"
    "Merge branch 'main' into x"
    "Revert \"feat(x): y\""
    "fixup! feat(x): y"
    "amend! feat(x): y"
    "Initial commit"
    $'docs(x): why this exists\n\nProse explaining the reason.\n\nBREAKING CHANGE: the flag is gone\n\nCo-Authored-By: A <a@example.com>'
)

REJECT=(
    "chore(x): trailing."
    "bump: version 0.1.4 → 0.2.0"
    "feat: no scope"
    "Feat(x): capital type"
    "feat(X): capital scope"
    "fix(x): Capital start"
    "style(x): unknown type"
    "chore(x_y): non-kebab scope"
    "wip"
    "chore(x-): scope ends in a hyphen"
    "Mergefoo into bar"
    "Reverting the thing"
    $'docs(x): a\nbody without a blank line'
    $'docs(x): a\n\nfeat(y): hidden typed body line'
    $'rdm(x): a\n\nProse.\n  fix(y)!: indented typed body line'
    "$LONG"
)

check() { uvx --from commitizen==4.19.1 cz check --message "$1" >/dev/null 2>&1; }

status=0
printf '%-7s %-7s %s\n' RESULT WANT SUBJECT
for s in "${ACCEPT[@]}"; do
    if check "$s"; then r=PASS; else r=FAIL; status=1; fi
    printf '%-7s %-7s %s\n' "$r" accept "$s"
done
for s in "${REJECT[@]}"; do
    if check "$s"; then r=FAIL; status=1; else r=PASS; fi
    printf '%-7s %-7s %s\n' "$r" reject "${s:0:80}$([[ ${#s} -gt 80 ]] && echo "... (${#s} chars)")"
done

if [[ "$status" -eq 0 ]]; then
    echo "probe_cz_check: all expectations met"
else
    echo "probe_cz_check: one or more expectations were not met" >&2
fi
exit "$status"
