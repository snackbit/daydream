import type { LibraryRecordingDetail, RecallEvidence } from '@daydream/shared';
import { timelineDurationMs } from '../../library-timeline';

export function studioRecallEvidenceAvailable(
  evidence: RecallEvidence,
  detail: LibraryRecordingDetail,
): boolean {
  const segment = detail.segments.find(
    (candidate) => candidate.id === evidence.segmentId,
  );
  return (
    evidence.recordingId === detail.id &&
    detail.availability === 'available' &&
    evidence.sourceId.length > 0 &&
    (evidence.sourceKind !== 'ocr' ||
      (evidence.documentId !== null &&
        evidence.documentId.length > 0 &&
        evidence.sourceFrameRef !== null &&
        evidence.sourceFrameRef.length > 0)) &&
    segment !== undefined &&
    segment.fileExists &&
    segment.durationMs !== null &&
    (evidence.sourceKind === 'ocr'
      ? segment.videoStreams && segment.videoStreams.length > 0
        ? segment.videoStreams.some(
            (stream) => stream.trackId === evidence.sourceTrackId,
          )
        : segment.videoStreamIndex !== null &&
          detail.tracks.find((track) => track.kind === 'video')?.id ===
            evidence.sourceTrackId
      : evidence.sourceKind === 'desktop-audio'
        ? segment.desktopAudioStreamIndex !== null &&
          evidence.sourceTrackId ===
            `desktop-audio-${segment.desktopAudioStreamIndex}`
        : segment.microphoneAudioStreamIndex !== null &&
          evidence.sourceTrackId ===
            `microphone-${segment.microphoneAudioStreamIndex}`) &&
    evidence.startOffsetMs >= segment.playbackOffsetMs &&
    evidence.endOffsetMs <= segment.playbackOffsetMs + segment.durationMs &&
    detail.tracks.some(
      (track) =>
        track.id === evidence.sourceTrackId &&
        track.kind ===
          (evidence.sourceKind === 'ocr' ? 'video' : evidence.sourceKind),
    ) &&
    evidence.startOffsetMs >= 0 &&
    evidence.endOffsetMs > evidence.startOffsetMs &&
    evidence.endOffsetMs <= timelineDurationMs(detail.segments) &&
    !detail.redactions.some(
      (range) =>
        (range.trackIds.length === 0 ||
          range.trackIds.includes(evidence.sourceTrackId)) &&
        range.startOffsetMs < evidence.endOffsetMs &&
        range.endOffsetMs > evidence.startOffsetMs,
    )
  );
}
