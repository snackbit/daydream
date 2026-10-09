import { useCallback, useEffect, useRef, type RefObject } from 'react';
import { driftCorrection } from './playback-clock';

interface PlaybackMediaElementsInput {
  isMuted: boolean;
  mutedTrackIds: Set<string>;
  redactedTrackIdsRef: RefObject<Set<string>>;
  volume: number;
}

export function usePlaybackMediaElements({
  isMuted,
  mutedTrackIds,
  redactedTrackIdsRef,
  volume,
}: PlaybackMediaElementsInput) {
  const audioRefs = useRef(new Map<string, HTMLAudioElement>());
  const fullscreenElementRef = useRef<HTMLElement | null>(null);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const majorDrift = useRef(new Map<HTMLMediaElement, number | null>());
  const clockElement = useCallback((): HTMLMediaElement | null => {
    const available = [...audioRefs.current.entries()].filter(
      ([, audio]) =>
        audio.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA && !audio.ended,
    );
    return (
      available.find(([id]) => id.startsWith('desktop-audio'))?.[1] ??
      available.find(([id]) => id.startsWith('microphone'))?.[1] ??
      videoRef.current
    );
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
      ]) {
        if (force) {
          setMediaCurrentTime(element, time);
          element.playbackRate = 1;
          majorDrift.current.delete(element);
        } else if (element === master) {
          element.playbackRate = 1;
          majorDrift.current.delete(element);
        } else if (element.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA) {
          const correction = driftCorrection(
            time - element.currentTime,
            performance.now(),
            majorDrift.current.get(element) ?? null,
          );
          majorDrift.current.set(element, correction.majorSinceMs);
          element.playbackRate = correction.rate;
          if (correction.seek) setMediaCurrentTime(element, time);
        }
      }
    },
    [clockElement, updateMediaSettings],
  );

  const pauseMediaElements = useCallback(() => {
    videoRef.current?.pause();
    for (const audio of audioRefs.current.values()) {
      audio.pause();
    }
  }, []);

  const registerAudioElement = useCallback(
    (trackId: string, element: HTMLAudioElement | null) => {
      if (element) {
        audioRefs.current.set(trackId, element);
        applyMediaSettings(element, trackId, redactedTrackIdsRef.current);
      } else {
        const previous = audioRefs.current.get(trackId);
        if (previous) majorDrift.current.delete(previous);
        audioRefs.current.delete(trackId);
      }
    },
    [applyMediaSettings, redactedTrackIdsRef],
  );

  function setFullscreenElement(element: HTMLElement | null) {
    fullscreenElementRef.current = element;
  }

  function setVideoElement(element: HTMLVideoElement | null) {
    videoRef.current = element;
  }

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
      requestFullscreen,
      setFullscreenElement,
      setVideoElement,
      syncAudioElements,
      updateMediaSettings,
    },
    refs: {
      audioRefs,
      videoRef,
    },
  };
}

function setMediaCurrentTime(media: HTMLMediaElement, seconds: number) {
  if (media.readyState === HTMLMediaElement.HAVE_NOTHING) {
    return;
  }
  try {
    media.currentTime = seconds;
  } catch {
    // Generated stream replacements may not expose a seekable range yet.
  }
}
