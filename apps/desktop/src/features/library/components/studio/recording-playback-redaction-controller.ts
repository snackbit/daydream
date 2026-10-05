import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type RefObject,
} from 'react';
import type { LibraryRecordingDetail } from '@daydream/shared';
import {
  nextRedactionBoundaryDelayMs,
  nextUnredactedVideoTimelineMsForRedaction,
  REDACTION_START_GUARD_MS,
  redactedTracksAtTimelineMs,
  redactedVideoEndTimelineMs,
  setsEqual,
} from './recording-playback-redactions';

interface PlaybackRedactionControllerInput {
  applyRedactedAudioSettings: (activeRedactedTrackIds: Set<string>) => void;
  currentSegment: { playbackOffsetMs: number } | null;
  currentTimelineMs: number;
  detail: LibraryRecordingDetail;
  isPlaying: boolean;
  redactedTrackIdsRef: RefObject<Set<string>>;
  streamStartMs: number;
  totalDurationMs: number;
  videoRef: RefObject<HTMLVideoElement | null>;
}

export function usePlaybackRedactionController({
  applyRedactedAudioSettings,
  currentSegment,
  currentTimelineMs,
  detail,
  isPlaying,
  redactedTrackIdsRef,
  streamStartMs,
  totalDurationMs,
  videoRef,
}: PlaybackRedactionControllerInput) {
  const redactionBoundaryTimerRef = useRef<number | null>(null);
  const [redactedTrackIds, setRedactedTrackIds] = useState<Set<string>>(() => {
    const initialRedactions = redactedTracksAtTimelineMs(
      detail,
      currentTimelineMs,
      REDACTION_START_GUARD_MS,
    );
    redactedTrackIdsRef.current = initialRedactions;

    return initialRedactions;
  });
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
  const [redactionSkipTargetMs, setRedactionSkipTargetMs] = useState<
    number | null
  >(null);
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
      setRedactionSkipTargetMs(
        nextRedactedVideoActive
          ? redactedVideoEndTimelineMs(
              detail,
              timelineMs,
              videoTrackIds,
              totalDurationMs,
            )
          : null,
      );
      setNextUnredactedVideoTimelineMs((currentTarget) =>
        currentTarget === nextSkipTarget ? currentTarget : nextSkipTarget,
      );
      applyRedactedAudioSettings(nextRedactedTrackIds);
    },
    [
      applyRedactedAudioSettings,
      detail,
      redactedTrackIdsRef,
      totalDurationMs,
      videoTrackIds,
    ],
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
  }, [updateRedactionStateForTimeline, videoRef, videoTimelineMs]);

  const clearRedactionBoundaryTimer = useCallback(() => {
    if (redactionBoundaryTimerRef.current !== null) {
      window.clearTimeout(redactionBoundaryTimerRef.current);
      redactionBoundaryTimerRef.current = null;
    }
  }, []);

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
    videoRef,
    videoTimelineMs,
  ]);

  useEffect(() => clearRedactionBoundaryTimer, [clearRedactionBoundaryTimer]);

  return {
    nextUnredactedVideoTimelineMs,
    redactionSkipTargetMs,
    redactedVideoActive,
    updateRedactionStateForTimeline,
    updateRedactionStateFromVideo,
    videoTimelineMs,
  };
}
