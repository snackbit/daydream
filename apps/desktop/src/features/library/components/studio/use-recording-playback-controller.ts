import {
  useCallback,
  useMemo,
  useRef,
  useState,
  type SyntheticEvent,
} from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';
import { segmentForTimeline, timelineDurationMs } from '../../library-timeline';
import type { PlaybackSpeed } from './playback/playback-speed';
import { usePlaybackTimelineClock } from './playback/use-playback-timeline-clock';
import { usePlaybackTransport } from './playback/use-playback-transport';
import { usePlaybackMediaElements } from './recording-playback-media-elements';
import { usePlaybackRedactionController } from './recording-playback-redaction-controller';
import { usePlaybackResumeTimer } from './recording-playback-resume-timer';
import { usePlaybackVideoRedactionSkip } from './use-redaction-auto-skip';

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
  autoSkipRedacted = true,
}: {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  playbackAssets: LibraryPlaybackAssets | null;
  autoSkipRedacted?: boolean;
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
  const [playbackRate, setPlaybackRate] = useState<PlaybackSpeed>(1);
  const [resumeProgress, setResumeProgress] = useState<number | null>(null);
  const { clearResumeTimer, startResumeTimer } = usePlaybackResumeTimer({
    setResumeProgress,
    playbackRate,
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
  const media = usePlaybackMediaElements({
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
      streamStartMs,
      scrubTimelineMs,
      totalDurationMs,
      isPlaying,
      pendingAutoplay,
      clockTime,
    });
  const redaction = usePlaybackRedactionController({
    applyRedactedAudioSettings,
    clockTime,
    currentSegment,
    audioRefs: media.refs.audioRefs,
    currentTimelineMs,
    detail,
    isPlaying,
    redactedTrackIdsRef,
    streamStartMs,
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
  const streamStartSeconds = streamStartMs / 1000;
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
    ]),
    resumeProgress,
    setIsPlaying,
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
    targetMs: autoSkipRedacted ? redactionSkipTargetMs : null,
    seek: seekTimeline,
    isPlaying,
    timelineMs: currentTimelineMs,
    seekRevision,
  });

  const handleEnded = useCallback(
    (event: SyntheticEvent<HTMLVideoElement>) => {
      if (!currentSegment || !isCurrentMedia(event.currentTarget)) {
        return;
      }

      const nextIndex = currentIndex + 1;
      if (nextIndex >= playableSegments.length) {
        setCurrentTime((currentSegment.durationMs ?? 0) / 1000);
        replaceStream(false);
        setPlaybackEndedAtEnd(true);
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

      replaceStream(shouldContinuePlayback);
      startResumeTimer(pauseGapMs, advanceToNextSegment);
    },
    [
      advanceToSegment,
      clearResumeTimer,
      currentIndex,
      currentSegment,
      replaceStream,
      isCurrentMedia,
      playableSegments.length,
      startResumeTimer,
    ],
  );

  function playNowAfterPause() {
    const nextIndex = currentIndex + 1;
    if (nextIndex < playableSegments.length) {
      advanceToSegment(nextIndex, playbackIntentRef.current);
    }
  }

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
    setCurrentTime(streamStartSeconds + clockTime());
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
      registerAudioElement,
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
