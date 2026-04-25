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
    this.files = newFiles;
    if (newFiles.length > 0 && this.selectedPaths.size === 0) {
      this.selectedPaths = new Set([newFiles[0].source_path]);
    }
  }

}

export const receipts = new ReceiptStore();
