import {
  useCallback,
  useEffectEvent,
  useLayoutEffect,
  useRef,
  useState,
  type RefObject,
  type SyntheticEvent,
} from 'react';
import { reportFrontendFailure } from '@/lib/diagnostics/failure-logging';
import type { usePlaybackMediaElements } from '../recording-playback-media-elements';
import type { usePlaybackRedactionController } from '../recording-playback-redaction-controller';

type Media = ReturnType<typeof usePlaybackMediaElements>;
type Redaction = ReturnType<typeof usePlaybackRedactionController>;
interface MediaSet {
  video: HTMLVideoElement;
  audio: [string, HTMLAudioElement][];
  elements: HTMLMediaElement[];
}
interface Attempt {
  generation: number;
  requestId: string;
  started: number;
}
type Phase = 'paused' | 'waiting' | 'starting' | 'playing' | 'failed';
interface PlaybackTransportInput {
  media: {
    refs: Pick<Media['refs'], 'videoRef' | 'audioRefs' | 'additionalVideoRefs'>;
    actions: Pick<
      Media['actions'],
      'syncAudioElements' | 'updateMediaSettings'
    >;
  };
  redaction: Pick<
    Redaction,
    | 'updateRedactionStateForTimeline'
    | 'updateRedactionStateFromVideo'
    | 'videoTimelineMs'
  >;
  streamIdentity: string;
  playbackIntentRef: RefObject<boolean>;
  resumeProgress: number | null;
  setIsPlaying: (playing: boolean) => void;
  sampleClock?: (() => void) | undefined;
  setPendingAutoplay: (pending: boolean) => void;
  setPlaybackError: (error: string | null) => void;
}
function pause(set: MediaSet | null) {
  for (const element of set?.elements ?? []) element.pause();
}
function ready(set: MediaSet) {
  return set.elements.every(
    (element) =>
      element.readyState >= HTMLMediaElement.HAVE_FUTURE_DATA &&
      !element.seeking,
  );
}

/** Each stream owns its starts. Retired promises cannot change current playback. */
export function usePlaybackTransport({
  media,
  redaction,
  streamIdentity,
  playbackIntentRef,
  resumeProgress,
  setIsPlaying,
  sampleClock,
  setPendingAutoplay,
  setPlaybackError,
}: PlaybackTransportInput) {
  const [playRequested, setPlayRequested] = useState(false);
  const owner = useRef<{
    generation: number;
    media: MediaSet | null;
    phase: Phase;
  }>({
    generation: 0,
    media: null,
    phase: 'paused',
  });
  const { videoRef, audioRefs, additionalVideoRefs } = media.refs;
  const isCurrentMedia = useCallback((element: HTMLMediaElement) => {
    const set = owner.current.media;
    return set !== null && set.elements.includes(element);
  }, []);
  const invalidate = useCallback(() => {
    owner.current.generation++;
    setIsPlaying(false);
  }, [setIsPlaying]);
  const replaceStream = useCallback(
    (wanted: boolean) => {
      const retired = owner.current.media;
      invalidate();
      owner.current.media = null;
      owner.current.phase = wanted ? 'waiting' : 'paused';
      playbackIntentRef.current = wanted;
      setPlayRequested(wanted);
      setPendingAutoplay(wanted);
      setPlaybackError(null);
      pause(retired);
    },
    [invalidate, playbackIntentRef, setPendingAutoplay, setPlaybackError],
  );

  function fail(
    code: 'playback_start_failed' | 'playback_media_failed',
    error: unknown,
    attempt?: Attempt,
  ) {
    if (owner.current.phase === 'failed') return;
    owner.current.phase = 'failed';
    invalidate();
    playbackIntentRef.current = false;
    setPlayRequested(false);
    setPendingAutoplay(false);
    setPlaybackError(
      code === 'playback_start_failed'
        ? 'Playback could not start. Try again.'
        : 'The media stream could not be played. Try again.',
    );
    pause(owner.current.media);
    reportFrontendFailure(
      'studio_playback',
      'execute',
      Object.assign(new Error('Playback failed', { cause: error }), { code }),
      {
        ...(attempt
          ? {
              requestId: attempt.requestId,
              elapsedMs: performance.now() - attempt.started,
            }
          : {}),
        deduplicate: false,
      },
    );
  }
  function tryStart() {
    const current = owner.current;
    const set = current.media;
    if (current.phase !== 'waiting' || resumeProgress !== null || !set) return;
    // Metadata-only loading may stop before canplay. A queued Play requests
    // actual media data without starting an unready group.
    for (const element of set.elements) element.preload = 'auto';
    if (set.elements.some((element) => element.error !== null)) {
      fail('playback_media_failed', null);
      return;
    }
    if (!ready(set)) return;
    redaction.updateRedactionStateForTimeline(
      redaction.videoTimelineMs(set.video),
    );
    media.actions.syncAudioElements(set.video.currentTime, true);
    if (!ready(set)) return;
    for (const [id, audio] of set.audio)
      media.actions.updateMediaSettings(audio, id);
    const attempt: Attempt = {
      generation: current.generation,
      requestId: crypto.randomUUID(),
      started: performance.now(),
    };
    current.phase = 'starting';
    setPendingAutoplay(true);
    // Call every play synchronously; do not start new audio after awaiting an old video.
    void Promise.allSettled(
      set.elements.map(async (element) => element.play()),
    ).then((results) => {
      if (owner.current.generation !== attempt.generation) return;
      const failure = results.find((result) => result.status === 'rejected');
      if (failure?.status === 'rejected') {
        fail('playback_start_failed', failure.reason, attempt);
        return;
      }
      setPendingAutoplay(false);
      owner.current.phase = 'playing';
      setIsPlaying(true);
    });
  }
  const bindStream = useEffectEvent(() => {
    owner.current.phase = playbackIntentRef.current ? 'waiting' : 'paused';
    const video = videoRef.current;
    const audio = [...audioRefs.current.entries()];
    const set = video
      ? {
          video,
          audio,
          elements: [
            video,
            ...audio.map(([, element]) => element),
            ...additionalVideoRefs.current.values(),
          ],
        }
      : null;
    owner.current.media = set;
    setIsPlaying(false);
    setPendingAutoplay(playbackIntentRef.current);
    tryStart();
    return set;
  });
  useLayoutEffect(() => {
    const set = bindStream();
    return () => {
      owner.current.generation++;
      owner.current.phase = 'paused';
      owner.current.media = null;
      pause(set);
    };
  }, [streamIdentity]);

  function requestPlayback(wanted: boolean) {
    if (!wanted && owner.current.phase === 'playing') sampleClock?.();
    invalidate();
    owner.current.phase = wanted ? 'waiting' : 'paused';
    playbackIntentRef.current = wanted;
    setPlayRequested(wanted);
    setPendingAutoplay(wanted);
    if (!wanted) {
      pause(owner.current.media);
      return;
    }
    setPlaybackError(null);
    tryStart();
  }
  function handleVideoCanPlay() {
    redaction.updateRedactionStateFromVideo();
    tryStart();
  }
  function handleBuffering(event: SyntheticEvent<HTMLMediaElement>) {
    if (
      !isCurrentMedia(event.currentTarget) ||
      event.currentTarget.readyState >= HTMLMediaElement.HAVE_FUTURE_DATA ||
      !playbackIntentRef.current ||
      resumeProgress !== null
    )
      return;
    sampleClock?.();
    invalidate();
    owner.current.phase = 'waiting';
    setPendingAutoplay(true);
    pause(owner.current.media);
  }
  function handleVideoPlay(event: SyntheticEvent<HTMLMediaElement>) {
    if (isCurrentMedia(event.currentTarget))
      redaction.updateRedactionStateFromVideo();
  }
  function handleVideoPause(event: SyntheticEvent<HTMLMediaElement>) {
    if (!isCurrentMedia(event.currentTarget)) return;
    setIsPlaying(false);
    if (
      (owner.current.phase === 'playing' ||
        owner.current.phase === 'starting') &&
      !event.currentTarget.ended
    )
      requestPlayback(false);
  }
  function handleMediaError(event: SyntheticEvent<HTMLMediaElement>) {
    if (isCurrentMedia(event.currentTarget))
      fail('playback_media_failed', event.currentTarget.error);
  }
  return {
    playRequested,
    replaceStream,
    requestPlayback,
    isCurrentMedia,
    handleVideoCanPlay,
    handleBuffering,
    handleVideoPlay,
    handleVideoPause,
    handleMediaError,
  };
}
