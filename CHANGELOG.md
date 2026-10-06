## v0.2.1 (2026-10-06)

### Fixed

- **backend**: run keychain calls on the blocking pool, not an async worker
- **backend**: keep the window responsive while the keychain prompts
- **backend**: store API keys in the macOS Keychain instead of a mock

## v0.2.0 (2026-10-05)

### Added

- **updater**: add in-app auto-update via tauri-plugin-updater
- **config**: set real updater pubkey
- **updater**: add native macos check-for-updates menu item
- **updater**: register tauri-plugin-updater in builder
- **updater**: wire initUpdaterStore into App onMount
- **updater**: implement download with progress and restart toast
- **updater**: implement update check with toast feedback
- **updater**: scaffold updater store with init and event listener
- **toast**: render action button in toast component
- **toast**: add action button and persistent flag to Toast type
- **config**: add updater plugin endpoint and pubkey

### Fixed

- **updater**: explain cross-device install failures from a mounted dmg
- **release**: resolve symlinks in checker entry-point guard
- **updater**: harden toast state against late events and stale toasts
- **updater**: show restart only after the install succeeds
- **updater**: classify install failures into actionable messages
- **menu**: keep the default macOS menus when adding check for updates
- **updater**: harden download flow and add local test server script
- **updater**: replace allow-download with allow-download-and-install
- **updater**: add missing process plugin and correct permission names
- **updater**: grant check, download and relaunch permissions
