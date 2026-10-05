#!/bin/sh
# Installs the commit-msg stub that runs this checkout's .githooks/commit-msg.
#
# core.hooksPath is pinned to one absolute hooks directory shared by every
# worktree of this repository, so we write a stub there instead of changing git
# config. The stub resolves .githooks through --show-toplevel, so each worktree
# runs its own copy; a checkout without .githooks/commit-msg (an older branch)
# is not linted rather than blocked.
#
# Usage: scripts/install-hooks.sh [--force]
#   --force  overwrite an existing commit-msg hook that is not our stub
set -eu

force=0
case "${1:-}" in
    "") ;;
    --force) force=1 ;;
    -h|--help) sed -n '2,/^set -eu/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "install-hooks: unknown argument: $1" >&2; exit 2 ;;
esac

hooks_dir=$(git rev-parse --git-path hooks)
hook="$hooks_dir/commit-msg"
marker='# sift-commit-msg-stub'

if [ -e "$hook" ] && ! grep -q "^$marker\$" "$hook" && [ "$force" -ne 1 ]; then
    echo "install-hooks: $hook exists and is not the sift stub; rerun with --force to overwrite it." >&2
    exit 1
fi

mkdir -p "$hooks_dir"
tmp="$hook.tmp.$$"
cat > "$tmp" <<'STUB'
#!/bin/sh
# sift-commit-msg-stub
# Written by scripts/install-hooks.sh. Runs the commit-msg hook of the checkout
# the commit is made in. No .githooks/commit-msg there means nothing to run.
target="$(git rev-parse --show-toplevel)/.githooks/commit-msg"
[ -e "$target" ] || exit 0
exec "$target" "$@"
STUB
chmod +x "$tmp"
mv "$tmp" "$hook"
echo "install-hooks: installed $hook"
