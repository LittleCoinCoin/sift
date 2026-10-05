# Release Config

**Goal**: Make root `package.json` the single version source, realigned to the last real tag `v0.1.4`, turn on signed updater artifacts, and add a checker that fails on any drift.
**Pre-conditions**:
- [ ] Working on `task/release_config`, branched from the integration tip
- [ ] `git ls-remote --tags LittleCoinCoin` lists `v0.1.4` and no `v0.1.5`
**Success Gates**:
- ⬜ `node scripts/check-release-config.mjs` exits 0 on the branch tip [run]
- ⬜ `node --test scripts/check-release-config.test.mjs` passes, and includes negative cases where the checker exits non-zero: a `Cargo.toml` version drift, a `Cargo.lock` sift-entry drift, `createUpdaterArtifacts: false`, an empty `pubkey`, an `http://` endpoint, and a literal `tauri.conf.json` version instead of `../package.json` [run]
- ⬜ `node -p "require('./package.json').version"` prints `0.1.4` [run]
- ⬜ `cd src-tauri && cargo metadata --locked --format-version 1 >/dev/null && cargo check` passes [run]
- ⬜ `git diff --exit-code LittleCoinCoin/main -- pnpm-lock.yaml` shows no lockfile change [run]
**References**: Tauri v2 config reference (`version` may be a path to a package.json; `bundle.createUpdaterArtifacts`); audit finding in the campaign README Context

## Step 1: Realign manifests to the last released tag
**Goal**: Version state agrees with the tag history, so commitizen computes the next version from `v0.1.4`.
**Implementation Logic**:
0.1.5 was bumped by hand (`68c78b6`) but never tagged on the remote or released. Commitizen resolves the current version from `package.json` and the previous release from tag `v0.1.4`, so leaving 0.1.5 in place would make the first bump inconsistent.
1. Set `package.json` and `src-tauri/Cargo.toml` `[package].version` to `0.1.4`.
2. Re-lock with `cargo update -w --manifest-path src-tauri/Cargo.toml`. Only the `sift` entry in `Cargo.lock` may change; confirm this with `git diff --stat`.
3. Leave `tauri.conf.json` to Step 2.
**Deliverables**: `package.json` (`version` = `0.1.4`), `src-tauri/Cargo.toml` (`version = "0.1.4"`), `src-tauri/Cargo.lock` (sift entry `0.1.4`)
**Consistency Checks**: `cd src-tauri && cargo metadata --locked --format-version 1 >/dev/null` (expected: PASS)
**Commit**: `chore(release): realign manifests to last released tag v0.1.4`

## Step 2: Single version source and updater artifacts
**Goal**: The bundle version can never disagree with `package.json`, and every release build emits signed `.app.tar.gz` + `.sig`.
**Implementation Logic**:
1. In `src-tauri/tauri.conf.json`, set `"version": "../package.json"`.
2. Add `"createUpdaterArtifacts": true` under `bundle`.
3. Keep the `plugins.updater` block unchanged.
4. Side effect: `tauri build` now requires `TAURI_SIGNING_PRIVATE_KEY`. Do not run `tauri build` as a gate here. `cargo check` exercises `tauri-build` config parsing, and `generate_context!` must accept the path form.
5. If `cargo check` rejects the path form, report it as a spec defect. Do not fall back to a literal version silently.
**Deliverables**: `src-tauri/tauri.conf.json` with `version: "../package.json"` and `bundle.createUpdaterArtifacts: true`
**Consistency Checks**: `node -e "const c=require('./src-tauri/tauri.conf.json'); process.exit(c.version==='../package.json' && c.bundle.createUpdaterArtifacts===true ? 0 : 1)" && (cd src-tauri && cargo check)` (expected: PASS)
**Commit**: `build(release): emit signed updater artifacts from a single version source`

## Step 3: Release-config checker
**Goal**: One dependency-free command that CI, `release.yml` and humans use to prove the release config is coherent, and to read the version.
**Implementation Logic**:
`scripts/check-release-config.mjs` (Node ≥ 22, no dependencies):
1. Accept `--root <dir>`, defaulting to the repo root, and `--print-version`, which prints only the version on success.
2. Assert all of the following:
   - `package.json` version is semver;
   - the `src-tauri/Cargo.toml` `[package]` version equals it;
   - the `src-tauri/Cargo.lock` `name = "sift"` entry's version equals it;
   - `tauri.conf.json` `version === "../package.json"`;
   - `bundle.createUpdaterArtifacts === true`;
   - `plugins.updater.pubkey` is non-empty base64;
   - every `plugins.updater.endpoints` entry starts with `https://`.
3. Print one line per check, and exit 1 on the first failure set.

`scripts/check-release-config.test.mjs` (`node:test`) copies the real files into `fs.mkdtemp` fixtures, mutates one fact per case, and asserts the exit codes, including the positive case.
**Deliverables**: `scripts/check-release-config.mjs` (functions `readVersions`, `checkTauriConf`, `main`; flags `--root`, `--print-version`); `scripts/check-release-config.test.mjs`
**Consistency Checks**: `node --test scripts/check-release-config.test.mjs && test "$(node scripts/check-release-config.mjs --print-version)" = 0.1.4` (expected: PASS)
**Commit**: `build(release): add release-config consistency checker`

## Gotchas
- **VERIFIER leaf.** Attack: can any drift between the manifests pass the checker? Can `Cargo.lock` gain unrelated changes? Does `cargo check` really parse the path-form version, or is it ignored?
- `.cz.toml` belongs to commit_convention, a sibling at the same level. Do not create or edit it here. The coordinator checks the seam at L1 close (`cz bump --get-next` = `0.2.0`).
- The local tag `v0.1.5` is deleted by the coordinator, not here.
