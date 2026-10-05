# Configure Updater Plugin in tauri.conf.json

**Goal**: Add the `plugins.updater` configuration block to `src-tauri/tauri.conf.json` so the plugin knows where to fetch the update manifest and how to verify artifacts.
**Pre-conditions**:
- [ ] Developer has the public key string from the signing key pair (output of `pnpm tauri signer generate`)
- [ ] Branch created from `main`
**Success Gates**:
- ✅ `src-tauri/tauri.conf.json` contains a top-level `"plugins"` key with a nested `"updater"` object [static]
- ✅ The `"endpoints"` array contains the GitHub `/releases/latest/download/latest.json` URL [static]
- ✅ `"dialog"` is set to `false` [static]
- ✅ `"pubkey"` is set to the developer public key (not a placeholder) [static]
**References**: [R01 Plan](../../../.claude/plans/assuming-the-current-project-enchanted-meteor.md) — endpoint URL rationale and pubkey explanation (§ Work breakdown B)

---

## Step 1: Add plugins.updater configuration block

**Goal**: Tell the updater plugin where to look for updates and how to verify them.

**Implementation Logic**: In `src-tauri/tauri.conf.json`, add a new top-level `"plugins"` object (the file currently has none). Inside it, add an `"updater"` object with three fields: `"endpoints"` (array containing `https://github.com/LittleCoinCoin/sift/releases/latest/download/latest.json`), `"dialog": false` (disables the built-in Tauri update dialog so the custom toast flow takes over), and `"pubkey"` (the developer's public key string). The endpoint URL uses GitHub's redirect mechanism — it resolves to the most recently published non-draft release's `latest.json` asset. Draft releases are invisible to this URL, which is intentional.

**Deliverables**: `src-tauri/tauri.conf.json` — new `"plugins"` top-level key containing `"updater"` with `"endpoints"`, `"dialog"`, and `"pubkey"` fields

**Consistency Checks**: `python3 -c "import json,sys; d=json.load(open('src-tauri/tauri.conf.json')); assert 'plugins' in d and 'updater' in d['plugins']"` (expected: PASS)

**Commit**: `feat(config): add updater plugin endpoint and pubkey`
