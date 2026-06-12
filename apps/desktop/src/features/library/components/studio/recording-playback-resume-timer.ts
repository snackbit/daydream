import { useCallback, useEffect, useRef } from 'react';

interface PlaybackResumeTimerInput {
  setResumeProgress: (progress: number | null) => void;
}

export function usePlaybackResumeTimer({
  setResumeProgress,
}: PlaybackResumeTimerInput) {
  const resumeTimerRef = useRef<number | null>(null);

  const clearResumeTimer = useCallback(() => {
    if (resumeTimerRef.current !== null) {
      window.clearInterval(resumeTimerRef.current);
      resumeTimerRef.current = null;
    }
  }, []);

  const startResumeTimer = useCallback(
    (pauseGapMs: number, onComplete: () => void) => {
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
          onComplete();
        }
      }, 50);
    },
    [clearResumeTimer, setResumeProgress],
  );

  useEffect(() => clearResumeTimer, [clearResumeTimer]);

  return {
    clearResumeTimer,
    startResumeTimer,
  };
}
