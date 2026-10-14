import { useCallback } from 'react';

export interface PlaybackTimelineClockInput {
  playbackOffsetMs: number | null;
  currentTimeSeconds: number;
  streamStartMs: number;
  scrubTimelineMs: number | null;
  totalDurationMs: number;
  isPlaying: boolean;
  pendingAutoplay: boolean;
  clockTime: () => number;
}

export function usePlaybackTimelineClock({
  playbackOffsetMs,
  currentTimeSeconds,
  streamStartMs,
  scrubTimelineMs,
  totalDurationMs,
  isPlaying,
  pendingAutoplay,
  clockTime,
}: PlaybackTimelineClockInput) {
  const currentTimelineMs =
    playbackOffsetMs !== null
      ? playbackOffsetMs + currentTimeSeconds * 1000
      : 0;
  const displayedTimelineMs = scrubTimelineMs ?? currentTimelineMs;
  const readPlaybackTimeMs = useCallback(() => {
    if (
      !isPlaying ||
      pendingAutoplay ||
      scrubTimelineMs !== null ||
      playbackOffsetMs === null
    )
      return displayedTimelineMs;
    return Math.max(
      0,
      Math.min(
        totalDurationMs,
        playbackOffsetMs + streamStartMs + clockTime() * 1000,
      ),
    );
  }, [
    isPlaying,
    pendingAutoplay,
    scrubTimelineMs,
    playbackOffsetMs,
    displayedTimelineMs,
    totalDurationMs,
    streamStartMs,
    clockTime,
  ]);
  return { currentTimelineMs, displayedTimelineMs, readPlaybackTimeMs };
}
