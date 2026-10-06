import type {
  ProcessingJobStatus,
  ProcessingChunkStatus,
} from './processing.js';

export interface BackgroundTaskStage {
  kind: string;
  total: number;
  completed: number;
  failed: number;
  skipped: number;
  running: number;
  queued: number;
  progress: number;
}

export interface BackgroundTask {
  id: string;
  kind: string;
  title: string;
  recordingId: string | null;
  modelId: string | null;
  status: ProcessingJobStatus;
  phase: string;
  progress: number | null;
  elapsedMs: number;
  etaMs: number | null;
  nextRetryAtMs: number | null;
  inputOpen: boolean;
  errorCode: string | null;
  updatedAtMs: number;
  stages: BackgroundTaskStage[];
}

export interface BackgroundTaskUnit {
  id: string;
  chunkId: string;
  sourceKind: string;
  startOffsetMs: number | null;
  endOffsetMs: number | null;
  status: ProcessingChunkStatus;
  progress: number;
  errorCode: string | null;
  attemptCount: number;
}

export interface BackgroundTaskCursor {
  updatedAtMs: number;
  id: string;
}
export interface ListBackgroundTasksInput {
  recordingId?: string | null;
  limit?: number;
  cursor?: BackgroundTaskCursor | null;
}
export interface BackgroundTaskPage {
  tasks: BackgroundTask[];
  nextCursor: BackgroundTaskCursor | null;
}
export interface GetBackgroundTaskInput {
  taskId: string;
}
export interface BackgroundTaskDetail {
  task: BackgroundTask;
  units: BackgroundTaskUnit[];
}
export interface BackgroundTasksChanged {
  recordingId: string | null;
  updatedAtMs: number;
}
export interface RetryProcessingUnitsInput {
  jobId: string;
  chunkId?: string | null;
  unitId?: string | null;
  useCurrentSettings?: boolean;
}
