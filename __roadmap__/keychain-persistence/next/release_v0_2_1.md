# Release v0.2.1

**Goal**: Land the campaign on `main`, cut v0.2.1 with honest release notes, and close the paper trail. Coordinator-direct, with every merge gated by the maintainer.
**Pre-conditions**:
- [ ] Both L1 leaves merged into `milestone/keychain-persistence`, and the maintainer's `pnpm e2e:native` run on that tip passed
**Success Gates**:
- ⬜ `__reports__/api_key_storage/00-keyring_mock_backend_v0.md` front matter reads `status: resolved`, and `__reports__/api_key_storage/README.md` lists round 01 as latest [static]
- ⬜ The integration PR into `main` is merged by the maintainer, and `uvx --from commitizen==4.19.1 cz bump --get-next` on `main` prints `0.2.1` [run]
- ⬜ The `CHANGELOG.md` section for 0.2.1 in the release PR says that API keys must be re-entered once and that after each update macOS asks once for the login keychain password ("Always Allow") [static]
- ⬜ `curl -fsSL https://github.com/LittleCoinCoin/sift/releases/latest/download/latest.json` returns version `0.2.1` with `darwin-aarch64` and `darwin-x86_64` [run]
**References**: `CONTRIBUTING.md` § Releasing; `scripts/release-pr.sh`; `__reports__/api_key_storage/01-keychain_signing_measurement_v0.md`

## Step 1: Close the api_key_storage reports
**Goal**: The report trail records the decision before the code reaches `main`.
**Implementation Logic**:
1. In `00-keyring_mock_backend_v0.md`, set the front matter `status: resolved`, and add a `## Resolution` section naming the `apple-native` fix and report 01.
2. In `__reports__/api_key_storage/README.md`, add the round 01 row (findings, decision recorded) and mark it latest. Rewrite `## Status`: the real Keychain ships in 0.2.1, there is one password prompt per update under ad-hoc signing, and a Developer ID is the known fix if that ever becomes unacceptable.
3. Commit type: the house type is `report(api-key-storage): …`. The roadmap grammar only admits stock types, so the field below reads `docs`.
**Deliverables**: `__reports__/api_key_storage/00-keyring_mock_backend_v0.md` (`status: resolved`, `## Resolution`); `__reports__/api_key_storage/README.md` (round 01 row, `## Status`)
**Consistency Checks**: `grep -q '^status: resolved$' __reports__/api_key_storage/00-keyring_mock_backend_v0.md && grep -q '01-keychain_signing_measurement_v0.md' __reports__/api_key_storage/README.md` (expected: PASS)
**Commit**: `docs(api-key-storage): resolve the mock-backend notice with the signing measurement`

## Step 2: Integration PR, then the release PR with honest notes
**Goal**: v0.2.1 is proposed with release notes that warn about the once-per-update prompt.
**Implementation Logic**:
1. Push `milestone/keychain-persistence` and open a PR into `main`. Its title is `fix(backend): persist API keys in the macOS Keychain`, because CI lints the PR title and the merge body. The maintainer merges it with a merge commit.
2. From an up-to-date `main`, run `scripts/release-pr.sh`. It branches `release/next`, runs `cz bump --changelog`, and opens `release(sift): v0.2.1`.
3. On `release/next`, edit only the new 0.2.1 section of `CHANGELOG.md` by adding a short user note:
   - API keys saved before 0.2.1 were never stored, so enter them once more.
   - After each update, macOS asks once for the login keychain password so the new version can read the keys. Choose "Always Allow".
4. Commit and push.
5. The maintainer reviews and merges the release PR. That merge is the release decision.
**Deliverables**: `CHANGELOG.md` (0.2.1 section with the keychain note)
**Consistency Checks**: `git show release/next:CHANGELOG.md | grep -qi 'always allow'` (expected: PASS)
**Commit**: `docs(release): explain the one-time keychain prompt in the 0.2.1 notes`

## Step 3: Verify the release and close the campaign
**Goal**: The published release is confirmed, and nothing from the campaign is left dangling.
**Implementation Logic**:
1. After `release.yml` finishes, confirm the `latest.json` gate above.
2. Complete the Reminder "Persist API keys in the macOS Keychain (enable keyring apple-native)" in list "Sift".
3. Create a Reminder "Consider a Developer ID to remove the per-update Keychain prompt and the Gatekeeper warning", linking report 01.
4. Remove the `experiment/keychain-signing` worktree and branch.
5. Mark the campaign nodes done with `dirtree-rdm`, on a branch that reaches `main` through a PR. The house commit type is `rdm(keychain-persistence): …`.
**Deliverables**: `__roadmap__/keychain-persistence/README.md` and `next/README.md` (statuses via `dirtree-rdm` only)
**Consistency Checks**: `curl -fsSL https://github.com/LittleCoinCoin/sift/releases/latest/download/latest.json | grep -q '"version": *"0.2.1"'` (expected: PASS)
**Commit**: `docs(keychain-persistence): mark the campaign done after v0.2.1 shipped`
