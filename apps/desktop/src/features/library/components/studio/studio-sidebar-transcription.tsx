import type {
  RecordingAiProcessingState,
  RecordingTranscriptionChunkState,
  RecordingTranscriptionTrackState,
} from '@daydream/shared';
import { MetadataRow } from './studio-sidebar-section';
import type {
  StudioLaneDraft,
  StudioTimeRangeSelection,
} from './studio-timeline-store';

export function TranscriptionTrackDetails({
  track,
}: {
  track: RecordingTranscriptionTrackState;
}) {
  return (
    <div className="space-y-1 border-t pt-2">
      <MetadataRow label="Speech" value={track.statusLabel} />
      <MetadataRow label="Chunks" value={String(track.chunks.length)} />
      <MetadataRow label="Text" value={String(track.textSegmentCount)} />
      {track.chunks.some((chunk) => chunk.errorCode) ? (
        <MetadataRow
          label="Error"
          value={
            track.chunks.find((chunk) => chunk.errorCode)?.errorCode ??
            'Unknown'
          }
        />
      ) : null}
    </div>
  );
}

export function TranscriptionChunkDetails({
  chunks,
}: {
  chunks: RecordingTranscriptionChunkState[];
}) {
  const text = chunks
    .map((chunk) => chunk.text)
    .filter((value): value is string => Boolean(value))
    .join(' ');

  return (
    <div className="space-y-1 border-t pt-2">
      <MetadataRow label="Chunks" value={String(chunks.length)} />
      <MetadataRow label="State" value={chunkSummary(chunks)} />
      {chunks.some((chunk) => chunk.resultCode) ? (
        <MetadataRow label="Result" value={chunkSummaryByResultCode(chunks)} />
      ) : null}
      {text ? <MetadataRow label="Text" value={text} /> : null}
    </div>
  );
}

export function TranscriptionSelectionDetails({
  aiProcessingState,
  selections,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  selections: StudioTimeRangeSelection[];
}) {
  const chunks =
    aiProcessingState?.transcriptionTracks?.flatMap((track) =>
      track.chunks.filter((chunk) =>
        selections.some(
          (selection) =>
            selection.laneIds.includes(track.libraryTrackId ?? '') &&
            (chunk.startOffsetMs ?? 0) < selection.endMs &&
            (chunk.endOffsetMs ?? chunk.startOffsetMs ?? 0) > selection.startMs,
        ),
      ),
    ) ?? [];

  if (chunks.length === 0) {
    return null;
  }

  return <TranscriptionChunkDetails chunks={chunks} />;
}

export function transcriptionTrackForLane(
  aiProcessingState: RecordingAiProcessingState | null,
  lane: StudioLaneDraft,
): RecordingTranscriptionTrackState | null {
  return (
    aiProcessingState?.transcriptionTracks?.find(
      (track) =>
        track.libraryTrackId === lane.trackId || track.sourceRole === lane.kind,
    ) ?? null
  );
}

function chunkSummary(chunks: RecordingTranscriptionChunkState[]): string {
  const counts = chunks.reduce<Record<string, number>>((summary, chunk) => {
    summary[chunk.result] = (summary[chunk.result] ?? 0) + 1;
    return summary;
  }, {});

  return Object.entries(counts)
    .map(([state, count]) => `${state}: ${count}`)
    .join(', ');
}

function chunkSummaryByResultCode(
  chunks: RecordingTranscriptionChunkState[],
): string {
  const counts = chunks.reduce<Record<string, number>>((summary, chunk) => {
    if (chunk.resultCode) {
      summary[chunk.resultCode] = (summary[chunk.resultCode] ?? 0) + 1;
    }
    return summary;
  }, {});

  return Object.entries(counts)
    .map(([state, count]) => `${state}: ${count}`)
    .join(', ');
}
