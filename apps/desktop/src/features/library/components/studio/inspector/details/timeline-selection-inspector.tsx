import type {
  LibraryRecordingDetail,
  LibraryRecordingRedaction,
  RecordingAiProcessingState,
} from '@daydream/shared';
import type {
  StudioLaneDraft,
  StudioTimeRangeSelection,
} from '../../timeline/state/studio-timeline-store';
import type { buildInspectorContent } from '../content/inspector-content';
import { StudioSidebarComponentSelection } from '../editing/studio-sidebar-component-selection';
import { StudioSidebarRedactionSection } from '../editing/studio-sidebar-redaction-section';
import { StudioSidebarSectionLane } from '../editing/studio-sidebar-section-lane';
import {
  StudioSidebarSectionSegment,
  type StudioSidebarSegmentSelection,
} from '../editing/studio-sidebar-section-segment';
import type { StudioSidebarActions } from '../studio-sidebar';

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
