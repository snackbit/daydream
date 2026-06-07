import type { RecorderPerformanceStats } from '@daydream/shared';
import { expect, test } from 'vitest';
import {
  deriveRecorderPerformanceDisplay,
  nextRecorderPerformanceStatsHistory,
  recorderPerformancePollIntervalMs,
} from './recorder-performance';

test('recorder performance math reports on-track recording stats', () => {
  const previous = stats({
    framesSentToWriter: 5,
    totalComposeDurationMs: 100,
    updatedAtMs: 1000,
  });
  const display = deriveRecorderPerformanceDisplay(
    stats({
      framesSentToWriter: 10,
      totalComposeDurationMs: 200,
      updatedAtMs: 2000,
    }),
    [previous],
  );

  expect(display.actualFps).toBe(5);
  expect(display.captureCostPercent).toBe(10);
  expect(display.desiredFps).toBe(5);
  expect(display.level).toBe('on-track');
});

test('recorder performance math flags slight lag from missed deadlines', () => {
  const previous = stats({
    framesSentToWriter: 5,
    schedulerMissedDeadlines: 0,
    totalComposeDurationMs: 0,
    updatedAtMs: 1000,
  });
  const display = deriveRecorderPerformanceDisplay(
    stats({
      framesSentToWriter: 9,
      schedulerMissedDeadlines: 1,
      totalComposeDurationMs: 100,
      updatedAtMs: 2000,
    }),
    [previous],
  );

  expect(display.actualFps).toBe(4);
  expect(display.level).toBe('lagging');
});

test('recorder performance math flags severe stalls', () => {
  const previous = stats({
    framesSentToWriter: 5,
    totalComposeDurationMs: 0,
    updatedAtMs: 1000,
  });
  const display = deriveRecorderPerformanceDisplay(
    stats({
      framesSentToWriter: 5,
      totalComposeDurationMs: 2600,
      updatedAtMs: 3000,
    }),
    [previous],
  );

  expect(display.actualFps).toBe(0);
  expect(display.captureCostPercent).toBe(130);
  expect(display.level).toBe('stalled');
});

test('recorder performance math smooths alternating frame deltas over three seconds', () => {
  const history = [
    stats({ framesSentToWriter: 0, totalComposeDurationMs: 0, updatedAtMs: 0 }),
    stats({
      framesSentToWriter: 2,
      totalComposeDurationMs: 5,
      updatedAtMs: 500,
    }),
    stats({
      framesSentToWriter: 5,
      totalComposeDurationMs: 10,
      updatedAtMs: 1000,
    }),
    stats({
      framesSentToWriter: 7,
      totalComposeDurationMs: 15,
      updatedAtMs: 1500,
    }),
    stats({
      framesSentToWriter: 10,
      totalComposeDurationMs: 20,
      updatedAtMs: 2000,
    }),
    stats({
      framesSentToWriter: 12,
      totalComposeDurationMs: 25,
      updatedAtMs: 2500,
    }),
  ];

  const display = deriveRecorderPerformanceDisplay(
    stats({
      framesSentToWriter: 15,
      totalComposeDurationMs: 30,
      updatedAtMs: 3000,
    }),
    history,
  );

  expect(display.actualFps).toBe(5);
  expect(display.level).toBe('on-track');
});

test('recorder performance math ignores samples older than the smoothing window', () => {
  const display = deriveRecorderPerformanceDisplay(
    stats({
      framesSentToWriter: 28,
      totalComposeDurationMs: 30,
      updatedAtMs: 5500,
    }),
    [
      stats({
        framesSentToWriter: 0,
        totalComposeDurationMs: 0,
        updatedAtMs: 0,
      }),
      stats({
        framesSentToWriter: 25,
        totalComposeDurationMs: 25,
        updatedAtMs: 5000,
      }),
    ],
  );

  expect(display.actualFps).toBe(6);
});

test('recorder performance history resets when the active segment changes', () => {
  const current = stats({
    framesSentToWriter: 12,
    segmentStartedAtMs: 2500,
    totalComposeDurationMs: 120,
    updatedAtMs: 2500,
  });
  const history = [
    stats({
      framesSentToWriter: 10,
      segmentStartedAtMs: 1000,
      totalComposeDurationMs: 100,
      updatedAtMs: 2000,
    }),
  ];

  const display = deriveRecorderPerformanceDisplay(current, history);
  const nextHistory = nextRecorderPerformanceStatsHistory(current, history);

  expect(display.actualFps).toBeNull();
  expect(display.level).toBe('idle');
  expect(nextHistory).toEqual([current]);
});

test('recorder performance polling slows down outside active recording states', () => {
  expect(recorderPerformancePollIntervalMs('recording')).toBe(500);
  expect(recorderPerformancePollIntervalMs('idle')).toBe(2000);
  expect(recorderPerformancePollIntervalMs(null)).toBe(2000);
});

function stats({
  framesSentToWriter,
  schedulerMissedDeadlines = 0,
  segmentStartedAtMs = 1000,
  totalComposeDurationMs,
  updatedAtMs,
}: {
  framesSentToWriter: number;
  schedulerMissedDeadlines?: number;
  segmentStartedAtMs?: number;
  totalComposeDurationMs: number;
  updatedAtMs: number;
}): RecorderPerformanceStats {
  return {
    desiredFps: 5,
    segmentStartedAtMs,
    state: 'recording',
    timing: {
      composedFrames: framesSentToWriter,
      framesReceivedBySource: {
        'monitor-1': framesSentToWriter,
      },
      framesSentToWriter,
      lastComposedTimestampMs: updatedAtMs,
      maxComposeDurationMs: 20,
      reusedFrameCompositions: framesSentToWriter,
      schedulerMissedDeadlines,
      schedulerTicks: framesSentToWriter,
      sourceTimestampMaxMs: updatedAtMs,
      sourceTimestampMinMs: 1000,
      totalComposeDurationMs,
    },
    updatedAtMs,
  };
}
