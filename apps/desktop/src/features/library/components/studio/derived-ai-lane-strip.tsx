import type {
  LibraryRecordingSegment,
  LibraryRecordingTrack,
  ProcessingChunk,
  RecordingAiProcessingState,
  RecordingTranscriptSegment,
  RecordingTranscriptionChunkState,
  RecordingTranscriptionTrackState,
} from '@daydream/shared';
import { cn } from '@/lib/utils';
import { TRACK_DERIVED_LANE_HEIGHT_PX } from './track-timeline-math';

export interface DerivedAiLane {
  chunks: RecordingTranscriptionChunkState[];
  description: string;
  label: string;
  status: string;
  statusLabel: string;
  trackState: RecordingTranscriptionTrackState | null;
}

type SegmentTranscriptionDisplayItem =
  | {
      chunk: RecordingTranscriptionChunkState;
      endOffsetMs: number | null;
      id: string;
      kind: 'chunk';
      startOffsetMs: number | null;
      text: string | null;
    }
  | {
      chunk: RecordingTranscriptionChunkState;
      endOffsetMs: number;
      id: string;
      kind: 'transcript-segment';
      startOffsetMs: number;
      text: string;
    };

export function buildDerivedLane({
  aiProcessingState,
  track,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  track: LibraryRecordingTrack;
}): DerivedAiLane {
  if (track.kind === 'video') {
    return {
      chunks: [],
      description: 'OCR and VLM processing are not implemented yet.',
      label: 'OCR + VLM',
      status: 'not-implemented',
      statusLabel: 'OCR and VLM pending',
      trackState: null,
    };
  }

  const trackState =
    aiProcessingState?.transcriptionTracks?.find(
      (candidate) =>
        candidate.libraryTrackId === track.id ||
        candidate.sourceRole === track.kind,
    ) ?? null;
  if (!trackState && aiProcessingState) {
    const chunks = legacyTranscriptionChunks(aiProcessingState, track);

    return {
      chunks,
      description: `${track.label} transcription`,
      label: 'Transcription',
      status: legacyStatus(chunks),
      statusLabel: legacyStatusLabel(chunks),
      trackState: null,
    };
  }

  return {
    chunks: trackState?.chunks ?? [],
    description: trackState?.description ?? `${track.label} transcription`,
    label: 'Transcription',
    status: trackState?.status ?? 'ready',
    statusLabel: trackState?.statusLabel ?? 'Ready to process',
    trackState,
  };
}

function legacyTranscriptionChunks(
  aiProcessingState: RecordingAiProcessingState,
  track: LibraryRecordingTrack,
): RecordingTranscriptionChunkState[] {
  const transcriptSegments =
    aiProcessingState.transcript.tracks.find(
      (transcriptTrack) =>
        transcriptTrack.trackId === track.id ||
        transcriptTrack.sourceRole === track.kind,
    )?.segments ?? [];

  return aiProcessingState.chunks
    .filter(
      (chunk) =>
        chunk.sourceKind === 'audio' &&
        (chunk.trackId === track.id || chunk.trackId === track.kind),
    )
    .map((chunk) => legacyTranscriptionChunk(chunk, transcriptSegments));
}

function legacyTranscriptionChunk(
  chunk: ProcessingChunk,
  transcriptSegments: RecordingTranscriptSegment[],
): RecordingTranscriptionChunkState {
  const startMs = chunk.startOffsetMs ?? 0;
  const endMs = chunk.endOffsetMs ?? startMs;
  const chunkTranscriptSegments = transcriptSegments.filter(
    (segment) => segment.startOffsetMs < endMs && segment.endOffsetMs > startMs,
  );
  const text = chunkTranscriptSegments
    .map((segment) => segment.text.trim())
    .filter(Boolean)
    .join(' ');
  const result =
    chunk.status === 'succeeded'
      ? chunk.resultCode === 'skipped-silence'
        ? 'skipped-silence'
        : text
          ? 'text'
          : 'no-speech'
      : chunk.status;

  return {
    attemptCount: chunk.attemptCount,
    endOffsetMs: chunk.endOffsetMs,
    errorCode: chunk.errorCode,
    id: chunk.id,
    progress: chunk.progress,
    result,
    resultCode: chunk.resultCode,
    segmentId: chunk.segmentId,
    startOffsetMs: chunk.startOffsetMs,
    status: chunk.status,
    text: text || null,
    transcriptSegments: chunkTranscriptSegments,
  };
}

function legacyStatus(chunks: RecordingTranscriptionChunkState[]): string {
  if (chunks.length === 0) {
    return 'ready';
  }
  if (
    chunks.some(
      (chunk) => chunk.result === 'queued' || chunk.result === 'running',
    )
  ) {
    return 'in-progress';
  }
  if (chunks.some((chunk) => chunk.result === 'failed')) {
    return 'failed';
  }
  if (chunks.some((chunk) => chunk.result === 'cancelled')) {
    return 'cancelled';
  }
  return chunks.some((chunk) => chunk.result === 'text')
    ? 'complete'
    : 'no-speech';
}

function legacyStatusLabel(chunks: RecordingTranscriptionChunkState[]): string {
  switch (legacyStatus(chunks)) {
    case 'in-progress':
      return 'Transcription in progress';
    case 'failed':
      return 'Transcription failed';
    case 'cancelled':
      return 'Transcription cancelled';
    case 'complete':
      return 'Transcription complete';
    case 'no-speech':
      return 'No speech detected';
    default:
      return 'Ready to process';
  }
}

export function SegmentTranscriptionLayer({
  color,
  lane,
  segment,
}: {
  color: string;
  lane: DerivedAiLane;
  segment: LibraryRecordingSegment;
}) {
  const segmentStartMs = segment.playbackOffsetMs;
  const segmentDurationMs = Math.max(1, segment.durationMs ?? 0);
  const segmentEndMs = segmentStartMs + segmentDurationMs;
  const chunks = lane.chunks.filter(
    (chunk) =>
      (chunk.startOffsetMs ?? 0) < segmentEndMs &&
      (chunk.endOffsetMs ?? chunk.startOffsetMs ?? 0) > segmentStartMs,
  );
  const displayItems = chunks.flatMap((chunk) =>
    transcriptionDisplayItems(chunk, segmentStartMs, segmentEndMs),
  );

  if (displayItems.length === 0) {
    return null;
  }

  return (
    <div
      aria-label={lane.statusLabel}
      className="bg-background/80 absolute inset-x-0 bottom-0 overflow-hidden border-t"
      style={{ height: TRACK_DERIVED_LANE_HEIGHT_PX }}
      title={lane.statusLabel}
    >
      {displayItems.map((item) => (
        <SegmentTranscriptionItem
          color={color}
          item={item}
          key={item.id}
          segmentDurationMs={segmentDurationMs}
          segmentStartMs={segmentStartMs}
        />
      ))}
    </div>
  );
}

function transcriptionDisplayItems(
  chunk: RecordingTranscriptionChunkState,
  segmentStartMs: number,
  segmentEndMs: number,
): SegmentTranscriptionDisplayItem[] {
  const transcriptSegments = chunk.transcriptSegments
    .filter(
      (transcriptSegment) =>
        transcriptSegment.startOffsetMs < segmentEndMs &&
        transcriptSegment.endOffsetMs > segmentStartMs,
    )
    .map((transcriptSegment) => ({
      chunk,
      endOffsetMs: transcriptSegment.endOffsetMs,
      id: `${chunk.id}:${transcriptSegment.id}`,
      kind: 'transcript-segment' as const,
      startOffsetMs: transcriptSegment.startOffsetMs,
      text: transcriptSegment.text,
    }));

  if (transcriptSegments.length > 0) {
    return transcriptSegments;
  }

  return [
    {
      chunk,
      endOffsetMs: chunk.endOffsetMs,
      id: chunk.id,
      kind: 'chunk',
      startOffsetMs: chunk.startOffsetMs,
      text: chunk.text,
    },
  ];
}

function SegmentTranscriptionItem({
  color,
  item,
  segmentDurationMs,
  segmentStartMs,
}: {
  color: string;
  item: SegmentTranscriptionDisplayItem;
  segmentDurationMs: number;
  segmentStartMs: number;
}) {
  const chunk = item.chunk;
  const startMs = Math.max(
    segmentStartMs,
    item.startOffsetMs ?? segmentStartMs,
  );
  const endMs = Math.max(
    startMs,
    Math.min(segmentStartMs + segmentDurationMs, item.endOffsetMs ?? startMs),
  );
  const left = ((startMs - segmentStartMs) / segmentDurationMs) * 100;
  const width = Math.max(1, ((endMs - startMs) / segmentDurationMs) * 100);
  const textItem =
    item.kind === 'transcript-segment' || chunk.result === 'text';

  return (
    <div
      aria-label={itemLabel(item)}
      className={cn(
        'absolute top-1 bottom-1 overflow-hidden rounded-sm border text-[0.625rem] font-medium opacity-50',
        (chunk.result === 'queued' || chunk.result === 'running') &&
          'ai-processing-construction border-amber-500/70',
        (chunk.result === 'failed' || chunk.result === 'cancelled') &&
          'border-destructive bg-destructive text-destructive-foreground',
        chunk.result === 'no-speech' &&
          'border-muted-foreground/50 bg-muted text-muted-foreground',
        chunk.result === 'skipped-silence' &&
          'border-sky-700/50 bg-sky-700/15 text-sky-950 dark:text-sky-100',
        textItem && 'border-transparent text-white',
      )}
      style={{
        backgroundColor: textItem ? color : undefined,
        left: `${left}%`,
        width: `${width}%`,
      }}
      title={itemTitle(item)}
    >
      {textItem ? (
        <span className="block truncate px-1">{item.text}</span>
      ) : null}
    </div>
  );
}

function itemLabel(item: SegmentTranscriptionDisplayItem): string {
  if (item.kind === 'transcript-segment') {
    return 'Transcription segment processed with text';
  }

  return chunkLabel(item.chunk);
}

function itemTitle(item: SegmentTranscriptionDisplayItem): string {
  if (item.kind === 'transcript-segment') {
    return item.text;
  }

  return chunkTitle(item.chunk);
}

function chunkLabel(chunk: RecordingTranscriptionChunkState): string {
  switch (chunk.result) {
    case 'queued':
    case 'running':
      return 'AI processing in progress';
    case 'failed':
      return 'AI processing failed';
    case 'cancelled':
      return 'Transcription chunk cancelled';
    case 'no-speech':
      return 'Transcription chunk processed with no speech';
    case 'skipped-silence':
      return 'Transcription chunk skipped because audio stayed below the silence threshold';
    case 'text':
      return 'Transcription chunk processed with text';
    case 'unknown':
      return 'Transcription chunk state unknown';
  }
}

function chunkTitle(chunk: RecordingTranscriptionChunkState): string {
  if (chunk.text) {
    return chunk.text;
  }
  if (chunk.errorCode) {
    return chunk.errorCode;
  }
  if (chunk.resultCode === 'skipped-silence') {
    return 'Skipped: below silence threshold.';
  }
  return chunkLabel(chunk);
}
