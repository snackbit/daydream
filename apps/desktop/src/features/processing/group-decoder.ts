import type {
  ProcessingApplicability,
  RecordingProcessingActionResult,
  RecordingProcessingGroup,
  RecordingProcessingGroupPage,
  RecordingProcessingLane,
  RecordingProcessingLaneDetail,
  RecordingProcessingLaneKind,
  RecordingProcessingUnit,
  ProcessingLaneActivity,
  ProcessingWaitingReason,
} from '@daydream/shared';
import {
  array,
  boolean,
  count,
  cursor,
  invalid,
  nullable,
  progress,
  record,
  status,
  string,
  unit,
} from './task-decoder';

function laneKind(value: unknown): RecordingProcessingLaneKind {
  return value === 'audio' ||
    value === 'ocr' ||
    value === 'vlm' ||
    value === 'activity' ||
    value === 'waveform'
    ? value
    : invalid();
}
function applicability(value: unknown): ProcessingApplicability {
  return value === 'available' ||
    value === 'disabled' ||
    value === 'no-source' ||
    value === 'protected' ||
    value === 'no-eligible-input' ||
    value === 'waiting'
    ? value
    : invalid();
}
function lane(value: unknown): RecordingProcessingLane {
  const r = record(value);
  const result = {
    kind: laneKind(r.kind),
    applicability: applicability(r.applicability),
    status: nullable(r.status, status),
    jobIds: array(r.jobIds, string),
    total: count(r.total),
    completed: count(r.completed),
    failed: count(r.failed),
    skipped: count(r.skipped),
    cancelled: count(r.cancelled),
    running: count(r.running),
    queued: count(r.queued),
    progress: nullable(r.progress, progress),
    inputOpen: boolean(r.inputOpen),
    errorCode: nullable(r.errorCode, string),
    nextRetryAtMs: nullable(r.nextRetryAtMs, count),
    ...(r.activity === undefined
      ? {}
      : { activity: array(r.activity, activity) }),
  };
  if (
    result.completed +
      result.failed +
      result.skipped +
      result.cancelled +
      result.running +
      result.queued !==
    result.total
  )
    return invalid();
  return result;
}
function activity(value: unknown): ProcessingLaneActivity {
  const r = record(value);
  if (r.status !== 'running' && r.status !== 'queued') return invalid();
  const waitingReasons: readonly ProcessingWaitingReason[] = [
    'ocr',
    'previous-source',
    'retry',
    'live-input',
    'protection',
    'capacity',
    'paused',
    'unknown',
  ];
  const waitingReason = nullable(
    r.waitingReason,
    (value): ProcessingWaitingReason => {
      const reason = waitingReasons.find((reason) => reason === value);
      return reason ?? invalid();
    },
  );
  const startOffsetMs = nullable(r.startOffsetMs, count),
    endOffsetMs = nullable(r.endOffsetMs, count);
  if (
    startOffsetMs !== null &&
    endOffsetMs !== null &&
    endOffsetMs <= startOffsetMs
  )
    return invalid();
  return {
    chunkId: string(r.chunkId),
    jobId: string(r.jobId),
    sourceTrackId: nullable(r.sourceTrackId, string),
    sourceLabel: nullable(r.sourceLabel, string),
    startOffsetMs,
    endOffsetMs,
    status: r.status,
    phase: string(r.phase),
    startedAtMs: nullable(r.startedAtMs, count),
    phaseStartedAtMs: nullable(r.phaseStartedAtMs, count),
    lastProgressAtMs: nullable(r.lastProgressAtMs, count),
    nextRetryAtMs: nullable(r.nextRetryAtMs, count),
    waitingReason,
    workerActive: nullable(r.workerActive, boolean),
  };
}
function group(value: unknown): RecordingProcessingGroup {
  const r = record(value);
  const index = record(r.indexCoverage);
  const lanes = array(r.lanes, lane);
  const kinds = new Set(lanes.map((lane) => lane.kind));
  // Waveform preparation is additive; older responses retain the four AI lanes.
  if (
    kinds.size !== lanes.length ||
    !(['audio', 'ocr', 'vlm', 'activity'] as const).every((kind) =>
      kinds.has(kind),
    )
  )
    return invalid();
  const indexCoverage = {
    applicability: applicability(index.applicability),
    indexed: count(index.indexed),
    total: count(index.total),
    pending: count(index.pending),
  };
  if (indexCoverage.indexed + indexCoverage.pending !== indexCoverage.total)
    return invalid();
  const result = {
    recordingId: string(r.recordingId),
    title: string(r.title),
    startedAtMs: nullable(r.startedAtMs, count),
    updatedAtMs: count(r.updatedAtMs),
    status: status(r.status),
    inputOpen: boolean(r.inputOpen),
    etaMs: nullable(r.etaMs, count),
    jobIds: array(r.jobIds, string),
    lanes,
    indexCoverage,
  };
  if (result.inputOpen && result.etaMs !== null) return invalid();
  return result;
}
export function decodeRecordingGroupPage(
  value: unknown,
): RecordingProcessingGroupPage {
  const r = record(value);
  const counts = record(r.counts);
  return {
    groups: array(r.groups, group),
    nextCursor: nullable(r.nextCursor, cursor),
    counts: {
      all: count(counts.all),
      active: count(counts.active),
      attention: count(counts.attention),
      history: count(counts.history),
      ...(counts.running === undefined
        ? {}
        : { running: count(counts.running) }),
      ...(counts.queued === undefined ? {} : { queued: count(counts.queued) }),
    },
    reconciledGroups: array(r.reconciledGroups, group),
  };
}
function unitCursor(value: unknown) {
  const r = record(value);
  return { chunkId: string(r.chunkId), unitId: string(r.unitId) };
}
function recordingUnit(value: unknown): RecordingProcessingUnit {
  const r = record(value);
  const sourceRole = r.sourceRole;
  if (
    sourceRole !== null &&
    sourceRole !== 'desktop-audio' &&
    sourceRole !== 'microphone'
  )
    return invalid();
  return {
    ...unit(r),
    jobId: string(r.jobId),
    sourceTrackId: nullable(r.sourceTrackId, string),
    sourceRole,
  };
}
export function decodeRecordingLaneDetail(
  value: unknown,
): RecordingProcessingLaneDetail {
  const r = record(value);
  return {
    lane: lane(r.lane),
    units: array(r.units, recordingUnit),
    nextCursor: nullable(r.nextCursor, unitCursor),
  };
}
export function decodeRecordingActionResult(
  value: unknown,
): RecordingProcessingActionResult {
  const r = record(value);
  return {
    results: array(r.results, (value) => {
      const result = record(value);
      return {
        jobId: string(result.jobId),
        succeeded: boolean(result.succeeded),
        errorCode: nullable(result.errorCode, string),
      };
    }),
  };
}
