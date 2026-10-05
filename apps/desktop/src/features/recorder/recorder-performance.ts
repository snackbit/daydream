import type { RecorderPerformanceStats, RecorderState } from '@daydream/shared';

export type RecorderPerformanceLevel =
  'idle' | 'lagging' | 'on-track' | 'stalled';

export interface RecorderPerformanceDisplay {
  actualFps: number | null;
  captureCostPercent: number | null;
  desiredFps: number | null;
  fpsRatio: number | null;
  level: RecorderPerformanceLevel;
}

export const RECORDER_PERFORMANCE_SMOOTHING_WINDOW_MS = 3000;

const EVALUATED_RECORDING_STATES = new Set<RecorderState>([
  'pausing',
  'recording',
  'stopping',
]);

export function recorderPerformancePollIntervalMs(
  state: RecorderState | null,
): number {
  return state !== null && EVALUATED_RECORDING_STATES.has(state) ? 500 : 2000;
}

export function deriveRecorderPerformanceDisplay(
  current: RecorderPerformanceStats | null,
  history: RecorderPerformanceStats[] = [],
): RecorderPerformanceDisplay {
  const desiredFps = current?.desiredFps ?? null;
  if (
    current === null ||
    desiredFps === null ||
    desiredFps <= 0 ||
    !EVALUATED_RECORDING_STATES.has(current.state)
  ) {
    return idleDisplay(desiredFps);
  }

  if (current.timing === null) {
    return {
      actualFps: null,
      captureCostPercent: null,
      desiredFps,
      fpsRatio: null,
      level: 'stalled',
    };
  }

  const smoothedHistory = nextRecorderPerformanceStatsHistory(current, history);
  const previous = oldestPreviousStats(current, smoothedHistory);

  if (previous === null) {
    return {
      actualFps: null,
      captureCostPercent: null,
      desiredFps,
      fpsRatio: null,
      level: 'idle',
    };
  }
  const previousTiming = previous.timing;
  if (previousTiming === null) {
    return idleDisplay(desiredFps);
  }

  const elapsedMs = current.updatedAtMs - previous.updatedAtMs;
  if (elapsedMs <= 0) {
    return idleDisplay(desiredFps);
  }

  const sentFrameDelta = Math.max(
    0,
    current.timing.framesSentToWriter - previousTiming.framesSentToWriter,
  );
  const missedDeadlineDelta = Math.max(
    0,
    current.timing.schedulerMissedDeadlines -
      previousTiming.schedulerMissedDeadlines,
  );
  const composeDurationDeltaMs = Math.max(
    0,
    current.timing.totalComposeDurationMs -
      previousTiming.totalComposeDurationMs,
  );

  const actualFps = (sentFrameDelta * 1000) / elapsedMs;
  const fpsRatio = actualFps / desiredFps;
  const captureCostPercent = (composeDurationDeltaMs / elapsedMs) * 100;
  const frameIntervalMs = 1000 / desiredFps;
  const noRecentFrames =
    sentFrameDelta === 0 && elapsedMs >= Math.max(2000, frameIntervalMs * 4);

  let level: RecorderPerformanceLevel = 'on-track';
  if (noRecentFrames || fpsRatio < 0.5 || captureCostPercent >= 100) {
    level = 'stalled';
  } else if (
    fpsRatio < 0.9 ||
    missedDeadlineDelta > 0 ||
    captureCostPercent >= 70
  ) {
    level = 'lagging';
  }

  return {
    actualFps,
    captureCostPercent,
    desiredFps,
    fpsRatio,
    level,
  };
}

export function nextRecorderPerformanceStatsHistory(
  current: RecorderPerformanceStats,
  history: RecorderPerformanceStats[],
): RecorderPerformanceStats[] {
  if (!canSmoothStats(current)) {
    return [];
  }

  const windowStartedAtMs =
    current.updatedAtMs - RECORDER_PERFORMANCE_SMOOTHING_WINDOW_MS;
  const retainedHistory = history.filter(
    (stats) =>
      canSmoothStats(stats) &&
      stats.updatedAtMs >= windowStartedAtMs &&
      stats.updatedAtMs <= current.updatedAtMs &&
      stats.desiredFps === current.desiredFps &&
      stats.segmentStartedAtMs === current.segmentStartedAtMs &&
      stats.state === current.state,
  );

  return [...retainedHistory, current];
}

function idleDisplay(desiredFps: number | null): RecorderPerformanceDisplay {
  return {
    actualFps: null,
    captureCostPercent: null,
    desiredFps,
    fpsRatio: null,
    level: 'idle',
  };
}

function canSmoothStats(stats: RecorderPerformanceStats): boolean {
  return (
    stats.desiredFps !== null &&
    stats.desiredFps > 0 &&
    EVALUATED_RECORDING_STATES.has(stats.state) &&
    stats.segmentStartedAtMs !== null &&
    stats.timing !== null
  );
}

function oldestPreviousStats(
  current: RecorderPerformanceStats,
  history: RecorderPerformanceStats[],
): RecorderPerformanceStats | null {
  return (
    history.find(
      (stats) =>
        stats !== current &&
        stats.timing !== null &&
        stats.updatedAtMs < current.updatedAtMs,
    ) ?? null
  );
}
