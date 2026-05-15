# Install Plugin Dependencies

**Goal**: Add `tauri-plugin-updater` (Rust) and `@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process` (JS) to the project, matching the existing version constraint convention.
**Pre-conditions**:
- [ ] Branch created from `main`
**Success Gates**:
- ✅ `cargo check` from `src-tauri/` exits 0 with `tauri-plugin-updater` resolved [run]
- ✅ `node_modules/@tauri-apps/plugin-updater` and `node_modules/@tauri-apps/plugin-process` directories exist after install [static]
**References**: [R01 Plan](../../../.claude/plans/assuming-the-current-project-enchanted-meteor.md) — dependency version rationale (§ Work breakdown A)

---

## Step 1: Add Rust crate

**Goal**: Declare `tauri-plugin-updater` as a Cargo dependency.

**Implementation Logic**: Add `tauri-plugin-updater = "2"` to the `[dependencies]` section of `src-tauri/Cargo.toml`. The `"2"` constraint matches the pattern already used by `tauri-plugin-dialog` and `tauri-plugin-opener` in the same file. No features flag needed — the default feature set covers the update check and install flow.

**Deliverables**: `src-tauri/Cargo.toml` — new `tauri-plugin-updater` entry in `[dependencies]`

**Consistency Checks**: `cargo check --manifest-path src-tauri/Cargo.toml` (expected: PASS)

**Commit**: `chore(deps): add tauri-plugin-updater rust crate`

---

## Step 2: Add JS packages

**Goal**: Declare the two JS counterpart packages as runtime dependencies.

**Implementation Logic**: Add `@tauri-apps/plugin-updater` and `@tauri-apps/plugin-process` at version `^2` to the `dependencies` section of `package.json`. The `^2` constraint matches `@tauri-apps/plugin-dialog` and `@tauri-apps/plugin-opener` already present. Run `pnpm install --frozen-lockfile` to update the lockfile — this should succeed cleanly since only new packages are being added.

**Deliverables**: `package.json` — two new `@tauri-apps/plugin-*` entries; `pnpm-lock.yaml` — lockfile updated

**Consistency Checks**: `pnpm install --frozen-lockfile` (expected: PASS)

**Commit**: `chore(deps): add plugin-updater and plugin-process js packages`
