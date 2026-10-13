import type {
  CreateLibraryRedactionRangeInput,
  LibraryRecordingDetail,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { ShieldOff, ShieldX } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { formatDuration } from '../../library-format';
import {
  selectInspectorEntries,
  type InspectorContentIndex,
} from './inspector/content/inspector-content';
import { InspectorContentEntryCard } from './inspector/content/inspector-content-entry';
import { useSelectionRedactionMutation } from './redactions/use-redaction-mutation';
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
import type { StudioVisualFrame } from './studio-visual-observations';

export interface StudioSidebarSegmentSelection extends StudioClipDraft {
  clipId: string;
}

export function StudioSidebarVisualFrameSection({
  frame,
}: {
  frame: StudioVisualFrame;
}) {
  const observations = frame.vlmObservations.filter(
    (item) => item.observationKind !== 'corrected-ocr',
  );

  return (
    <StudioSidebarSection
      title={`Visual frame ${formatDuration(
        frame.frameTimestampMs,
      )} - ${formatDuration(frame.endTimestampMs)}`}
    >
      <MetadataRow
        label="Sources"
        value={`${frame.sourceFrameRefs.length} frame ${
          frame.sourceFrameRefs.length === 1 ? 'ref' : 'refs'
        }`}
      />
      <MetadataRow label="VLM" value={String(observations.length)} />
      <MetadataRow label="OCR" value={String(frame.ocrRegions.length)} />

      <div className="space-y-2 border-t pt-2">
        <div className="text-muted-foreground">Screen observations</div>
        {observations.length > 0 ? (
          observations.map((observation) => (
            <div
              className="bg-muted/60 rounded-sm border p-2 text-xs leading-5 whitespace-pre-wrap"
              key={observation.id}
            >
              {observation.text}
            </div>
          ))
        ) : (
          <div className="text-muted-foreground rounded-sm border border-dashed p-2">
            No VLM observation for this frame.
          </div>
        )}
      </div>
    </StudioSidebarSection>
  );
}

export function StudioSidebarSectionLane({
  aiProcessingState,
  lane,
  debugEnabled = false,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  lane: StudioLaneDraft;
  debugEnabled?: boolean;
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
      {debugEnabled ? (
        <MetadataRow label="Source" value={lane.description} />
      ) : null}
      {transcriptionTrack && !debugEnabled ? (
        <p className="text-muted-foreground text-xs">
          {transcriptionTrack.statusLabel}
        </p>
      ) : null}
      {transcriptionTrack && debugEnabled ? (
        <TranscriptionTrackDetails track={transcriptionTrack} />
      ) : (
        <TranscriptionSelectionDetails
          aiProcessingState={aiProcessingState}
          selections={[
            {
              id: 'lane',
              laneIds: [lane.trackId],
              startMs: 0,
              endMs: Number.MAX_SAFE_INTEGER,
            },
          ]}
        />
      )}
    </StudioSidebarSection>
  );
}

export function StudioSidebarSectionSegment({
  aiProcessingState,
  segment,
  debugEnabled = false,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  segment: StudioSidebarSegmentSelection;
  debugEnabled?: boolean;
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
      {debugEnabled ? (
        <>
          <MetadataRow label="Segment" value={segment.segmentId} />
          {chunks.length > 0 ? (
            <TranscriptionChunkDetails chunks={chunks} />
          ) : null}
        </>
      ) : (
        <TranscriptionSelectionDetails
          aiProcessingState={aiProcessingState}
          segmentId={segment.segmentId}
          selections={[
            {
              id: segment.clipId,
              laneIds: [segment.trackId],
              startMs: 0,
              endMs: Number.MAX_SAFE_INTEGER,
            },
          ]}
        />
      )}
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
  content,
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
  content?: InspectorContentIndex;
}) {
  const mutation = useSelectionRedactionMutation({
    ranges: selections,
    onCreate: onCreateRedactions,
    onUnredact: onUnredactRanges,
    onClear: clearSelection,
  });
  const intersectingRedactions = redactionsIntersectingRanges(
    detail.redactions,
    selections,
  );
  const totalDurationMs = selections.reduce(
    (durationMs, selection) =>
      durationMs + Math.max(0, selection.endMs - selection.startMs),
    0,
  );

  return (
    <StudioSidebarSection title="Selection">
      <MetadataRow label="Duration" value={formatDuration(totalDurationMs)} />
      <MetadataRow
        label="Targets"
        value={`${new Set(selections.flatMap((range) => range.laneIds)).size} ${new Set(selections.flatMap((range) => range.laneIds)).size === 1 ? 'lane' : 'lanes'}`}
      />
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
      {content ? (
        <ol className="space-y-2">
          {selectInspectorEntries(content.entries, selections).map((entry) => (
            <InspectorContentEntryCard key={entry.id} entry={entry} />
          ))}
        </ol>
      ) : (
        <TranscriptionSelectionDetails
          aiProcessingState={aiProcessingState}
          selections={selections}
        />
      )}
      <div className="grid gap-2">
        {mutation.pending ? (
          <p role="status" className="text-muted-foreground text-xs">
            Updating redactions…
          </p>
        ) : null}
        {mutation.error ? (
          <Alert variant="destructive">
            <AlertDescription>{mutation.error}</AlertDescription>
          </Alert>
        ) : null}
        <Button
          disabled={mutation.pending}
          type="button"
          variant="outline"
          onClick={mutation.redact}
        >
          <ShieldX data-icon="inline-start" />
          Redact
        </Button>
        {intersectingRedactions.length > 0 ? (
          <Button
            disabled={mutation.pending}
            type="button"
            variant="outline"
            onClick={mutation.unredact}
          >
            <ShieldOff data-icon="inline-start" />
            Unredact
          </Button>
        ) : null}
      </div>
    </StudioSidebarSection>
  );
}
