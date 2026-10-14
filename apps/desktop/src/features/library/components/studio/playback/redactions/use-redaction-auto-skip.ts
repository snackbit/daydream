import { useCallback, useEffect, useEffectEvent, useState } from 'react';

const COUNTDOWN_MS = 3_000;

export function usePlaybackVideoRedactionSkip({
  recordingId,
  targetMs,
  timelineMs,
  isPlaying,
  seekRevision,
  seek,
}: {
  recordingId: string;
  targetMs: number | null;
  timelineMs: number;
  isPlaying: boolean;
  seekRevision: number;
  seek: (target: number, options?: { resume?: boolean }) => void;
}) {
  const autoSkip = useCallback(
    (target: number) => seek(target, { resume: true }),
    [seek],
  );
  const manualSkip = useCallback(() => {
    if (targetMs !== null) seek(targetMs);
  }, [seek, targetMs]);
  const seconds = useRedactionAutoSkip({
    recordingId,
    isPlaying,
    timelineMs,
    targetMs,
    seekRevision,
    onSkip: autoSkip,
  });
  return { targetMs, seconds, manualSkip };
}

/** The target identifies one continuous masked video span. Pausing or seeking
 * cancels its timer; normal media updates must not restart the countdown. */
export function useRedactionAutoSkip({
  recordingId = '',
  isPlaying,
  timelineMs,
  targetMs,
  seekRevision,
  onSkip,
}: {
  recordingId?: string;
  isPlaying: boolean;
  timelineMs: number;
  targetMs: number | null;
  seekRevision: number;
  onSkip: (targetMs: number) => void;
}) {
  const completeSkip = useEffectEvent(onSkip);
  const [countdown, setCountdown] = useState<{
    recordingId: string;
    targetMs: number;
    seekRevision: number;
    seconds: number;
  } | null>(null);
  const enabled =
    isPlaying &&
    targetMs !== null &&
    (targetMs - timelineMs > COUNTDOWN_MS ||
      (countdown?.targetMs === targetMs &&
        countdown.recordingId === recordingId &&
        countdown.seekRevision === seekRevision));
  useEffect(() => {
    if (!enabled || targetMs === null) return;
    const target = targetMs;
    setCountdown({ recordingId, targetMs: target, seekRevision, seconds: 3 });
    const interval = window.setInterval(
      () =>
        setCountdown((current) =>
          current &&
          current.recordingId === recordingId &&
          current.targetMs === target &&
          current.seekRevision === seekRevision
            ? { ...current, seconds: Math.max(1, current.seconds - 1) }
            : current,
        ),
      1_000,
    );
    const timeout = window.setTimeout(() => completeSkip(target), COUNTDOWN_MS);
    return () => {
      window.clearInterval(interval);
      window.clearTimeout(timeout);
    };
  }, [enabled, recordingId, seekRevision, targetMs]);
  return enabled &&
    countdown?.targetMs === targetMs &&
    countdown.recordingId === recordingId &&
    countdown.seekRevision === seekRevision
    ? countdown.seconds
    : null;
}
