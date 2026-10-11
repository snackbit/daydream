import type {
  PreviewRecordingReprocessingInput,
  RecordingReprocessingPreview,
  ReprocessingOperation,
  StartRecordingReprocessingInput,
} from '@daydream/shared';
import { processingJob } from '@/features/ai/ocr/ocr-response-decoder';
import {
  array,
  boolean,
  count,
  invalid,
  nullable,
  record,
  string,
} from '@/features/processing/task-decoder';
import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';

export function reprocessingOperation(value: unknown): ReprocessingOperation {
  if (
    value === 'transcription' ||
    value === 'ocr' ||
    value === 'screen-observations'
  )
    return value;
  return invalid();
}
export function decodeReprocessingPreview(
  value: unknown,
): RecordingReprocessingPreview {
  const row = record(value);
  return {
    recordingId: string(row.recordingId),
    planFingerprint: string(row.planFingerprint),
    blockedReason: nullable(row.blockedReason, string),
    operations: array(row.operations, (value) => {
      const item = record(value);
      return {
        operation: reprocessingOperation(item.operation),
        enabled: boolean(item.enabled),
        remote: boolean(item.remote),
        providerLabel: string(item.providerLabel),
        blockedReason: nullable(item.blockedReason, string),
      };
    }),
    scopes: array(row.scopes, (value) => {
      const item = record(value),
        startOffsetMs = count(item.startOffsetMs),
        endOffsetMs = count(item.endOffsetMs);
      if (endOffsetMs <= startOffsetMs) return invalid();
      return {
        id: string(item.id),
        trackId: string(item.trackId),
        sourceLabel: string(item.sourceLabel),
        operation: reprocessingOperation(item.operation),
        startOffsetMs,
        endOffsetMs,
      };
    }),
  };
}
export async function previewRecordingReprocessing(
  input: PreviewRecordingReprocessingInput,
) {
  return decodeResponse(
    'previewRecordingReprocessing',
    await invoke<unknown>('previewRecordingReprocessing', { input }),
    decodeReprocessingPreview,
  );
}
export async function startRecordingReprocessing(
  input: StartRecordingReprocessingInput,
) {
  return decodeResponse(
    'startRecordingReprocessing',
    await invoke<unknown>('startRecordingReprocessing', { input }),
    processingJob,
  );
}
