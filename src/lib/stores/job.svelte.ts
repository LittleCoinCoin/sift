import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { get } from 'svelte/store';
import { showToast, registerJobActiveCheck, progress } from './log';

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
  file_started_at?: number;
  job_started_at?: number;
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

export type JobState =
  | { case: 'idle' }
  | { case: 'running';          jobId: string; files: FileEntry[] }
  | { case: 'paused';           jobId: string; files: FileEntry[] }
  | { case: 'resuming';         jobId: string; files: FileEntry[] }
  | { case: 'cancelConfirming'; jobId: string; files: FileEntry[]; summary: JobSummary; cameFrom: 'running' | 'paused' | 'resuming' }
  | { case: 'cancelling';       jobId: string }
  | { case: 'done';             lastJobId: string; outcome: JobSummary | null }
  | { case: 'cancelled';        lastJobId: string; outcome: JobSummary }
  | { case: 'failed';           lastJobId: string; outcome: JobSummary | null }

let _initialized = false;

class JobStore {
  state = $state<JobState>({ case: 'idle' });

  get isActive(): boolean {
    const c = this.state.case;
    return c === 'running' || c === 'paused' || c === 'resuming' || c === 'cancelling' || c === 'cancelConfirming';
  }

  // Maps union case → legacy JobStatus string for backward compat.
  get status(): JobStatus {
    switch (this.state.case) {
      case 'running':          return 'running';
      case 'paused':           return 'paused';
      case 'resuming':         return 'resuming';
      case 'cancelling':       return 'cancelling';
      case 'cancelConfirming': return 'paused';
      case 'cancelled':        return 'cancelled';
      case 'failed':           return 'failed';
      default:                 return 'idle';
    }
  }

  get cancelOpen(): boolean {
    return this.state.case === 'cancelConfirming';
  }

  get cancelSummary(): JobSummary | null {
    return this.state.case === 'cancelConfirming' ? this.state.summary : null;
  }

  start = async (config: JobConfig) => {
    const jobId = await invoke<string>('start_job', { config });
    this.state = { case: 'running', jobId, files: config.files };
  };

  pause = async () => {
    const s = this.state;
    if (s.case !== 'running' && s.case !== 'resuming') return;
    await invoke('pause_job', { jobId: s.jobId });
  };

  resume = async () => {
    const s = this.state;
    if (s.case !== 'paused') return;
    await invoke('resume_job', { jobId: s.jobId });
  };

  cancel = async () => {
    const s = this.state;
    if (!('jobId' in s)) return;
    await invoke('cancel_job', { jobId: s.jobId });
  };

  // Builds a predicted cancellation summary, transitions to cancelConfirming, pauses if needed.
  requestCancel = async () => {
    const s = this.state;
    if (s.case !== 'running' && s.case !== 'paused' && s.case !== 'resuming') return;

    const cameFrom = s.case;
    const p = get(progress);
    const done = p?.done ?? 0;
    const total = p?.total ?? s.files.length;
    const currentFile = p?.current_file;

    const processedPaths = new Set(s.files.slice(0, done).map(f => f.path));
    const files: FileOutcome[] = s.files.map(f => {
      if (processedPaths.has(f.path)) return { path: f.path, status: 'processed', in_flight: false };
      if (f.path === currentFile)     return { path: f.path, status: 'abandoned', in_flight: true };
      return { path: f.path, status: 'abandoned', in_flight: false };
    });

    const abandoned = files.filter(f => f.status === 'abandoned').length;
    const summary: JobSummary = {
      job_id: s.jobId,
      status: 'cancelled',
      total,
      processed: done,
      abandoned,
      failed: 0,
      files,
    };

    // Transition synchronously so the modal opens immediately (matching original cancelOpen=true behavior).
    this.state = { case: 'cancelConfirming', jobId: s.jobId, files: s.files, summary, cameFrom };

    if (cameFrom === 'running' || cameFrom === 'resuming') {
      try { await invoke('pause_job', { jobId: s.jobId }); } catch {}
    }
  };

  // Confirmed from the cancel modal — close modal atomically then send cancel to backend.
  confirmCancel = async () => {
    const s = this.state;
    if (s.case !== 'cancelConfirming') return;
    const { jobId } = s;
    this.state = { case: 'cancelling', jobId };
    try { await invoke('cancel_job', { jobId }); } catch {}
  };

  // Dismissed from the cancel modal — close modal and resume the job.
  dismissCancelConfirm = async () => {
    const s = this.state;
    if (s.case !== 'cancelConfirming') return;
    const { jobId, files } = s;
    this.state = { case: 'resuming', jobId, files };
    try { await invoke('resume_job', { jobId }); } catch {}
  };

  reset = () => {
    this.state = { case: 'idle' };
  };
}

export const job = new JobStore();

export async function initJobStore() {
  if (_initialized) return;
  _initialized = true;

  registerJobActiveCheck(() => job.isActive);

  await listen<{ job_id: string; status: JobStatus }>('job_status', ({ payload }) => {
    const s = job.state;
    const files = 'files' in s ? s.files : [];
    const jobId = payload.job_id;

    if (s.case === 'running' && payload.status === 'paused') {
      showToast('info', 'Job paused');
    }

    switch (payload.status) {
      case 'running':
        job.state = { case: 'running', jobId, files };
        break;
      case 'paused':
        // Skip if cancelConfirming: this paused event is the auto-pause from requestCancel().
        if (s.case !== 'cancelConfirming') {
          job.state = { case: 'paused', jobId, files };
        }
        break;
      case 'resuming':
        if (s.case !== 'cancelConfirming') {
          job.state = { case: 'resuming', jobId, files };
        }
        break;
      case 'cancelling':
        job.state = { case: 'cancelling', jobId };
        break;
    }
  });

  // Fired when job completes normally — emit a single summary toast (Visual Spec §7)
  await listen<JobSummary>('job_done', ({ payload }) => {
    const msg = payload.failed > 0
      ? `Processed ${payload.processed}/${payload.total} receipts (${payload.failed} failed).`
      : `Processed ${payload.processed} receipt${payload.processed !== 1 ? 's' : ''}.`;
    job.state = { case: 'done', lastJobId: payload.job_id, outcome: payload };
    showToast(payload.failed > 0 ? 'warn' : 'success', msg);
  });

  // Fired when job is cancelled — update state; modal was already shown predictively on requestCancel.
  await listen<JobSummary>('job_cancelled', ({ payload }) => {
    job.state = { case: 'cancelled', lastJobId: payload.job_id, outcome: payload };
  });
}
