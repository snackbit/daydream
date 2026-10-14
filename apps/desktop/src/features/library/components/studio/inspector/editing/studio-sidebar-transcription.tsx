import type {
  RecordingAiProcessingState,
  RecordingTranscriptionChunkState,
  RecordingTranscriptionTrackState,
} from '@daydream/shared';
import { ProcessingError } from '@/features/processing/components/processing-error';
import { aiProcessingErrorSummary } from '../../timeline/processing/ai-processing-error-copy';
import type {
  StudioLaneDraft,
  StudioTimeRangeSelection,
} from '../../timeline/state/studio-timeline-store';
import { transcriptSelectionEntries } from '../content/inspector-content';
import { InspectorContentEntryCard } from '../content/inspector-content-entry';
import { MetadataRow } from './studio-sidebar-section';

export function TranscriptionTrackDetails({
  track,
}: {
  track: RecordingTranscriptionTrackState;
}) {
  const errorCode =
    track.chunks.find((chunk) => chunk.errorCode)?.errorCode ?? null;

  return (
    <div className="space-y-1 border-t pt-2">
      <MetadataRow label="Speech" value={track.statusLabel} />
      <MetadataRow label="Chunks" value={String(track.chunks.length)} />
      <MetadataRow label="Text" value={String(track.textSegmentCount)} />
      {errorCode ? (
        <>
          <ProcessingError errorCode={errorCode} />
        </>
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
      {chunks.some((chunk) => chunk.errorCode) ? (
        <MetadataRow label="Issue" value={chunkErrorSummary(chunks)} />
      ) : null}
      {chunks.some((chunk) => chunk.errorCode) ? (
        <MetadataRow label="Error" value={chunkErrorCodeSummary(chunks)} />
      ) : null}
      {chunks.some((chunk) => chunk.resultCode) ? (
        <MetadataRow label="Result" value={chunkSummaryByResultCode(chunks)} />
      ) : null}
      {text ? <MetadataRow label="Text" value={text} /> : null}
    </div>
  );
}

function chunkErrorSummary(chunks: RecordingTranscriptionChunkState[]): string {
  const errorCode = chunks.find((chunk) => chunk.errorCode)?.errorCode;

  return errorCode ? aiProcessingErrorSummary(errorCode) : 'Unknown';
}

function chunkErrorCodeSummary(
  chunks: RecordingTranscriptionChunkState[],
): string {
  const counts = chunks.reduce<Record<string, number>>((summary, chunk) => {
    if (chunk.errorCode) {
      summary[chunk.errorCode] = (summary[chunk.errorCode] ?? 0) + 1;
    }
    return summary;
  }, {});

  return Object.entries(counts)
    .map(([state, count]) => `${state}: ${count}`)
    .join(', ');
}

export function TranscriptionSelectionDetails({
  aiProcessingState,
  selections,
  segmentId,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  selections: StudioTimeRangeSelection[];
  segmentId?: string;
}) {
  const entries = transcriptSelectionEntries(
    aiProcessingState,
    selections,
    segmentId,
  );
  if (entries.length === 0) {
    return null;
  }

  return (
    <ol className="space-y-2">
      {entries.map((entry) => (
        <InspectorContentEntryCard entry={entry} key={entry.id} />
      ))}
    </ol>
  );
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
