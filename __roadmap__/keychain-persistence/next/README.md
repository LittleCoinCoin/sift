# Release v0.2.1

## Context
Runs after both L1 leaves are merged and the maintainer's `pnpm e2e:native` run on the integration tip has passed. It turns the integration branch into the published v0.2.1 through the repository's PR-based release flow (`CONTRIBUTING.md` § Releasing, `scripts/release-pr.sh`, `.github/workflows/release.yml`). Every merge is the maintainer's decision; the coordinator prepares the branches and verifies the results.

## Goal
v0.2.1, with persistent Keychain API keys, is the latest published release, and its notes explain the one-time prompt after each update.

## Pre-conditions
- [ ] L1 leaves `apple_native_backend` and `e2e_keychain_assertions` merged into `milestone/keychain-persistence`
- [ ] The maintainer's `pnpm e2e:native` run on the integration tip exited 0 with the three keychain lines of the campaign Success Gates

## Success Gates
- ⬜ `uvx --from commitizen==4.19.1 cz bump --get-next` prints `0.2.1` on `main` after the integration PR merges [run]
- ⬜ `curl -fsSL https://github.com/LittleCoinCoin/sift/releases/latest/download/latest.json` returns version `0.2.1` [run]

## Status
```mermaid
graph TD
    release_v0_2_1[Release v0.2.1]:::planned
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `release_v0_2_1.md` | 📄 Leaf Task | ⬜ Planned |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
