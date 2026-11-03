import {
  useCallback,
  useLayoutEffect,
  useRef,
  type ReactNode,
  type SyntheticEvent,
} from 'react';
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
  seekable?: boolean;
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
  seekable = false,
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
  const group = useRef<HTMLDivElement | null>(null);
  useLayoutEffect(() => {
    const element = group.current;
    return () => {
      // WebKit can retain detached pipelines until GC. Retire them with the group.
      for (const media of element?.querySelectorAll('video,audio') ?? []) {
        if (!(media instanceof HTMLMediaElement)) continue;
        // Transport owns pause; load() cancels and releases the retired source.
        media.removeAttribute('src');
        media.load();
      }
    };
  }, []);
  const prepareElement = useCallback(
    (element: HTMLMediaElement) => {
      if (!seekable) return;
      const target = Math.max(0, startMs / 1000);
      if (Math.abs(element.currentTime - target) > 0.01)
        element.currentTime = target;
    },
    [seekable, startMs],
  );
  const loadedMetadata = useCallback(
    (event: SyntheticEvent<HTMLDivElement>) => {
      if (event.target instanceof HTMLMediaElement)
        prepareElement(event.target);
    },
    [prepareElement],
  );
  useLayoutEffect(() => {
    if (!seekable) return;
    for (const element of group.current?.querySelectorAll('video,audio') ?? [])
      if (element instanceof HTMLMediaElement && element.readyState > 0)
        prepareElement(element);
  }, [seekable, startMs, revision, prepareElement]);
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
      ref={group}
      onLoadedMetadataCapture={loadedMetadata}
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
          src={
            seekable
              ? asset.videoUrl
              : playbackStreamUrl(asset.videoUrl, startMs, revision, durationMs)
          }
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
            preload={active && !playback.state.playRequested ? 'none' : 'auto'}
            actions={actions}
            src={
              seekable
                ? audio.url
                : playbackStreamUrl(audio.url, startMs, revision, durationMs)
            }
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
                seekable={seekable}
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
