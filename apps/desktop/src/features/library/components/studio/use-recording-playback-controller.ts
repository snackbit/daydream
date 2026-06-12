import { useCallback, useMemo, useRef, useState } from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';
import { segmentForTimeline, timelineDurationMs } from '../../library-timeline';
import { usePlaybackMediaElements } from './recording-playback-media-elements';
import { usePlaybackRedactionController } from './recording-playback-redaction-controller';
import { usePlaybackResumeTimer } from './recording-playback-resume-timer';

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
  const playbackIntentRef = useRef(false);
  const redactedTrackIdsRef = useRef(new Set<string>());
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
  const { clearResumeTimer, startResumeTimer } = usePlaybackResumeTimer({
    setResumeProgress,
  });
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
  const media = usePlaybackMediaElements({
    isMuted,
    mutedTrackIds,
    redactedTrackIdsRef,
    volume,
  });
  const { videoRef } = media.refs;
  const {
    applyRedactedAudioSettings,
    pauseMediaElements,
    registerAudioElement,
    requestFullscreen,
    setFullscreenElement,
    syncAudioElements,
    updateMediaSettings,
  } = media.actions;
  const redaction = usePlaybackRedactionController({
    applyRedactedAudioSettings,
    currentSegment,
    currentTimelineMs,
    detail,
    isPlaying,
    redactedTrackIdsRef,
    streamStartMs,
    totalDurationMs,
    videoRef,
  });
  const {
    nextUnredactedVideoTimelineMs,
    redactedVideoActive,
    updateRedactionStateForTimeline,
    updateRedactionStateFromVideo,
    videoTimelineMs,
  } = redaction;
  const streamStartSeconds = streamStartMs / 1000;
  const snapshot: RecordingPlaybackSnapshot = {
    displayedTimelineMs,
    isPlaying,
    seekRevision,
    totalDurationMs,
  };

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
      [...media.refs.audioRefs.current.entries()].map(
        async ([trackId, audio]) => {
          updateMediaSettings(audio, trackId);
          await audio.play();
        },
      ),
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

    startResumeTimer(pauseGapMs, advanceToNextSegment);
  }, [
    advanceToSegment,
    clearResumeTimer,
    currentIndex,
    currentSegment,
    pauseMediaElements,
    playableSegments.length,
    startResumeTimer,
  ]);

  function playNowAfterPause() {
    const nextIndex = currentIndex + 1;
    if (nextIndex < playableSegments.length) {
      advanceToSegment(nextIndex, playbackIntentRef.current);
    }
  }

  function setVideoElement(element: HTMLVideoElement | null) {
    media.actions.setVideoElement(element);
    if (element) {
      updateRedactionStateForTimeline(videoTimelineMs(element));
    }
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
