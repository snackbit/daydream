import type {
  LibraryRecordingDetail,
  LibraryRecordingTrack,
  ProcessingChunk,
  ProcessingChunkStatus,
  ProcessingJobStatus,
  RecordingAiProcessingState,
  RecordingReprocessingScope,
  VisualFrameProcessingState,
} from '@daydream/shared';

export type TimelineProcessingStatus =
  'planned' | 'queued' | 'running' | 'paused' | 'failed' | 'cancelled';
export interface TimelineProcessingRange {
  id: string;
  operation: 'transcription' | 'ocr' | 'screen-observations';
  status: TimelineProcessingStatus;
  startOffsetMs: number;
  endOffsetMs: number;
  progress: number;
}
export interface TimelineProcessingInput {
  detail: LibraryRecordingDetail;
  track: LibraryRecordingTrack;
  state: RecordingAiProcessingState | null;
  plannedScopes?: readonly TimelinePlannedScope[];
}
export interface TimelinePlannedScope extends RecordingReprocessingScope {
  status?: 'planned' | 'queued';
}
export function timelineProcessingRanges({
  detail,
  track,
  state,
  plannedScopes = [],
}: TimelineProcessingInput): TimelineProcessingRange[] {
  if (state && state.recordingId !== detail.id) return [];
  const ranges: TimelineProcessingRange[] = [];
  const chunks = (state?.chunks ?? []).filter((chunk) =>
    matchesTrack(chunk, detail, track, state),
  );
  // Old IPC payloads can expose audio work through the transcript read model only.
  if (track.kind !== 'video' && !chunks.length) {
    const audio = state?.transcriptionTracks?.find(
      (source) =>
        source.libraryTrackId === track.id || source.sourceRole === track.kind,
    );
    for (const chunk of audio?.chunks ?? []) {
      addChunk(
        {
          ...chunk,
          sourceKind: 'audio',
          trackId: track.id,
          jobId: state?.job?.id ?? '',
          recordingId: detail.id,
          createdAtMs: 0,
          updatedAtMs: 0,
        },
        [],
        state?.job?.status,
        ranges,
      );
    }
  }
  const framesByChunk = new Map<string, VisualFrameProcessingState[]>();
  for (const frame of state?.visualFrames ?? []) {
    const frames = framesByChunk.get(frame.chunkId) ?? [];
    frames.push(frame);
    framesByChunk.set(frame.chunkId, frames);
  }
  for (const chunk of chunks)
    addChunk(
      chunk,
      framesByChunk.get(chunk.id) ?? [],
      state?.job?.status,
      ranges,
    );
  const latestJobId =
    state?.job?.id ?? state?.reprocessingScopes?.at(-1)?.jobId;
  for (const scope of state?.reprocessingScopes ?? []) {
    if (
      scope.trackId !== track.id ||
      scope.jobId !== latestJobId ||
      scope.status === 'succeeded'
    )
      continue;
    const status = processingStatus(scope.status, state?.job?.status);
    if (!status) continue;
    const covered = chunks.flatMap((chunk) =>
      operation(chunk) === scope.operation &&
      chunk.startOffsetMs !== null &&
      chunk.endOffsetMs !== null
        ? [
            {
              startOffsetMs: chunk.startOffsetMs,
              endOffsetMs: chunk.endOffsetMs,
            },
          ]
        : [],
    );
    for (const part of subtractRanges(scope, covered))
      ranges.push({
        ...part,
        id: `scope:${scope.id}`,
        operation: scope.operation,
        status,
        progress: 0,
      });
  }
  for (const scope of plannedScopes) {
    if (scope.trackId === track.id)
      ranges.push({
        ...scope,
        id: `planned:${scope.id}`,
        status: scope.status ?? 'planned',
        progress: 0,
      });
  }
  const segments = detail.segments.filter(
    (segment) =>
      segment.fileExists &&
      segment.state === 'finalized' &&
      segment.durationMs !== null &&
      segment.durationMs > 0 &&
      (track.kind === 'video'
        ? segment.videoStreams?.length
          ? segment.videoStreams.some((source) => source.trackId === track.id)
          : segment.videoStreamIndex !== null
        : track.kind === 'microphone'
          ? segment.microphoneAudioStreamIndex !== null
          : segment.desktopAudioStreamIndex !== null),
  );
  const hidden = detail.redactions.filter(
    (range) => !range.trackIds.length || range.trackIds.includes(track.id),
  );
  return ranges
    .flatMap((range) =>
      segments.flatMap((segment) => {
        const startOffsetMs = Math.max(
          segment.playbackOffsetMs,
          range.startOffsetMs,
        );
        const endOffsetMs = Math.min(
          segment.playbackOffsetMs + (segment.durationMs ?? 0),
          range.endOffsetMs,
        );
        if (endOffsetMs <= startOffsetMs) return [];
        return subtractRanges({ startOffsetMs, endOffsetMs }, hidden).map(
          (part) => ({
            ...range,
            ...part,
            id: `${range.id}:${segment.id}:${part.startOffsetMs}`,
          }),
        );
      }),
    )
    .sort(
      (a, b) =>
        a.startOffsetMs - b.startOffsetMs ||
        a.operation.localeCompare(b.operation) ||
        a.id.localeCompare(b.id),
    );
}

function matchesTrack(
  chunk: ProcessingChunk,
  detail: LibraryRecordingDetail,
  track: LibraryRecordingTrack,
  state: RecordingAiProcessingState | null,
): boolean {
  if (chunk.sourceKind === 'audio') {
    const source = state?.transcriptionTracks?.find(
      (source) =>
        source.libraryTrackId === track.id || source.sourceRole === track.kind,
    );
    return (
      track.kind !== 'video' &&
      (chunk.trackId === track.id ||
        chunk.trackId === track.kind ||
        (source?.trackId !== null &&
          source?.trackId !== undefined &&
          chunk.trackId === source.trackId))
    );
  }
  return (
    track.kind === 'video' &&
    (chunk.sourceKind === 'ocr' || chunk.sourceKind === 'vlm') &&
    (chunk.trackId === track.id ||
      (chunk.trackId === null &&
        detail.tracks.find((source) => source.kind === 'video')?.id ===
          track.id))
  );
}

function operation(
  chunk: ProcessingChunk,
): TimelineProcessingRange['operation'] {
  return chunk.sourceKind === 'audio'
    ? 'transcription'
    : chunk.sourceKind === 'vlm'
      ? 'screen-observations'
      : 'ocr';
}

function processingStatus(
  status: ProcessingChunkStatus | ProcessingJobStatus,
  parent?: ProcessingJobStatus,
): TimelineProcessingStatus | null {
  if (status === 'succeeded') return null;
  if (
    (status === 'queued' || status === 'running') &&
    (parent === 'paused' ||
      parent === 'failed' ||
      parent === 'cancelled' ||
      parent === 'completed-with-errors')
  )
    return parent === 'completed-with-errors' ? 'failed' : parent;
  return status === 'completed-with-errors' ? 'failed' : status;
}

function addChunk(
  chunk: ProcessingChunk,
  frames: VisualFrameProcessingState[],
  parent: ProcessingJobStatus | undefined,
  ranges: TimelineProcessingRange[],
) {
  const status = processingStatus(chunk.status, parent),
    start = chunk.startOffsetMs,
    end = chunk.endOffsetMs;
  if (!status || start === null || end === null || end <= start) return;
  const samples = frames
    .filter(
      (frame) =>
        frame.frameTimestampMs >= start && frame.frameTimestampMs < end,
    )
    .toSorted((a, b) => a.frameTimestampMs - b.frameTimestampMs);
  if (chunk.sourceKind === 'audio' || !samples.length) {
    ranges.push({
      id: chunk.id,
      operation: operation(chunk),
      status,
      startOffsetMs: start,
      endOffsetMs: end,
      progress: Math.max(0, Math.min(1, chunk.progress)),
    });
    return;
  }
  for (const [index, frame] of samples.entries()) {
    const stoppedOwner =
      chunk.status === 'failed' ||
      chunk.status === 'cancelled' ||
      chunk.status === 'completed-with-errors'
        ? chunk.status
        : parent;
    const frameStatus = processingStatus(frame.status, stoppedOwner);
    if (!frameStatus) continue;
    ranges.push({
      id: frame.id,
      operation: operation(chunk),
      status: frameStatus,
      startOffsetMs: index === 0 ? start : frame.frameTimestampMs,
      endOffsetMs: samples[index + 1]?.frameTimestampMs ?? end,
      progress: Math.max(0, Math.min(1, frame.progress)),
    });
  }
}

interface Range {
  startOffsetMs: number;
  endOffsetMs: number;
}
function subtractRanges(range: Range, cuts: readonly Range[]): Range[] {
  let remaining = [range];
  for (const cut of cuts)
    remaining = remaining.flatMap((part) => {
      if (
        cut.endOffsetMs <= part.startOffsetMs ||
        cut.startOffsetMs >= part.endOffsetMs
      )
        return [part];
      const split: Range[] = [];
      if (cut.startOffsetMs > part.startOffsetMs)
        split.push({
          startOffsetMs: part.startOffsetMs,
          endOffsetMs: cut.startOffsetMs,
        });
      if (cut.endOffsetMs < part.endOffsetMs)
        split.push({
          startOffsetMs: cut.endOffsetMs,
          endOffsetMs: part.endOffsetMs,
        });
      return split;
    });
  return remaining;
}
