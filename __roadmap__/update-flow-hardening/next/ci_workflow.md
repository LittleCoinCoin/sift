# CI Workflow

**Goal**: Run every local gate online on each pull request and on every push to `main`, with stable job names that branch protection can require.
**Pre-conditions**:
- [ ] Working on `task/ci_workflow`, branched from the integration tip after L1
**Success Gates**:
- ⬜ `actionlint .github/workflows/ci.yml` exits 0 [run]
- ⬜ The job ids are exactly `commits`, `config`, `frontend`, `rust`: `grep -E '^  [a-z-]+:$' .github/workflows/ci.yml` prints those four [static]
- ⬜ `grep -q 'contents: read' .github/workflows/ci.yml`, and every non-`actions/` `uses:` is pinned to a 40-hex SHA with a version comment [static]
- ⬜ Local replay of every job command passes: `pnpm check && pnpm build && node --test src/lib/stores/*.test.ts scripts/*.test.mjs && node scripts/check-release-config.mjs && (cd src-tauri && cargo test --lib)` [run]
**References**: colgrep-mcp `.github/workflows/ci.yml` (`commits` job shape); campaign README Gotchas (remote name, no `tauri build` in CI)

## Step 1: CI workflow
**Goal**: One workflow, four independent jobs, least privilege.
**Implementation Logic**:
`.github/workflows/ci.yml`:
- **Triggers:** `on: pull_request` and `push: branches: [main]`. `permissions: contents: read` at the top. `concurrency` per ref with `cancel-in-progress: true`.
- **`commits`** (`if: github.event_name == 'pull_request'`, ubuntu-latest):
  - `actions/checkout` with `fetch-depth: 0`, then `astral-sh/setup-uv`.
  - `uvx --from commitizen==4.19.1 cz check --rev-range origin/${{ github.base_ref }}..HEAD`.
  - Then `uvx --from commitizen==4.19.1 cz check --message "$PR_TITLE"`, with `PR_TITLE` passed through `env:` (never interpolated into the script) because GitHub uses the PR title as the merge body.
- **`config`** (ubuntu): `node scripts/check-release-config.mjs` and `node --test scripts/*.test.mjs`.
- **`frontend`** (ubuntu): pnpm (`pnpm/action-setup`, version from `packageManager` or 9), `setup-node` 22 with pnpm cache, then `pnpm install --frozen-lockfile`, `pnpm check`, `pnpm build`, `node --test src/lib/stores/*.test.ts`.
  - Node 22 needs `--experimental-strip-types` for `.ts` tests. Prefer Node 24 if `setup-node` supports it, and record which.
- **`rust`** (macos-latest):
  - pnpm + node, `pnpm install --frozen-lockfile`, `pnpm build` (`frontendDist` must exist for `generate_context!`).
  - `Swatinem/rust-cache` with `workspaces: src-tauri`, then `cargo test --lib` in `src-tauri`. This excludes `tests/ocr_pipeline.rs`, which needs a local Ollama.
- **Never** run `tauri build`: it needs the signing key once `createUpdaterArtifacts` is on.
- Pin third-party actions (`pnpm/action-setup`, `Swatinem/rust-cache`, `astral-sh/setup-uv`) to SHAs resolved with `gh api repos/<o>/<r>/git/ref/tags/<tag>`, plus a `# vX.Y.Z` comment. Use the current major for `actions/*`.
**Deliverables**: `.github/workflows/ci.yml` (jobs `commits`, `config`, `frontend`, `rust`)
**Consistency Checks**: `actionlint .github/workflows/ci.yml` (expected: PASS)
**Commit**: `ci(ci): gate commits, release config, frontend and rust on every PR`

## Gotchas
- Not a verifier leaf; it merges on the implementer's report. The coordinator checks at L2 close that the job names match CONTRIBUTING §Gates and the branch-protection command in publish_cycle.
- Locally the remote is `LittleCoinCoin`, but in CI it is `origin`. Only the workflow may say `origin/`.
- The keyring tests may need a login keychain on the runner. If `cargo test --lib` fails there for keychain reasons only, report it; do not mark tests `#[ignore]` without saying so.
