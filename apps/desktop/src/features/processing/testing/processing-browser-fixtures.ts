import type {
  BackgroundTask,
  RecordingProcessingGroup,
  RecordingProcessingGroupPage,
  RecordingProcessingLane,
  RecordingProcessingLaneKind,
  ProcessingLaneActivity,
} from '@daydream/shared';

export function activity(
  patch: Partial<ProcessingLaneActivity> = {},
): ProcessingLaneActivity {
  return {
    chunkId: 'chunk',
    jobId: 'job',
    sourceTrackId: 'screen',
    sourceLabel: 'Monitor',
    startOffsetMs: 0,
    endOffsetMs: 1000,
    status: 'running',
    phase: 'recognizing-text',
    startedAtMs: 1000,
    phaseStartedAtMs: 1100,
    lastProgressAtMs: null,
    nextRetryAtMs: null,
    waitingReason: null,
    workerActive: true,
    ...patch,
  };
}

export function lane(
  kind: RecordingProcessingLaneKind,
  patch: Partial<RecordingProcessingLane> = {},
): RecordingProcessingLane {
  return {
    kind,
    applicability: 'available',
    status: 'running',
    jobIds: ['synthetic-job'],
    total: 10,
    completed: 4,
    failed: 0,
    skipped: 0,
    cancelled: 0,
    running: 1,
    queued: 5,
    progress: 0.45,
    inputOpen: false,
    errorCode: null,
    nextRetryAtMs: null,
    ...patch,
  };
}
export function group(
  patch: Partial<RecordingProcessingGroup> = {},
): RecordingProcessingGroup {
  return {
    recordingId: 'synthetic-recording',
    title: 'Synthetic recording',
    startedAtMs: 1000,
    updatedAtMs: 3000,
    status: 'running',
    inputOpen: false,
    etaMs: 5000,
    jobIds: ['synthetic-job'],
    lanes: [
      lane('audio'),
      lane('ocr'),
      lane('vlm', {
        total: 0,
        completed: 0,
        running: 0,
        queued: 0,
        progress: null,
        jobIds: [],
        status: null,
        applicability: 'disabled',
      }),
      lane('activity'),
      lane('waveform'),
    ],
    indexCoverage: {
      applicability: 'available',
      indexed: 4,
      total: 10,
      pending: 6,
    },
    ...patch,
  };
}
export function groupPage(
  groups: RecordingProcessingGroup[] = [group()],
  patch: Partial<RecordingProcessingGroupPage> = {},
): RecordingProcessingGroupPage {
  return {
    groups,
    nextCursor: null,
    reconciledGroups: [],
    counts: {
      all: groups.length,
      active: groups.filter((group) =>
        ['running', 'queued', 'paused'].includes(group.status),
      ).length,
      attention: groups.filter((group) =>
        group.lanes.some((lane) => lane.failed > 0),
      ).length,
      history: groups.filter(
        (group) => !['running', 'queued', 'paused'].includes(group.status),
      ).length,
    },
    ...patch,
  };
}
export function task(patch: Partial<BackgroundTask> = {}): BackgroundTask {
  return {
    id: 'shared-recall',
    kind: 'recall-index',
    title: 'Recall evidence indexing',
    recordingId: null,
    modelId: null,
    status: 'running',
    phase: 'running',
    progress: 0.5,
    elapsedMs: 1000,
    etaMs: null,
    nextRetryAtMs: null,
    inputOpen: false,
    errorCode: null,
    updatedAtMs: 3000,
    stages: [],
    ...patch,
  };
}
