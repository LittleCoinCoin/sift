# Keychain Prompt After Update: Signing Measurement (v0)

Date: 2026-10-06

---
type: findings
topic: api_key_storage
produced-by: experiment branch `experiment/keychain-signing` (d44fd2d enables `apple-native`, 272b88b relaxes the B isolation check), `pnpm e2e:native`
date: 2026-10-06
domain: code
confidence: confirmed for the outcome (two valid runs, both exit 0, both dialogs observed by the maintainer); inferred for the cause
decision: ship the real Keychain with ad-hoc signing; one prompt per update is documented; Developer ID deferred
---

## Headline

With `keyring`'s `apple-native` feature on, API keys really persist. But after an update, the new build cannot read the item the old build wrote until the user types their **login keychain password** in a "Sift wants to access "sift-e2e" in your keychain" dialog and clicks *Always Allow*. A free self-signed certificate does **not** avoid this. The dialog appeared even though both builds carried the same certificate-based designated requirement, and the signing Mac trusted the certificate.

| | Ad-hoc (run 1b, 10:12) | Self-signed (run 2b, 14:24) |
|---|---|---|
| Signature of B (0.0.1) and A (0.2.0) | `Signature=adhoc`; DR `cdhash H"f678fb0b…"` vs `cdhash H"9bd308de…"` | `Authority=Sift E2E Self-Signed`, `TeamIdentifier=not set`; both DR `identifier "dev.eliottjacopin.sift.e2e" and certificate leaf = H"1ce188e9…"` |
| 0.0.1 write | `keychain set ok`, `keychain roundtrip ok` (no dialog) | same |
| 0.2.0 read after update | no probe result in 26 s; password dialog seen | no probe result in 26 s; password dialog seen |
| Update flow, read-only cases, cleanup | all PASS, exit 0 | all PASS, exit 0 |

## Evidence

- Probe lines (from `happy.log`), run 1b: `10:14:21.595 keychain set ok`, `10:14:21.605 keychain roundtrip ok`, `10:14:32.357 booted 0.2.0`. After that the mock server received no POST from 0.2.0 until the app was killed at about 10:15:00. Run 2b shows the same pattern: `14:26:19.626` set ok, `14:26:30.432` booted 0.2.0, then silence.
- The read-only cases launch copies of the original B build, which have the same cdhash, and wrote with no dialog (`10:15:24.683 keychain set ok`). So the item's access follows the exact build that wrote it.
- The signature data is `codesign -dv --verbose=4` and `codesign -d -r-` output for `$TMPDIR/sift-e2e/{A,B}/Sift.app`, captured right after each run.
- Screenshots: none. `screencapture` failed (`could not create image from display`) because the terminal lacks the Screen Recording permission. The dialog text comes from the maintainer.

## Cause (inferred)

Since macOS 10.12, a login-keychain item carries a *partition list* naming the code allowed to read it without asking. For an app with no Apple Team ID, the partition entry falls back to the build's code hash. That hash changes with every update, so each update needs the keychain password to add the new build. A certificate-based designated requirement does not help, because only an Apple-issued certificate carries a Team ID. Expected fix if ever needed: a paid Developer ID (Team ID partition, plus notarization). That has not been measured.

## Protocol defects found on the way

1. **`verify_keychain_isolation` rejects build B once `apple-native` is on.** The check needed `sift-e2e` directly before `api-key` in B's `.rodata`, and the new layout puts it after. The fix is to check B for the literal alone, the same proof the check already accepts for A. Owned by the `e2e_keychain_assertions` leaf.
2. **Tauri's `APPLE_CERTIFICATE` import accepts only Apple-prefixed certificate names.** `tauri-macos-sign` `keychain/identity.rs` lists only `Developer ID Application:`, `Apple Development:` and similar. Self-signed signing works only through a keychain on the search list plus `APPLE_SIGNING_IDENTITY`.
3. **A failed `APPLE_CERTIFICATE` import leaves Tauri's temporary keychain in the user's search list.** It was removed by hand twice. This is moot now that self-signing is dropped.
4. **The auto-mode classifier blocks agents from running `pnpm e2e:native` on a branch that relaxes the isolation check.** The maintainer ran the E2E runs by hand.
5. **The `cargo` lockfile gains one package, `security-framework 2.11.1`.** keyring's iOS dependency pulls it in, and `cargo tree --target aarch64-apple-darwin` and `--target x86_64-apple-darwin` do not build it.

## Consequences for the implementation

- Enable `apple-native`. Users re-enter their API keys once, because nothing was ever persisted before.
- Run the six keyring commands off the main thread (`#[tauri::command(async)]`). Tauri runs blocking commands on the main thread, so a pending password dialog during `get_api_key` (`SettingsPanel.svelte` calls it on mount) can freeze the window. The maintainer reported "nothing was happening" until they answered.
- Make the E2E assert the persisted write and record the post-update outcome (`blocked` is expected under ad-hoc), instead of printing it.
- Note in the CHANGELOG that after each update macOS asks once for the login keychain password ("Always Allow").
