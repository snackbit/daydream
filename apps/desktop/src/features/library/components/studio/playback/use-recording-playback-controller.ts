import { useCallback, useMemo, useRef, useState } from 'react';
import {
  segmentForTimeline,
  timelineDurationMs,
} from '../../../library-timeline';
import { usePlaybackTimelineClock } from './clock/use-playback-timeline-clock';
import type { PlaybackSpeed } from './controls/playback-speed';
import { usePlaybackMediaElements } from './media/recording-playback-media-elements';
import type {
  RecordingPlaybackControllerInput,
  RecordingPlaybackSnapshot,
} from './recording-playback-contracts';
import { usePlaybackRedactionController } from './redactions/recording-playback-redaction-controller';
import { usePlaybackVideoRedactionSkip } from './redactions/use-redaction-auto-skip';
import { usePlaybackResumeTimer } from './transport/recording-playback-resume-timer';
import { usePlaybackSegmentLifecycle } from './transport/use-playback-segment-lifecycle';
import { usePlaybackTransport } from './transport/use-playback-transport';

export type { RecordingPlaybackSnapshot } from './recording-playback-contracts';

export type RecordingPlaybackController = ReturnType<
  typeof useRecordingPlaybackController
>;

export function useRecordingPlaybackController({
  detail,
  mutedTrackIds,
  playbackAssets,
  autoSkipRedacted = true,
  shortGaps = false,
  skipOnlyWhenAllSources = false,
}: RecordingPlaybackControllerInput) {
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
  const [playbackRate, setPlaybackRate] = useState<PlaybackSpeed>(1);
  const [resumeProgress, setResumeProgress] = useState<number | null>(null);
  const { clearResumeTimer, startResumeTimer, setResumeTimerPaused } =
    usePlaybackResumeTimer({
      setResumeProgress,
      playbackRate: shortGaps ? 1 : playbackRate,
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
  const canSeekInStream = currentPlaybackSegment?.canSeekInStream === true;
  const mediaClockOffsetMs = canSeekInStream
    ? -(currentSegment?.mediaStartMs ?? 0)
    : streamStartMs;
  const media = usePlaybackMediaElements({
    canSeekInStream:
      canSeekInStream || playbackAssets?.canSeekInStream === true,
    isMuted,
    mutedTrackIds,
    redactedTrackIdsRef,
    volume,
    playbackRate,
  });
  const { videoRef } = media.refs;
  const {
    applyRedactedAudioSettings,
    clockTime,
    registerAudioElement,
    requestFullscreen,
    setFullscreenElement,
    syncAudioElements,
  } = media.actions;
  const { currentTimelineMs, displayedTimelineMs, readPlaybackTimeMs } =
    usePlaybackTimelineClock({
      playbackOffsetMs: currentSegment?.playbackOffsetMs ?? null,
      currentTimeSeconds: currentTime,
      streamStartMs: mediaClockOffsetMs,
      scrubTimelineMs,
      totalDurationMs,
      isPlaying,
      pendingAutoplay,
      clockTime,
    });
  const sampleBoundary = useCallback(
    () => setCurrentTime(mediaClockOffsetMs / 1000 + clockTime()),
    [mediaClockOffsetMs, clockTime],
  );
  const redaction = usePlaybackRedactionController({
    onBoundary: sampleBoundary,
    applyRedactedAudioSettings,
    clockTime,
    currentSegment,
    audioRefs: media.refs.audioRefs,
    currentTimelineMs,
    detail,
    isPlaying,
    redactedTrackIdsRef,
    streamStartMs: mediaClockOffsetMs,
    totalDurationMs,
    videoRef,
    playbackRate,
  });
  const {
    nextUnredactedVideoTimelineMs,
    redactedVideoActive,
    updateRedactionStateForTimeline,
    videoTimelineMs,
    redactionSkipTargetMs,
  } = redaction;
  const snapshot: RecordingPlaybackSnapshot = {
    displayedTimelineMs,
    isPlaying,
    seekRevision,
    totalDurationMs,
  };

  const {
    replaceStream,
    requestPlayback,
    playRequested,
    isCurrentMedia,
    handleVideoCanPlay,
    handleBuffering,
    handleVideoPlay,
    handleVideoPause,
    handleMediaError,
  } = usePlaybackTransport({
    media,
    redaction,
    playbackIntentRef,
    streamIdentity: JSON.stringify([
      detail.id,
      currentPlaybackSegment?.videoUrl,
      currentPlaybackSegment?.audio.map((audio) => [audio.trackId, audio.url]),
      streamStartMs,
      streamRevision,
      skipOnlyWhenAllSources,
    ]),
    resumeProgress,
    setIsPlaying,
    sampleClock: sampleBoundary,
    setPendingAutoplay,
    setPlaybackError,
  });

  const advanceToSegment = useCallback(
    (nextIndex: number, shouldPlay: boolean) => {
      clearResumeTimer();
      replaceStream(shouldPlay);
      setCurrentIndex(nextIndex);
      setCurrentTime(0);
      setPlaybackEndedAtEnd(false);
      setPlaybackError(null);
      setResumeProgress(null);
      setStreamStartMs(0);
      updateRedactionStateForTimeline(
        playableSegments[nextIndex]?.playbackOffsetMs ?? 0,
      );
      setStreamRevision((revision) => revision + 1);
      setSeekRevision((revision) => revision + 1);
    },
    [
      clearResumeTimer,
      playableSegments,
      updateRedactionStateForTimeline,
      replaceStream,
    ],
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
        replaceStream(false);
        setCurrentIndex(lastIndex);
        setCurrentTime((lastSegment.durationMs ?? 0) / 1000);
        setPlaybackEndedAtEnd(true);
        setPlaybackError(null);
        setResumeProgress(null);
        setStreamStartMs(Math.max(0, (lastSegment.durationMs ?? 0) - 1000));
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
      replaceStream(shouldResume);
      setCurrentIndex(target.index);
      setCurrentTime(target.offsetMs / 1000);
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
      replaceStream,
      playableSegments,
      totalDurationMs,
      updateRedactionStateForTimeline,
    ],
  );

  async function togglePlayback() {
    const video = videoRef.current;
    if (resumeProgress !== null) {
      const wanted = !playbackIntentRef.current;
      playbackIntentRef.current = wanted;
      setResumeTimerPaused(!wanted);
      replaceStream(wanted);
      return;
    }
    if (playbackEndedAtEnd) {
      seekTimeline(0, { resume: true });
      return;
    }
    if (!video) {
      return;
    }
    if (!playbackIntentRef.current) {
      setScrubTimelineMs(null);
      setPlaybackEndedAtEnd(false);
      requestPlayback(true);
    } else {
      requestPlayback(false);
    }
  }

  function handleTimelineInput(nextTimelineMs: number) {
    setScrubTimelineMs(Math.min(nextTimelineMs, totalDurationMs));
  }

  function commitTimelineInput(nextTimelineMs: number) {
    setScrubTimelineMs(null);
    seekTimeline(nextTimelineMs);
  }

  function cancelTimelineInput() {
    setScrubTimelineMs(null);
  }

  const {
    seconds: redactionCountdownSeconds,
    manualSkip: skipToNextUnredactedVideo,
  } = usePlaybackVideoRedactionSkip({
    recordingId: detail.id,
    targetMs:
      autoSkipRedacted &&
      (!skipOnlyWhenAllSources ||
        (currentSegment?.videoStreams ?? []).every((source) =>
          detail.redactions.some(
            (cut) =>
              cut.trackIds.includes(source.trackId) &&
              currentTimelineMs >= cut.startOffsetMs &&
              currentTimelineMs < cut.endOffsetMs,
          ),
        ))
        ? redactionSkipTargetMs
        : null,
    seek: seekTimeline,
    isPlaying,
    timelineMs: currentTimelineMs,
    seekRevision,
  });

  const { handleEnded, playNowAfterPause } = usePlaybackSegmentLifecycle({
    currentSegment,
    currentIndex,
    streamRevision,
    streamStartMs: mediaClockOffsetMs,
    playbackRate,
    isPlaying,
    resumeProgress,
    shortGaps,
    playbackIntentRef,
    advanceToSegment,
    replaceStream,
    isCurrentMedia,
    setCurrentTime,
    setPlaybackEndedAtEnd,
    clearResumeTimer,
    startResumeTimer,
    setResumeTimerPaused,
    clockTime,
    segmentCount: playableSegments.length,
  });

  const pausePlayback = useCallback(() => {
    setResumeTimerPaused(true);
    requestPlayback(false);
  }, [setResumeTimerPaused, requestPlayback]);

  const setVideoElement = useCallback(
    (element: HTMLVideoElement | null) => {
      media.actions.setVideoElement(element);
      if (element) {
        updateRedactionStateForTimeline(videoTimelineMs(element));
      }
    },
    [
      media.actions.setVideoElement,
      updateRedactionStateForTimeline,
      videoTimelineMs,
    ],
  );

  function handleVideoTimeUpdate(video: HTMLVideoElement) {
    if (!isCurrentMedia(video)) return;
    updateRedactionStateForTimeline(videoTimelineMs(video));
    syncAudioElements(video.currentTime);
  }

  return {
    actions: {
      commitTimelineInput,
      cancelTimelineInput,
      handleBuffering,
      handleEnded,
      handleTimelineInput,
      handleVideoCanPlay,
      handleMediaError,
      handleVideoPause,
      handleVideoPlay,
      handleVideoTimeUpdate,
      playNowAfterPause,
      pausePlayback,
      registerAudioElement,
      registerVideoElement: media.actions.registerVideoElement,
      requestFullscreen,
      readPlaybackTimeMs,
      seekTimeline,
      setFullscreenElement,
      setIsMuted,
      setVideoElement,
      setVolume,
      setPlaybackRate,
      skipToNextUnredactedVideo,
      togglePlayback,
    },
    state: {
      canSeekInStream,
      seekRevision,
      currentPlaybackSegment,
      currentSegment,
      displayedTimelineMs,
      isMuted,
      isPlaying,
      playRequested,
      playbackRate,
      nextUnredactedVideoTimelineMs,
      playableSegments,
      playbackError,
      redactedVideoActive,
      redactionCountdownSeconds,
      redactionSkipTargetMs,
      resumeProgress,
      snapshot,
      streamRevision,
      streamStartMs,
      totalDurationMs,
      volume,
    },
  };
}
