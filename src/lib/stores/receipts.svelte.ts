export interface ReceiptFile {
  type: 'image' | 'pdf';
  path: string;
}

export interface ReceiptRecord {
  date: string;
  category: string;
  entity: string;
  amount: string;
  payment_method: string;
  source_path: string;
}

export type EditableField = keyof Omit<ReceiptRecord, 'source_path'>;

class ReceiptStore {
  files = $state<ReceiptFile[]>([]);
  records = $state<Map<string, ReceiptRecord>>(new Map());
  selectedPath = $state<string | null>(null);
  processing = $state<Set<string>>(new Set());
  currentDir = $state<string>('');

  get selectedFile(): ReceiptFile | null {
    return this.files.find(f => f.path === this.selectedPath) ?? null;
  }

  get selectedRecord(): ReceiptRecord | null {
    return this.selectedPath ? (this.records.get(this.selectedPath) ?? null) : null;
  }

  setDir(dir: string) {
    this.currentDir = dir;
  }

  selectFile(path: string) {
    this.selectedPath = path;
  }

  setFiles(newFiles: ReceiptFile[]) {
    this.files = newFiles;
    if (newFiles.length > 0 && !this.selectedPath) {
      this.selectedPath = newFiles[0].path;
    }
  }

  setRecord(path: string, record: ReceiptRecord) {
    const next = new Map(this.records);
    next.set(path, record);
    this.records = next;
  }

  updateField(path: string, field: EditableField, value: string) {
    const existing = this.records.get(path);
    if (!existing) return;
    const next = new Map(this.records);
    next.set(path, { ...existing, [field]: value });
    this.records = next;
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
