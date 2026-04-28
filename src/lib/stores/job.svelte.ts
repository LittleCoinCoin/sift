import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { showToast, registerJobActiveCheck } from './log';

// Lowercase to match Rust's serde(rename_all = "snake_case")
export type JobStatus =
  | 'idle' | 'running' | 'paused' | 'resuming'
  | 'cancelling' | 'cancelled' | 'completed' | 'failed';

export type Phase = 'ocr' | 'extract';

export interface ProgressEvent {
  done: number;
  total: number;
  avg_ms: number;
  job_id?: string;
  status?: string;
  current_file?: string;
  phase?: Phase;
}

export type FileOutcomeStatus = 'processed' | 'abandoned' | 'failed';

export interface FileOutcome {
  path: string;
  status: FileOutcomeStatus;
  in_flight: boolean;
}

export interface JobSummary {
  job_id: string;
  status: JobStatus;
  total: number;
  processed: number;
  abandoned: number;
  failed: number;
  files: FileOutcome[];
}

// Snake_case field names match backend serde defaults (no rename on JobConfig/FileEntry).
export interface FileEntry {
  path: string;
  file_type: string;
}

export interface JobConfig {
  files: FileEntry[];
  api_url: string;
  api_key: string;
  ocr_model: string;
  extraction_url: string;
  extraction_api_key: string;
  extraction_model: string;
  active_system_prompt: string;
  json_schema_keys: string[];
}

const ACTIVE_STATUSES: ReadonlySet<JobStatus> = new Set([
  'running', 'paused', 'resuming', 'cancelling',
]);

let _initialized = false;

class JobStore {
  status = $state<JobStatus>('idle');
  cancelSummary = $state<JobSummary | null>(null);
  cancelModalMode = $state<'confirm' | 'summary' | null>(null);
  private _jobId: string | null = null;

  get isActive(): boolean {
    return ACTIVE_STATUSES.has(this.status);
  }

  get cancelSummaryOpen(): boolean {
    return this.cancelModalMode !== null;
  }

  async start(config: JobConfig) {
    const jobId = await invoke<string>('start_job', { config });
    this._jobId = jobId;
    this.status = 'running';
  }

  async pause() {
    if (!this._jobId) return;
    await invoke('pause_job', { jobId: this._jobId });
    // Status will be confirmed via job_status event from orchestrator.
  }

  async resume() {
    if (!this._jobId) return;
    await invoke('resume_job', { jobId: this._jobId });
    // job_status: resuming is emitted immediately by handle, then running by orchestrator.
  }

  async cancel() {
    if (!this._jobId) return;
    await invoke('cancel_job', { jobId: this._jobId });
    // job_status: cancelling is emitted immediately by handle.
  }

  // Pause the job and open the cancel-confirmation modal.
  async requestCancel() {
    if (!this._jobId) return;
    if (this.status === 'running' || this.status === 'resuming') {
      await this.pause();
    }
    this.cancelModalMode = 'confirm';
  }

  // Confirmed from the confirm modal — close modal and send cancel to backend.
  async confirmCancel() {
    this.cancelModalMode = null;
    await this.cancel();
  }

  // Dismissed from the confirm modal — close modal and resume the job.
  async dismissCancelConfirm() {
    this.cancelModalMode = null;
    await this.resume();
  }

  reset() {
    this.status = 'idle';
    this._jobId = null;
    this.cancelSummary = null;
    this.cancelModalMode = null;
  }

  dismissCancelSummary() {
    this.cancelModalMode = null;
    this.cancelSummary = null;
  }
}

export const job = new JobStore();

export async function initJobStore() {
  if (_initialized) return;
  _initialized = true;

  registerJobActiveCheck(() => job.isActive);

  await listen<{ job_id: string; status: JobStatus }>('job_status', ({ payload }) => {
    const prev = job.status;
    job.status = payload.status;
    if (prev === 'running' && payload.status === 'paused') {
      showToast('info', 'Job paused');
    }
  });

  // Fired when job completes normally — emit a single summary toast (Visual Spec §7)
  await listen<JobSummary>('job_done', ({ payload }) => {
    job.status = 'idle';
    const msg = payload.failed > 0
      ? `Processed ${payload.processed}/${payload.total} receipts (${payload.failed} failed).`
      : `Processed ${payload.processed} receipt${payload.processed !== 1 ? 's' : ''}.`;
    showToast(payload.failed > 0 ? 'warn' : 'success', msg);
  });

  // Fired when job is cancelled — open cancel summary modal instead of toast (Visual Spec §7)
  await listen<JobSummary>('job_cancelled', ({ payload }) => {
    job.status = 'cancelled';
    job.cancelSummary = payload;
    job.cancelModalMode = 'summary';
  });
}
