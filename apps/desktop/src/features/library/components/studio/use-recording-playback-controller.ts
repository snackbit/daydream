import {
  useCallback,
  useEffect,
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
  shortGaps = false,
  skipOnlyWhenAllSources = false,
}: {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  playbackAssets: LibraryPlaybackAssets | null;
  autoSkipRedacted?: boolean;
  shortGaps?: boolean;
  skipOnlyWhenAllSources?: boolean;
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
      skipOnlyWhenAllSources,
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

  const completedStream = useRef<string | null>(null);
  const finishCurrentSegment = useCallback(() => {
    if (!currentSegment || resumeProgress !== null) {
      return;
    }
    const key = `${currentSegment.id}:${streamRevision}`;
    if (completedStream.current === key) return;
    completedStream.current = key;

    const nextIndex = currentIndex + 1;
    if (nextIndex >= playableSegments.length) {
      setCurrentTime((currentSegment.durationMs ?? 0) / 1000);
      replaceStream(false);
      setPlaybackEndedAtEnd(true);
      return;
    }
    const advanceToNextSegment = () =>
      advanceToSegment(nextIndex, playbackIntentRef.current);
    const pauseGapMs = Math.max(0, currentSegment.pauseAfterMs);
    if (pauseGapMs === 0) {
      advanceToNextSegment();
      return;
    }

    replaceStream(playbackIntentRef.current);
    setCurrentTime((currentSegment.durationMs ?? 0) / 1000);
    startResumeTimer(
      shortGaps ? Math.min(3000, pauseGapMs) : pauseGapMs,
      advanceToNextSegment,
    );
    setResumeTimerPaused(!playbackIntentRef.current);
  }, [
    advanceToSegment,
    clearResumeTimer,
    currentIndex,
    currentSegment,
    replaceStream,
    isCurrentMedia,
    playableSegments.length,
    startResumeTimer,
    shortGaps,
    resumeProgress,
    streamRevision,
    setResumeTimerPaused,
  ]);

  function handleEnded(event: SyntheticEvent<HTMLVideoElement>) {
    if (isCurrentMedia(event.currentTarget)) finishCurrentSegment();
  }
  // Calendar slices can end before their physical MKV stream ends.
  useEffect(() => {
    if (
      !isPlaying ||
      !currentSegment ||
      currentSegment.mediaStartMs === undefined
    )
      return;
    let timer = 0;
    function check() {
      const remaining =
        (currentSegment?.durationMs ?? 0) - streamStartMs - clockTime() * 1000;
      if (remaining <= 0) finishCurrentSegment();
      else
        timer = window.setTimeout(
          check,
          Math.min(100, remaining / playbackRate),
        );
    }
    check();
    return () => window.clearTimeout(timer);
  }, [
    isPlaying,
    currentSegment,
    streamStartMs,
    clockTime,
    playbackRate,
    finishCurrentSegment,
  ]);

  function playNowAfterPause() {
    const nextIndex = currentIndex + 1;
    if (nextIndex < playableSegments.length) {
      advanceToSegment(nextIndex, playbackIntentRef.current);
    }
  }
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
    if (!playbackEndedAtEnd) setCurrentTime(streamStartSeconds + clockTime());
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
