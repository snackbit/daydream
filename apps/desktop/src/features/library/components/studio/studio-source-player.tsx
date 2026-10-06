import { useEffect, useRef } from 'react';
import type {
  LibraryRecordingDetail,
  LibraryPlaybackAssets,
  LibraryPlaybackVideoAsset,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { StudioPlayer } from './studio-player';
import type { RecordingPlaybackController } from './use-recording-playback-controller';
import type { StudioVideoSources } from './use-studio-video-sources';

export function StudioSourcePlayer({
  detail,
  assets,
  sources,
  playback,
  onRefreshPlayback,
}: {
  detail: LibraryRecordingDetail;
  assets: LibraryPlaybackAssets | null;
  sources: StudioVideoSources;
  playback: RecordingPlaybackController;
  onRefreshPlayback: () => void;
}) {
  const segment = playback.state.currentSegment;
  const videos =
    assets?.segments.find((candidate) => candidate.segmentId === segment?.id)
      ?.videos ?? [];
  return (
    <div className="flex h-full min-h-0 flex-col">
      {sources.tracks.length > 1 ? (
        <div className="flex shrink-0 items-center gap-2 border-b p-2 text-xs">
          <Select
            value={sources.selectedTrackId ?? ''}
            onValueChange={(trackId) => {
              sources.select(trackId);
              playback.actions.seekTimeline(
                playback.state.displayedTimelineMs,
                { resume: playback.state.isPlaying },
              );
            }}
          >
            <SelectTrigger aria-label="Video source" className="h-7 max-w-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {sources.tracks.map((track) => (
                <SelectItem value={track.id} key={track.id}>
                  {track.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button
            size="sm"
            variant="ghost"
            aria-pressed={sources.grid}
            onClick={sources.toggleGrid}
          >
            Grid view
          </Button>
        </div>
      ) : null}
      <div
        className={
          sources.grid
            ? 'grid min-h-0 flex-1 grid-cols-1 overflow-y-auto lg:grid-cols-2'
            : 'min-h-0 flex-1'
        }
      >
        <StudioPlayer
          detail={sources.selectedDetail}
          onRefreshPlayback={onRefreshPlayback}
          playback={playback}
          playbackAssets={sources.selectedAssets}
        />
        {sources.grid && segment
          ? videos
              .filter((video) => video.trackId !== sources.selectedTrackId)
              .map((video) => (
                <SourceGridVideo
                  key={video.trackId}
                  video={video}
                  label={
                    sources.tracks.find((track) => track.id === video.trackId)
                      ?.label ?? 'Video source'
                  }
                  detail={detail}
                  playback={playback}
                  segmentOffset={segment.playbackOffsetMs}
                />
              ))
          : null}
      </div>
    </div>
  );
}

function SourceGridVideo({
  video,
  label,
  detail,
  playback,
  segmentOffset,
}: {
  video: LibraryPlaybackVideoAsset;
  label: string;
  detail: LibraryRecordingDetail;
  playback: RecordingPlaybackController;
  segmentOffset: number;
}) {
  const ref = useRef<HTMLVideoElement | null>(null);
  const anchor = useRef({ segmentId: '', seekRevision: -1, start: 0 });
  const segmentId = playback.state.currentSegment?.id ?? '';
  const revision = playback.state.seekRevision;
  const timeline = playback.state.displayedTimelineMs;
  // Media element synchronization owns the native stream clock, not React state.
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    if (
      anchor.current.segmentId !== segmentId ||
      anchor.current.seekRevision !== revision
    ) {
      anchor.current = {
        segmentId,
        seekRevision: revision,
        start: Math.max(0, timeline - segmentOffset),
      };
      const uri = new URL(video.url);
      uri.searchParams.set('startMs', String(Math.round(anchor.current.start)));
      uri.searchParams.set('request', String(revision));
      element.src = uri.toString();
    }
    const target = Math.max(
      0,
      (timeline - segmentOffset - anchor.current.start) / 1000,
    );
    if (
      Math.abs(element.currentTime - target) > 0.3 &&
      Number.isFinite(element.duration)
    )
      element.currentTime = target;
    if (playback.state.isPlaying) void element.play().catch(() => undefined);
    else element.pause();
  }, [
    video.url,
    segmentId,
    revision,
    timeline,
    segmentOffset,
    playback.state.isPlaying,
  ]);
  const masked = detail.redactions.some(
    (cut) =>
      timeline >= cut.startOffsetMs &&
      timeline < cut.endOffsetMs &&
      (cut.trackIds.length === 0 || cut.trackIds.includes(video.trackId)),
  );
  return (
    <figure className="relative flex min-h-40 flex-col overflow-hidden border-l bg-black">
      <figcaption className="bg-background shrink-0 border-b px-2 py-1 text-xs">
        {label}
      </figcaption>
      <video
        aria-label={`${label} synchronized video`}
        muted
        playsInline
        preload="metadata"
        ref={ref}
        className="min-h-0 w-full flex-1 object-contain"
      />
      {masked ? (
        <div
          aria-label={`${label} redacted`}
          className="absolute inset-0 flex items-center justify-center bg-black text-xs text-white"
        >
          Redacted
        </div>
      ) : null}
    </figure>
  );
}
