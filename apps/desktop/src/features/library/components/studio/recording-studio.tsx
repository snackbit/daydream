import { useEffect, useState } from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
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
  detail: LibraryRecordingDetail;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
}

export function RecordingStudio({
  detail,
  playbackAssets,
  thumbnailStatus,
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
              detail={detail}
              mutedTrackIds={mutedTrackIds}
              playback={playback}
              playbackAssets={playbackAssets}
              thumbnailStatus={thumbnailStatus}
              onToggleTrackMuted={toggleTrackMuted}
            />
          </ResizablePanel>
        </ResizablePanelGroup>
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel defaultSize={26} minSize={18}>
        <StudioSidebar detail={detail} />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
