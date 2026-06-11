import type {
  LibraryRecordingTrack,
  ProcessingChunk,
  RecordingAiProcessingState,
  RecordingTranscriptSegment,
} from '@daydream/shared';
import { TRACK_DERIVED_LANE_HEIGHT_PX } from './track-timeline-math';

export interface DerivedAiLane {
  chunks: ProcessingChunk[];
  description: string;
  label: string;
  transcriptSegments: RecordingTranscriptSegment[];
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
      transcriptSegments: [],
    };
  }

  const chunks =
    aiProcessingState?.chunks.filter(
      (chunk) => chunk.sourceKind === 'audio' && chunk.trackId === track.id,
    ) ?? [];
  const transcriptSegments =
    aiProcessingState?.transcript.tracks.find(
      (transcriptTrack) => transcriptTrack.trackId === track.id,
    )?.segments ?? [];

  return {
    chunks,
    description: audioTranscriptionDescription(chunks, transcriptSegments),
    label: 'Transcription',
    transcriptSegments,
  };
}

function audioTranscriptionDescription(
  chunks: ProcessingChunk[],
  transcriptSegments: RecordingTranscriptSegment[],
) {
  if (chunks.length === 0) {
    return 'Transcription pending';
  }

  if (
    chunks.some(
      (chunk) => chunk.status === 'queued' || chunk.status === 'running',
    )
  ) {
    return 'Transcription in progress';
  }

  if (chunks.some((chunk) => chunk.status === 'failed')) {
    return 'Transcription failed';
  }

  if (chunks.some((chunk) => chunk.status === 'cancelled')) {
    return 'Transcription cancelled';
  }

  return transcriptSegments.length > 0
    ? 'Transcription complete'
    : 'No speech detected';
}

export function DerivedAiLaneStrip({
  color,
  lane,
  timelineWidthPx,
  totalDurationMs,
}: {
  color: string;
  lane: DerivedAiLane;
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  return (
    <div
      aria-label={lane.label}
      className="bg-muted/70 absolute inset-x-0 bottom-0 overflow-hidden border-t"
      style={{ height: TRACK_DERIVED_LANE_HEIGHT_PX }}
      title={lane.description}
    >
      {lane.chunks.map((chunk) => (
        <DerivedAiChunk
          chunk={chunk}
          color={color}
          key={chunk.id}
          timelineWidthPx={timelineWidthPx}
          totalDurationMs={totalDurationMs}
          transcriptSegments={lane.transcriptSegments}
        />
      ))}
    </div>
  );
}

function DerivedAiChunk({
  chunk,
  color,
  timelineWidthPx,
  totalDurationMs,
  transcriptSegments,
}: {
  chunk: ProcessingChunk;
  color: string;
  timelineWidthPx: number;
  totalDurationMs: number;
  transcriptSegments: RecordingTranscriptSegment[];
}) {
  const startMs = chunk.startOffsetMs ?? 0;
  const endMs = Math.max(startMs, chunk.endOffsetMs ?? startMs);
  const left = (startMs / totalDurationMs) * timelineWidthPx;
  const width = Math.max(
    2,
    ((endMs - startMs) / totalDurationMs) * timelineWidthPx,
  );

  if (chunk.status === 'queued' || chunk.status === 'running') {
    return (
      <div
        aria-label="AI processing in progress"
        className="ai-processing-construction absolute inset-y-1 rounded-sm border border-amber-500/70"
        style={{ left, width }}
        title="Processing"
      />
    );
  }

  if (chunk.status === 'failed' || chunk.status === 'cancelled') {
    return (
      <div
        aria-label="AI processing failed"
        className="bg-destructive absolute inset-y-1 rounded-sm"
        style={{ left, width }}
        title={chunk.errorCode ?? 'Processing failed'}
      />
    );
  }

  if (chunk.status !== 'succeeded') {
    return null;
  }

  const chunkTranscriptSegments = transcriptSegments.filter(
    (segment) => segment.startOffsetMs < endMs && segment.endOffsetMs > startMs,
  );

  return (
    <div
      aria-label={
        chunkTranscriptSegments.length > 0
          ? 'Transcript processed with text'
          : 'Transcript processed with no speech'
      }
      className="absolute inset-y-1 overflow-hidden rounded-sm"
      style={{ left, width }}
      title={
        chunkTranscriptSegments.length > 0
          ? chunkTranscriptSegments.map((segment) => segment.text).join(' ')
          : 'Processed; no transcription text'
      }
    >
      <div
        className="absolute inset-0"
        style={{ backgroundColor: color, opacity: 0.5 }}
      />
      {chunkTranscriptSegments.map((segment) => (
        <TranscriptBlock
          chunkEndMs={endMs}
          chunkStartMs={startMs}
          color={color}
          key={segment.id}
          segment={segment}
        />
      ))}
    </div>
  );
}

function TranscriptBlock({
  chunkEndMs,
  chunkStartMs,
  color,
  segment,
}: {
  chunkEndMs: number;
  chunkStartMs: number;
  color: string;
  segment: RecordingTranscriptSegment;
}) {
  const segmentStartMs = Math.max(chunkStartMs, segment.startOffsetMs);
  const segmentEndMs = Math.min(chunkEndMs, segment.endOffsetMs);
  if (segmentEndMs <= segmentStartMs) {
    return null;
  }

  const chunkDurationMs = chunkEndMs - chunkStartMs || 1;

  return (
    <div
      className="absolute inset-y-0 flex items-center overflow-hidden px-1 text-[0.625rem] font-medium text-white"
      style={{
        backgroundColor: color,
        left: `${((segmentStartMs - chunkStartMs) / chunkDurationMs) * 100}%`,
        width: `${((segmentEndMs - segmentStartMs) / chunkDurationMs) * 100}%`,
      }}
      title={segment.text}
    >
      <span className="truncate">{segment.text}</span>
    </div>
  );
}
