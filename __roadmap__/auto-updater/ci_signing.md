# Add CI Signing Secrets

**Goal**: Expose `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` to the existing release workflow so `tauri-action` generates and uploads a signed `latest.json` on every release.
**Pre-conditions**:
- [ ] Developer has generated the signing key pair (`pnpm tauri signer generate`) and stored the private key as a GitHub Repository Secret named `TAURI_SIGNING_PRIVATE_KEY`
- [ ] Branch created from `main`
**Success Gates**:
- ✅ The "Build and bundle" step in `release.yml` references both secrets [static]
- ✅ After a test release push, the GitHub release contains a `latest.json` asset alongside the DMG files [behavioral — requires a real tag push]
**References**: [R01 Plan](../../../.claude/plans/assuming-the-current-project-enchanted-meteor.md) — CI setup rationale (§ Work breakdown G)

---

## Step 1: Add signing environment variables to the release workflow

**Goal**: Enable `tauri-action` to sign update artifacts and generate `latest.json`.

**Implementation Logic**: In `.github/workflows/release.yml`, find the "Build and bundle" step that calls `tauri-apps/tauri-action@v0`. Add `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` to its `env:` block, sourcing both from GitHub Secrets. These sit alongside the existing `GITHUB_TOKEN` and `APPLE_SIGNING_IDENTITY` env vars already in that step. No other changes to the workflow are needed — `tauri-action` automatically generates and uploads `latest.json` when the private key is present. The matrix build (aarch64 + x86_64) is handled correctly: `tauri-action` merges both platform entries into a single `latest.json` on the same GitHub release.

**Deliverables**: `.github/workflows/release.yml` — `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` added to the `env:` block of the "Build and bundle" step

**Consistency Checks**: `COLGREP_BYPASS=1 grep -q "TAURI_SIGNING_PRIVATE_KEY" .github/workflows/release.yml` (expected: PASS)

**Commit**: `ci(release): add updater signing secrets to build step`
