import type {
  LibraryRecordingDetail,
  OcrRegion,
  RecordingVisualObservations,
  VlmObservation,
  ProcessingChunkStatus,
  OcrTimelineFrame,
  OcrOccurrenceStatus,
} from '@daydream/shared';
import { timelineDurationMs } from '../../library-timeline';

export interface StudioVisualFrame {
  /** Navigation-cell end, not the duration of observed OCR evidence. */
  endTimestampMs: number;
  frameTimestampMs: number;
  id: string;
  ocrRegions: OcrRegion[];
  sourceFrameRef: string;
  sourceFrameRefs: string[];
  vlmObservations: VlmObservation[];
  ocrStatus?: ProcessingChunkStatus | OcrOccurrenceStatus;
  ocrOutputCount?: number;
  segmentId?: string | null;
  sourceTrackId?: string;
  ocrDocumentId?: string;
  ocrTextEmpty?: boolean | null;
  ocrSampleEndMs?: number;
  ocrTextPreview?: string | null;
}

export function visualFrameId({
  frameTimestampMs,
  segmentId,
  sourceTrackId,
}: {
  frameTimestampMs: number;
  sourceFrameRef: string;
  segmentId?: string | null;
  sourceTrackId?: string;
}): string {
  return `frame:${sourceTrackId ?? 'unmapped'}:${segmentId ?? 'unmapped'}:${frameTimestampMs}`;
}

export function buildStudioVisualFrames(
  visualObservations: RecordingVisualObservations | null,
  detail: LibraryRecordingDetail,
  ocrFrames: OcrTimelineFrame[] = [],
): StudioVisualFrame[] {
  if (!visualObservations && ocrFrames.length === 0) {
    return [];
  }

  const frames = new Map<string, StudioVisualFrame>();
  const sourceTrackId = detail.tracks.find(
    (track) => track.kind === 'video',
  )?.id;
  for (const sample of ocrFrames) {
    if (
      sample.recordingId !== detail.id ||
      !visibleTimestamp(detail, sample.sourceTrackId, sample.frameTimestampMs)
    )
      continue;
    const frame = frameForObservation(frames, sample);
    frame.ocrStatus = sample.status;
    frame.ocrOutputCount = sample.ocrOutputCount;
    frame.ocrDocumentId = sample.documentId;
    frame.ocrTextEmpty = sample.textEmpty;
    frame.ocrSampleEndMs = sample.sampleEndMs;
    frame.ocrTextPreview = sample.textPreview ?? null;
  }
  for (const region of visualObservations?.ocrRegions ?? []) {
    const regionSource =
      region.sourceFrameRef.match(/:stream:([^:]+):/)?.[1] ?? sourceTrackId;
    if (
      visualObservations?.recordingId !== detail.id ||
      !visibleTimestamp(detail, regionSource, region.frameTimestampMs)
    )
      continue;
    const frame = frameForObservation(frames, {
      ...region,
      segmentId: segmentIdForTimestamp(detail, region.frameTimestampMs),
      ...(regionSource ? { sourceTrackId: regionSource } : {}),
    });
    frame.ocrRegions.push(region);
  }
  for (const observation of visualObservations?.vlmObservations ?? []) {
    const observationSource =
      observation.sourceFrameRef.match(/:stream:([^:]+):/)?.[1] ??
      sourceTrackId;
    if (
      visualObservations?.recordingId !== detail.id ||
      !visibleTimestamp(detail, observationSource, observation.frameTimestampMs)
    )
      continue;
    const frame = frameForObservation(frames, {
      ...observation,
      segmentId: segmentIdForTimestamp(detail, observation.frameTimestampMs),
      ...(observationSource ? { sourceTrackId: observationSource } : {}),
    });
    frame.vlmObservations.push(observation);
  }

  const orderedFrames = [...frames.values()].sort(
    (left, right) =>
      left.frameTimestampMs - right.frameTimestampMs ||
      left.sourceFrameRef.localeCompare(right.sourceFrameRef),
  );
  const nextBySource = new Map<string, number>();
  return [...orderedFrames]
    .reverse()
    .map((frame) => {
      const sourceKey = JSON.stringify([
        frame.sourceTrackId ?? null,
        frame.segmentId ?? null,
      ]);
      const endTimestampMs = frameEndTimestampMs({
        detail,
        frameTimestampMs: frame.frameTimestampMs,
        nextFrameTimestampMs: nextBySource.get(sourceKey) ?? null,
      });
      // Navigation cells fill until the next observation in this source/segment.
      // The actual evidence bound stays separately in ocrSampleEndMs for privacy.
      nextBySource.set(sourceKey, frame.frameTimestampMs);
      return {
        ...frame,
        endTimestampMs,
        ocrRegions: [...frame.ocrRegions].sort(compareOcrRegions),
        sourceFrameRefs: [...frame.sourceFrameRefs].sort(),
        vlmObservations: [...frame.vlmObservations].sort(
          compareVlmObservations,
        ),
      };
    })
    .reverse();
}

function visibleTimestamp(
  detail: LibraryRecordingDetail,
  source: string | undefined,
  timestamp: number,
): boolean {
  return !detail.redactions.some(
    (redaction) =>
      timestamp >= redaction.startOffsetMs &&
      timestamp < redaction.endOffsetMs &&
      (redaction.trackIds.length === 0 ||
        (source !== undefined && redaction.trackIds.includes(source))),
  );
}

function frameForObservation(
  frames: Map<string, StudioVisualFrame>,
  observation: Pick<
    OcrRegion | VlmObservation,
    'frameTimestampMs' | 'sourceFrameRef'
  > & { segmentId?: string | null; sourceTrackId?: string },
): StudioVisualFrame {
  const id = visualFrameId(observation);
  const existing = frames.get(id);
  if (existing) {
    if (!existing.sourceFrameRefs.includes(observation.sourceFrameRef)) {
      existing.sourceFrameRefs.push(observation.sourceFrameRef);
    }

    return existing;
  }

  const frame = {
    endTimestampMs: observation.frameTimestampMs,
    frameTimestampMs: observation.frameTimestampMs,
    id,
    segmentId: observation.segmentId ?? null,
    ...(observation.sourceTrackId
      ? { sourceTrackId: observation.sourceTrackId }
      : {}),
    ocrRegions: [],
    sourceFrameRef: observation.sourceFrameRef,
    sourceFrameRefs: [observation.sourceFrameRef],
    vlmObservations: [],
  } satisfies StudioVisualFrame;
  frames.set(id, frame);
  return frame;
}

function segmentIdForTimestamp(
  detail: LibraryRecordingDetail,
  timestampMs: number,
): string | null {
  return (
    detail.segments.find(
      (segment) =>
        timestampMs >= segment.playbackOffsetMs &&
        timestampMs < segment.playbackOffsetMs + (segment.durationMs ?? 0),
    )?.id ?? null
  );
}

function compareOcrRegions(left: OcrRegion, right: OcrRegion): number {
  return (
    left.region.y - right.region.y ||
    left.region.x - right.region.x ||
    (right.confidence ?? -1) - (left.confidence ?? -1) ||
    left.id.localeCompare(right.id)
  );
}

function compareVlmObservations(
  left: VlmObservation,
  right: VlmObservation,
): number {
  return (
    left.frameTimestampMs - right.frameTimestampMs ||
    left.observationKind.localeCompare(right.observationKind) ||
    left.id.localeCompare(right.id)
  );
}

function frameEndTimestampMs({
  detail,
  frameTimestampMs,
  nextFrameTimestampMs,
}: {
  detail: LibraryRecordingDetail;
  frameTimestampMs: number;
  nextFrameTimestampMs: number | null;
}): number {
  const recordingDurationMs = timelineDurationMs(detail.segments);
  const containingSegment = detail.segments.find((segment) => {
    const segmentStartMs = segment.playbackOffsetMs;
    const segmentEndMs = segmentStartMs + (segment.durationMs ?? 0);

    return (
      frameTimestampMs >= segmentStartMs && frameTimestampMs < segmentEndMs
    );
  });
  const segmentEndTimestampMs = containingSegment
    ? containingSegment.playbackOffsetMs + (containingSegment.durationMs ?? 0)
    : recordingDurationMs;
  const endTimestampMs = Math.min(
    nextFrameTimestampMs ?? segmentEndTimestampMs,
    segmentEndTimestampMs,
    recordingDurationMs,
  );

  return Math.max(frameTimestampMs + 1, endTimestampMs);
}
