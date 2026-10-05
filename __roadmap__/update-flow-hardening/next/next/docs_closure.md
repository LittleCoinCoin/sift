# Docs Closure

**Goal**: Align CONTRIBUTING, README and the updater report index with the shipped gates, release flow and update behaviour.
**Pre-conditions**:
- [ ] L2 merged into `milestone/update-flow-hardening`
**Success Gates**:
- ⬜ `for j in commits config frontend rust; do grep -q "$j" CONTRIBUTING.md || exit 1; done` passes [run]
- ⬜ `CONTRIBUTING.md` has `## Gates` and `## Releasing` sections; `README.md` has an update section that names 0.2.0 as the first auto-updating release [static]
- ⬜ `__reports__/updater_audit/README.md` rows: 00 resolved, 01 resolved, 02 deferred, 03 superseded by 04/05; and 04, 05 listed [static]
**References**: `.github/workflows/ci.yml` and `release.yml` (job names, flow); `__reports__/updater_audit/04-*`, `05-*`

## Step 1: CONTRIBUTING gates and releasing
**Goal**: A contributor can reproduce CI locally and cut a release without reading workflow YAML.
**Implementation Logic**:
Add two sections to `CONTRIBUTING.md`:

**§Gates:** the local command for each `ci.yml` job, `scripts/install-hooks.sh`, `pnpm e2e:ui`, and `pnpm e2e:native`.

**§Releasing:**
- `scripts/release-pr.sh` → PR → merge commit only (squash and rebase merges are disabled) → `release.yml` decides, builds, verifies (minisign + Intel `lipo`) and auto-publishes.
- What `decide` does, and why merges at an already-tagged version are no-ops.
- Yank: `gh release edit vX --draft=true`; installed copies stay put (`allowDowngrades` is false); ship the fix through a new release PR.
- The rehearsal on PRs that touch release machinery.
- The remote-name rule (`<remote>/main`).
**Deliverables**: `CONTRIBUTING.md` (`## Gates`, `## Releasing`)
**Consistency Checks**: `for j in commits config frontend rust; do grep -q "$j" CONTRIBUTING.md || exit 1; done` (expected: PASS)
**Commit**: `docs(contributing): document local gates and the release-PR flow`

## Step 2: README and report index
**Goal**: Users and stakeholders see the true state.
**Implementation Logic**:
1. **`README.md`, an install/update section:** 0.2.0 is the first published and auto-updating release. Users on ≤ 0.1.4 install 0.2.0 manually once. Updates are offered in-app about 4 s after launch and via Sift → Check for Updates…; run Sift from Applications, not from the DMG.
2. **`__reports__/updater_audit/README.md`:** update the table and the Status prose for 00–05.
3. **`__roadmap__/auto-updater/README.md`:** add a one-paragraph Gotchas note that its gates were met in code but not in release reality, pointing to this campaign. Edit the prose only, never the managed sections.

**Commit type:** split the work into a `docs(readme)` commit and a `report(updater-audit)` commit. The roadmap grammar admits only one Commit line, so the field below reads `docs`. The coordinator records the actual pair.
**Deliverables**: `README.md` (update section); `__reports__/updater_audit/README.md`; `__roadmap__/auto-updater/README.md` (Gotchas note)
**Consistency Checks**: `grep -q '04-e2e_ui_results_v0.md' __reports__/updater_audit/README.md && grep -q '05-e2e_native_results_v0.md' __reports__/updater_audit/README.md` (expected: PASS)
**Commit**: `docs(readme): explain installing and updating Sift`

## Gotchas
- Coordinator-direct. Not a verifier leaf.
