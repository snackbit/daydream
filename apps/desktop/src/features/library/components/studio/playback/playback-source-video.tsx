import { useCallback, useEffect, useRef } from 'react';
import type {
  LibraryPlaybackVideoAsset,
  LibraryRecordingRedaction,
} from '@daydream/shared';
import { REDACTION_START_GUARD_MS } from '../recording-playback-redactions';
import type { RecordingPlaybackController } from '../use-recording-playback-controller';
import { playbackStreamUrl } from './playback-media-group';

export interface PlaybackSourceVideoProps {
  active: boolean;
  video: LibraryPlaybackVideoAsset;
  label: string;
  redactions: LibraryRecordingRedaction[];
  playback: RecordingPlaybackController;
  startMs: number;
  revision: number;
  timelineOffsetMs: number;
  streamOffsetMs: number;
  durationMs?: number | undefined;
}
export function PlaybackSourceVideo({
  active,
  video,
  label,
  redactions,
  playback,
  startMs,
  revision,
  timelineOffsetMs,
  streamOffsetMs,
  durationMs,
}: PlaybackSourceVideoProps) {
  const ref = useRef<HTMLVideoElement | null>(null),
    maskRef = useRef<HTMLDivElement | null>(null);
  const { registerVideoElement } = playback.actions;
  const attach = useCallback(
    (element: HTMLVideoElement | null) => {
      ref.current = element;
      if (active) registerVideoElement(video.trackId, element);
    },
    [active, registerVideoElement, video.trackId],
  );
  const timeline = playback.state.displayedTimelineMs;
  function hiddenAt(time: number) {
    return redactions.some(
      (cut) =>
        (cut.trackIds.length === 0 || cut.trackIds.includes(video.trackId)) &&
        time >= cut.startOffsetMs - REDACTION_START_GUARD_MS &&
        time < cut.endOffsetMs,
    );
  }
  const masked = hiddenAt(timeline);
  useEffect(() => {
    if (!active) return;
    let scheduled = 0;
    function protect() {
      const mask = maskRef.current;
      if (!mask) return;
      const actual =
        timelineOffsetMs +
        streamOffsetMs +
        (ref.current?.currentTime ?? 0) * 1000;
      mask.hidden = !redactions.some(
        (cut) =>
          (cut.trackIds.length === 0 || cut.trackIds.includes(video.trackId)) &&
          [timeline, actual].some(
            (time) =>
              time >= cut.startOffsetMs - REDACTION_START_GUARD_MS &&
              time < cut.endOffsetMs,
          ),
      );
      if (playback.state.isPlaying) scheduled = requestAnimationFrame(protect);
    }
    protect();
    return () => cancelAnimationFrame(scheduled);
  }, [
    active,
    redactions,
    video.trackId,
    timeline,
    timelineOffsetMs,
    streamOffsetMs,
    playback.state.isPlaying,
  ]);
  return (
    <figure className="relative flex h-full min-h-0 min-w-0 flex-col overflow-hidden border-l bg-black">
      <figcaption className="bg-background text-foreground shrink-0 border-b px-2 py-1 text-xs">
        {label}
      </figcaption>
      <video
        ref={attach}
        aria-label={active ? `${label} synchronized video` : undefined}
        muted
        playsInline
        preload="auto"
        src={playbackStreamUrl(video.url, startMs, revision, durationMs)}
        className="min-h-0 w-full flex-1 object-contain"
        onCanPlay={active ? playback.actions.handleVideoCanPlay : undefined}
        onWaiting={active ? playback.actions.handleBuffering : undefined}
        onStalled={active ? playback.actions.handleBuffering : undefined}
        onError={active ? playback.actions.handleMediaError : undefined}
      />
      <div
        ref={maskRef}
        hidden={!masked}
        aria-label={active ? `${label} redacted` : undefined}
        className="absolute inset-0 flex items-center justify-center bg-black text-xs text-white [&[hidden]]:hidden"
      >
        Redacted
      </div>
    </figure>
  );
}
