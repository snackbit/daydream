import type {
  LibraryRecordingDetail,
  RecordingAiProcessingState,
  RecordingTranscriptionTrackState,
} from '@daydream/shared';
import {
  availabilityLabel,
  formatDuration,
  formatFileSize,
  formatTimestamp,
} from '../../../library-format';
import { transcriptionSourceLabel } from './inspector-format';
import { InspectorField, InspectorSection } from './inspector-section';

export interface RecordingInspectorSummaryProps {
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
  debugEnabled?: boolean;
}

export function RecordingInspectorSummary({
  aiProcessingState,
  detail,
  debugEnabled = false,
}: RecordingInspectorSummaryProps) {
  const transcriptionTracks = aiProcessingState?.transcriptionTracks ?? [];

  return (
    <>
      <InspectorSection title="Recording details">
        <dl className="space-y-1.5">
          <InspectorField
            label="Title"
            value={detail.title ?? 'Untitled recording'}
          />
          <InspectorField
            label="Status"
            value={`${detail.status} · ${availabilityLabel(detail.availability)}`}
          />
          <InspectorField
            label="Started"
            value={formatTimestamp(detail.startedAtMs)}
          />
          <InspectorField
            label="Duration"
            value={formatDuration(detail.durationMs)}
          />
          <InspectorField
            label="Segments"
            value={String(detail.segments.length)}
          />
          <InspectorField
            label="Stored locally"
            value={formatFileSize(detail.sizeBytes)}
          />
          <InspectorField
            label="Redactions"
            value={String(detail.redactions.length)}
          />
          {debugEnabled ? (
            <InspectorField
              label="Recording ID"
              value={<code>{detail.id}</code>}
            />
          ) : null}
        </dl>
      </InspectorSection>

      <InspectorSection title="Processing">
        {aiProcessingState?.job ? (
          <dl className="space-y-1.5">
            <InspectorField
              label="Job status"
              value={aiProcessingState.job.status}
            />
            <InspectorField
              label="Processing chunks"
              value={String(aiProcessingState.chunks.length)}
            />
          </dl>
        ) : (
          <p className="text-muted-foreground text-xs">
            No processing job status is available.
          </p>
        )}
      </InspectorSection>

      <InspectorSection title="Audio transcription">
        {transcriptionTracks.length > 0 ? (
          <div className="divide-y">
            {transcriptionTracks.map((track) => (
              <TranscriptionSourceSummary
                detail={detail}
                key={track.trackKey}
                track={track}
                debugEnabled={debugEnabled}
              />
            ))}
          </div>
        ) : (
          <p className="text-muted-foreground text-xs">
            No source-specific transcription status is available.
          </p>
        )}
      </InspectorSection>
    </>
  );
}

function TranscriptionSourceSummary({
  detail,
  track,
  debugEnabled,
}: {
  detail: LibraryRecordingDetail;
  track: RecordingTranscriptionTrackState;
  debugEnabled: boolean;
}) {
  const sourceName = transcriptionSourceLabel(track.sourceRole);
  const libraryTrack = track.libraryTrackId
    ? detail.tracks.find((candidate) => candidate.id === track.libraryTrackId)
    : null;

  return (
    <section className="grid gap-1.5 py-2 first:pt-0 last:pb-0">
      <h4 className="text-xs font-medium">{sourceName}</h4>
      <dl className="space-y-1">
        <InspectorField label="Status" value={track.statusLabel} />
        <InspectorField
          label="Capture track"
          value={libraryTrack?.label ?? track.libraryTrackId ?? 'Unavailable'}
        />
        {debugEnabled ? (
          <InspectorField
            label="Track ID"
            value={
              track.libraryTrackId ? (
                <code>{track.libraryTrackId}</code>
              ) : (
                'Unavailable'
              )
            }
          />
        ) : null}
        <InspectorField label="Chunks" value={String(track.chunks.length)} />
        <InspectorField
          label="Text segments"
          value={String(track.textSegmentCount)}
        />
      </dl>
    </section>
  );
}
