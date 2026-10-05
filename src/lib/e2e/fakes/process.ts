// Scripted stand-in for `@tauri-apps/plugin-process`, aliased in vite.e2e.config.ts.
import type { relaunch as realRelaunch } from '@tauri-apps/plugin-process';

let calls = 0;

export async function relaunch(): Promise<void> {
  calls += 1;
}

// Compile-time proof that the fake keeps the real signature.
export const _conforms: typeof realRelaunch = relaunch;

export function relaunchCalls(): number {
  return calls;
}
