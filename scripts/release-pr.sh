#!/usr/bin/env bash
# Cuts a release branch and opens the release PR. It never creates a tag: CI
# is the only tag author, and it tags only when the merged PR changed the
# version.
#
# Usage: scripts/release-pr.sh [--no-push] [--force]
#   --no-push  stop after the local release commit (inspect it, then push by hand)
#   --force    reuse an existing local `release/next` branch (its old tip is
#              restored if this run is rolled back)
#
# Steps: fetch tags, require a clean tree, branch `release/next` from
# <remote>/main, run `cz bump --changelog` (version files, Cargo.lock,
# CHANGELOG.md, one `release(sift): vX.Y.Z` commit), delete the tag cz created,
# then push and `gh pr create` unless --no-push.
#
# Nothing to release (cz exit 3 = no commits since the last tag, exit 21 =
# commits but none bumpable) is not an error: the script says so and exits 0.
# Any other failure rolls everything back (files, commit, tag, branch) and exits
# non-zero with cz's exit code when cz was the cause.

set -euo pipefail

BRANCH=release/next
CZ="commitizen==4.19.1"
NO_PUSH=0
FORCE=0
for arg in "$@"; do
  case "$arg" in
    --no-push) NO_PUSH=1 ;;
    --force) FORCE=1 ;;
    -h|--help) sed -n '2,19p' "$0"; exit 0 ;;
    *) echo "release-pr: unknown argument: $arg (see --help)" >&2; exit 2 ;;
  esac
done

die() { echo "release-pr: $*" >&2; exit 1; }

for tool in git node uvx; do
  command -v "$tool" >/dev/null 2>&1 || die "$tool is required but not on PATH"
done
if [ "$NO_PUSH" -eq 0 ]; then
  command -v gh >/dev/null 2>&1 || die "gh is required to open the PR (or pass --no-push)"
fi

cd "$(git rev-parse --show-toplevel)"

# `cz bump` commits with `git commit -a`: any modified tracked file would leak
# into the release commit, and the rollback below runs `git clean`, which is
# only safe when the tree held nothing of the user's. Untracked files count too.
if [ -n "$(git status --porcelain)" ]; then
  git status --short >&2
  die "working tree is not clean; commit or stash first (cz bump commits with -a)"
fi

# Remote name: local clones call it anything (e.g. LittleCoinCoin), CI says origin.
REMOTE="${REMOTE:-$(git config branch.main.remote || echo origin)}"
git fetch "$REMOTE" --tags
BASE="$(git rev-parse --verify -q "refs/remotes/$REMOTE/main^{commit}")" \
  || die "$REMOTE/main not found after fetch"

# Where to return to: the current branch, or the commit when HEAD is detached.
if ORIG="$(git symbolic-ref -q --short HEAD)"; then
  SWITCH_BACK="git switch -q $ORIG"
else
  ORIG="$(git rev-parse HEAD)"
  SWITCH_BACK="git switch -q --detach $ORIG"
fi
[ "$ORIG" != "$BRANCH" ] || die "already on $BRANCH; switch to another branch first"

OLD_TIP=""
if git show-ref --verify --quiet "refs/heads/$BRANCH"; then
  [ "$FORCE" -eq 1 ] || die "branch $BRANCH already exists (delete it, or pass --force to reuse it)"
  OLD_TIP="$(git rev-parse "refs/heads/$BRANCH")"
fi

TAGS_BEFORE="$(git tag --list)"
STATE=idle

# Undo everything this run did. Runs on every exit path that left STATE=started,
# so an unexpected failure cannot strand a half-bumped tree. Never fails.
rollback() {
  set +e
  git reset -q --hard "$BASE"
  git clean -fdq
  local t
  for t in $(git tag --list); do
    if ! printf '%s\n' "$TAGS_BEFORE" | grep -qxF -- "$t"; then
      git tag -d "$t" >/dev/null
    fi
  done
  $SWITCH_BACK
  if [ -n "$OLD_TIP" ]; then
    git branch -q -f "$BRANCH" "$OLD_TIP"
  else
    git branch -q -D "$BRANCH"
  fi
}
cleanup() {
  local rc=$?
  trap - EXIT
  if [ "$STATE" = started ]; then
    [ "$rc" -eq 0 ] || echo "release-pr: failed (exit $rc), rolling back to $ORIG" >&2
    rollback >/dev/null 2>&1 || echo "release-pr: rollback did not complete, inspect the tree" >&2
  fi
  exit "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

STATE=started
if [ -n "$OLD_TIP" ]; then
  git switch -q --no-track -C "$BRANCH" "$REMOTE/main"
else
  git switch -q --no-track -c "$BRANCH" "$REMOTE/main"
fi

# A leftover local tag for the next version makes cz commit and then fail to tag
# (exit 7). Name it up front, evaluated on <remote>/main. The rollback would
# keep the tag either way, because it only deletes tags this run created.
if NEXT="$(uvx --from "$CZ" cz bump --get-next 2>/dev/null)" && [ -n "$NEXT" ] \
  && git rev-parse -q --verify "refs/tags/v$NEXT" >/dev/null; then
  die "local tag v$NEXT already exists (left over from an earlier run?); delete it with: git tag -d v$NEXT"
fi

set +e
uvx --from "$CZ" cz bump --changelog --yes
cz_rc=$?
set -e
case "$cz_rc" in
  0) ;;
  3)
    echo "release-pr: no commits since the last tag; nothing to release."
    exit 0 ;;
  21)
    echo "release-pr: commits since the last tag, but none is bumpable (feat, fix, perf or a breaking change); nothing to release."
    exit 0 ;;
  *)
    echo "release-pr: cz bump failed with exit code $cz_rc" >&2
    exit "$cz_rc" ;;
esac

VERSION="$(node scripts/check-release-config.mjs --print-version)" \
  || die "version files disagree after the bump (see node scripts/check-release-config.mjs)"
TAG="v$VERSION"

# Defence in depth for `cz bump -a`: the release commit may touch only these.
[ "$(git rev-list --count "$BASE..HEAD")" -eq 1 ] || die "expected exactly one release commit on $BRANCH"
[ "$(git log -1 --format=%s)" = "release(sift): $TAG" ] || die "unexpected release commit subject: $(git log -1 --format=%s)"
for f in $(git diff --name-only "$BASE" HEAD); do
  case "$f" in
    CHANGELOG.md|package.json|src-tauri/Cargo.toml|src-tauri/Cargo.lock) ;;
    *) die "release commit touches unexpected file: $f" ;;
  esac
done

# CI is the only tag author. Delete only the tag cz just made on this commit.
[ "$(git rev-parse -q --verify "refs/tags/$TAG^{commit}" || true)" = "$(git rev-parse HEAD)" ] \
  || die "tag $TAG does not point at the release commit; not deleting it"
git tag -d "$TAG" >/dev/null
if git ls-remote --exit-code --tags "$REMOTE" "refs/tags/$TAG" >/dev/null 2>&1; then
  die "$REMOTE already has tag $TAG; this version was already released"
fi

NOTES="$(mktemp "${TMPDIR:-/tmp}/release-notes.XXXXXX")"
awk -v h="## $TAG (" 'index($0, h) == 1 { f = 1; next } /^## v/ { f = 0 } f' CHANGELOG.md > "$NOTES"
[ -n "$(tr -d '[:space:]' < "$NOTES")" ] || { rm -f "$NOTES"; die "CHANGELOG.md has no '## $TAG (...)' section"; }

STATE=finished
echo "release-pr: created $BRANCH with 'release(sift): $TAG' (no local tag)."

if [ "$NO_PUSH" -eq 1 ]; then
  rm -f "$NOTES"
  echo "release-pr: --no-push: stopped before pushing. Inspect with: git show $BRANCH"
  exit 0
fi

if [ "$FORCE" -eq 1 ]; then
  git push --force-with-lease --set-upstream "$REMOTE" "$BRANCH"
else
  git push --set-upstream "$REMOTE" "$BRANCH"
fi
gh pr create --base main --head "$BRANCH" --title "release(sift): $TAG" --body-file "$NOTES"
rm -f "$NOTES"
$SWITCH_BACK
echo "release-pr: opened the PR; merging it is the release decision."
