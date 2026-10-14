import { useCallback, useEffect, useRef, type RefObject } from 'react';
import { driftCorrection } from '../clock/playback-clock';
import type { PlaybackSpeed } from '../controls/playback-speed';

interface PlaybackMediaElementsInput {
  canSeekInStream?: boolean;
  isMuted: boolean;
  mutedTrackIds: Set<string>;
  redactedTrackIdsRef: RefObject<Set<string>>;
  volume: number;
  playbackRate?: PlaybackSpeed;
}

export function usePlaybackMediaElements({
  isMuted,
  mutedTrackIds,
  redactedTrackIdsRef,
  volume,
  playbackRate = 1,
  canSeekInStream = true,
}: PlaybackMediaElementsInput) {
  const audioRefs = useRef(new Map<string, HTMLAudioElement>());
  const additionalVideoRefs = useRef(new Map<string, HTMLVideoElement>());
  const fullscreenElementRef = useRef<HTMLElement | null>(null);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const majorDrift = useRef(new Map<HTMLMediaElement, number | null>());
  const clockElement = useCallback((): HTMLMediaElement | null => {
    const available = [...audioRefs.current.entries()].filter(
      ([, audio]) =>
        audio.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA &&
        audio.error === null &&
        (!audio.ended || !audio.paused),
    );
    const preferred = (entries: typeof available) =>
      entries.find(([id]) => id.startsWith('desktop-audio'))?.[1] ??
      entries.find(([id]) => id.startsWith('microphone'))?.[1];
    const playing = preferred(
      available.filter(([, element]) => !element.paused),
    );
    if (playing) return playing;
    const video = videoRef.current;
    if (
      video &&
      !video.paused &&
      video.error === null &&
      video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA
    )
      return video;
    return preferred(available) ?? video;
  }, []);
  const clockTime = useCallback(
    () => clockElement()?.currentTime ?? 0,
    [clockElement],
  );

  const applyMediaSettings = useCallback(
    (
      media: HTMLMediaElement,
      trackId: string | undefined,
      activeRedactedTrackIds: Set<string>,
    ) => {
      media.volume = volume;
      media.muted =
        isMuted ||
        (trackId
          ? mutedTrackIds.has(trackId) || activeRedactedTrackIds.has(trackId)
          : false);
    },
    [isMuted, mutedTrackIds, volume],
  );

  const updateMediaSettings = useCallback(
    (media: HTMLMediaElement, trackId?: string) => {
      applyMediaSettings(media, trackId, redactedTrackIdsRef.current);
    },
    [applyMediaSettings, redactedTrackIdsRef],
  );

  const applyRedactedAudioSettings = useCallback(
    (activeRedactedTrackIds: Set<string>) => {
      for (const [trackId, audio] of audioRefs.current) {
        applyMediaSettings(audio, trackId, activeRedactedTrackIds);
      }
    },
    [applyMediaSettings],
  );

  const syncAudioElements = useCallback(
    (currentTimeSeconds: number, force = false) => {
      const master = clockElement();
      const time = force
        ? currentTimeSeconds
        : (master?.currentTime ?? currentTimeSeconds);
      for (const [trackId, audio] of audioRefs.current) {
        updateMediaSettings(audio, trackId);
      }
      for (const element of [
        ...audioRefs.current.values(),
        ...(videoRef.current ? [videoRef.current] : []),
        ...additionalVideoRefs.current.values(),
      ]) {
        element.defaultPlaybackRate = playbackRate;
        if (force) {
          if (canSeekInStream) setMediaCurrentTime(element, time);
          element.playbackRate = playbackRate;
          majorDrift.current.delete(element);
        } else if (element === master) {
          element.playbackRate = playbackRate;
          majorDrift.current.delete(element);
        } else if (
          element.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA &&
          canSeekInStream &&
          element.seekable.length > 0
        ) {
          const correction = driftCorrection(
            time - element.currentTime,
            performance.now(),
            majorDrift.current.get(element) ?? null,
          );
          majorDrift.current.set(element, correction.majorSinceMs);
          element.playbackRate = playbackRate * correction.rate;
          if (correction.seek) setMediaCurrentTime(element, time);
        } else {
          // Live fragmented streams can expose no seekable range. GStreamer's
          // rate correction requires a seek and can stall such a stream.
          if (element.playbackRate !== playbackRate)
            element.playbackRate = playbackRate;
          majorDrift.current.delete(element);
        }
      }
    },
    [clockElement, updateMediaSettings, playbackRate, canSeekInStream],
  );

  const pauseMediaElements = useCallback(() => {
    videoRef.current?.pause();
    for (const audio of audioRefs.current.values()) {
      audio.pause();
    }
    for (const video of additionalVideoRefs.current.values()) video.pause();
  }, []);

  const registerAudioElement = useCallback(
    (trackId: string, element: HTMLAudioElement | null) => {
      if (element) {
        audioRefs.current.set(trackId, element);
        element.defaultPlaybackRate = playbackRate;
        element.playbackRate = playbackRate;
        element.preservesPitch = true;
        applyMediaSettings(element, trackId, redactedTrackIdsRef.current);
      } else {
        const previous = audioRefs.current.get(trackId);
        if (previous) majorDrift.current.delete(previous);
        audioRefs.current.delete(trackId);
      }
    },
    [applyMediaSettings, redactedTrackIdsRef, playbackRate],
  );

  function setFullscreenElement(element: HTMLElement | null) {
    fullscreenElementRef.current = element;
  }

  const setVideoElement = useCallback(
    (element: HTMLVideoElement | null) => {
      videoRef.current = element;
      if (element) {
        updateMediaSettings(element);
        element.defaultPlaybackRate = playbackRate;
        element.playbackRate = playbackRate;
        element.preservesPitch = true;
      }
    },
    [updateMediaSettings, playbackRate],
  );

  useEffect(() => {
    for (const element of [
      ...audioRefs.current.values(),
      ...(videoRef.current ? [videoRef.current] : []),
      ...additionalVideoRefs.current.values(),
    ]) {
      element.defaultPlaybackRate = playbackRate;
      element.playbackRate = playbackRate;
      element.preservesPitch = true;
    }
    majorDrift.current.clear();
  }, [playbackRate]);
  const registerVideoElement = useCallback(
    (trackId: string, element: HTMLVideoElement | null) => {
      if (element) {
        additionalVideoRefs.current.set(trackId, element);
        element.muted = true;
        element.defaultPlaybackRate = playbackRate;
        element.playbackRate = playbackRate;
      } else {
        const previous = additionalVideoRefs.current.get(trackId);
        if (previous) majorDrift.current.delete(previous);
        additionalVideoRefs.current.delete(trackId);
      }
    },
    [playbackRate],
  );

  function requestFullscreen() {
    return fullscreenElementRef.current?.requestFullscreen?.();
  }

  useEffect(() => {
    const video = videoRef.current;
    if (video) {
      updateMediaSettings(video);
    }
  }, [updateMediaSettings]);

  useEffect(() => {
    for (const [trackId, audio] of audioRefs.current) {
      updateMediaSettings(audio, trackId);
    }
  }, [updateMediaSettings]);

  return {
    actions: {
      applyRedactedAudioSettings,
      clockTime,
      pauseMediaElements,
      registerAudioElement,
      registerVideoElement,
      requestFullscreen,
      setFullscreenElement,
      setVideoElement,
      syncAudioElements,
      updateMediaSettings,
    },
    refs: {
      audioRefs,
      videoRef,
      additionalVideoRefs,
    },
  };
}

function setMediaCurrentTime(media: HTMLMediaElement, seconds: number) {
  if (
    media.readyState === HTMLMediaElement.HAVE_NOTHING ||
    media.seekable.length === 0 ||
    media.currentTime === seconds
  ) {
    return;
  }
  try {
    media.currentTime = seconds;
  } catch {
    // Generated stream replacements may not expose a seekable range yet.
  }
}
