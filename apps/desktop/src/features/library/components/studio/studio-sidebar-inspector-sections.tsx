import { useMemo } from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  LibraryRecordingDetail,
  OcrRegion,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { FileText, ShieldOff, ShieldX } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { formatDuration } from '../../library-format';
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
  const ocrText = useMemo(
    () => buildOcrText(frame.ocrRegions),
    [frame.ocrRegions],
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
      <MetadataRow label="VLM" value={String(frame.vlmObservations.length)} />
      <MetadataRow label="OCR" value={String(frame.ocrRegions.length)} />

      <div className="space-y-2 border-t pt-2">
        <div className="text-muted-foreground">Screen observations</div>
        {frame.vlmObservations.length > 0 ? (
          frame.vlmObservations.map((observation) => (
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

      <div className="border-t pt-2">
        <VisualFrameOcrDialog frame={frame} ocrText={ocrText} />
      </div>
    </StudioSidebarSection>
  );
}

function VisualFrameOcrDialog({
  frame,
  ocrText,
}: {
  frame: StudioVisualFrame;
  ocrText: string;
}) {
  const title = `OCR text ${formatDuration(frame.frameTimestampMs)} - ${formatDuration(
    frame.endTimestampMs,
  )}`;

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button
          className="w-full justify-start"
          disabled={frame.ocrRegions.length === 0}
          size="sm"
          type="button"
          variant="outline"
        >
          <FileText data-icon="inline-start" />
          OCR text ({frame.ocrRegions.length})
        </Button>
      </DialogTrigger>
      <DialogContent className="max-h-[min(42rem,calc(100vh-2rem))] max-w-2xl grid-rows-[auto_minmax(0,1fr)]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>
            Text candidates reconstructed from Tesseract regions for this visual
            frame.
          </DialogDescription>
        </DialogHeader>
        <ScrollArea className="min-h-0 rounded-sm border">
          <div className="text-foreground p-3 text-xs leading-5 whitespace-pre-wrap">
            {ocrText || 'No OCR text.'}
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}

function buildOcrText(regions: OcrRegion[]): string {
  const lines: Array<{ items: OcrRegion[]; y: number }> = [];
  for (const region of [...regions].sort(compareOcrRegionReadingOrder)) {
    const line = lines.find(
      (candidate) =>
        Math.abs(candidate.y - region.region.y) <=
        Math.max(8, region.region.height * 0.6),
    );

    if (line) {
      line.items.push(region);
      line.y = Math.min(line.y, region.region.y);
    } else {
      lines.push({ items: [region], y: region.region.y });
    }
  }

  return lines
    .sort((left, right) => left.y - right.y)
    .map((line) =>
      line.items
        .sort(compareOcrRegionReadingOrder)
        .map((region) => region.text.trim())
        .filter(Boolean)
        .join(' ')
        .replace(/\s+([,.;:!?%)\]])/g, '$1')
        .replace(/([([{])\s+/g, '$1'),
    )
    .filter(Boolean)
    .join('\n');
}

function compareOcrRegionReadingOrder(
  left: OcrRegion,
  right: OcrRegion,
): number {
  return (
    left.region.y - right.region.y ||
    left.region.x - right.region.x ||
    left.id.localeCompare(right.id)
  );
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
