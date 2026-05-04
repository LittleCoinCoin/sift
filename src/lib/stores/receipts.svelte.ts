import { invoke } from '@tauri-apps/api/core';
import { showToast } from './log';

export interface ReceiptEntry {
  source_path: string;
  status: 'Unprocessed' | 'Processing' | 'Processed';
  fields: Record<string, string> | null;
  source_mtime: number;
}

export type EditableField = string;

class ReceiptStore {
  files = $state<ReceiptEntry[]>([]);
  selectedPaths = $state<Set<string>>(new Set());
  #pendingCommits = new Map<string, Promise<void>>();

  get selectedFile(): ReceiptEntry | null {
    const [first] = this.selectedPaths;
    return first ? (this.files.find(f => f.source_path === first) ?? null) : null;
  }

  selectFile(path: string) {
    this.selectedPaths = new Set([path]);
  }

  addSelection(path: string) {
    this.selectedPaths = new Set([...this.selectedPaths, path]);
  }

  removeSelection(path: string) {
    const next = new Set(this.selectedPaths);
    next.delete(path);
    this.selectedPaths = next;
  }

  toggleSelection(path: string) {
    if (this.selectedPaths.has(path)) {
      this.removeSelection(path);
    } else {
      this.addSelection(path);
    }
  }

  clearSelection() {
    this.selectedPaths = new Set();
  }

  setFiles(newFiles: ReceiptEntry[]) {
    const sorted = [...newFiles].sort((a, b) => a.source_path.localeCompare(b.source_path));
    this.files = sorted;
    if (sorted.length > 0 && this.selectedPaths.size === 0) {
      this.selectedPaths = new Set([sorted[0].source_path]);
    }
  }

  updateField(path: string, key: string, value: string) {
    const entry = this.files.find(f => f.source_path === path);
    if (!entry || !entry.fields) return;
    entry.fields[key] = value;
  }

  commitField(path: string): Promise<void> {
    const entry = this.files.find(f => f.source_path === path);
    if (!entry || !entry.fields) return Promise.resolve();
    const fields = { ...entry.fields };

    const previous = this.#pendingCommits.get(path) ?? Promise.resolve();
    const next = previous
      .catch(() => undefined)
      .then(() => invoke<void>('update_receipt_fields', { sourcePath: path, fields }))
      .catch((e: unknown) => {
        showToast('error', `Failed to save edits: ${e instanceof Error ? e.message : String(e)}`);
      })
      .finally(() => {
        if (this.#pendingCommits.get(path) === next) {
          this.#pendingCommits.delete(path);
        }
      });
    this.#pendingCommits.set(path, next);
    return next;
  }

  flushPendingCommits(): Promise<void> {
    return Promise.all([...this.#pendingCommits.values()]).then(() => undefined);
  }
}

export const receipts = new ReceiptStore();
