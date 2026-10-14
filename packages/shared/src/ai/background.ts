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
  dayId?: string | null;
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
  scope?: 'shared';
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
  sourceKind?: string;
  recordingId: string | null;
  updatedAtMs: number;
}
export interface RetryProcessingUnitsInput {
  jobId: string;
  sourceKind?: RecordingProcessingLaneKind;
  chunkId?: string | null;
  unitId?: string | null;
  useCurrentSettings?: boolean;
}

export type RecordingProcessingLaneKind =
  'audio' | 'ocr' | 'vlm' | 'activity' | 'waveform';
export type RecordingProcessingFilter =
  'all' | 'active' | 'attention' | 'history';
export type ProcessingApplicability =
  | 'available'
  | 'disabled'
  | 'no-source'
  | 'protected'
  | 'no-eligible-input'
  | 'waiting';
export type ProcessingWaitingReason =
  | 'ocr'
  | 'previous-source'
  | 'retry'
  | 'live-input'
  | 'protection'
  | 'capacity'
  | 'paused'
  | 'unknown';
export interface ProcessingLaneActivity {
  chunkId: string;
  jobId: string;
  sourceTrackId: string | null;
  sourceLabel: string | null;
  startOffsetMs: number | null;
  endOffsetMs: number | null;
  status: 'running' | 'queued';
  phase: string;
  startedAtMs: number | null;
  phaseStartedAtMs: number | null;
  lastProgressAtMs: number | null;
  nextRetryAtMs: number | null;
  waitingReason: ProcessingWaitingReason | null;
  workerActive: boolean | null;
  totalUnits?: number | null;
  completedUnits?: number | null;
}
export interface RecordingProcessingLane {
  kind: RecordingProcessingLaneKind;
  applicability: ProcessingApplicability;
  status: ProcessingJobStatus | null;
  jobIds: string[];
  total: number;
  completed: number;
  failed: number;
  skipped: number;
  cancelled: number;
  running: number;
  queued: number;
  progress: number | null;
  inputOpen: boolean;
  errorCode: string | null;
  nextRetryAtMs: number | null;
  /** Bounded current/next work, absent on older native versions. */
  activity?: ProcessingLaneActivity[];
  etaMs?: number | null;
}
export interface RecordingIndexCoverage {
  applicability: ProcessingApplicability;
  indexed: number;
  total: number;
  pending: number;
  status?:
    | 'ready'
    | 'queued'
    | 'running'
    | 'paused'
    | 'failed'
    | 'cancelled'
    | 'blocked'
    | 'succeeded'
    | null;
  errorCode?: string | null;
}
export interface RecordingProcessingGroup {
  recordingId: string;
  title: string;
  startedAtMs: number | null;
  updatedAtMs: number;
  status: ProcessingJobStatus;
  inputOpen: boolean;
  etaMs: number | null;
  jobIds: string[];
  lanes: RecordingProcessingLane[];
  indexCoverage: RecordingIndexCoverage;
}
export interface ListRecordingProcessingGroupsInput {
  recordingId?: string | null;
  recordingIds?: string[];
  filter?: RecordingProcessingFilter;
  limit?: number;
  cursor?: BackgroundTaskCursor | null;
  /** Reconcile loaded cards after deletions, invalidation and lost events. */
  knownRecordingIds?: string[];
}
export interface RecordingProcessingGroupPage {
  groups: RecordingProcessingGroup[];
  nextCursor: BackgroundTaskCursor | null;
  counts: {
    all: number;
    active: number;
    attention: number;
    history: number;
    running?: number;
    queued?: number;
  };
  reconciledGroups: RecordingProcessingGroup[];
}
export interface RecordingProcessingUnit extends BackgroundTaskUnit {
  jobId: string;
  sourceTrackId: string | null;
  sourceRole: 'desktop-audio' | 'microphone' | null;
}
export interface GetRecordingProcessingLaneInput {
  recordingId: string;
  sourceKind: RecordingProcessingLaneKind;
  limit?: number;
  cursor?: { chunkId: string; unitId: string } | null;
  history?: boolean;
}
export interface RecordingProcessingLaneDetail {
  lane: RecordingProcessingLane;
  units: RecordingProcessingUnit[];
  nextCursor: { chunkId: string; unitId: string } | null;
}
export interface ControlRecordingProcessingGroupsInput {
  recordingId: string;
  action: 'pause' | 'resume' | 'cancel' | 'retry' | 'optimize-visual';
  sourceKind?: RecordingProcessingLaneKind;
  useCurrentSettings?: boolean;
}
export interface RecordingProcessingActionResult {
  results: { jobId: string; succeeded: boolean; errorCode: string | null }[];
}
