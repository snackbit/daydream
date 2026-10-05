import type { RecordingAiProcessingState } from '@daydream/shared';

export function ocrFrameProcessingRevision(
  state: RecordingAiProcessingState | null,
  timestampMs: number,
  segmentId: string | null | undefined,
): string {
  if (!state) return '';
  const job = state.job
    ? `${state.job.id}:${state.job.status}:${state.job.errorCode ?? ''}`
    : '';
  const frames = state.visualFrames?.filter(
    (frame) =>
      frame.sourceKind === 'ocr' &&
      frame.frameTimestampMs === timestampMs &&
      (!segmentId || frame.segmentId === segmentId),
  );
  const revision =
    frames && frames.length > 0
      ? frames
          .map((frame) => `${frame.id}:${frame.status}:${frame.updatedAtMs}`)
          .join('|')
      : state.chunks
          .filter(
            (chunk) =>
              chunk.sourceKind === 'ocr' &&
              (chunk.startOffsetMs === null ||
                chunk.startOffsetMs <= timestampMs) &&
              (chunk.endOffsetMs === null || chunk.endOffsetMs > timestampMs),
          )
          .map((chunk) => `${chunk.id}:${chunk.status}:${chunk.updatedAtMs}`)
          .join('|');
  return `${job}|${revision}`;
}
