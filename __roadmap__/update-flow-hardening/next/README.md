# CI Release and E2E

## Context
Level 2 of `update-flow-hardening`. It starts once L1 is merged: commitizen config, single-source version 0.1.4 with updater artifacts, truthful install toasts, the typecheck baseline and the restored macOS menu. It produces the online gates (`ci.yml`), the release-PR → verify → auto-publish pipeline (`release.yml`, `scripts/release-pr.sh`, `scripts/build-latest-json.mjs`), and two agent-run E2E layers with reports 04 and 05. The four leaves touch disjoint files and run in parallel.

## Goal
Every PR is gated online, a merged release PR becomes a verified published release with no human step, and the update UX is proven end to end by agents.

## Pre-conditions
- [ ] L1 merged into `milestone/update-flow-hardening`, and the L1-close integration check passed (`cz bump --get-next` = `0.2.0`, `node scripts/check-release-config.mjs` exits 0)

## Success Gates
- ✅ `actionlint .github/workflows/*.yml` exits 0 [run]
- ✅ `node --test scripts/*.test.mjs src/lib/stores/*.test.ts` passes [run]
- ✅ Reports `__reports__/updater_audit/04-e2e_ui_results_v0.md` and `05-e2e_native_results_v0.md` exist and record every scenario as passing [static]

## Gotchas
- **Seam between release_workflow and e2e_native.** e2e_native builds its mock `latest.json` itself: the `signature` field is the `.sig` file content verbatim. It does not import `scripts/build-latest-json.mjs`, so the two leaves stay independent. The coordinator checks at level close that both produce the same manifest shape.
- **Seam with install_feedback.** The E2E leaves read the toast copy from `INSTALL_MESSAGES` in `src/lib/stores/updater-errors.ts`; they never duplicate the strings.
- **`package.json` scripts.** e2e_ui adds `e2e:ui`, e2e_native adds `e2e:native`. These are different keys; the coordinator resolves any trivial rebase conflict.

## Status
```mermaid
graph TD
    ci_workflow[CI Workflow]:::done
    release_workflow[Release Workflow]:::done
    e2e_ui[E2E UX Harness]:::done
    e2e_native[E2E Native Update]:::done
    next[Docs Closure Level]:::done
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `ci_workflow.md` | 📄 Leaf Task | ✅ Done |
| `release_workflow.md` | 📄 Leaf Task | ✅ Done |
| `e2e_ui.md` | 📄 Leaf Task | ✅ Done |
| `e2e_native.md` | 📄 Leaf Task | ✅ Done |
| `next/` | 📁 Directory | ✅ Done |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
