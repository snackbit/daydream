import type {
  LibraryRecordingDetail,
  LibraryRecordingRedaction,
  RecordingAiProcessingState,
} from '@daydream/shared';
import type { StudioSidebarActions } from '../studio-sidebar';
import {
  StudioSidebarComponentSelection,
  StudioSidebarSectionLane,
  StudioSidebarSectionSegment,
  type StudioSidebarSegmentSelection,
} from '../studio-sidebar-inspector-sections';
import { StudioSidebarRedactionSection } from '../studio-sidebar-redaction-section';
import type {
  StudioLaneDraft,
  StudioTimeRangeSelection,
} from '../studio-timeline-store';
import type { buildInspectorContent } from './content/inspector-content';

export interface TimelineSelectionInspectorProps {
  actions: StudioSidebarActions;
  aiProcessingState: RecordingAiProcessingState | null;
  clearSelection: () => void;
  detail: LibraryRecordingDetail;
  ranges: StudioTimeRangeSelection[];
  selectedLanes: StudioLaneDraft[];
  selectedRedactions: LibraryRecordingRedaction[];
  selectedSegments: StudioSidebarSegmentSelection[];
  debugEnabled: boolean;
  content: ReturnType<typeof buildInspectorContent>;
}

export function TimelineSelectionInspector({
  actions,
  aiProcessingState,
  clearSelection,
  detail,
  ranges,
  selectedLanes,
  selectedRedactions,
  selectedSegments,
  debugEnabled,
  content,
}: TimelineSelectionInspectorProps) {
  if (ranges.length > 0) {
    return (
      <StudioSidebarComponentSelection
        aiProcessingState={aiProcessingState}
        clearSelection={clearSelection}
        detail={detail}
        onCreateRedactions={actions.onCreateRedactions}
        onUnredactRanges={actions.onUnredactRanges}
        selections={ranges}
        content={content}
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
        debugEnabled={debugEnabled}
      />
    ));
  }

  if (selectedLanes.length > 0) {
    return selectedLanes.map((lane) => (
      <StudioSidebarSectionLane
        aiProcessingState={aiProcessingState}
        key={lane.trackId}
        lane={lane}
        debugEnabled={debugEnabled}
      />
    ));
  }

  return null;
}
