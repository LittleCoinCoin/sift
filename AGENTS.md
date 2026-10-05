# AGENTS.md

Sift is a macOS desktop app: Tauri v2, a Svelte 5 frontend in `src/`, and a Rust
backend in `src-tauri/`. It sends folders of documents through
OpenAI-compatible endpoints for OCR and field extraction, and exports a CSV. It
updates itself through `tauri-plugin-updater` from GitHub releases.

This file is for coding agents. `README.md` is for users and `CONTRIBUTING.md` is
for contributors. CONTRIBUTING is the source of truth for the commit convention,
the gates, direct pushes and the release flow: read it and follow it. This file
does not restate it.

## Working principles

**Pragmatism over dogma governs everything below**, and every other rule in
this file and in CONTRIBUTING: git, process, testing, verification,
delegation, roadmaps, reports. Rules are practices we chose because they help
and make work predictable for scalable management, and exceptions exist.
When following a rule to the letter costs more than the rule protects, take
the exception, say why, and keep it predictable: write a recurring exception
down where the rule lives. Apply each principle in proportion to the risk. A
user-facing or silent-failure path earns the full rigour; a mechanical change
does not.

- **Branch protection and PRs.** Mechanical maintenance goes straight to
  `main`; anything users can notice goes through a PR (CONTRIBUTING, "Direct
  pushes to `main`").
- **No ceremony for bookkeeping.** Never open a PR, or plan a roadmap step,
  just to record results or update a status. Fold records into the next real
  change or push them directly.
- **A check that cannot fail proves nothing.** Before you trust a gate, make it
  fail once: plant a defect, mutate the code, run a negative control. Two
  examples from this repo:
  - `svelte-check` checked 0 files until `tsconfig.json` existed.
  - The first release rehearsal caught a signing key that did not match the
    pinned public key.
- **Measure, don't assume.** Record what you ran and what it printed. When a
  spec, a brief or this file is wrong, report it with evidence rather than
  building on it.
- **Performance is part of every change, unprompted.** Sift must be a good
  citizen of the machine it runs on. A fast app means faster dev cycles and
  happier users, and wasted cycles add up across every app on the machine.
  - **Before** implementing a feature, weigh its cost:
    - startup time;
    - idle CPU and wakeups;
    - memory;
    - bundle and download size;
    - responsiveness.

    Prefer the design that keeps them flat. If a feature has a real cost, say
    so in the PR with numbers and the trade-off you chose.
  - **Measure** the metrics a change touches, before and after, and put the
    numbers in the commit body:
    - max RSS with `/usr/bin/time -l`;
    - CPU and wakeups with `ps -o %cpu,rss` or `top -stats pid,cpu,idlew,mem`;
    - `du -sh` of the `.app`, `.dmg` and update tarball;
    - `vite build` chunk sizes;
    - wall-clock timings.

    `perf` commits require a measured before/after (CONTRIBUTING).
  - **Responsiveness tiers.** These are budgets from the time an input arrives
    until its visible feedback is on screen, borrowed from real-time games:

    | Tier | Frame rate | Budget per input → visible feedback |
    |:----:|:----------:|:-----------------------------------:|
    | **S** | 144 FPS | ≤ 6.9 ms |
    | **A** | 120 FPS | ≤ 8.3 ms |
    | **B** | 60 FPS | ≤ 16.7 ms |
    | **C** | 30 FPS | ≤ 33.3 ms |

    - **Direct manipulation** (typing, clicks, toggles, scrolling, zooming or
      panning a document, opening a panel) targets **S or A**.
    - **B** is acceptable.
    - **C** is the floor. Anything slower is a bug to fix or justify.
    - **Work that cannot finish inside the budget** (OCR, extraction, export,
      update download) must still show its first feedback within the tier:
      a state change, progress or a spinner. It then runs off the critical
      path, reports progress and can be cancelled.
    - **Never block** the webview's main thread or the Tauri async runtime
      with work that could be streamed, batched, cached or moved off-thread.
    - **When a change touches an interactive path,** state its measured tier
      in the commit or PR. Measure with Safari Web Inspector's Timelines on a
      debug build (Tauri on macOS runs in WKWebView), or with
      `performance.now()` around the handler plus a `requestAnimationFrame`
      after the update.
  - **"It's I/O-bound" is not a reason to stop.** It is where to look next:
    - remove redundant reads, writes and IPC round-trips;
    - stream instead of loading whole files;
    - overlap I/O with work, and cache what is re-read;
    - shrink payloads;
    - don't poll when an event exists.
  - **Take easy wins when you see them:** a smaller footprint, fewer
    re-renders, lazy loading, dropping an unused dependency. Do them in
    passing when they're in scope and cheap. Otherwise record them (a notice
    report or a task) instead of letting them go.
  - **No idle work.** No timers, watchers or background loops running when
    nothing has changed, unless the user asked for that work.
- **When coordinating subagents, the coordinator is pragmatic, not dogmatic.**
  - A dispatch costs ~10–30 min and ~150–300k tokens.
  - Anything cheaper to do directly than to brief, the coordinator does
    itself, with the same commit discipline. That covers a fix up to one
    function, a config line, a mechanical fix after a verifier finding,
    conflict resolution, and the coordinator-direct items in the tree.
  - Leaves with real design content are dispatched.
  - Send anything whose failure would be silent or user-facing to an
    adversarial verifier, and name what it should attack.

## Map

| Path | What lives there |
|:-----|:-----------------|
| `src/lib/stores/updater.svelte.ts` | update UX state machine: one toast, an install phase, "ready" only after install resolves |
| `src/lib/stores/updater-errors.ts` | `classifyInstallError` and `INSTALL_MESSAGES`, the user-visible copy that tests and E2E read from source |
| `src-tauri/src/lib.rs` | Tauri setup: default macOS menu plus "Check for Updates…" |
| `src-tauri/src/keyring_store.rs` | API-key storage; `SIFT_KEYRING_SERVICE` is a compile-time override for test builds |
| `src-tauri/build.rs` | downloads pdfium for the target arch; rebuilds when `package.json` changes |
| `scripts/` | release checker and assembler, `release-pr.sh`, hook installer and probe, E2E runners, mock update server |
| `.github/workflows/` | `ci.yml` (four required checks) and `release.yml` (decide → build → verify → publish) |
| `__roadmap__/<campaign>/` | campaign roadmaps, managed only with `dirtree-rdm` (managing-roadmaps skill) |
| `__reports__/<topic>/` | reports with round-versioned names and a `README.md` index per topic. Open items live in each index's Status section |

`__roadmap__/` itself has no README. Never hand-edit the Mermaid, Nodes or
Progress sections of a roadmap README.

## Environment

- **Toolchain:** Node 24 and pnpm (`pnpm install --frozen-lockfile`), Rust and
  cargo in `src-tauri/`. Python only through `uv` / `uvx`; commitizen is pinned
  as `uvx --from commitizen==4.19.1 cz`. There is no conda or mamba environment.
- **Remote name:** the git remote is `LittleCoinCoin` in this checkout and
  `origin` in CI. Scripts resolve it with `git config branch.main.remote`;
  never hardcode `origin` outside workflow YAML.
- **Hook:** run `bash scripts/install-hooks.sh` once per clone. The commit-msg
  hook then lints commits in every worktree, and fails closed without `uv`.
- **Worktrees:** `.worktreeinclude` copies pdfium into worktrees Claude creates.
  Tags are shared by every worktree of the repo, so run `cz bump` or
  `release-pr.sh` experiments only in a throwaway clone under `$TMPDIR`. A
  stray local `v*` tag breaks the next real bump.
- **Dependencies:** no new dependency without saying so; `pnpm-lock.yaml`
  changes are reviewed on purpose.

## Safety rails

- **Signing key.** The updater's private key lives only in the maintainer's
  vault and in GitHub secrets: never ask for it, read it or print it. Its
  public half is pinned twice, in `tauri.conf.json` and as `EXPECTED_PUBKEY` in
  `scripts/check-release-config.mjs`. Rotating it means changing both on
  purpose, and installed copies trust only the old key.
- **Local builds.** `tauri build` needs `TAURI_SIGNING_PRIVATE_KEY`, because
  updater artifacts are on. Build locally with
  `--config '{"bundle":{"createUpdaterArtifacts":false}}'`; tests use
  throwaway keys under `$TMPDIR`.
- **Releases** reach every installed copy within seconds of launch. Cut them
  only with `scripts/release-pr.sh`, then a PR, then a merge commit. Never
  create `v*` tags, publish or edit releases by hand, and never re-run a
  release with `dry_run=false` without the maintainer.
- **E2E runs:**
  - never install into `/Applications`;
  - never touch the keychain service `sift` (test builds use `sift-e2e`);
  - stop only processes you started, by PID or profile directory, never with
    a name match like `pkill -f "Google Chrome"`.
- **Outward actions** beyond maintenance pushes need the maintainer's go-ahead
  first: opening PRs, repo settings, releases, deleting remote branches. Ask
  once per action.
