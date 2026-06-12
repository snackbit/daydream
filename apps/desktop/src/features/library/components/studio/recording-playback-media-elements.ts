import { useCallback, useEffect, useRef, type RefObject } from 'react';

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
      for (const [trackId, audio] of audioRefs.current) {
        updateMediaSettings(audio, trackId);
        if (force || Math.abs(audio.currentTime - currentTimeSeconds) > 0.25) {
          setMediaCurrentTime(audio, currentTimeSeconds);
        }
      }
    },
    [updateMediaSettings],
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
