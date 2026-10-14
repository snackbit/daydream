import { useCallback, useEffect, useRef, type SyntheticEvent } from 'react';
import type { RefObject } from 'react';
import type { LibraryRecordingDetail } from '@daydream/shared';
import type { PlaybackSpeed } from '../controls/playback-speed';
import type { usePlaybackResumeTimer } from './recording-playback-resume-timer';

export interface PlaybackSegmentLifecycleInput extends Pick<
  ReturnType<typeof usePlaybackResumeTimer>,
  'clearResumeTimer' | 'startResumeTimer' | 'setResumeTimerPaused'
> {
  currentSegment: LibraryRecordingDetail['segments'][number] | null;
  currentIndex: number;
  streamRevision: number;
  streamStartMs: number;
  playbackRate: PlaybackSpeed;
  isPlaying: boolean;
  resumeProgress: number | null;
  shortGaps: boolean;
  playbackIntentRef: RefObject<boolean>;
  advanceToSegment: (index: number, shouldPlay: boolean) => void;
  replaceStream: (shouldPlay: boolean) => void;
  isCurrentMedia: (element: HTMLMediaElement) => boolean;
  setCurrentTime: (seconds: number) => void;
  setPlaybackEndedAtEnd: (ended: boolean) => void;
  clockTime: () => number;
  segmentCount: number;
}

export function usePlaybackSegmentLifecycle({
  currentSegment,
  currentIndex,
  streamRevision,
  streamStartMs,
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
  segmentCount,
}: PlaybackSegmentLifecycleInput) {
  const completedStream = useRef<string | null>(null);
  const finishCurrentSegment = useCallback(() => {
    if (!currentSegment || resumeProgress !== null) {
      return;
    }
    const key = `${currentSegment.id}:${streamRevision}`;
    if (completedStream.current === key) return;
    completedStream.current = key;

    const nextIndex = currentIndex + 1;
    if (nextIndex >= segmentCount) {
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
    segmentCount,
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
    if (nextIndex < segmentCount) {
      advanceToSegment(nextIndex, playbackIntentRef.current);
    }
  }

  return { handleEnded, playNowAfterPause };
}
