import type { ReactNode } from 'react';
import type {
  LibraryRecordingDetail,
  RecordingAiProcessingState,
  RecordingTranscriptionChunkState,
  RecordingTranscriptionTrackState,
} from '@daydream/shared';
import { AlertTriangle, Lock, Scissors, ShieldX } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';
import { formatDuration, formatTimestamp } from '../../library-format';
import {
  STUDIO_TIMELINE_COLORS,
  studioColorById,
  useStudioTimelineStore,
  type StudioClipDraft,
  type StudioLaneDraft,
  type StudioTimeRangeSelection,
} from './studio-timeline-store';

interface StudioSidebarSegmentSelection extends StudioClipDraft {
  clipId: string;
}

export function StudioSidebar({
  aiProcessingState,
  detail,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
}) {
  const clipsById = useStudioTimelineStore((state) => state.clipsById);
  const lanesById = useStudioTimelineStore((state) => state.lanesById);
  const selection = useStudioTimelineStore((state) => state.selection);
  const selectedLanes = selection.laneIds
    .map((laneId) => lanesById[laneId])
    .filter((lane): lane is StudioLaneDraft => Boolean(lane));
  const selectedSegments = selection.clipIds
    .map((clipId): StudioSidebarSegmentSelection | null => {
      const clip = clipsById[clipId];
      return clip ? { ...clip, clipId } : null;
    })
    .filter((segment): segment is StudioSidebarSegmentSelection =>
      Boolean(segment),
    );
  const content = renderStudioSidebarContent({
    aiProcessingState,
    detail,
    selectedLanes,
    selectedRanges: selection.ranges,
    selectedSegments,
  });

  return (
    <aside className="bg-sidebar text-sidebar-foreground flex h-full flex-col">
      <div className="border-b p-3">
        <div className="text-sm font-medium">Inspector</div>
        <div className="text-muted-foreground mt-1 text-xs">
          Selection and recording metadata
        </div>
      </div>
      <ScrollArea className="min-h-0 flex-1">
        <div className="text-xs">
          {detail.availability !== 'available' ? (
            <Alert
              className="m-3"
              variant={
                detail.availability === 'locked' ? 'default' : 'destructive'
              }
            >
              {detail.availability === 'locked' ? <Lock /> : <AlertTriangle />}
              <AlertTitle>
                {detail.availability === 'locked'
                  ? 'Recording in progress'
                  : 'Missing video files'}
              </AlertTitle>
              <AlertDescription>
                {detail.missingReasons.join(' ') ||
                  'Recording files are unavailable.'}
              </AlertDescription>
            </Alert>
          ) : null}
          {content}
        </div>
      </ScrollArea>
    </aside>
  );
}

function renderStudioSidebarContent({
  aiProcessingState,
  detail,
  selectedLanes,
  selectedRanges,
  selectedSegments,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
  selectedLanes: StudioLaneDraft[];
  selectedRanges: StudioTimeRangeSelection[];
  selectedSegments: StudioSidebarSegmentSelection[];
}) {
  if (selectedRanges.length > 0) {
    return (
      <StudioSidebarComponentSelection
        aiProcessingState={aiProcessingState}
        selections={selectedRanges}
      />
    );
  }

  if (selectedSegments.length > 0) {
    return selectedSegments.map((segment) => (
      <StudioSidebarSectionSegment
        aiProcessingState={aiProcessingState}
        key={segment.clipId}
        segment={segment}
      />
    ));
  }

  if (selectedLanes.length > 0) {
    return selectedLanes.map((lane) => (
      <StudioSidebarSectionLane
        aiProcessingState={aiProcessingState}
        key={lane.trackId}
        lane={lane}
      />
    ));
  }

  return (
    <StudioSidebarRecordingSection
      aiProcessingState={aiProcessingState}
      detail={detail}
    />
  );
}

function StudioSidebarSectionLane({
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

function StudioSidebarSectionSegment({
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

function StudioSidebarComponentSelection({
  aiProcessingState,
  selections,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  selections: StudioTimeRangeSelection[];
}) {
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
      <div className="grid grid-cols-2 gap-2">
        <Button type="button" variant="outline">
          <ShieldX data-icon="inline-start" />
          Redact
        </Button>
        <Button type="button" variant="outline">
          <Scissors data-icon="inline-start" />
          Cut
        </Button>
      </div>
    </StudioSidebarSection>
  );
}

function StudioSidebarRecordingSection({
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

function TranscriptionTrackDetails({
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

function TranscriptionChunkDetails({
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

function TranscriptionSelectionDetails({
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

function transcriptionTrackForLane(
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

function EditableLaneFields({
  lane,
  onColorChange,
  onRename,
}: {
  lane: StudioLaneDraft;
  onColorChange: (colorId: string) => void;
  onRename: (label: string) => void;
}) {
  const color = studioColorById(lane.colorId);

  return (
    <div className="grid gap-2">
      <label className="grid gap-1">
        <span className="text-muted-foreground">Name</span>
        <Input
          aria-label="Track name"
          value={lane.label}
          onChange={(event) => onRename(event.currentTarget.value)}
        />
      </label>
      <div className="grid gap-1">
        <div className="text-muted-foreground">Color</div>
        <ColorPalettePopover
          selectedColorId={color.id}
          onColorChange={onColorChange}
        />
      </div>
    </div>
  );
}

function ColorPalettePopover({
  onColorChange,
  selectedColorId,
}: {
  onColorChange: (colorId: string) => void;
  selectedColorId: string;
}) {
  const selectedColor = studioColorById(selectedColorId);

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          aria-label="Select track color"
          className="w-full justify-start"
          type="button"
          variant="outline"
        >
          <span
            aria-hidden="true"
            className="size-3 rounded-full"
            style={{ backgroundColor: selectedColor.strong }}
          />
          {selectedColor.label}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-56">
        <div className="grid grid-cols-6 gap-1">
          {STUDIO_TIMELINE_COLORS.map((color) => (
            <button
              aria-label={color.label}
              aria-pressed={color.id === selectedColorId}
              className={cn(
                'focus-visible:ring-ring/40 size-7 rounded-md border transition-transform outline-none focus-visible:ring-2',
                color.id === selectedColorId && 'ring-ring ring-2',
              )}
              key={color.id}
              style={{
                backgroundColor: color.strong,
                borderColor: color.border,
              }}
              type="button"
              onClick={() => onColorChange(color.id)}
            />
          ))}
        </div>
      </PopoverContent>
    </Popover>
  );
}

function StudioSidebarSection({
  children,
  title,
}: {
  children: ReactNode;
  title: ReactNode;
}) {
  return (
    <section className="space-y-2 border-b">
      <div className="text-muted-foreground bg-background border-b p-3 font-medium">
        {title}
      </div>
      <div className="space-y-2 p-3">{children}</div>
    </section>
  );
}

function MetadataRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[5rem_1fr] gap-2">
      <div className="text-muted-foreground">{label}</div>
      <div className="truncate">{value}</div>
    </div>
  );
}

function trackKindLabel(kind: StudioLaneDraft['kind']): string {
  switch (kind) {
    case 'desktop-audio':
      return 'Desktop audio';
    case 'microphone':
      return 'Microphone';
    case 'video':
      return 'Video';
  }
}
