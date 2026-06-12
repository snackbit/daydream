import type {
  CreateLibraryRedactionRangeInput,
  LibraryRecordingDetail,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { ShieldOff, ShieldX } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { formatDuration, formatTimestamp } from '../../library-format';
import { redactionsIntersectingRanges } from './studio-redactions';
import { EditableLaneFields, trackKindLabel } from './studio-sidebar-editing';
import { MetadataRow, StudioSidebarSection } from './studio-sidebar-section';
import {
  TranscriptionChunkDetails,
  TranscriptionSelectionDetails,
  TranscriptionTrackDetails,
  transcriptionTrackForLane,
} from './studio-sidebar-transcription';
import {
  useStudioTimelineStore,
  type StudioClipDraft,
  type StudioLaneDraft,
  type StudioTimeRangeSelection,
} from './studio-timeline-store';

export interface StudioSidebarSegmentSelection extends StudioClipDraft {
  clipId: string;
}

export function StudioSidebarSectionLane({
  aiProcessingState,
  lane,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  lane: StudioLaneDraft;
}) {
  const renameLane = useStudioTimelineStore((state) => state.renameLane);
  const setLaneColor = useStudioTimelineStore((state) => state.setLaneColor);
  const transcriptionTrack = transcriptionTrackForLane(aiProcessingState, lane);

  return (
    <StudioSidebarSection title={lane.label}>
      <EditableLaneFields
        lane={lane}
        onColorChange={(colorId) => setLaneColor(lane.trackId, colorId)}
        onRename={(label) => renameLane(lane.trackId, label)}
      />
      <MetadataRow label="Type" value={trackKindLabel(lane.kind)} />
      <MetadataRow label="Source" value={lane.description} />
      {transcriptionTrack ? (
        <TranscriptionTrackDetails track={transcriptionTrack} />
      ) : null}
    </StudioSidebarSection>
  );
}

export function StudioSidebarSectionSegment({
  aiProcessingState,
  segment,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  segment: StudioSidebarSegmentSelection;
}) {
  const lanesById = useStudioTimelineStore((state) => state.lanesById);
  const renameClip = useStudioTimelineStore((state) => state.renameClip);
  const lane = lanesById[segment.trackId] ?? null;
  const transcriptionTrack =
    lane === null ? null : transcriptionTrackForLane(aiProcessingState, lane);
  const chunks =
    transcriptionTrack?.chunks.filter(
      (chunk) => chunk.segmentId === segment.segmentId,
    ) ?? [];

  return (
    <StudioSidebarSection title={segment.title}>
      <label className="grid gap-1">
        <span className="text-muted-foreground">Title</span>
        <Input
          aria-label="Clip title"
          value={segment.title}
          onChange={(event) =>
            renameClip(segment.clipId, event.currentTarget.value)
          }
        />
      </label>
      <MetadataRow label="Track" value={lane?.label ?? ''} />
      <MetadataRow label="Segment" value={segment.segmentId} />
      {chunks.length > 0 ? <TranscriptionChunkDetails chunks={chunks} /> : null}
    </StudioSidebarSection>
  );
}

export function StudioSidebarComponentSelection({
  aiProcessingState,
  clearSelection,
  detail,
  onCreateRedactions,
  onUnredactRanges,
  selections,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  clearSelection: () => void;
  detail: LibraryRecordingDetail;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  selections: StudioTimeRangeSelection[];
}) {
  const intersectingRedactions = redactionsIntersectingRanges(
    detail.redactions,
    selections,
  );
  const selectedLaneCount = new Set(
    selections.flatMap((selection) => selection.laneIds),
  ).size;
  const totalDurationMs = selections.reduce(
    (durationMs, selection) =>
      durationMs + Math.max(0, selection.endMs - selection.startMs),
    0,
  );

  return (
    <StudioSidebarSection title="Selection">
      <MetadataRow label="Ranges" value={String(selections.length)} />
      <MetadataRow
        label="Targets"
        value={`${selectedLaneCount} ${selectedLaneCount === 1 ? 'lane' : 'lanes'}`}
      />
      <MetadataRow label="Duration" value={formatDuration(totalDurationMs)} />
      <div className="space-y-1">
        {selections.map((selection, index) => (
          <MetadataRow
            key={selection.id}
            label={`Range ${index + 1}`}
            value={`${formatDuration(selection.startMs)} - ${formatDuration(
              selection.endMs,
            )}`}
          />
        ))}
      </div>
      <TranscriptionSelectionDetails
        aiProcessingState={aiProcessingState}
        selections={selections}
      />
      <div className="grid gap-2">
        <Button
          type="button"
          variant="outline"
          onClick={() => {
            void onCreateRedactions(
              selections.map((selection) => ({
                endOffsetMs: Math.round(selection.endMs),
                startOffsetMs: Math.round(selection.startMs),
                trackIds: selection.laneIds,
              })),
            ).then(clearSelection);
          }}
        >
          <ShieldX data-icon="inline-start" />
          Redact
        </Button>
        {intersectingRedactions.length > 0 ? (
          <Button
            type="button"
            variant="outline"
            onClick={() => {
              void onUnredactRanges(
                selections.map((selection) => ({
                  endOffsetMs: Math.round(selection.endMs),
                  startOffsetMs: Math.round(selection.startMs),
                  trackIds: selection.laneIds,
                })),
              ).then(clearSelection);
            }}
          >
            <ShieldOff data-icon="inline-start" />
            Unredact
          </Button>
        ) : null}
      </div>
    </StudioSidebarSection>
  );
}

export function StudioSidebarRecordingSection({
  aiProcessingState,
  detail,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
}) {
  return (
    <StudioSidebarSection title="Recording">
      <MetadataRow label="Recording" value={detail.id} />
      <MetadataRow
        label="Started"
        value={formatTimestamp(detail.startedAtMs)}
      />
      <MetadataRow label="Duration" value={formatDuration(detail.durationMs)} />
      {aiProcessingState?.job ? (
        <>
          <MetadataRow label="AI job" value={aiProcessingState.job.status} />
          <MetadataRow
            label="Chunks"
            value={String(aiProcessingState.chunks.length)}
          />
        </>
      ) : null}
      {aiProcessingState?.transcriptionTracks?.map((track) => (
        <TranscriptionTrackDetails key={track.trackKey} track={track} />
      ))}
    </StudioSidebarSection>
  );
}
