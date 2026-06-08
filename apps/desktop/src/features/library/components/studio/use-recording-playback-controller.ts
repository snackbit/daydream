import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';
import { segmentForTimeline, timelineDurationMs } from '../../library-timeline';

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
  const playbackIntentRef = useRef(false);
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
  const streamStartSeconds = streamStartMs / 1000;
  const snapshot: RecordingPlaybackSnapshot = {
    displayedTimelineMs,
    isPlaying,
    seekRevision,
    totalDurationMs,
  };

  const updateMediaSettings = useCallback(
    (media: HTMLMediaElement, trackId?: string) => {
      media.volume = volume;
      media.muted = isMuted || (trackId ? mutedTrackIds.has(trackId) : false);
    },
    [isMuted, mutedTrackIds, volume],
  );

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
      setStreamRevision((revision) => revision + 1);
      setSeekRevision((revision) => revision + 1);
      setIsPlaying(shouldPlay);
    },
    [clearResumeTimer],
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
      setStreamRevision((revision) => revision + 1);
      setSeekRevision((revision) => revision + 1);
    },
    [clearResumeTimer, pauseMediaElements, playableSegments, totalDurationMs],
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
    } else {
      audioRefs.current.delete(trackId);
    }
  }

  function setVideoElement(element: HTMLVideoElement | null) {
    videoRef.current = element;
  }

  function requestFullscreen() {
    return videoRef.current?.requestFullscreen?.();
  }

  function handleVideoPlay() {
    playbackIntentRef.current = true;
    setIsPlaying(true);
    setPendingAutoplay(false);
    setPlaybackEndedAtEnd(false);
  }

  function handleVideoPause() {
    setIsPlaying(false);
  }

  function handleVideoTimeUpdate(video: HTMLVideoElement) {
    setCurrentTime(streamStartSeconds + video.currentTime);
    syncAudioElements(video.currentTime);
  }

  function handleVideoCanPlay() {
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

  useEffect(() => clearResumeTimer, [clearResumeTimer]);

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
      setIsMuted,
      setVideoElement,
      setVolume,
      togglePlayback,
    },
    state: {
      currentPlaybackSegment,
      currentSegment,
      displayedTimelineMs,
      isMuted,
      isPlaying,
      playableSegments,
      playbackError,
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
