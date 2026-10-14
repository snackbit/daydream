import { useCallback, useMemo } from 'react';
import type {
  LibraryRecordingDetail,
  LibraryPlaybackAssets,
} from '@daydream/shared';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { StudioPlayer } from '../controls/studio-player';
import type { RecordingPlaybackController } from '../use-recording-playback-controller';
import type { StudioVideoSources } from './use-studio-video-sources';

export interface StudioSourcePlayerProps {
  detail: LibraryRecordingDetail;
  assets: LibraryPlaybackAssets | null;
  sources: StudioVideoSources;
  playback: RecordingPlaybackController;
  onRefreshPlayback: () => void;
}
export function StudioSourcePlayer({
  detail,
  sources,
  playback,
  onRefreshPlayback,
}: StudioSourcePlayerProps) {
  const segment = playback.state.currentSegment;
  const primary = segment?.videoTrackId ?? sources.selectedTrackId;
  const labels = useMemo(
    () =>
      new Map(
        detail.tracks
          .filter((t) => t.kind === 'video')
          .map((t) => [t.id, t.label]),
      ),
    [detail.tracks],
  );
  const available = (trackId: string) =>
    !segment?.videoStreams?.length ||
    segment.videoStreams.some((s) => s.trackId === trackId);
  const visibleTracks = sources.tracks.filter(
    (t) => available(t.id) || t.id === sources.selectedTrackId,
  );
  const seek = playback.actions.seekTimeline,
    time = playback.state.displayedTimelineMs,
    resume = playback.state.playRequested;
  const choose = useCallback(
    (id: string) => {
      if (id === 'all') {
        if (!sources.grid) sources.toggleGrid();
      } else sources.select(id);
      seek(time, { resume });
    },
    [sources, seek, time, resume],
  );
  return (
    <div
      role="region"
      aria-label="Recording player"
      className="rounded-workspace bg-background flex h-full min-h-0 flex-col overflow-hidden border"
    >
      {sources.tracks.length > 1 ? (
        <div className="bg-muted flex shrink-0 flex-wrap items-center gap-3 px-5 py-3 text-[13px]">
          <Select
            value={sources.grid ? 'all' : (primary ?? '')}
            onValueChange={choose}
          >
            <SelectTrigger aria-label="Video source" className="max-w-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All monitors</SelectItem>
              {visibleTracks.map((t) => (
                <SelectItem key={t.id} value={t.id} disabled={!available(t.id)}>
                  {t.label}
                  {!available(t.id) ? ' (not recorded)' : null}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      ) : null}
      <div className="min-h-0 flex-1">
        <StudioPlayer
          detail={sources.selectedDetail}
          onRefreshPlayback={onRefreshPlayback}
          playback={playback}
          playbackAssets={sources.selectedAssets}
          grid={sources.grid}
          sourceLabels={labels}
          sourceLabel={sources.grid ? labels.get(primary ?? '') : undefined}
        />
      </div>
    </div>
  );
}
