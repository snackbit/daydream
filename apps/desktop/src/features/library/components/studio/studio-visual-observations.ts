import type {
  LibraryRecordingDetail,
  OcrRegion,
  RecordingVisualObservations,
  VlmObservation,
} from '@daydream/shared';

export interface StudioVisualFrame {
  endTimestampMs: number;
  frameTimestampMs: number;
  id: string;
  ocrRegions: OcrRegion[];
  sourceFrameRef: string;
  sourceFrameRefs: string[];
  vlmObservations: VlmObservation[];
}

export function visualFrameId({
  frameTimestampMs,
}: {
  frameTimestampMs: number;
  sourceFrameRef: string;
}): string {
  return `frame:${frameTimestampMs}`;
}

export function buildStudioVisualFrames(
  visualObservations: RecordingVisualObservations | null,
  detail: LibraryRecordingDetail,
): StudioVisualFrame[] {
  if (!visualObservations) {
    return [];
  }

  const frames = new Map<string, StudioVisualFrame>();
  for (const region of visualObservations.ocrRegions) {
    const frame = frameForObservation(frames, region);
    frame.ocrRegions.push(region);
  }
  for (const observation of visualObservations.vlmObservations) {
    const frame = frameForObservation(frames, observation);
    frame.vlmObservations.push(observation);
  }

  return [...frames.values()]
    .map((frame) => ({
      ...frame,
      endTimestampMs: frameEndTimestampMs({
        detail,
        frameTimestampMs: frame.frameTimestampMs,
        nextFrameTimestampMs: nextFrameTimestampMs(frames, frame),
      }),
      ocrRegions: [...frame.ocrRegions].sort(compareOcrRegions),
      sourceFrameRefs: [...frame.sourceFrameRefs].sort(),
      vlmObservations: [...frame.vlmObservations].sort(compareVlmObservations),
    }))
    .sort(
      (left, right) =>
        left.frameTimestampMs - right.frameTimestampMs ||
        left.sourceFrameRef.localeCompare(right.sourceFrameRef),
    );
}

function frameForObservation(
  frames: Map<string, StudioVisualFrame>,
  observation: Pick<
    OcrRegion | VlmObservation,
    'frameTimestampMs' | 'sourceFrameRef'
  >,
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
    ocrRegions: [],
    sourceFrameRef: observation.sourceFrameRef,
    sourceFrameRefs: [observation.sourceFrameRef],
    vlmObservations: [],
  } satisfies StudioVisualFrame;
  frames.set(id, frame);
  return frame;
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

function nextFrameTimestampMs(
  frames: Map<string, StudioVisualFrame>,
  frame: StudioVisualFrame,
): number | null {
  const nextTimestampMs = [...frames.values()]
    .map((candidate) => candidate.frameTimestampMs)
    .filter((timestampMs) => timestampMs > frame.frameTimestampMs)
    .sort((left, right) => left - right)[0];

  return nextTimestampMs ?? null;
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
  const recordingDurationMs =
    detail.durationMs ??
    Math.max(
      frameTimestampMs + 1,
      ...detail.segments.map(
        (segment) => segment.playbackOffsetMs + (segment.durationMs ?? 0),
      ),
    );
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
