import type {
  CreateLibraryRedactionRangeInput,
  LibraryRecordingDetail,
  LibraryRecordingRedaction,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { AlertTriangle, Lock } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { ScrollArea } from '@/components/ui/scroll-area';
import { RecordingInspectorSummary } from './inspector/recording-inspector-summary';
import { SelectedSourceInspector } from './inspector/selected-source-inspector';
import { VisualSourceInspector } from './inspector/visual-source-inspector';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import {
  StudioSidebarComponentSelection,
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
import type { StudioVisualFrame } from './studio-visual-observations';

export interface StudioSidebarActions {
  onClearActivitySelection: () => void;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onDeleteRedaction: (redactionId: string) => Promise<void>;
  onSeek: (timelineOffsetMs: number) => void;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
}

export interface StudioSidebarModel {
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
  selection: StudioInspectorSelection | null;
  visualFrames: StudioVisualFrame[];
}

export interface StudioSidebarProps {
  actions: StudioSidebarActions;
  model: StudioSidebarModel;
}

export function StudioSidebar({ actions, model }: StudioSidebarProps) {
  const clipsById = useStudioTimelineStore((state) => state.clipsById);
  const lanesById = useStudioTimelineStore((state) => state.lanesById);
  const timelineSelection = useStudioTimelineStore((state) => state.selection);
  const clearSelection = useStudioTimelineStore(
    (state) => state.clearSelection,
  );

  const selectedLanes = timelineSelection.laneIds
    .map((laneId) => lanesById[laneId])
    .filter((lane): lane is StudioLaneDraft => Boolean(lane));
  const selectedSegments = timelineSelection.clipIds
    .map((clipId): StudioSidebarSegmentSelection | null => {
      const clip = clipsById[clipId];
      return clip ? { ...clip, clipId } : null;
    })
    .filter((segment): segment is StudioSidebarSegmentSelection =>
      Boolean(segment),
    );
  const selectedRedactions = timelineSelection.redactionIds
    .map((redactionId) =>
      model.detail.redactions.find((redaction) => redaction.id === redactionId),
    )
    .filter((redaction): redaction is LibraryRecordingRedaction =>
      Boolean(redaction),
    );
  const selectedVisualFrames = timelineSelection.visualFrameIds
    .map((frameId) => model.visualFrames.find((frame) => frame.id === frameId))
    .filter((frame): frame is StudioVisualFrame => Boolean(frame));
  const hasTimelineSelection =
    selectedLanes.length > 0 ||
    selectedSegments.length > 0 ||
    selectedRedactions.length > 0 ||
    selectedVisualFrames.length > 0 ||
    timelineSelection.ranges.length > 0;

  const timelineContent = renderTimelineSelection({
    actions,
    aiProcessingState: model.aiProcessingState,
    clearSelection,
    detail: model.detail,
    ranges: timelineSelection.ranges,
    selectedLanes,
    selectedRedactions,
    selectedSegments,
  });

  return (
    <aside className="bg-sidebar text-sidebar-foreground flex h-full min-h-0 flex-col overflow-hidden">
      <div className="shrink-0 border-b px-2.5 py-2">
        <h2 className="text-xs font-semibold tracking-tight">Inspector</h2>
      </div>
      <ScrollArea className="min-h-0 flex-1 overflow-hidden">
        <div className="text-xs">
          {model.detail.availability !== 'available' ? (
            <Alert
              className="m-2"
              variant={
                model.detail.availability === 'locked'
                  ? 'default'
                  : 'destructive'
              }
            >
              {model.detail.availability === 'locked' ? (
                <Lock />
              ) : (
                <AlertTriangle />
              )}
              <AlertTitle>
                {model.detail.availability === 'locked'
                  ? 'Recording in progress'
                  : 'Missing video files'}
              </AlertTitle>
              <AlertDescription>
                {model.detail.missingReasons.join(' ') ||
                  'Recording files are unavailable.'}
              </AlertDescription>
            </Alert>
          ) : null}

          {model.selection ? (
            <SelectedSourceInspector
              onClearSelection={actions.onClearActivitySelection}
              onSeek={actions.onSeek}
              selection={model.selection}
            />
          ) : selectedVisualFrames.length > 0 ? (
            selectedVisualFrames.map((frame) => (
              <VisualSourceInspector
                frame={frame}
                key={frame.id}
                onClear={clearSelection}
                onSeek={actions.onSeek}
              />
            ))
          ) : (
            timelineContent
          )}

          {model.selection === null && !hasTimelineSelection ? (
            <RecordingInspectorSummary
              aiProcessingState={model.aiProcessingState}
              detail={model.detail}
            />
          ) : null}
        </div>
      </ScrollArea>
    </aside>
  );
}

function renderTimelineSelection({
  actions,
  aiProcessingState,
  clearSelection,
  detail,
  ranges,
  selectedLanes,
  selectedRedactions,
  selectedSegments,
}: {
  actions: StudioSidebarActions;
  aiProcessingState: RecordingAiProcessingState | null;
  clearSelection: () => void;
  detail: LibraryRecordingDetail;
  ranges: StudioTimeRangeSelection[];
  selectedLanes: StudioLaneDraft[];
  selectedRedactions: LibraryRecordingRedaction[];
  selectedSegments: StudioSidebarSegmentSelection[];
}) {
  if (ranges.length > 0) {
    return (
      <StudioSidebarComponentSelection
        aiProcessingState={aiProcessingState}
        clearSelection={clearSelection}
        detail={detail}
        onCreateRedactions={actions.onCreateRedactions}
        onUnredactRanges={actions.onUnredactRanges}
        selections={ranges}
      />
    );
  }

  if (selectedRedactions.length > 0) {
    return selectedRedactions.map((redaction) => (
      <StudioSidebarRedactionSection
        detail={detail}
        key={redaction.id}
        onDeleteRedaction={actions.onDeleteRedaction}
        redaction={redaction}
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

  return null;
}
