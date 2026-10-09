import type {
  BackgroundTask,
  BackgroundTaskDetail,
  BackgroundTaskPage,
  BackgroundTaskStage,
  BackgroundTaskUnit,
  BackgroundTasksChanged,
  ProcessingChunkStatus,
  ProcessingJobStatus,
} from '@daydream/shared';

const statuses: readonly ProcessingJobStatus[] = [
  'cancelled',
  'completed-with-errors',
  'failed',
  'paused',
  'queued',
  'running',
  'succeeded',
];
export function invalid(): never {
  throw new Error(
    'Background processing returned an invalid response. Reload to reconnect.',
  );
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
export function record(value: unknown): Record<string, unknown> {
  if (!isRecord(value)) return invalid();
  return value;
}
export function string(value: unknown): string {
  return typeof value === 'string' ? value : invalid();
}
function number(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value)
    ? value
    : invalid();
}
export function count(value: unknown): number {
  const n = number(value);
  return Number.isSafeInteger(n) && n >= 0 ? n : invalid();
}
export function progress(value: unknown): number {
  const n = number(value);
  return n >= 0 && n <= 1 ? n : invalid();
}
export function boolean(value: unknown): boolean {
  return typeof value === 'boolean' ? value : invalid();
}
export function nullable<T>(
  value: unknown,
  decode: (value: unknown) => T,
): T | null {
  return value === null ? null : decode(value);
}
export function array<T>(value: unknown, decode: (value: unknown) => T): T[] {
  return Array.isArray(value) ? value.map(decode) : invalid();
}
export function status(value: unknown): ProcessingJobStatus {
  for (const status of statuses) if (status === value) return status;
  return invalid();
}
function unitStatus(value: unknown): ProcessingChunkStatus {
  const result = status(value);
  return result === 'paused' ? invalid() : result;
}
function stage(value: unknown): BackgroundTaskStage {
  const r = record(value);
  return {
    kind: string(r.kind),
    total: count(r.total),
    completed: count(r.completed),
    failed: count(r.failed),
    skipped: count(r.skipped),
    running: count(r.running),
    queued: count(r.queued),
    progress: progress(r.progress),
  };
}
function task(value: unknown): BackgroundTask {
  const r = record(value);
  return {
    id: string(r.id),
    kind: string(r.kind),
    title: string(r.title),
    recordingId: nullable(r.recordingId, string),
    modelId: nullable(r.modelId, string),
    ...(r.dayId === undefined ? {} : { dayId: nullable(r.dayId, string) }),
    status: status(r.status),
    phase: string(r.phase),
    progress: nullable(r.progress, progress),
    elapsedMs: count(r.elapsedMs),
    etaMs: nullable(r.etaMs, count),
    nextRetryAtMs: nullable(r.nextRetryAtMs, count),
    inputOpen: boolean(r.inputOpen),
    errorCode: nullable(r.errorCode, string),
    updatedAtMs: count(r.updatedAtMs),
    stages: array(r.stages, stage),
  };
}
export function unit(value: unknown): BackgroundTaskUnit {
  const r = record(value);
  return {
    id: string(r.id),
    chunkId: string(r.chunkId),
    sourceKind: string(r.sourceKind),
    startOffsetMs: nullable(r.startOffsetMs, count),
    endOffsetMs: nullable(r.endOffsetMs, count),
    status: unitStatus(r.status),
    progress: progress(r.progress),
    errorCode: nullable(r.errorCode, string),
    attemptCount: count(r.attemptCount),
  };
}
export function cursor(value: unknown) {
  const r = record(value);
  return { id: string(r.id), updatedAtMs: count(r.updatedAtMs) };
}
export function decodeTaskPage(value: unknown): BackgroundTaskPage {
  const r = record(value);
  return {
    tasks: array(r.tasks, task),
    nextCursor: nullable(r.nextCursor, cursor),
  };
}
export function decodeTaskDetail(value: unknown): BackgroundTaskDetail {
  const r = record(value);
  return { task: task(r.task), units: array(r.units, unit) };
}
export function decodeTaskChange(value: unknown): BackgroundTasksChanged {
  const r = record(value);
  return {
    recordingId: nullable(r.recordingId, string),
    updatedAtMs: count(r.updatedAtMs),
  };
}
