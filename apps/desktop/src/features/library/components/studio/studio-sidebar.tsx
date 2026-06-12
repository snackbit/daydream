import type {
  CreateLibraryRedactionRangeInput,
  LibraryRecordingDetail,
  LibraryRecordingRedaction,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { AlertTriangle, Lock } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  StudioSidebarComponentSelection,
  StudioSidebarRecordingSection,
  StudioSidebarSectionLane,
  StudioSidebarSectionSegment,
  type StudioSidebarSegmentSelection,
} from './studio-sidebar-inspector-sections';
import { StudioSidebarRedactionSection } from './studio-sidebar-redaction-section';
import {
  useStudioTimelineStore,
  type StudioLaneDraft,
  type StudioTimeRangeSelection,
} from './studio-timeline-store';

export function StudioSidebar({
  aiProcessingState,
  detail,
  onCreateRedactions,
  onDeleteRedaction,
  onUnredactRanges,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onDeleteRedaction: (redactionId: string) => Promise<void>;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
}) {
  const clipsById = useStudioTimelineStore((state) => state.clipsById);
  const lanesById = useStudioTimelineStore((state) => state.lanesById);
  const selection = useStudioTimelineStore((state) => state.selection);
  const clearSelection = useStudioTimelineStore(
    (state) => state.clearSelection,
  );
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
  const selectedRedactions = selection.redactionIds
    .map((redactionId) =>
      detail.redactions.find((redaction) => redaction.id === redactionId),
    )
    .filter((redaction): redaction is LibraryRecordingRedaction =>
      Boolean(redaction),
    );
  const content = renderStudioSidebarContent({
    aiProcessingState,
    clearSelection,
    detail,
    onCreateRedactions,
    onDeleteRedaction,
    onUnredactRanges,
    selectedLanes,
    selectedRanges: selection.ranges,
    selectedRedactions,
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
  clearSelection,
  detail,
  onCreateRedactions,
  onDeleteRedaction,
  onUnredactRanges,
  selectedLanes,
  selectedRanges,
  selectedRedactions,
  selectedSegments,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  clearSelection: () => void;
  detail: LibraryRecordingDetail;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onDeleteRedaction: (redactionId: string) => Promise<void>;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  selectedLanes: StudioLaneDraft[];
  selectedRanges: StudioTimeRangeSelection[];
  selectedRedactions: LibraryRecordingRedaction[];
  selectedSegments: StudioSidebarSegmentSelection[];
}) {
  if (selectedRanges.length > 0) {
    return (
      <StudioSidebarComponentSelection
        aiProcessingState={aiProcessingState}
        clearSelection={clearSelection}
        detail={detail}
        onCreateRedactions={onCreateRedactions}
        onUnredactRanges={onUnredactRanges}
        selections={selectedRanges}
      />
    );
  }

  if (selectedRedactions.length > 0) {
    return selectedRedactions.map((redaction) => (
      <StudioSidebarRedactionSection
        detail={detail}
        key={redaction.id}
        redaction={redaction}
        onDeleteRedaction={onDeleteRedaction}
      />
    ));
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
