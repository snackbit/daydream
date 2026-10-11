import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { studioStoryRecording } from '../../recording-studio-route/stories/studio-route-fixture';
import { usePlaybackRedactionController } from './recording-playback-redaction-controller';
import { usePlaybackResumeTimer } from './recording-playback-resume-timer';

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

test.each([0.25, 2])(
  'redaction boundary protection follows %sx playback without waiting for timeupdate',
  (rate) => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    vi.spyOn(window, 'requestAnimationFrame').mockReturnValue(1);
    vi.spyOn(window, 'cancelAnimationFrame').mockImplementation(
      () => undefined,
    );
    const video = document.createElement('video'),
      audio = document.createElement('audio');
    video.playbackRate = rate;
    audio.playbackRate = rate;
    let time = 0;
    Object.defineProperty(video, 'currentTime', { get: () => time });
    Object.defineProperty(audio, 'currentTime', { get: () => time });
    const protectedTracks = { current: new Set<string>() };
    const apply = vi.fn((ids: Set<string>) => {
      audio.muted = ids.has('desktop-audio-1');
    });
    const view = renderHook(() =>
      usePlaybackRedactionController({
        applyRedactedAudioSettings: apply,
        clockTime: () => time,
        audioRefs: { current: new Map([['desktop-audio-1', audio]]) },
        currentSegment: { playbackOffsetMs: 0 },
        currentTimelineMs: 0,
        detail: {
          ...studioStoryRecording,
          tracks: [
            {
              id: 'video-0',
              kind: 'video',
              codec: 'h264',
              label: 'Screen',
              description: '',
              streamIndex: 0,
            },
          ],
          redactions: [
            {
              id: 'synthetic-mask',
              recordingId: studioStoryRecording.id,
              createdAtMs: 0,
              startOffsetMs: 1000,
              endOffsetMs: 2000,
              reason: null,
              trackIds: ['video-0', 'desktop-audio-1'],
            },
          ],
        },
        isPlaying: true,
        redactedTrackIdsRef: protectedTracks,
        streamStartMs: 0,
        totalDurationMs: 10000,
        videoRef: { current: video },
        playbackRate: rate,
      }),
    );
    expect(audio.muted).toBe(false);
    expect(view.result.current.redactedVideoActive).toBe(false);
    time = 0.925;
    act(() => vi.advanceTimersByTime(Math.ceil(925 / rate)));
    expect(audio.muted).toBe(true);
    expect(view.result.current.redactedVideoActive).toBe(true);
    view.unmount();
  },
);

test('recorded pause gaps follow speed changes, complete once and cancel on replacement or unmount', () => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval', 'performance'] });
  const progress = vi.fn(),
    complete = vi.fn();
  const view = renderHook(
    ({ rate }) =>
      usePlaybackResumeTimer({
        playbackRate: rate,
        setResumeProgress: progress,
      }),
    { initialProps: { rate: 0.25 } },
  );
  act(() => view.result.current.startResumeTimer(1000, complete));
  act(() => vi.advanceTimersByTime(2000));
  expect(progress).toHaveBeenLastCalledWith(0.5);
  view.rerender({ rate: 2 });
  act(() => vi.advanceTimersByTime(200));
  expect(complete).not.toHaveBeenCalled();
  act(() => vi.advanceTimersByTime(50));
  expect(complete).toHaveBeenCalledTimes(1);
  expect(progress).toHaveBeenLastCalledWith(1);
  act(() => view.result.current.startResumeTimer(1000, complete));
  act(() => view.result.current.clearResumeTimer());
  act(() => vi.advanceTimersByTime(2000));
  expect(complete).toHaveBeenCalledTimes(1);
  act(() => view.result.current.startResumeTimer(1000, complete));
  view.unmount();
  vi.advanceTimersByTime(2000);
  expect(complete).toHaveBeenCalledTimes(1);
});
