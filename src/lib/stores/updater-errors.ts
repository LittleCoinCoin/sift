// Pure helpers shared by the updater store and the E2E harness.
// No Tauri or Svelte imports: this module must load under plain Node.

export const INSTALL_MESSAGES = {
  permission:
    "Couldn't install the update: move Sift to your Applications folder (or approve the administrator prompt) and try again.",
  verification: 'The update failed verification and was not installed.',
  failedPrefix: 'Update failed: ',
  available: (version: string) => `Update ${version} available`,
  installLabel: 'Install',
  downloading: 'Downloading update…',
  downloadingPercent: (percent: number) => `Downloading update… ${percent}%`,
  downloadingKb: (kb: number) => `Downloading update… ${kb} KB`,
  installing: 'Installing update…',
  ready: 'Update ready — restart to apply',
  restartLabel: 'Restart Now',
  restartFailedPrefix: 'Restart failed: ',
} as const;

// tauri-plugin-updater rejects with plain strings (Error Display text), not
// a stable API. Substrings below are taken from tauri-plugin-updater 2.10.1,
// minisign-verify 0.2.5 and std::io::Error (rename/permission failures).
const VERIFICATION_PATTERNS = [
  'signature verification failed',
  'different key than the one provided',
  'invalid encoding in minisign data',
  'unexpected signature algorithm',
  'could not be decoded',
];

const PERMISSION_PATTERNS = [
  'failed to move the new app into place', // admin prompt cancelled or failed
  'user canceled', // AppleScript, error -128
  'user cancelled',
  '(-128)',
  'permission denied',
  'permissiondenied',
  'operation not permitted',
  'read-only file system',
  'authentication failed or was cancelled', // Linux pkexec path
];

export function classifyInstallError(raw: unknown): string {
  const text = String(raw);
  const lower = text.toLowerCase();
  if (VERIFICATION_PATTERNS.some((p) => lower.includes(p))) return INSTALL_MESSAGES.verification;
  if (PERMISSION_PATTERNS.some((p) => lower.includes(p))) return INSTALL_MESSAGES.permission;
  return `${INSTALL_MESSAGES.failedPrefix}${text}`;
}
