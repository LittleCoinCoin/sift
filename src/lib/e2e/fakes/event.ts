// Scripted stand-in for `@tauri-apps/api/event`, aliased in vite.e2e.config.ts.
// `listen()` records handlers so the harness can fire events as the macOS menu would.
import type {
  listen as realListen,
  Event,
  EventCallback,
  EventName,
  Options,
  UnlistenFn,
} from '@tauri-apps/api/event';

const handlers = new Map<string, Set<EventCallback<unknown>>>();
let nextEventId = 0;

export async function listen<T>(
  event: EventName,
  handler: EventCallback<T>,
  _options?: Options,
): Promise<UnlistenFn> {
  const set = handlers.get(event) ?? new Set<EventCallback<unknown>>();
  set.add(handler as EventCallback<unknown>);
  handlers.set(event, set);
  return () => {
    set.delete(handler as EventCallback<unknown>);
  };
}

// Compile-time proof that the fake keeps the real signature.
export const _conforms: typeof realListen = listen;

export function listenerCount(event: string): number {
  return handlers.get(event)?.size ?? 0;
}

/** Fire `event` at every captured handler, like the Rust side's `emit`. */
export function emitForTest(event: string, payload?: unknown): void {
  const envelope: Event<unknown> = { event, id: nextEventId++, payload };
  for (const handler of [...(handlers.get(event) ?? [])]) handler(envelope);
}
