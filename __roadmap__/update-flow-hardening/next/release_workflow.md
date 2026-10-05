# Release Workflow

**Goal**: Turn a merged release PR into a verified, auto-published GitHub release whose `latest.json` installed apps can trust. Rehearse the whole pipeline on any PR that touches it, without ever touching a real release.
**Pre-conditions**:
- [ ] Working on `task/release_workflow`, branched from the integration tip after L1
- [ ] `brew install minisign` done locally (bottled 0.12), needed for the fixture tests
**Success Gates**:
- ⬜ `actionlint .github/workflows/release.yml` exits 0 [run]
- ⬜ `grep -q 'uploadUpdaterJson: false' .github/workflows/release.yml && ! grep -q includeUpdaterJson .github/workflows/release.yml` [static]
- ⬜ Every non-`actions/` `uses:` in `release.yml` is pinned to a 40-hex SHA [static]
- ⬜ `node --test scripts/build-latest-json.test.mjs` passes. Fixtures are signed at test time with a throwaway key: both platforms are emitted, a wrong-key `.sig` exits non-zero, and a missing platform exits non-zero [run]
- ⬜ `scripts/release-pr.sh --no-push` in a scratch clone at the integration tip produces a `release(sift): v0.2.0` commit and leaves no local `v0.2.0` tag. On a clone with no commits since the tag (exit 3) and with only non-bumping commits (exit 21), it exits 0 with a clear message [run]
**References**: tauri-action `action.yml` (inputs `releaseId`, `uploadUpdaterJson`, `uploadUpdaterSignatures`); colgrep-mcp `.github/workflows/publish.yml` (guard and CHANGELOG-section extraction); the `.sig` and pubkey formats (base64 of minisign files, prehashed `ED` signatures)

## Step 1: latest.json assembler and verifier
**Goal**: One locally testable script that proves the production key pairing and builds the manifest. It does not rely on tauri-action's draft-URL behaviour.
**Implementation Logic**:
`scripts/build-latest-json.mjs` (Node, no dependencies) takes `--assets <dir> --version <V> --tag <tag> --repo <owner/repo> --conf src-tauri/tauri.conf.json --out <file>`. It then:
1. base64-decodes `plugins.updater.pubkey` to a temporary `.pub`;
2. for each `*.app.tar.gz`, base64-decodes the matching `.sig` and runs `minisign -Vm <tar.gz> -p <pub> -x <sig>`, failing on any mismatch;
3. maps assets to `darwin-aarch64` (`aarch64`) and `darwin-x86_64` (`x64`), failing if either is missing or ambiguous;
4. writes `{version, notes, pub_date, platforms{<key>{signature: <.sig content verbatim>, url: https://github.com/<repo>/releases/download/<tag>/<asset>}}}`.

`scripts/build-latest-json.test.mjs` generates a throwaway key with `pnpm tauri signer generate -w <tmp> --ci -p ''` and signs dummy tarballs with `pnpm tauri signer sign`. It writes a temporary conf containing the throwaway pubkey, then covers the positive, wrong-key and missing-platform cases.
**Deliverables**: `scripts/build-latest-json.mjs` (`decodeMinisign`, `verifyAsset`, `mapPlatforms`, `main`); `scripts/build-latest-json.test.mjs`
**Consistency Checks**: `node --test scripts/build-latest-json.test.mjs` (expected: PASS)
**Commit**: `build(release): assemble and verify latest.json from signed assets`

## Step 2: Release-PR script
**Goal**: Cutting a release is one command whose output is a PR, never a tag.
**Implementation Logic**:
`scripts/release-pr.sh [--no-push]`:
1. Resolve `REMOTE="${REMOTE:-$(git config branch.main.remote || echo origin)}"`, then `git fetch "$REMOTE" --tags`. Require a clean tree.
2. Run `git switch -c release/next "$REMOTE/main"`, failing if the branch exists unless `--force`.
3. Run `uvx --from commitizen==4.19.1 cz bump --changelog --yes`.
   - Exit 3 (no commits since the tag) and exit 21 (nothing bumpable): print why, switch back, delete the branch, exit 0.
   - Any other non-zero exit is an error.
4. Delete the local tag cz created (`git tag -d "v$(node scripts/check-release-config.mjs --print-version)"`). CI is the only tag author.
5. Unless `--no-push`, push the branch and `gh pr create --base main --title "release(sift): v<V>"`, with a body containing the new CHANGELOG section.
**Deliverables**: `scripts/release-pr.sh` (flags `--no-push`, `--force`)
**Consistency Checks**: `bash -n scripts/release-pr.sh` (expected: PASS)
**Commit**: `build(release): add release-pr script that cuts a cz bump branch`

## Step 3: Release workflow
**Goal**: A merge to `main` that changes the version becomes a published, verified release. Every other merge is a no-op.
**Implementation Logic**:
Rewrite `.github/workflows/release.yml`.

**Triggers and settings:**
- `push: branches: [main]`.
- `pull_request` with `paths` = the workflow itself, `src-tauri/tauri.conf.json`, `.cz.toml`, `scripts/build-latest-json.mjs`, `scripts/check-release-config.mjs`. These runs are rehearsal mode.
- `workflow_dispatch` with input `dry_run` (boolean, default true).
- `concurrency: {group: release, cancel-in-progress: false}`. Per-job `permissions` (only the jobs that write get `contents: write`).
- Every job that uses secrets carries `if: github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository`.

**Jobs:**
1. **`decide`** (ubuntu): V = `node scripts/check-release-config.mjs --print-version`.
   - Rehearsal (PR or dry_run): tag = `dryrun-${{ github.run_id }}`, `release=true`, `dry=true`.
   - Otherwise: `release=true` only if `git ls-remote --tags origin "refs/tags/vV"` is empty **and** `gh release view vV --json isDraft` does not show a published release.
   - Outputs: `version`, `tag`, `release`, `dry`.
2. **`create-release`** (needs `decide`, if `release`):
   - Delete any existing **draft** with this tag (`gh release view` + `isDraft`; never delete a published one).
   - `gh release create <tag> --draft --target ${{ github.sha }} --title "Sift v<V>"`, with notes = the `## v<V> (` section of CHANGELOG.md (port colgrep-mcp's extraction) plus the bridge note: "Versions ≤ 0.1.4 have no updater: install this version manually once; later updates install from inside the app."
   - Output `releaseId` (`gh api` id).
3. **`build`** (needs `create-release`): matrix `aarch64-apple-darwin` / `x86_64-apple-darwin` on macos-latest.
   - rust target, `rust-cache` with `key: ${{ matrix.target }}`, `pnpm install --frozen-lockfile`.
   - tauri-action pinned by SHA, with `releaseId`, `uploadUpdaterJson: false`, `args: --target <t>`, signing secrets as today, `APPLE_SIGNING_IDENTITY: '-'`.
4. **`assemble-verify`** (needs `build`, macos-latest):
   - `brew install minisign`.
   - `gh release download <tag> --pattern '*.app.tar.gz*'`.
   - `node scripts/build-latest-json.mjs … --tag v<V>`. Use the real tag in URLs even in rehearsal; the draft is deleted afterwards.
   - **Intel gate:** extract the x64 tarball; `lipo -archs` on `Sift.app/Contents/MacOS/*` and on the bundled `libpdfium.dylib` must both print `x86_64`. Do the same for aarch64.
   - `gh release upload <tag> latest.json --clobber`.
5. **`publish`** (needs `assemble-verify`, if not `dry`):
   - `gh release edit vV --draft=false --latest`.
   - Then smoke-test anonymously: `curl -fsSL https://github.com/<repo>/releases/latest/download/latest.json` has version V and both platforms, and `curl -fsIL` on one asset URL succeeds.
   - On any smoke failure, `gh release edit vV --draft=true` and fail the job.
6. **`cleanup`** (if `dry` and `always()`): `gh release delete <tag> --yes`. No git tag exists, because drafts create none.
**Deliverables**: `.github/workflows/release.yml` (jobs `decide`, `create-release`, `build`, `assemble-verify`, `publish`, `cleanup`)
**Consistency Checks**: `actionlint .github/workflows/release.yml` (expected: PASS)
**Commit**: `ci(release): build, verify and auto-publish on release-PR merge`

## Gotchas
- **VERIFIER leaf.** Attack:
  - Can a merge at 0.1.4 republish over the existing v0.1.4 **draft**? (The `ls-remote` tag check must win.)
  - Can a rehearsal touch any `v*` release?
  - Are secrets reachable from a fork PR?
  - Is a re-run after a failed build safe?
  - Can `publish` run without the minisign proof?
  - Can the Intel and ARM assets be swapped?
  - Can the pinned-SHA comments lie?
- tauri-action has **no** `includeUpdaterJson` input (measured); the input is `uploadUpdaterJson`. `@v0` floats, so pin a SHA.
- cz exit codes (measured on 4.19.1): 3 = no commits since the last tag, 21 = nothing bumpable.
- Rollback and yank are documented by docs_closure from this design: `gh release edit vX --draft=true`. `allowDowngrades` is false, so installed vX stays put.
