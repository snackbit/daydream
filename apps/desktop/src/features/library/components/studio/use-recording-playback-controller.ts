import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';
import { segmentForTimeline, timelineDurationMs } from '../../library-timeline';
import {
  nextRedactionBoundaryDelayMs,
  nextUnredactedVideoTimelineMsForRedaction,
  REDACTION_START_GUARD_MS,
  redactedTracksAtTimelineMs,
  setsEqual,
} from './recording-playback-redactions';

export interface RecordingPlaybackSnapshot {
  displayedTimelineMs: number;
  isPlaying: boolean;
  seekRevision: number;
  totalDurationMs: number;
}

export type RecordingPlaybackController = ReturnType<
  typeof useRecordingPlaybackController
>;

export function useRecordingPlaybackController({
  detail,
  mutedTrackIds,
  playbackAssets,
}: {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  playbackAssets: LibraryPlaybackAssets | null;
}) {
  const audioRefs = useRef(new Map<string, HTMLAudioElement>());
  const fullscreenElementRef = useRef<HTMLElement | null>(null);
  const playbackIntentRef = useRef(false);
  const redactionBoundaryTimerRef = useRef<number | null>(null);
  const resumeTimerRef = useRef<number | null>(null);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const playableSegments = useMemo(
    () =>
      detail.segments.filter(
        (segment) => segment.state === 'finalized' && segment.fileExists,
      ),
    [detail.segments],
  );
  const [currentIndex, setCurrentIndex] = useState(0);
  const [currentTime, setCurrentTime] = useState(0);
  const [isMuted, setIsMuted] = useState(false);
  const [isPlaying, setIsPlaying] = useState(false);
  const [playbackEndedAtEnd, setPlaybackEndedAtEnd] = useState(false);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const [pendingAutoplay, setPendingAutoplay] = useState(false);
  const [resumeProgress, setResumeProgress] = useState<number | null>(null);
  const [scrubTimelineMs, setScrubTimelineMs] = useState<number | null>(null);
  const [seekRevision, setSeekRevision] = useState(0);
  const [streamRevision, setStreamRevision] = useState(0);
  const [streamStartMs, setStreamStartMs] = useState(0);
  const [volume, setVolume] = useState(0.8);
  const currentSegment = playableSegments[currentIndex] ?? null;
  const currentPlaybackSegment = currentSegment
    ? playbackAssets?.segments.find(
        (segment) => segment.segmentId === currentSegment.id,
      )
    : null;
  const totalDurationMs = timelineDurationMs(playableSegments);
  const currentTimelineMs = currentSegment
    ? currentSegment.playbackOffsetMs + currentTime * 1000
    : 0;
  const displayedTimelineMs = scrubTimelineMs ?? currentTimelineMs;
  const [redactedTrackIds, setRedactedTrackIds] = useState<Set<string>>(() =>
    redactedTracksAtTimelineMs(
      detail,
      currentTimelineMs,
      REDACTION_START_GUARD_MS,
    ),
  );
  const redactedTrackIdsRef = useRef(redactedTrackIds);
  const videoTrackIds = useMemo(
    () =>
      detail.tracks
        .filter((track) => track.kind === 'video')
        .map((track) => track.id),
    [detail.tracks],
  );
  const redactedVideoActive = videoTrackIds.some((trackId) =>
    redactedTrackIds.has(trackId),
  );
  const [nextUnredactedVideoTimelineMs, setNextUnredactedVideoTimelineMs] =
    useState<number | null>(() =>
      redactedVideoActive
        ? nextUnredactedVideoTimelineMsForRedaction(
            detail,
            currentTimelineMs,
            videoTrackIds,
            totalDurationMs,
          )
        : null,
    );
  const streamStartSeconds = streamStartMs / 1000;
  const snapshot: RecordingPlaybackSnapshot = {
    displayedTimelineMs,
    isPlaying,
    seekRevision,
    totalDurationMs,
  };

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
    [applyMediaSettings],
  );

  const updateRedactionStateForTimeline = useCallback(
    (timelineMs: number) => {
      const nextRedactedTrackIds = redactedTracksAtTimelineMs(
        detail,
        timelineMs,
        REDACTION_START_GUARD_MS,
      );
      if (!setsEqual(redactedTrackIdsRef.current, nextRedactedTrackIds)) {
        redactedTrackIdsRef.current = nextRedactedTrackIds;
        setRedactedTrackIds(nextRedactedTrackIds);
      }
      const nextRedactedVideoActive = videoTrackIds.some((trackId) =>
        nextRedactedTrackIds.has(trackId),
      );
      const nextSkipTarget = nextRedactedVideoActive
        ? nextUnredactedVideoTimelineMsForRedaction(
            detail,
            timelineMs,
            videoTrackIds,
            totalDurationMs,
          )
        : null;
      setNextUnredactedVideoTimelineMs((currentTarget) =>
        currentTarget === nextSkipTarget ? currentTarget : nextSkipTarget,
      );
      for (const [trackId, audio] of audioRefs.current) {
        applyMediaSettings(audio, trackId, nextRedactedTrackIds);
      }
    },
    [applyMediaSettings, detail, totalDurationMs, videoTrackIds],
  );

  const videoTimelineMs = useCallback(
    (video: HTMLVideoElement) =>
      currentSegment
        ? currentSegment.playbackOffsetMs +
          streamStartMs +
          video.currentTime * 1000
        : 0,
    [currentSegment, streamStartMs],
  );

  const updateRedactionStateFromVideo = useCallback(() => {
    const video = videoRef.current;
    if (video) {
      updateRedactionStateForTimeline(videoTimelineMs(video));
    }
  }, [updateRedactionStateForTimeline, videoTimelineMs]);

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

  const clearResumeTimer = useCallback(() => {
    if (resumeTimerRef.current !== null) {
      window.clearInterval(resumeTimerRef.current);
      resumeTimerRef.current = null;
    }
  }, []);

  const clearRedactionBoundaryTimer = useCallback(() => {
    if (redactionBoundaryTimerRef.current !== null) {
      window.clearTimeout(redactionBoundaryTimerRef.current);
      redactionBoundaryTimerRef.current = null;
    }
  }, []);

  const playMediaElements = useCallback(async (): Promise<boolean> => {
    const video = videoRef.current;
    if (!video) {
      return false;
    }
    setPlaybackError(null);
    syncAudioElements(video.currentTime, true);
    try {
      await video.play();
    } catch (error) {
      playbackIntentRef.current = false;
      setIsPlaying(false);
      setPendingAutoplay(false);
      setPlaybackError(`Playback failed: ${errorMessage(error)}`);
      return false;
    }
    const audioResults = await Promise.allSettled(
      [...audioRefs.current.entries()].map(async ([trackId, audio]) => {
        updateMediaSettings(audio, trackId);
        await audio.play();
      }),
    );
    const audioFailure = audioResults.find(
      (result) => result.status === 'rejected',
    );
    if (audioFailure?.status === 'rejected') {
      setPlaybackError(
        `Audio playback failed: ${errorMessage(audioFailure.reason)}`,
      );
    }

    return true;
  }, [syncAudioElements, updateMediaSettings]);

  const advanceToSegment = useCallback(
    (nextIndex: number, shouldPlay: boolean) => {
      clearResumeTimer();
      playbackIntentRef.current = shouldPlay;
      setCurrentIndex(nextIndex);
      setCurrentTime(0);
      setPlaybackEndedAtEnd(false);
      setPlaybackError(null);
      setPendingAutoplay(shouldPlay);
      setResumeProgress(null);
      setStreamStartMs(0);
      updateRedactionStateForTimeline(
        playableSegments[nextIndex]?.playbackOffsetMs ?? 0,
      );
      setStreamRevision((revision) => revision + 1);
      setSeekRevision((revision) => revision + 1);
      setIsPlaying(shouldPlay);
    },
    [clearResumeTimer, playableSegments, updateRedactionStateForTimeline],
  );

  const seekTimeline = useCallback(
    (nextTimelineMs: number, options: { resume?: boolean } = {}) => {
      if (nextTimelineMs >= totalDurationMs) {
        clearResumeTimer();
        const lastIndex = playableSegments.length - 1;
        const lastSegment = playableSegments[lastIndex];
        if (!lastSegment) {
          return;
        }
        pauseMediaElements();
        playbackIntentRef.current = false;
        setCurrentIndex(lastIndex);
        setCurrentTime((lastSegment.durationMs ?? 0) / 1000);
        setPendingAutoplay(false);
        setIsPlaying(false);
        setPlaybackEndedAtEnd(true);
        setPlaybackError(null);
        setResumeProgress(null);
        setStreamStartMs(lastSegment.durationMs ?? 0);
        updateRedactionStateForTimeline(totalDurationMs);
        setStreamRevision((revision) => revision + 1);
        setSeekRevision((revision) => revision + 1);
        return;
      }

      const target = segmentForTimeline(playableSegments, nextTimelineMs);
      if (!target) {
        return;
      }
      const shouldResume = options.resume ?? playbackIntentRef.current;
      clearResumeTimer();
      pauseMediaElements();
      playbackIntentRef.current = shouldResume;
      setCurrentIndex(target.index);
      setCurrentTime(target.offsetMs / 1000);
      setIsPlaying(shouldResume);
      setPendingAutoplay(shouldResume);
      setPlaybackEndedAtEnd(false);
      setPlaybackError(null);
      setResumeProgress(null);
      setStreamStartMs(target.offsetMs);
      updateRedactionStateForTimeline(nextTimelineMs);
      setStreamRevision((revision) => revision + 1);
      setSeekRevision((revision) => revision + 1);
    },
    [
      clearResumeTimer,
      pauseMediaElements,
      playableSegments,
      totalDurationMs,
      updateRedactionStateForTimeline,
    ],
  );

  async function togglePlayback() {
    const video = videoRef.current;
    if (resumeProgress !== null) {
      return;
    }
    if (playbackEndedAtEnd) {
      seekTimeline(0, { resume: true });
      return;
    }
    if (!video) {
      return;
    }
    if (video.paused) {
      setPlaybackEndedAtEnd(false);
      const played = await playMediaElements();
      playbackIntentRef.current = played;
      setIsPlaying(played);
    } else {
      playbackIntentRef.current = false;
      setPendingAutoplay(false);
      pauseMediaElements();
      setIsPlaying(false);
    }
  }

  function handleTimelineInput(nextTimelineMs: number) {
    setScrubTimelineMs(Math.min(nextTimelineMs, totalDurationMs));
  }

  function commitTimelineInput(nextTimelineMs: number) {
    setScrubTimelineMs(null);
    seekTimeline(nextTimelineMs);
  }

  function skipToNextUnredactedVideo() {
    if (nextUnredactedVideoTimelineMs === null) {
      return;
    }
    seekTimeline(nextUnredactedVideoTimelineMs);
  }

  const handleEnded = useCallback(() => {
    if (!currentSegment) {
      return;
    }

    const nextIndex = currentIndex + 1;
    if (nextIndex >= playableSegments.length) {
      setCurrentTime((currentSegment.durationMs ?? 0) / 1000);
      playbackIntentRef.current = false;
      setIsPlaying(false);
      setPlaybackEndedAtEnd(true);
      pauseMediaElements();
      return;
    }
    const shouldContinuePlayback = playbackIntentRef.current;
    const advanceToNextSegment = () =>
      advanceToSegment(nextIndex, shouldContinuePlayback);
    const pauseGapMs = Math.max(0, currentSegment.pauseAfterMs);
    if (pauseGapMs === 0) {
      advanceToNextSegment();
      return;
    }

    setResumeProgress(0);
    const startedAt = window.performance.now();
    clearResumeTimer();
    resumeTimerRef.current = window.setInterval(() => {
      const progress = Math.min(
        1,
        (window.performance.now() - startedAt) / pauseGapMs,
      );
      setResumeProgress(progress);
      if (progress >= 1) {
        clearResumeTimer();
        advanceToNextSegment();
      }
    }, 50);
  }, [
    advanceToSegment,
    clearResumeTimer,
    currentIndex,
    currentSegment,
    pauseMediaElements,
    playableSegments.length,
  ]);

  function playNowAfterPause() {
    const nextIndex = currentIndex + 1;
    if (nextIndex < playableSegments.length) {
      advanceToSegment(nextIndex, playbackIntentRef.current);
    }
  }

  function registerAudioElement(
    trackId: string,
    element: HTMLAudioElement | null,
  ) {
    if (element) {
      audioRefs.current.set(trackId, element);
      applyMediaSettings(element, trackId, redactedTrackIdsRef.current);
    } else {
      audioRefs.current.delete(trackId);
    }
  }

  function setFullscreenElement(element: HTMLElement | null) {
    fullscreenElementRef.current = element;
  }

  function setVideoElement(element: HTMLVideoElement | null) {
    videoRef.current = element;
    if (element) {
      updateRedactionStateForTimeline(videoTimelineMs(element));
    }
  }

  function requestFullscreen() {
    return fullscreenElementRef.current?.requestFullscreen?.();
  }

  function handleVideoPlay() {
    playbackIntentRef.current = true;
    setIsPlaying(true);
    setPendingAutoplay(false);
    setPlaybackEndedAtEnd(false);
    updateRedactionStateFromVideo();
  }

  function handleVideoPause() {
    setIsPlaying(false);
  }

  function handleVideoTimeUpdate(video: HTMLVideoElement) {
    setCurrentTime(streamStartSeconds + video.currentTime);
    updateRedactionStateForTimeline(videoTimelineMs(video));
    syncAudioElements(video.currentTime);
  }

  function handleVideoCanPlay() {
    updateRedactionStateFromVideo();
    if (!pendingAutoplay || resumeProgress !== null) {
      return;
    }
    setPendingAutoplay(false);
    void playMediaElements();
  }

  function handleVideoError() {
    playbackIntentRef.current = false;
    setIsPlaying(false);
    setPendingAutoplay(false);
    setPlaybackError('Playback failed: video stream is not supported.');
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

  useEffect(() => {
    updateRedactionStateForTimeline(currentTimelineMs);
  }, [currentTimelineMs, updateRedactionStateForTimeline]);

  useEffect(() => {
    if (!isPlaying) {
      return;
    }

    let animationFrameId = window.requestAnimationFrame(function tick() {
      updateRedactionStateFromVideo();
      animationFrameId = window.requestAnimationFrame(tick);
    });

    return () => window.cancelAnimationFrame(animationFrameId);
  }, [isPlaying, updateRedactionStateFromVideo]);

  useEffect(() => {
    clearRedactionBoundaryTimer();
    if (!isPlaying) {
      return;
    }

    const scheduleNextBoundary = () => {
      updateRedactionStateFromVideo();
      const video = videoRef.current;
      if (!video) {
        return;
      }
      const delayMs = nextRedactionBoundaryDelayMs(
        detail,
        videoTimelineMs(video),
      );
      if (delayMs === null) {
        return;
      }
      redactionBoundaryTimerRef.current = window.setTimeout(
        scheduleNextBoundary,
        delayMs,
      );
    };

    scheduleNextBoundary();

    return clearRedactionBoundaryTimer;
  }, [
    clearRedactionBoundaryTimer,
    detail,
    isPlaying,
    updateRedactionStateFromVideo,
    videoTimelineMs,
  ]);

  useEffect(() => clearResumeTimer, [clearResumeTimer]);
  useEffect(() => clearRedactionBoundaryTimer, [clearRedactionBoundaryTimer]);

  return {
    actions: {
      commitTimelineInput,
      handleEnded,
      handleTimelineInput,
      handleVideoCanPlay,
      handleVideoError,
      handleVideoPause,
      handleVideoPlay,
      handleVideoTimeUpdate,
      playNowAfterPause,
      registerAudioElement,
      requestFullscreen,
      seekTimeline,
      setFullscreenElement,
      setIsMuted,
      setVideoElement,
      setVolume,
      skipToNextUnredactedVideo,
      togglePlayback,
    },
    state: {
      currentPlaybackSegment,
      currentSegment,
      displayedTimelineMs,
      isMuted,
      isPlaying,
      nextUnredactedVideoTimelineMs,
      playableSegments,
      playbackError,
      redactedVideoActive,
      resumeProgress,
      snapshot,
      streamRevision,
      streamStartMs,
      totalDurationMs,
      volume,
    },
  };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
