import { useEffect, useState } from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  RecordingAiProcessingState,
  RecordingVisualObservations,
} from '@daydream/shared';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { StudioPlayer } from './studio-player';
import { StudioSidebar } from './studio-sidebar';
import { useStudioTimelineStore } from './studio-timeline-store';
import { TrackTimeline } from './track-timeline';
import { useRecordingPlaybackController } from './use-recording-playback-controller';

export interface RecordingStudioProps {
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onDeleteRedaction: (redactionId: string) => Promise<void>;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  visualObservations: RecordingVisualObservations | null;
}

export function RecordingStudio({
  aiProcessingState,
  detail,
  onCreateRedactions,
  onDeleteRedaction,
  onUnredactRanges,
  playbackAssets,
  thumbnailStatus,
  visualObservations,
}: RecordingStudioProps) {
  const [mutedTrackIds, setMutedTrackIds] = useState<Set<string>>(
    () => new Set(),
  );
  const initializeTimeline = useStudioTimelineStore(
    (state) => state.initializeFromDetail,
  );
  const playback = useRecordingPlaybackController({
    detail,
    mutedTrackIds,
    playbackAssets,
  });

  useEffect(() => {
    initializeTimeline(detail);
  }, [detail, initializeTimeline]);

  function toggleTrackMuted(trackId: string) {
    setMutedTrackIds((current) => {
      const next = new Set(current);
      if (next.has(trackId)) {
        next.delete(trackId);
      } else {
        next.add(trackId);
      }

      return next;
    });
  }

  return (
    <ResizablePanelGroup className="min-h-0 flex-1" orientation="horizontal">
      <ResizablePanel defaultSize={74} minSize={50}>
        <ResizablePanelGroup orientation="vertical">
          <ResizablePanel defaultSize={68} minSize={45}>
            <StudioPlayer
              detail={detail}
              playback={playback}
              playbackAssets={playbackAssets}
            />
          </ResizablePanel>
          <ResizableHandle withHandle />
          <ResizablePanel defaultSize={32} minSize={18}>
            <TrackTimeline
              aiProcessingState={aiProcessingState}
              detail={detail}
              mutedTrackIds={mutedTrackIds}
              playback={playback}
              playbackAssets={playbackAssets}
              thumbnailStatus={thumbnailStatus}
              visualObservations={visualObservations}
              onCreateRedactions={onCreateRedactions}
              onUnredactRanges={onUnredactRanges}
              onToggleTrackMuted={toggleTrackMuted}
            />
          </ResizablePanel>
        </ResizablePanelGroup>
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel defaultSize={26} minSize={18}>
        <StudioSidebar
          aiProcessingState={aiProcessingState}
          detail={detail}
          visualObservations={visualObservations}
          onCreateRedactions={onCreateRedactions}
          onDeleteRedaction={onDeleteRedaction}
          onUnredactRanges={onUnredactRanges}
        />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
