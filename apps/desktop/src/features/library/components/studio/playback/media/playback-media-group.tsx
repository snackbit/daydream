import { useCallback, type ReactNode, type SyntheticEvent } from 'react';
import type {
  LibraryPlaybackSegment,
  LibraryRecordingDetail,
} from '@daydream/shared';
import type { RecordingPlaybackController } from '../use-recording-playback-controller';
import { PlaybackAudio } from './playback-audio';
import { PlaybackSourceVideo } from './playback-source-video';

export function playbackStreamUrl(
  url: string,
  startMs: number,
  revision: number,
  durationMs?: number,
): string {
  return `${url}${url.includes('?') ? '&' : '?'}startMs=${Math.max(0, Math.floor(startMs))}&request=${Math.max(0, Math.floor(revision))}${durationMs === undefined ? '' : `&durationMs=${Math.max(1, Math.floor(durationMs))}`}`;
}
export interface PlaybackMediaGroupProps {
  active: boolean;
  asset: LibraryPlaybackSegment;
  startMs: number;
  revision: number;
  playback: RecordingPlaybackController;
  label?: string | undefined;
  children?: ReactNode;
  grid?: boolean;
  detail: LibraryRecordingDetail;
  labels?: ReadonlyMap<string, string> | undefined;
  timelineOffsetMs: number;
  streamOffsetMs: number;
}
/** The prepared group's DOM and buffers survive its promotion to active media. */
export function PlaybackMediaGroup({
  active,
  asset,
  startMs,
  revision,
  playback,
  label,
  children,
  grid = false,
  detail,
  labels,
  timelineOffsetMs,
  streamOffsetMs,
}: PlaybackMediaGroupProps) {
  const { actions } = playback;
  const slice = detail.segments.find(
    (segment) => segment.id === asset.segmentId,
  );
  const durationMs =
    slice?.mediaStartMs === undefined
      ? undefined
      : Math.max(1, (slice.durationMs ?? 0) - streamOffsetMs);
  const { setVideoElement, handleVideoTimeUpdate } = actions;
  const attachVideo = useCallback(
    (element: HTMLVideoElement | null) => {
      if (active) setVideoElement(element);
    },
    [active, setVideoElement],
  );
  const update = useCallback(
    (event: SyntheticEvent<HTMLVideoElement>) => {
      if (active) handleVideoTimeUpdate(event.currentTarget);
    },
    [active, handleVideoTimeUpdate],
  );
  return (
    <div
      hidden={!active}
      data-grid={grid}
      className="h-full min-h-0 bg-black data-[grid=true]:grid data-[grid=true]:auto-rows-fr data-[grid=true]:grid-cols-1 data-[grid=true]:overflow-auto lg:data-[grid=true]:grid-cols-2 [&[hidden]]:hidden"
    >
      <div className="relative flex h-full min-h-0 flex-col">
        {label && active ? (
          <div className="bg-background text-foreground shrink-0 border-b px-2 py-1 text-xs">
            {label}
          </div>
        ) : null}
        <video
          ref={attachVideo}
          aria-label={
            active ? (label ? `${label} video` : 'Recording video') : undefined
          }
          className="min-h-0 w-full flex-1 object-contain"
          preload="auto"
          muted={!active}
          src={playbackStreamUrl(asset.videoUrl, startMs, revision, durationMs)}
          onPause={active ? actions.handleVideoPause : undefined}
          onPlay={active ? actions.handleVideoPlay : undefined}
          onTimeUpdate={update}
          onCanPlay={active ? actions.handleVideoCanPlay : undefined}
          onSeeked={active ? actions.handleVideoCanPlay : undefined}
          onWaiting={active ? actions.handleBuffering : undefined}
          onStalled={active ? actions.handleBuffering : undefined}
          onEnded={active ? actions.handleEnded : undefined}
          onError={active ? actions.handleMediaError : undefined}
        />
        {asset.audio.map((audio) => (
          <PlaybackAudio
            key={audio.trackId}
            active={active}
            audio={audio}
            actions={actions}
            src={playbackStreamUrl(audio.url, startMs, revision, durationMs)}
          />
        ))}
        {active ? children : null}
      </div>
      {grid
        ? asset.videos
            ?.filter((video) => video.url !== asset.videoUrl)
            .map((video) => (
              <PlaybackSourceVideo
                key={video.trackId}
                active={active}
                video={video}
                label={labels?.get(video.trackId) ?? 'Monitor'}
                redactions={detail.redactions}
                playback={playback}
                startMs={startMs}
                revision={revision}
                timelineOffsetMs={timelineOffsetMs}
                streamOffsetMs={streamOffsetMs}
                durationMs={durationMs}
              />
            ))
        : null}
    </div>
  );
}
