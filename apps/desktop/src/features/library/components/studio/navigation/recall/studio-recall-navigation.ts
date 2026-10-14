import type { LibraryRecordingDetail, RecallEvidence } from '@daydream/shared';
import { timelineDurationMs } from '../../../../library-timeline';

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
      : audioTrackId(evidence, segment) === evidence.sourceTrackId) &&
    evidence.startOffsetMs >= segment.playbackOffsetMs &&
    evidenceSpanAvailable(evidence, detail, segment) &&
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

function evidenceSpanAvailable(
  evidence: RecallEvidence,
  detail: LibraryRecordingDetail,
  anchor: LibraryRecordingDetail['segments'][number],
): boolean {
  if (evidence.sourceKind === 'ocr')
    return (
      anchor.durationMs !== null &&
      evidence.endOffsetMs <= anchor.playbackOffsetMs + anchor.durationMs
    );
  let cursor = evidence.startOffsetMs;
  let segment = anchor;
  while (cursor < evidence.endOffsetMs) {
    const duration = segment.durationMs;
    const trackId = audioTrackId(evidence, segment);
    if (
      !segment.fileExists ||
      segment.state !== 'finalized' ||
      duration === null ||
      trackId === null ||
      cursor < segment.playbackOffsetMs ||
      cursor >= segment.playbackOffsetMs + duration
    )
      return false;
    const end = Math.min(
      evidence.endOffsetMs,
      segment.playbackOffsetMs + duration,
    );
    if (
      detail.redactions.some(
        (range) =>
          range.startOffsetMs < end &&
          range.endOffsetMs > cursor &&
          (!range.trackIds.length || range.trackIds.includes(trackId)),
      )
    )
      return false;
    cursor = end;
    if (cursor === evidence.endOffsetMs) return true;
    const next = detail.segments.filter(
      (candidate) => candidate.playbackOffsetMs === cursor,
    );
    const candidate = next[0];
    if (
      next.length !== 1 ||
      !candidate ||
      segment.endedAtMs !== candidate.startedAtMs
    )
      return false;
    segment = candidate;
  }
  return false;
}

function audioTrackId(
  evidence: RecallEvidence,
  segment: LibraryRecordingDetail['segments'][number],
): string | null {
  const stream =
    evidence.sourceKind === 'microphone'
      ? segment.microphoneAudioStreamIndex
      : segment.desktopAudioStreamIndex;
  return stream === null ? null : `${evidence.sourceKind}-${stream}`;
}

/** A citation's anchor track can differ from later fragments of the same source. */
export function studioRecallEvidenceTrackIds(
  evidence: RecallEvidence,
  detail: LibraryRecordingDetail,
): string[] {
  if (evidence.sourceKind === 'ocr') return [evidence.sourceTrackId];
  const ids = new Set([evidence.sourceTrackId]);
  for (const segment of detail.segments) {
    if (
      segment.durationMs === null ||
      segment.playbackOffsetMs >= evidence.endOffsetMs ||
      segment.playbackOffsetMs + segment.durationMs <= evidence.startOffsetMs
    )
      continue;
    const id = audioTrackId(evidence, segment);
    if (id !== null) ids.add(id);
  }
  return [...ids];
}
