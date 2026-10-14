import type { RecordingAiProcessingState } from '@daydream/shared';
import {
  useStudioTimelineStore,
  type StudioLaneDraft,
} from '../../timeline/state/studio-timeline-store';
import { EditableLaneFields, trackKindLabel } from './studio-sidebar-editing';
import { MetadataRow, StudioSidebarSection } from './studio-sidebar-section';
import {
  TranscriptionSelectionDetails,
  TranscriptionTrackDetails,
  transcriptionTrackForLane,
} from './studio-sidebar-transcription';

export interface StudioSidebarSectionLaneProps {
  aiProcessingState: RecordingAiProcessingState | null;
  lane: StudioLaneDraft;
  debugEnabled?: boolean;
}

export function StudioSidebarSectionLane({
  aiProcessingState,
  lane,
  debugEnabled = false,
}: StudioSidebarSectionLaneProps) {
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
