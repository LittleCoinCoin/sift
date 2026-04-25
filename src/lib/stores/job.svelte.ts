import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { showToast, registerJobActiveCheck } from './log';

export type JobStatus = 'Idle' | 'Running' | 'Paused' | 'Resuming' | 'Cancelling' | 'Cancelled';

export interface JobSummary {
  processed: number;
  failed: number;
  total: number;
}

const ACTIVE_STATUSES: ReadonlySet<JobStatus> = new Set([
  'Running', 'Paused', 'Resuming', 'Cancelling',
]);

let _initialized = false;

class JobStore {
  status = $state<JobStatus>('Idle');
  cancelSummary = $state<JobSummary | null>(null);
  cancelSummaryOpen = $state(false);

  get isActive(): boolean {
    return ACTIVE_STATUSES.has(this.status);
  }

  async start(paths: string[]) {
    await invoke('start_job', { paths });
    this.status = 'Running';
  }

  async pause() {
    await invoke('pause_job');
    this.status = 'Paused';
  }

  async resume() {
    await invoke('resume_job');
    this.status = 'Resuming';
  }

  async cancel() {
    await invoke('cancel_job');
    this.status = 'Cancelling';
  }

  reset() {
    this.status = 'Idle';
    this.cancelSummary = null;
    this.cancelSummaryOpen = false;
  }

  dismissCancelSummary() {
    this.cancelSummaryOpen = false;
    this.cancelSummary = null;
  }
}

export const job = new JobStore();

export async function initJobStore() {
  if (_initialized) return;
  _initialized = true;

  registerJobActiveCheck(() => job.isActive);

  await listen<{ status: JobStatus }>('job_status', ({ payload }) => {
    job.status = payload.status;
  });

  // Fired when job completes normally — emit a single summary toast (Visual Spec §7)
  await listen<JobSummary>('job_done', ({ payload }) => {
    job.status = 'Idle';
    const msg = payload.failed > 0
      ? `Processed ${payload.processed}/${payload.total} receipts (${payload.failed} failed).`
      : `Processed ${payload.processed} receipt${payload.processed !== 1 ? 's' : ''}.`;
    showToast(payload.failed > 0 ? 'warn' : 'success', msg);
  });

  // Fired when job is cancelled — open cancel summary modal instead of toast (Visual Spec §7)
  await listen<JobSummary>('job_cancelled', ({ payload }) => {
    job.status = 'Cancelled';
    job.cancelSummary = payload;
    job.cancelSummaryOpen = true;
  });
}
