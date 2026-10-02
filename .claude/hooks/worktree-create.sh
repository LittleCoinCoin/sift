#!/bin/bash
# WorktreeCreate hook: replaces Claude Code's default `git worktree add`.
# Contract: create the worktree, then print its absolute path (and nothing else) on stdout.
set -euo pipefail

in=$(cat)
wt=$(jq -r .worktree_path <<<"$in")
name=$(jq -r .worktree_name <<<"$in")
root=$(jq -r .cwd <<<"$in")
cd "$root"

git worktree add -b "worktree-$name" "$wt" HEAD >&2

case "$(uname -s)" in
  Darwin*) lib=libpdfium.dylib ;;
  Linux*) lib=libpdfium.so ;;
  MINGW*|MSYS*|CYGWIN*) lib=pdfium.dll ;;
  *) lib= ;;
esac

# The main checkout holds the real library, even when the session started inside another worktree.
main=$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")
if [ -n "$lib" ] && [ -f "$main/src-tauri/$lib" ]; then
  mkdir -p "$wt/src-tauri"
  ln -sf "$main/src-tauri/$lib" "$wt/src-tauri/$lib"
fi

echo "$wt"
