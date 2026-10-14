import { cleanup, renderHook } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import {
  usePlaybackTimelineClock,
  type PlaybackTimelineClockInput,
} from './use-playback-timeline-clock';

afterEach(cleanup);
test('live clock reads use the stream and run origins while paused, loading and scrub positions stay authoritative', () => {
  let seconds = 2;
  const clockTime = vi.fn(() => seconds);
  const input: PlaybackTimelineClockInput = {
    playbackOffsetMs: 7000,
    currentTimeSeconds: 3,
    streamStartMs: 1000,
    scrubTimelineMs: null,
    totalDurationMs: 20000,
    isPlaying: true,
    pendingAutoplay: false,
    clockTime,
  };
  const view = renderHook(
    (props: PlaybackTimelineClockInput) => usePlaybackTimelineClock(props),
    { initialProps: input },
  );
  expect(view.result.current.readPlaybackTimeMs()).toBe(10000);
  seconds = 2.15;
  expect(view.result.current.readPlaybackTimeMs()).toBe(10150);
  for (const change of [{ isPlaying: false }, { pendingAutoplay: true }]) {
    view.rerender({ ...input, ...change });
    clockTime.mockClear();
    expect(view.result.current.readPlaybackTimeMs()).toBe(10000);
    expect(clockTime).not.toHaveBeenCalled();
  }
  view.rerender({ ...input, scrubTimelineMs: 12345 });
  expect(view.result.current.readPlaybackTimeMs()).toBe(12345);
  view.rerender({ ...input, playbackOffsetMs: 15000, streamStartMs: 1000 });
  expect(view.result.current.readPlaybackTimeMs()).toBe(18150);
  seconds = 30;
  expect(view.result.current.readPlaybackTimeMs()).toBe(20000);
});
