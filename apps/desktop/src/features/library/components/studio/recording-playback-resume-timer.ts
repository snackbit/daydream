import { useCallback, useEffect, useEffectEvent, useRef } from 'react';

interface PlaybackResumeTimerInput {
  setResumeProgress: (progress: number | null) => void;
  playbackRate?: number;
}

export function usePlaybackResumeTimer({
  setResumeProgress,
  playbackRate = 1,
}: PlaybackResumeTimerInput) {
  const resumeTimerRef = useRef<number | null>(null);
  const gap = useRef<{
    duration: number;
    elapsed: number;
    sampledAt: number;
    rate: number;
    complete: () => void;
  } | null>(null);

  const clearResumeTimer = useCallback(() => {
    if (resumeTimerRef.current !== null) {
      window.clearInterval(resumeTimerRef.current);
      resumeTimerRef.current = null;
    }
    gap.current = null;
  }, []);

  const startResumeTimer = useCallback(
    (pauseGapMs: number, onComplete: () => void) => {
      setResumeProgress(0);
      clearResumeTimer();
      gap.current = {
        duration: pauseGapMs,
        elapsed: 0,
        sampledAt: performance.now(),
        rate: playbackRate,
        complete: onComplete,
      };
      resumeTimerRef.current = window.setInterval(() => {
        const active = gap.current;
        if (!active) return;
        const now = performance.now();
        active.elapsed += (now - active.sampledAt) * active.rate;
        active.sampledAt = now;
        const progress = Math.min(1, active.elapsed / active.duration);
        setResumeProgress(progress);
        if (progress >= 1) {
          clearResumeTimer();
          active.complete();
        }
      }, 50);
    },
    [clearResumeTimer, setResumeProgress, playbackRate],
  );

  const changeRate = useEffectEvent(() => {
    const active = gap.current;
    if (!active) return;
    const now = performance.now();
    active.elapsed += (now - active.sampledAt) * active.rate;
    active.sampledAt = now;
    active.rate = playbackRate;
  });
  useEffect(() => {
    changeRate();
  }, [playbackRate]);

  useEffect(() => clearResumeTimer, [clearResumeTimer]);

  return {
    clearResumeTimer,
    startResumeTimer,
  };
}
