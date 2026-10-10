import type { ProcessingJobStatus } from './processing.js';

export type ReprocessingOperation =
  'transcription' | 'ocr' | 'screen-observations';
export interface ReprocessingRange {
  startOffsetMs: number;
  endOffsetMs: number;
}
export interface ReprocessingTarget {
  trackId: string;
  ranges: ReprocessingRange[] | null;
}
export interface PreviewRecordingReprocessingInput {
  recordingId: string;
  targets: ReprocessingTarget[];
  operations?: ReprocessingOperation[];
}
export interface StartRecordingReprocessingInput extends PreviewRecordingReprocessingInput {
  planFingerprint: string;
}
export interface ReprocessingOperationInfo {
  operation: ReprocessingOperation;
  enabled: boolean;
  remote: boolean;
  providerLabel: string;
  blockedReason: string | null;
}
export interface RecordingReprocessingScope extends ReprocessingRange {
  id: string;
  trackId: string;
  sourceLabel: string;
  operation: ReprocessingOperation;
}
export interface RecordingReprocessingPreview {
  recordingId: string;
  planFingerprint: string;
  scopes: RecordingReprocessingScope[];
  operations: ReprocessingOperationInfo[];
  blockedReason: string | null;
}
export interface RecordingReprocessingProgress extends RecordingReprocessingScope {
  jobId: string;
  status: ProcessingJobStatus;
}
