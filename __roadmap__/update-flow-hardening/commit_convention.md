# Commit Convention

**Goal**: Formalise the history-derived `type(scope): description` convention as commitizen config and enforce it with a fail-closed local hook in every worktree, so CI's later check is never the first line of defence.
**Pre-conditions**:
- [ ] Working on `task/commit_convention`, branched from the integration tip of `milestone/update-flow-hardening`
- [ ] `uvx --from commitizen==4.19.1 cz version` prints `4.19.1`
**Success Gates**:
- ⬜ `bash scripts/probe_cz_check.sh` exits 0. It **rejects**: `chore(x): trailing.`, `bump: version 0.1.4 → 0.2.0`, `feat: no scope`, `Feat(x): capital type`, `feat(X): capital scope`, `fix(x): Capital start`, and a 101-char subject. It **accepts**: `rdm(update-flow-hardening): author roadmap`, `report(updater-audit): add e2e results`, `release(sift): v0.2.0`, `ci(release): pin actions`, `Merge pull request #5 from a/b`, `Merge task/a into milestone/b` [run]
- ⬜ In a fresh hand-made worktree, after `bash scripts/install-hooks.sh`: `git commit --allow-empty -m bad` exits non-zero, and `PATH=/usr/bin:/bin git commit --allow-empty -m "chore(x): valid"` also exits non-zero (fails closed without uvx) [run]
- ⬜ `uvx --from commitizen==4.19.1 cz check --rev-range LittleCoinCoin/main..HEAD` passes [run]
- ⬜ `uvx --from commitizen==4.19.1 cz changelog --dry-run` output contains no `worktree`-scoped entry [run]
**References**: colgrep-mcp `server/pyproject.toml` [tool.commitizen*] (config shape to port); colgrep-mcp `dev/skills/landing-and-release/references/commitizen-gotchas.md` (`$` anchor, `!` vs footer, `--rev-range` cannot show rejections); colgrep-mcp `dev/skills/landing-and-release/scripts/probe_cz_check.sh` (probe pattern)

## Step 1: Commitizen configuration
**Goal**: One `.cz.toml` that owns the vocabulary, the bump mapping and the version-file wiring.
**Implementation Logic**:
1. Create `.cz.toml` at the repo root with `[tool.commitizen]`:
   - `name = "cz_customize"`, `tag_format = "v$version"`, `major_version_zero = true`
   - `version_provider = "npm"`: root `package.json` is the canonical version
   - `version_files = ["src-tauri/Cargo.toml:^version =", "src-tauri/Cargo.lock:^name = \"sift\"$"]`. The second entry rewrites nothing; it stages the file the pre-bump hook re-locked.
   - `pre_bump_hooks = ["cargo update -w --manifest-path src-tauri/Cargo.toml"]`
   - `bump_message = "release(sift): v$new_version"`
   - `update_changelog_on_bump = true`, `changelog_incremental = true`, `changelog_start_rev = "v0.1.4"`
   - `allowed_prefixes = ["Merge", "Revert", "Pull request", "fixup!", "squash!", "Initial commit"]`
2. Add `[tool.commitizen.customize]`:
   - `schema_pattern`: types `feat|fix|perf|refactor|test|docs|build|ci|chore|rdm|report|release`, mandatory kebab-case scope `[a-z][a-z0-9-]*`, optional `!`, a lowercase/digit/backtick start, no trailing period, subject ≤ 100 chars, with `$`/newline anchoring as in the prior art
   - `bump_pattern` / `bump_map`: feat → MINOR, fix and perf → PATCH, `BREAKING CHANGE` → MAJOR
   - `commit_parser`
   - `changelog_pattern`: only feat/fix/perf/BREAKING, and exclude the scopes `worktree|roadmap|commit-convention` with a negative lookahead
   - `change_type_map`, `change_type_order`, `message_template`, `questions`
3. Keep the existing history valid. `b389baa..HEAD` must pass `cz check --rev-range`; its only non-conforming subjects are `Merge …`.
4. If a cz 4.19.1 key named here does not exist (e.g. `changelog_start_rev`), report it as a spec defect with the measured alternative. Do not silently drop it.
**Deliverables**: `.cz.toml` with the `[tool.commitizen]` and `[tool.commitizen.customize]` tables and the keys named above
**Consistency Checks**: `uvx --from commitizen==4.19.1 cz check --message "rdm(update-flow-hardening): author roadmap"` (expected: PASS); `uvx --from commitizen==4.19.1 cz check --message "bump: version 0.1.4 → 0.2.0"` (expected: FAIL)
**Commit**: `build(commit-convention): add commitizen config derived from history`

## Step 2: Validator probe
**Goal**: A reproducible accept/reject probe. `--rev-range` only ever sees accepted commits, so it cannot show a rejection.
**Implementation Logic**:
Port the colgrep-mcp `probe_cz_check.sh` as `scripts/probe_cz_check.sh`. It calls `uvx --from commitizen==4.19.1 cz check --message` for each subject listed in this leaf's first Success Gate, prints a PASS/FAIL table, and exits non-zero if any subject lands on the wrong side.
**Deliverables**: `scripts/probe_cz_check.sh` with `ACCEPT` and `REJECT` arrays
**Consistency Checks**: `bash scripts/probe_cz_check.sh` (expected: PASS)
**Commit**: `test(commit-convention): probe accepted and rejected subjects`

## Step 3: Fail-closed local hook
**Goal**: Every commit, in every worktree, is linted before it exists.
**Implementation Logic**:
1. `.githooks/commit-msg` (POSIX sh, executable) runs `uvx --from commitizen==4.19.1 cz check --allow-abort --commit-msg-file "$1"`. If `uvx` is not on PATH, it prints how to install uv and exits 1. It never passes silently.
2. Measured: `core.hooksPath` is pinned to an absolute `<main>/.git/hooks` in both `.git/config` and every worktree's `config.worktree`, and worktree config wins. So do **not** change git config. `scripts/install-hooks.sh` writes an executable stub at `$(git rev-parse --git-path hooks)/commit-msg` that execs `"$(git rev-parse --show-toplevel)/.githooks/commit-msg" "$@"`. It is idempotent and refuses to overwrite a non-stub hook without `--force`.
3. Every worktree resolves `--git-path hooks` to the same pinned directory, so one install covers all worktrees. Each worktree runs its own `.githooks` copy via `--show-toplevel`. Verify this in a fresh worktree.
**Deliverables**: `.githooks/commit-msg`; `scripts/install-hooks.sh` with a `--force` flag and a stub marker comment `# sift-commit-msg-stub`
**Consistency Checks**: the second Success Gate, run in a fresh hand-made worktree (expected: PASS)
**Commit**: `build(commit-convention): enforce commit messages with a fail-closed local hook`

## Step 4: Contributor documentation
**Goal**: The convention is readable without opening `.cz.toml`.
**Implementation Logic**:
Create `CONTRIBUTING.md` with a `## Commit convention` section only. `docs_closure` adds §Gates and §Releasing later. It covers:
- the type table (type, bump, use-for), including `rdm` for `__roadmap__/`, `report` for `__reports__/`, and `release`, which only cz writes
- the scope rules and seed scopes from history
- the subject rules, and why the body says WHY
- the merge-commit body rule: WHY prose, never starting with `type(scope):`
- how to install the hook (`bash scripts/install-hooks.sh`)
**Deliverables**: `CONTRIBUTING.md` with a `## Commit convention` heading
**Consistency Checks**: `for t in feat fix perf refactor test docs build ci chore rdm report release; do grep -q "^| .$t. |" CONTRIBUTING.md || exit 1; done` (expected: PASS)
**Commit**: `docs(contributing): document the commit convention and hook install`

## Gotchas
- **VERIFIER leaf.** Attack: does the hook fail open (no uvx, a different worktree, `--no-verify` aside)? Is the reject list missing a realistic bad subject? Does the schema accept every subject in `git log b389baa..HEAD`?
- The hook must not lint the `Merge …` messages git writes for `--no-ff` merges. `allowed_prefixes` covers them; prove it with the probe.
- Commits made before this leaf merges are not linted locally. CI's `--rev-range` check covers them later.
