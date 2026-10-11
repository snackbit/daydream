import type {
  LibraryRecordingSegment,
  LibraryRecordingTrack,
  RecordingTranscriptSegment,
  RecordingTranscriptionChunkState,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { formatDuration } from '../../library-format';
import { aiProcessingErrorDetails } from './ai-processing-error-copy';
import { type DerivedAiLane } from './derived-ai-lane-model';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import { TRACK_TRANSCRIPTION_LANE_HEIGHT_PX } from './track-timeline-math';

export { buildDerivedLane } from './derived-ai-lane-model';
export type { DerivedAiLane } from './derived-ai-lane-model';
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
      transcriptSegment: RecordingTranscriptSegment;
    };

export function SegmentTranscriptionLayer({
  color,
  lane,
  segment,
  track,
  onInspectTranscript,
  hiddenRanges = [],
  publishedRanges = [],
  revealOffsetMs = null,
}: {
  color: string;
  lane: DerivedAiLane;
  segment: LibraryRecordingSegment;
  track: LibraryRecordingTrack;
  onInspectTranscript: (selection: StudioInspectorSelection) => void;
  hiddenRanges?: readonly { startOffsetMs: number; endOffsetMs: number }[];
  publishedRanges?: readonly { startOffsetMs: number; endOffsetMs: number }[];
  revealOffsetMs?: number | null;
}) {
  const segmentStartMs = segment.playbackOffsetMs;
  const segmentDurationMs = Math.max(1, segment.durationMs ?? 0);
  const segmentEndMs = segmentStartMs + segmentDurationMs;
  const chunks = lane.chunks.filter(
    (chunk) =>
      (chunk.startOffsetMs ?? 0) < segmentEndMs &&
      (chunk.endOffsetMs ?? chunk.startOffsetMs ?? 0) > segmentStartMs,
  );
  // Current scopes precede retained history. Paint historical status behind
  // current transcript actions when their processing windows overlap.
  const seen = new Set<string>();
  const displayItems = chunks
    .toReversed()
    .flatMap((chunk) =>
      transcriptionDisplayItems(chunk, segmentStartMs, segmentEndMs),
    )
    .filter((item) => {
      const id =
        item.kind === 'transcript-segment'
          ? item.transcriptSegment.id
          : item.id;
      if (
        seen.has(id) ||
        (hiddenRanges.some(
          (range) =>
            (item.startOffsetMs ?? 0) < range.endOffsetMs &&
            (item.endOffsetMs ?? 0) > range.startOffsetMs,
        ) &&
          !publishedRanges.some(
            (range) =>
              item.kind === 'transcript-segment' &&
              (item.startOffsetMs ?? 0) < range.endOffsetMs &&
              (item.endOffsetMs ?? 0) > range.startOffsetMs,
          ))
      )
        return false;
      seen.add(id);
      return true;
    });

  if (displayItems.length === 0) {
    return null;
  }

  return (
    <div
      aria-label={lane.statusLabel}
      className="bg-background/80 absolute inset-x-0 bottom-0 overflow-hidden border-t"
      style={{ height: TRACK_TRANSCRIPTION_LANE_HEIGHT_PX }}
      title={lane.statusLabel}
    >
      {displayItems.map((item) => (
        <SegmentTranscriptionItem
          color={color}
          item={item}
          key={item.id}
          segmentDurationMs={segmentDurationMs}
          segmentStartMs={segmentStartMs}
          track={track}
          onInspectTranscript={onInspectTranscript}
          covered={
            revealOffsetMs !== null && (item.endOffsetMs ?? 0) > revealOffsetMs
          }
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
      transcriptSegment,
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
  track,
  onInspectTranscript,
  covered,
}: {
  color: string;
  item: SegmentTranscriptionDisplayItem;
  segmentDurationMs: number;
  segmentStartMs: number;
  track: LibraryRecordingTrack;
  onInspectTranscript: (selection: StudioInspectorSelection) => void;
  covered: boolean;
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
  const style = {
    backgroundColor: textItem
      ? `color-mix(in oklch, ${color} 20%, var(--card))`
      : undefined,
    left: `${left}%`,
    width: `${width}%`,
  };
  const className = cn(
    'absolute top-1 bottom-1 flex h-auto min-h-0 items-center overflow-hidden rounded-sm border text-xs font-medium',
    (chunk.result === 'queued' || chunk.result === 'running') &&
      'border-primary/50 bg-primary/10 text-foreground',
    (chunk.result === 'failed' || chunk.result === 'cancelled') &&
      'border-destructive/60 bg-destructive/10 text-destructive',
    chunk.result === 'no-speech' &&
      'border-muted-foreground/50 bg-muted text-muted-foreground',
    chunk.result === 'skipped-silence' &&
      'border-border bg-muted/60 text-muted-foreground',
    textItem && 'border-border/70 bg-card/90 text-foreground',
  );

  function handleInspect() {
    if (item.kind !== 'transcript-segment') {
      return;
    }
    const sourceRole = transcriptionSourceRole(track.kind);
    onInspectTranscript({
      chunkResult: chunk.result,
      confidence: item.transcriptSegment.confidence,
      endOffsetMs: item.transcriptSegment.endOffsetMs,
      kind: 'transcript-segment',
      model: item.transcriptSegment.model,
      provider: item.transcriptSegment.provider,
      segmentId: item.transcriptSegment.segmentId,
      sourceRole,
      startOffsetMs: item.transcriptSegment.startOffsetMs,
      text: item.transcriptSegment.text,
      timelineOffsetMs: item.transcriptSegment.startOffsetMs,
      trackId: track.id,
      trackLabel: track.label,
    });
  }

  if (item.kind === 'transcript-segment') {
    return (
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            disabled={covered}
            aria-hidden={covered || undefined}
            aria-label={`Inspect ${track.label} transcript ${formatDuration(startMs)} to ${formatDuration(endMs)}`}
            className={cn(
              className,
              'justify-start rounded-sm px-1 py-0 text-left font-normal whitespace-normal hover:brightness-105',
            )}
            data-library-action
            data-transcript-segment-id={item.transcriptSegment.id}
            style={style}
            title={item.text}
            type="button"
            variant="ghost"
            onClick={handleInspect}
          >
            <span className="line-clamp-2 w-full text-left leading-4">
              {item.text}
            </span>
          </Button>
        </TooltipTrigger>
        <TooltipContent>{`${track.label} · ${formatDuration(startMs)}–${formatDuration(endMs)}`}</TooltipContent>
      </Tooltip>
    );
  }

  return (
    <div
      aria-label={itemLabel(item)}
      className={className}
      style={style}
      title={itemTitle(item)}
    >
      {textItem ? (
        <span className="line-clamp-2 px-1 leading-4">{item.text}</span>
      ) : null}
    </div>
  );
}

function transcriptionSourceRole(
  trackKind: LibraryRecordingTrack['kind'],
): 'desktop-audio' | 'microphone' {
  return trackKind === 'microphone' ? 'microphone' : 'desktop-audio';
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
    case 'completed-with-errors':
      return 'Transcript available with failed sections';
    case 'cancelled':
      return 'Transcription chunk cancelled';
    case 'no-speech':
      return 'Transcription chunk processed with no speech';
    case 'skipped-silence':
      return 'Transcription chunk skipped because no speech was detected';
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
    return aiProcessingErrorDetails(chunk.errorCode);
  }
  if (chunk.resultCode === 'skipped-silence') {
    return 'Skipped: below silence threshold.';
  }
  return chunkLabel(chunk);
}
