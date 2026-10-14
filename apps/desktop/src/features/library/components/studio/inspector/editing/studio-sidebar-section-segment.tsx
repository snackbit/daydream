import type { RecordingAiProcessingState } from '@daydream/shared';
import { Input } from '@/components/ui/input';
import {
  useStudioTimelineStore,
  type StudioClipDraft,
} from '../../timeline/state/studio-timeline-store';
import { MetadataRow, StudioSidebarSection } from './studio-sidebar-section';
import {
  TranscriptionChunkDetails,
  TranscriptionSelectionDetails,
  transcriptionTrackForLane,
} from './studio-sidebar-transcription';

export interface StudioSidebarSectionSegmentProps {
  aiProcessingState: RecordingAiProcessingState | null;
  segment: StudioSidebarSegmentSelection;
  debugEnabled?: boolean;
}

export function StudioSidebarSectionSegment({
  aiProcessingState,
  segment,
  debugEnabled = false,
}: StudioSidebarSectionSegmentProps) {
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

export interface StudioSidebarSegmentSelection extends StudioClipDraft {
  clipId: string;
}
