# Update Flow Hardening

## Context
The auto-updater campaign (`__roadmap__/auto-updater/`) shipped code, but the 2026-10-03 audit found that no user can receive an update. Release builds never produce `.sig` files or `latest.json` because `createUpdaterArtifacts` is missing, every GitHub release is a draft, and the install flow reports "ready" before installing. This campaign fixes the pipeline and the UX, and moves development to a PR cycle with commitizen-enforced commits, online CI, and a release-PR → auto-publish release model. It produces v0.2.0, the first publicly published release. The plan of record is the approved plan of the 2026-10-03 audit session. Prior art for commitizen is `~/Documents/src/CrackingShells/colgrep-mcp` (`server/pyproject.toml [tool.commitizen*]`, `CONTRIBUTING.md`, `.github/workflows/`, `dev/skills/landing-and-release/`).

## Goal
Make "click to download, click to install" work for real users, proven by agent-run E2E and a verified, auto-published v0.2.0, under an enforced commit and release discipline.

## Pre-conditions
- [ ] Integration branch `milestone/update-flow-hardening` cut from local `main` at `bfef3c5`
- [ ] Environment contract measured in a fresh hand-made worktree: `pnpm install --frozen-lockfile`, `pnpm build`, `cargo test --lib` (42 passed) all green, and results attributed to that worktree
- [ ] `uvx --from commitizen==4.19.1 cz version` prints `4.19.1`; `actionlint --version` prints `1.7.12`

## Success Gates
- ✅ `node scripts/check-release-config.mjs` exits 0 on the integration tip [run]
- ✅ `uvx --from commitizen==4.19.1 cz check --rev-range LittleCoinCoin/main..HEAD` passes on the integration tip [run]
- ✅ `uvx --from commitizen==4.19.1 cz bump --get-next` prints `0.2.0` on the integration tip [run]
- ✅ The E2E reports `__reports__/updater_audit/04-*` (UX harness) and `05-*` (native update) record all scenarios passing [static]
- ✅ `curl -fsSL https://github.com/LittleCoinCoin/sift/releases/latest/download/latest.json` returns version `0.2.0` with `darwin-aarch64` and `darwin-x86_64` platforms [run]

## Gotchas
- **Coordinator-direct items at L1.** Too small to brief, so the coordinator does them on the integration branch with the same commit discipline:
  - *typecheck baseline*: a `tsconfig.json`, so `svelte-check` checks files. Measured: it checks 0 files today, and a deliberate type error passes. Also fix the one error it surfaces (script-less `src/lib/Caustics.svelte`), and add the `"check": "svelte-check"` script.
  - *macos menu*: restore the default menu in `src-tauri/src/lib.rs` and insert "Check for Updates…".
  - *tag*: `git tag -d v0.1.5`, which is local-only and was never pushed.
- **L1 close integration check (coordinator):** `cz bump --get-next` prints `0.2.0`, and `node scripts/check-release-config.mjs` passes. `.cz.toml` (commit_convention) and the manifests (release_config) are written by different leaves; this is their seam.
- **Remote name.** It is `LittleCoinCoin` locally and `origin` in CI. Scripts resolve it with `REMOTE="${REMOTE:-$(git config branch.main.remote || echo origin)}"`, and docs say `<remote>/main`.
- **Signing key.** With `createUpdaterArtifacts`, every `tauri build` needs `TAURI_SIGNING_PRIVATE_KEY`. CI never runs `tauri build`. The E2E uses a throwaway key in `$TMPDIR` that is never committed.
- **No new devDependencies anywhere in the campaign:** `git diff --exit-code pnpm-lock.yaml` per leaf.
- **Verifier leaves** (an adversarial review is dispatched before merge): commit_convention, release_config, install_feedback, release_workflow, e2e_ui, e2e_native. Mechanical leaves (ci_workflow, docs_closure) merge on the implementer's report.
- **Merge commits.** Subject `Merge task/<leaf> into milestone/update-flow-hardening`. The body is WHY prose that never starts with `type(scope):`, because commitizen would read it as a bump.

## Status
```mermaid
graph TD
    commit_convention[Commit Convention]:::planned
    release_config[Release Config]:::planned
    install_feedback[Install Feedback]:::planned
    next[CI Release and E2E]:::planned
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `commit_convention.md` | 📄 Leaf Task | ⬜ Planned |
| `release_config.md` | 📄 Leaf Task | ⬜ Planned |
| `install_feedback.md` | 📄 Leaf Task | ⬜ Planned |
| `next/` | 📁 Directory | ⬜ Planned |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
