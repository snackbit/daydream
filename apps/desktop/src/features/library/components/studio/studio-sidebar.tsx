import type { ReactNode } from 'react';
import type { LibraryRecordingDetail } from '@daydream/shared';
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

export function StudioSidebar({ detail }: { detail: LibraryRecordingDetail }) {
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
  detail,
  selectedLanes,
  selectedRanges,
  selectedSegments,
}: {
  detail: LibraryRecordingDetail;
  selectedLanes: StudioLaneDraft[];
  selectedRanges: StudioTimeRangeSelection[];
  selectedSegments: StudioSidebarSegmentSelection[];
}) {
  if (selectedRanges.length > 0) {
    return <StudioSidebarComponentSelection selections={selectedRanges} />;
  }

  if (selectedSegments.length > 0) {
    return selectedSegments.map((segment) => (
      <StudioSidebarSectionSegment key={segment.clipId} segment={segment} />
    ));
  }

  if (selectedLanes.length > 0) {
    return selectedLanes.map((lane) => (
      <StudioSidebarSectionLane key={lane.trackId} lane={lane} />
    ));
  }

  return <StudioSidebarRecordingSection detail={detail} />;
}

function StudioSidebarSectionLane({ lane }: { lane: StudioLaneDraft }) {
  const renameLane = useStudioTimelineStore((state) => state.renameLane);
  const setLaneColor = useStudioTimelineStore((state) => state.setLaneColor);

  return (
    <StudioSidebarSection title={lane.label}>
      <EditableLaneFields
        lane={lane}
        onColorChange={(colorId) => setLaneColor(lane.trackId, colorId)}
        onRename={(label) => renameLane(lane.trackId, label)}
      />
      <MetadataRow label="Type" value={trackKindLabel(lane.kind)} />
      <MetadataRow label="Source" value={lane.description} />
    </StudioSidebarSection>
  );
}

function StudioSidebarSectionSegment({
  segment,
}: {
  segment: StudioSidebarSegmentSelection;
}) {
  const lanesById = useStudioTimelineStore((state) => state.lanesById);
  const renameClip = useStudioTimelineStore((state) => state.renameClip);
  const lane = lanesById[segment.trackId] ?? null;

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
    </StudioSidebarSection>
  );
}

function StudioSidebarComponentSelection({
  selections,
}: {
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
  detail,
}: {
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
    </StudioSidebarSection>
  );
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
