export interface ReceiptEntry {
  source_path: string;
  status: 'Unprocessed' | 'Processing' | 'Processed';
  fields: Record<string, string> | null;
  source_mtime: number;
}

export type EditableField = string;

class ReceiptStore {
  files = $state<ReceiptEntry[]>([]);
  selectedPath = $state<string | null>(null);
  processing = $state<Set<string>>(new Set());

  get selectedFile(): ReceiptEntry | null {
    return this.files.find(f => f.source_path === this.selectedPath) ?? null;
  }

  selectFile(path: string) {
    this.selectedPath = path;
  }

  setFiles(newFiles: ReceiptEntry[]) {
    this.files = newFiles;
    if (newFiles.length > 0 && !this.selectedPath) {
      this.selectedPath = newFiles[0].source_path;
    }
  }

  setProcessing(path: string, active: boolean) {
    const next = new Set(this.processing);
    if (active) next.add(path); else next.delete(path);
    this.processing = next;
  }

  isProcessing(path: string): boolean {
    return this.processing.has(path);
  }
}

export const receipts = new ReceiptStore();
