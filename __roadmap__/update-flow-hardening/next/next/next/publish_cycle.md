# Publish Cycle

**Goal**: Land the campaign on `main` through a PR, prove the release rehearsal online, cut v0.2.0 through a release PR, and protect `main`. Each outward step is confirmed by the user.
**Pre-conditions**:
- [ ] L3 merged; on the integration tip `node scripts/check-release-config.mjs`, `cz check --rev-range LittleCoinCoin/main..HEAD` and `cz bump --get-next` (= 0.2.0) pass
**Success Gates**:
- ⬜ The integration PR shows `ci.yml` (`commits`, `config`, `frontend`, `rust`) green, and the `release.yml` rehearsal green, with its `dryrun-*` draft deleted [run]
- ⬜ After the user merges the integration PR, `release.yml` on `main` reports `release=false` (tag v0.1.4 exists) [run]
- ⬜ After the user merges the 0.2.0 release PR, `release.yml` publishes v0.2.0, and the anonymous `latest.json` check passes [run]
- ⬜ Branch protection on `main` requires the four checks and up-to-date branches; squash and rebase merges are disabled [run]
- ⬜ The user confirms: Cmd+C/V work in Settings, cancelling the admin prompt shows the readable message, and the 0.2.0 DMG installs [behavioral]
**References**: `CONTRIBUTING.md` §Releasing; `.github/workflows/release.yml`

## Step 1: Integration PR and online rehearsal
**Goal**: Everything runs once online before anything ships.
**Implementation Logic**:
With user confirmation:
1. `git push LittleCoinCoin milestone/update-flow-hardening`.
2. `gh pr create --base main` with a title that passes `cz check` (e.g. `build(release): harden the update flow and release pipeline`) and a body summarising the campaign.
3. Watch `ci.yml` and the `release.yml` rehearsal. Fix failures on the branch, through dispatch or coordinator-direct.
4. The user merges with a **merge commit**.
5. Confirm `release.yml` on `main` decided `release=false`.

Record the URLs in the Progress trail.
**Deliverables**: pull request URL; green run URLs recorded in the campaign Progress notes
**Consistency Checks**: `gh pr checks --watch` (expected: PASS)
**Commit**: `chore(release): record integration PR and rehearsal results`

## Step 2: Release PR for 0.2.0
**Goal**: The first release cut by the new flow.
**Implementation Logic**:
With user confirmation:
1. Run `scripts/release-pr.sh` from an up-to-date `main`.
2. Review the CHANGELOG with the user, and wait for CI green.
3. The user merges with a merge commit.
4. Watch `release.yml` through `publish`, then verify the anonymous `latest.json` and the release notes, including the bridge note.
**Deliverables**: v0.2.0 published release; `CHANGELOG.md` on `main`
**Consistency Checks**: `curl -fsSL https://github.com/LittleCoinCoin/sift/releases/latest/download/latest.json` (expected: PASS)
**Commit**: `chore(release): record the v0.2.0 publication`

## Step 3: Protect main and user checks
**Goal**: The gates become mandatory, and the human-only checks are recorded.
**Implementation Logic**:
With user confirmation:
1. `gh api -X PATCH repos/LittleCoinCoin/sift -f allow_squash_merge=false -f allow_rebase_merge=false -f allow_merge_commit=true`.
2. `gh api -X PUT repos/LittleCoinCoin/sift/branches/main/protection` with the required status checks `commits`, `config`, `frontend`, `rust` and `strict=true`.
3. Ask the user to run the three behavioural checks, and record the outcomes in report 05's addendum.
4. Clean up local refs, with confirmation: `test/updater-flow`, `milestone/auto-updater`, stale `worktree-agent-*` branches and the envprobe worktree.
**Deliverables**: repository merge settings; `main` branch protection; addendum in `__reports__/updater_audit/05-e2e_native_results_v0.md`
**Consistency Checks**: `gh api repos/LittleCoinCoin/sift/branches/main/protection --jq '.required_status_checks.contexts'` (expected: PASS)
**Commit**: `docs(updater-audit): record user checks and branch protection`

## Gotchas
- Outward-facing, so it is run by the coordinator with per-action user confirmation, never by a subagent.
- Steps 1–2 produce commits only if notes are recorded in the repo; the roadmap Progress update is batched (`rdm` type).
- Rehearsal runs need `TAURI_SIGNING_PRIVATE_KEY` from repo secrets. They run only for same-repo PR branches.
- **Outcome (2026-10-05):** v0.2.0 published by the pipeline (run 37272671132), and `main` protected with the four checks. The first rehearsal caught a signing-key mismatch between the repo secret and the pinned pubkey; the key was rotated to `1884DC5376300376` before anything shipped (report 05 addendum). The behavioural gate is partly met: the user installed the 0.2.0 DMG. Cmd+C/V in Settings, the admin-prompt cancel, and the first in-app update from 0.2.0 are carried over to the 0.2.1 release.
