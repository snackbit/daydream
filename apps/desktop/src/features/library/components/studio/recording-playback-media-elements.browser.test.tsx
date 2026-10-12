import { cleanup, renderHook } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { usePlaybackMediaElements } from './recording-playback-media-elements';

afterEach(cleanup);
test('desktop audio owns the clock across rollover times and ordinary drift never seeks audio', () => {
  const media = <T extends 'audio' | 'video'>(tag: T) => {
    const element = document.createElement(tag);
    let time = 0;
    const seek = vi.fn((seconds: number) => {
      time = seconds;
    });
    Object.defineProperties(element, {
      readyState: { get: () => HTMLMediaElement.HAVE_ENOUGH_DATA },
      currentTime: { get: () => time, set: seek },
    });
    return {
      element,
      seek,
      advance: (seconds: number) => {
        time = seconds;
      },
    };
  };
  const desktop = media('audio'),
    microphone = media('audio'),
    video = media('video');
  const protectedTracks = { current: new Set<string>() };
  const view = renderHook(() =>
    usePlaybackMediaElements({
      isMuted: false,
      mutedTrackIds: new Set<string>(),
      redactedTrackIdsRef: protectedTracks,
      volume: 1,
      playbackRate: 2,
    }),
  );
  view.result.current.refs.audioRefs.current.set(
    'desktop-audio-1',
    desktop.element,
  );
  view.result.current.refs.audioRefs.current.set(
    'microphone-2',
    microphone.element,
  );
  view.result.current.refs.videoRef.current = video.element;
  view.result.current.actions.syncAudioElements(0, true);
  for (const item of [desktop, microphone, video]) item.seek.mockClear();
  for (let seconds = 0.5; seconds <= 65; seconds += 0.5) {
    desktop.advance(seconds);
    microphone.advance(seconds - 0.1);
    video.advance(seconds + 0.12);
    view.result.current.actions.syncAudioElements(video.element.currentTime);
    expect(view.result.current.actions.clockTime()).toBe(seconds);
  }
  for (const item of [desktop, microphone, video])
    expect(item.seek).not.toHaveBeenCalled();
  expect(desktop.element.playbackRate).toBe(2);
  expect(microphone.element.playbackRate).toBeGreaterThan(2);
  expect(video.element.playbackRate).toBeLessThan(2);
  expect(video.element.defaultPlaybackRate).toBe(2);
  protectedTracks.current.add('desktop-audio-1');
  view.result.current.actions.applyRedactedAudioSettings(
    protectedTracks.current,
  );
  expect(desktop.element.muted).toBe(true);
  expect(microphone.element.muted).toBe(false);
});

test('new media and monitor followers inherit the selected source-load speed and pause together', () => {
  const view = renderHook(
    ({ rate }) =>
      usePlaybackMediaElements({
        isMuted: false,
        mutedTrackIds: new Set<string>(),
        redactedTrackIdsRef: { current: new Set<string>() },
        volume: 1,
        playbackRate: rate,
      }),
    { initialProps: { rate: 1 as 1 | 2 } },
  );
  const video = document.createElement('video'),
    follower = document.createElement('video'),
    audio = document.createElement('audio');
  view.result.current.actions.setVideoElement(video);
  view.result.current.actions.registerVideoElement('monitor-2', follower);
  view.result.current.actions.registerAudioElement('desktop-audio-1', audio);
  view.rerender({ rate: 2 });
  for (const media of [video, follower, audio]) {
    expect(media.playbackRate).toBe(2);
    expect(media.defaultPlaybackRate).toBe(2);
  }
  const replacement = document.createElement('video');
  view.result.current.actions.setVideoElement(replacement);
  expect(replacement.defaultPlaybackRate).toBe(2);
  const pauseVideo = vi
      .spyOn(replacement, 'pause')
      .mockImplementation(() => undefined),
    pauseFollower = vi
      .spyOn(follower, 'pause')
      .mockImplementation(() => undefined),
    pauseAudio = vi.spyOn(audio, 'pause').mockImplementation(() => undefined);
  view.result.current.actions.pauseMediaElements();
  expect(pauseVideo).toHaveBeenCalledOnce();
  expect(pauseFollower).toHaveBeenCalledOnce();
  expect(pauseAudio).toHaveBeenCalledOnce();
});
