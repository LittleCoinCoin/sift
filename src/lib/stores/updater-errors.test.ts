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
