import type {
  LibraryRecordingDetail,
  OcrTimelineFrame,
} from '@daydream/shared';

export function visibleOcrTimelineFrames(
  detail: LibraryRecordingDetail,
  frames: OcrTimelineFrame[],
): OcrTimelineFrame[] {
  return frames.filter(
    (frame) =>
      frame.recordingId === detail.id &&
      !detail.redactions.some(
        (redaction) =>
          redaction.trackIds.includes(frame.sourceTrackId) &&
          frame.frameTimestampMs < redaction.endOffsetMs &&
          frame.sampleEndMs > redaction.startOffsetMs,
      ),
  );
}
