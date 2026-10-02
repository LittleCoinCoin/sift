#!/usr/bin/env python3
"""WorktreeCreate hook: replaces Claude Code's default `git worktree add`.

Contract: create the worktree, then print its absolute path (and nothing else) on stdout.
Everything else goes to stderr.
"""
import json
import subprocess
import sys
from pathlib import Path

LIBS = {"darwin": "libpdfium.dylib", "linux": "libpdfium.so", "win32": "pdfium.dll"}


def git(*args: str, cwd: str) -> str:
    done = subprocess.run(
        ["git", *args], cwd=cwd, check=True, text=True,
        stdout=subprocess.PIPE, stderr=sys.stderr,
    )
    return done.stdout.strip()


def main() -> None:
    payload = json.load(sys.stdin)
    wt = payload["worktree_path"]
    name = payload["worktree_name"]
    root = payload["cwd"]

    # git's own progress output would corrupt the path on stdout, so it is sent to stderr
    subprocess.run(
        ["git", "worktree", "add", "-b", f"worktree-{name}", wt, "HEAD"],
        cwd=root, check=True, stdout=sys.stderr, stderr=sys.stderr,
    )

    lib = LIBS.get(sys.platform)
    if lib:
        # The main checkout holds the real library, even when the session started in another worktree.
        common = Path(git("rev-parse", "--path-format=absolute", "--git-common-dir", cwd=root))
        src = common.parent / "src-tauri" / lib
        if src.is_file():
            dst = Path(wt) / "src-tauri" / lib
            dst.parent.mkdir(parents=True, exist_ok=True)
            dst.unlink(missing_ok=True)
            dst.symlink_to(src)

    print(wt)


if __name__ == "__main__":
    main()
