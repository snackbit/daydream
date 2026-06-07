import { useEffect, useRef, useState } from 'react';
import type { RecorderPerformanceStats, RecorderState } from '@daydream/shared';
import { getRecorderPerformanceStats } from '@/features/recorder/recorder-client';
import {
  deriveRecorderPerformanceDisplay,
  nextRecorderPerformanceStatsHistory,
  recorderPerformancePollIntervalMs,
  type RecorderPerformanceDisplay,
  type RecorderPerformanceLevel,
} from '@/features/recorder/recorder-performance';
import { cn } from '@/lib/utils';

const levelLabels = {
  idle: 'Idle',
  lagging: 'Slightly lagging',
  'on-track': 'On track',
  stalled: 'Severe lag or stalled',
} satisfies Record<RecorderPerformanceLevel, string>;

const levelClasses = {
  idle: 'bg-muted-foreground/60',
  lagging: 'bg-amber-500',
  'on-track': 'bg-emerald-500',
  stalled: 'bg-red-500',
} satisfies Record<RecorderPerformanceLevel, string>;

export function RecorderStatusBar() {
  const statsHistoryRef = useRef<RecorderPerformanceStats[]>([]);
  const [display, setDisplay] = useState<RecorderPerformanceDisplay>(() =>
    deriveRecorderPerformanceDisplay(null),
  );
  const [error, setError] = useState<string | null>(null);
  const [state, setState] = useState<RecorderState>('idle');

  useEffect(() => {
    let cancelled = false;
    let timeoutId: number | null = null;

    async function loadStats() {
      let nextState: RecorderState | null = null;
      try {
        const stats = await getRecorderPerformanceStats();
        if (cancelled) {
          return;
        }
        const nextDisplay = deriveRecorderPerformanceDisplay(
          stats,
          statsHistoryRef.current,
        );
        statsHistoryRef.current = nextRecorderPerformanceStatsHistory(
          stats,
          statsHistoryRef.current,
        );
        nextState = stats.state;
        setDisplay(nextDisplay);
        setError(null);
        setState(stats.state);
      } catch (loadError) {
        if (cancelled) {
          return;
        }
        setError(
          loadError instanceof Error ? loadError.message : 'Unavailable',
        );
      } finally {
        if (!cancelled) {
          timeoutId = window.setTimeout(
            loadStats,
            recorderPerformancePollIntervalMs(nextState),
          );
        }
      }
    }

    void loadStats();

    return () => {
      cancelled = true;
      if (timeoutId !== null) {
        window.clearTimeout(timeoutId);
      }
    };
  }, []);

  return (
    <div
      aria-label="Recorder status"
      className="flex w-full min-w-0 items-center justify-between gap-4"
    >
      <div className="flex min-w-0 items-center gap-2">
        <span
          aria-label={`Recorder performance: ${levelLabels[display.level]}`}
          className={cn(
            'size-2 shrink-0 rounded-full',
            levelClasses[display.level],
          )}
          title={levelLabels[display.level]}
        />
        <span className="truncate">{formatRecorderState(state)}</span>
      </div>

      <div className="flex shrink-0 items-center gap-4 tabular-nums">
        <span>{formatFps(display)}</span>
        <span>{formatCaptureCost(display.captureCostPercent, error)}</span>
      </div>
    </div>
  );
}

function formatRecorderState(state: RecorderState): string {
  switch (state) {
    case 'idle':
      return 'Recorder idle';
    case 'paused':
      return 'Recorder paused';
    case 'pausing':
      return 'Recorder pausing';
    case 'recording':
      return 'Recording';
    case 'selecting':
      return 'Recorder selecting source';
    case 'stopping':
      return 'Recorder stopping';
    case 'unavailable':
      return 'Recorder unavailable';
    case 'error':
      return 'Recorder error';
  }
}

function formatFps(display: RecorderPerformanceDisplay): string {
  if (display.desiredFps === null) {
    return 'FPS --';
  }
  if (display.actualFps === null) {
    return `FPS -- / ${formatNumber(display.desiredFps)}`;
  }

  return `FPS ${formatNumber(display.actualFps)} / ${formatNumber(
    display.desiredFps,
  )}`;
}

function formatCaptureCost(
  captureCostPercent: number | null,
  error: string | null,
): string {
  if (error !== null) {
    return 'Capture stats unavailable';
  }
  if (captureCostPercent === null) {
    return 'Capture cost --';
  }

  return `Capture cost ${Math.round(captureCostPercent)}%`;
}

function formatNumber(value: number): string {
  return value.toFixed(1);
}
