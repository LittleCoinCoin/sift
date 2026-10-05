import { test } from 'node:test';
import assert from 'node:assert/strict';
import { classifyInstallError } from './updater-errors.ts';

const PERMISSION =
  "Couldn't install the update: move Sift to your Applications folder (or approve the administrator prompt) and try again.";
const VERIFICATION = 'The update failed verification and was not installed.';

test('AppleScript cancel (-128 / User canceled) maps to the permission copy', () => {
  assert.equal(
    classifyInstallError('execution error: User canceled. (-128)'),
    PERMISSION,
  );
});

test('permission denied maps to the permission copy', () => {
  assert.equal(classifyInstallError('PermissionDenied (os error 1)'), PERMISSION);
  assert.equal(classifyInstallError('Permission denied (os error 13)'), PERMISSION);
});

test('plugin message for a failed admin move maps to the permission copy', () => {
  // tauri-plugin-updater 2.10.1 returns this io::Error text when the
  // AppleScript admin prompt is cancelled or fails (src/updater.rs).
  assert.equal(classifyInstallError('Failed to move the new app into place'), PERMISSION);
});

test('read-only filesystem maps to the permission copy', () => {
  assert.equal(classifyInstallError('Read-only file system (os error 30)'), PERMISSION);
});

test('signature verification failures map to the verification copy', () => {
  assert.equal(classifyInstallError('The signature verification failed'), VERIFICATION);
  assert.equal(
    classifyInstallError('The signature was created with a different key than the one provided'),
    VERIFICATION,
  );
});

test('matching is case-insensitive', () => {
  assert.equal(classifyInstallError('READ-ONLY FILE SYSTEM'), PERMISSION);
});

test('unknown strings pass through unchanged behind the generic prefix', () => {
  assert.equal(classifyInstallError('disk on fire'), 'Update failed: disk on fire');
});

test('non-string inputs are stringified', () => {
  assert.equal(classifyInstallError(new Error('boom')), 'Update failed: Error: boom');
  assert.equal(classifyInstallError(42), 'Update failed: 42');
  assert.equal(classifyInstallError(undefined), 'Update failed: undefined');
});

// One case per classifier pattern, each input matching ONLY that pattern, so
// blanking any single pattern fails at least one test.
const PERMISSION_CASES: Record<string, string> = {
  'failed to move the new app into place': 'Failed to move the new app into place',
  'user canceled': 'User canceled',
  'user cancelled': 'User cancelled',
  usercancelled: 'UserCancelled',
  '(-128)': 'AppleScript error (-128)',
  'permission denied': 'Permission denied (os error 13)',
  permissiondenied: 'PermissionDenied (os error 1)',
  'operation not permitted': 'Operation not permitted (os error 1)',
  'read-only file system': 'Read-only file system (os error 30)',
  // measured by the native E2E: updating a Sift that runs from a mounted .dmg
  'cross-device link': 'Cross-device link (os error 18)',
  'authentication failed or was cancelled': 'Authentication failed or was cancelled',
};

const VERIFICATION_CASES: Record<string, string> = {
  'signature verification failed': 'The signature verification failed',
  'different key than the one provided':
    'The signature was created with a different key than the one provided',
  'invalid encoding in minisign data': 'Invalid encoding in minisign data',
  'unexpected signature algorithm': 'Unexpected signature algorithm',
  'could not be decoded':
    'The signature abc could not be decoded, please check if it is a valid base64 string.',
};

for (const [pattern, input] of Object.entries(PERMISSION_CASES)) {
  test(`permission pattern: ${pattern}`, () => {
    assert.equal(classifyInstallError(input), PERMISSION);
  });
}

for (const [pattern, input] of Object.entries(VERIFICATION_CASES)) {
  test(`verification pattern: ${pattern}`, () => {
    assert.equal(classifyInstallError(input), VERIFICATION);
  });
}

test('report-01 literal "-128 UserCancelled" is classified as permission/cancel', () => {
  assert.equal(classifyInstallError('-128 UserCancelled'), PERMISSION);
});

test('network failures are not mistaken for permission or verification errors', () => {
  const status = 'Download request failed with status: 403';
  assert.equal(classifyInstallError(status), `Update failed: ${status}`);
  const send = 'error sending request for url (https://github.com/o/r/releases/latest/download/latest.json)';
  assert.equal(classifyInstallError(send), `Update failed: ${send}`);
});
