# Publish Level

## Context
Final level of `update-flow-hardening`. Everything before it is local. This level crosses into GitHub: the push, PRs, repo settings and the first real release. Every outward action needs the user's explicit per-action confirmation, and PR merges are done by the user.

## Goal
v0.2.0 is published by the new pipeline, and `main` is protected by the new gates.

## Pre-conditions
- [ ] L3 merged; the campaign-level gates that can run locally pass on the integration tip

## Success Gates
- ✅ `curl -fsSL https://github.com/LittleCoinCoin/sift/releases/latest/download/latest.json` shows version `0.2.0` with both darwin platforms [run]
- ✅ `gh api repos/LittleCoinCoin/sift/branches/main/protection` lists the four `ci.yml` checks as required [run]

## Gotchas
User-gated. Each push, PR creation, settings change and merge is confirmed by the user, step by step.

## Status
```mermaid
graph TD
    publish_cycle[Publish Cycle]:::planned
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `publish_cycle.md` | 📄 Leaf Task | ⬜ Planned |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
