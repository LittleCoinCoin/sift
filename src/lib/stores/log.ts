import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';

export type LogLevel = 'info' | 'warn' | 'error' | 'success';

export interface LogEvent {
  level: LogLevel;
  message: string;
  timestamp: number;
}

export interface ProgressState {
  done: number;
  total: number;
  avg_ms: number;
  current_file?: string;
}

export interface Toast extends LogEvent {
  id: number;
}

let _nextId = 0;
let _initialized = false;
let _isJobActive: () => boolean = () => false;

export function registerJobActiveCheck(fn: () => boolean) {
  _isJobActive = fn;
}

export const toasts = writable<Toast[]>([]);
export const progress = writable<ProgressState | null>(null);

export function dismissToast(id: number) {
  toasts.update(ts => ts.filter(t => t.id !== id));
}

export function showToast(level: LogLevel, message: string) {
  const toast: Toast = { level, message, timestamp: Date.now(), id: _nextId++ };
  toasts.update(ts => [toast, ...ts].slice(0, 5));
  const delay = level === 'error' ? 0 : level === 'warn' ? 8000 : 4000;
  if (delay > 0) setTimeout(() => dismissToast(toast.id), delay);
}

export async function initLogStore() {
  if (_initialized) return;
  _initialized = true;

  await listen<LogEvent>('log', ({ payload }) => {
    // Suppress per-file success toasts while a job is running (Visual Spec §7)
    if (payload.level === 'success' && _isJobActive()) return;
    const toast: Toast = { ...payload, id: _nextId++ };
    toasts.update(ts => [toast, ...ts].slice(0, 5));

    const delay =
      payload.level === 'error' ? 0 :
      payload.level === 'warn'  ? 8000 : 4000;
    if (delay > 0) setTimeout(() => dismissToast(toast.id), delay);
  });

  await listen<ProgressState>('progress', ({ payload }) => {
    progress.set(payload);
    if (payload.done >= payload.total) {
      setTimeout(() => progress.set(null), 1500);
    }
  });
}
