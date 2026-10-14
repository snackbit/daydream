import type {
  LibraryRecordingTrack,
  ProcessingChunk,
  RecordingAiProcessingState,
  RecordingTranscriptSegment,
  RecordingTranscriptionChunkState,
  RecordingTranscriptionTrackState,
} from '@daydream/shared';

export interface DerivedAiLane {
  chunks: RecordingTranscriptionChunkState[];
  description: string;
  label: string;
  status: string;
  statusLabel: string;
  trackState: RecordingTranscriptionTrackState | null;
}

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

  const trackStates =
    aiProcessingState?.transcriptionTracks?.filter(
      (candidate) =>
        candidate.libraryTrackId === track.id ||
        candidate.sourceRole === track.kind,
    ) ?? [];
  const firstTrackState = trackStates[0] ?? null;
  // A day source lane can contain several recordings. Every projected chunk
  // retains its source identity and clock; the first recording cannot own the
  // entire lane or suppress later recordings' published speech.
  const chunks = trackStates.flatMap((state) => state.chunks);
  const trackState =
    firstTrackState && trackStates.length > 1
      ? {
          ...firstTrackState,
          chunks,
          textSegmentCount: trackStates.reduce(
            (count, state) => count + state.textSegmentCount,
            0,
          ),
          status: legacyStatus(chunks),
          statusLabel: legacyStatusLabel(chunks),
        }
      : firstTrackState;
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
  const transcriptSegments = aiProcessingState.transcript.tracks
    .filter(
      (transcriptTrack) =>
        transcriptTrack.trackId === track.id ||
        transcriptTrack.sourceRole === track.kind,
    )
    .flatMap((transcriptTrack) => transcriptTrack.segments);

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

function legacyStatus(
  chunks: RecordingTranscriptionChunkState[],
): RecordingTranscriptionTrackState['status'] {
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
