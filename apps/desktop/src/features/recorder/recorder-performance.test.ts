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
      framesSentToWriter: 20,
      totalComposeDurationMs: 400,
      updatedAtMs: 4000,
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
      framesSentToWriter: 17,
      schedulerMissedDeadlines: 1,
      totalComposeDurationMs: 100,
      updatedAtMs: 4000,
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
      totalComposeDurationMs: 3900,
      updatedAtMs: 4000,
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

test('recorder performance math keeps a baseline when polling samples are sparse', () => {
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

  expect(display.actualFps).toBeCloseTo(28 / 5.5);
});

test('recorder performance history resets when cumulative counters decrease', () => {
  const current = stats({
    framesSentToWriter: 0,
    segmentStartedAtMs: 2500,
    totalComposeDurationMs: 0,
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
  desiredFps = 5,
  framesSentToWriter,
  schedulerMissedDeadlines = 0,
  segmentStartedAtMs = 1000,
  totalComposeDurationMs,
  updatedAtMs,
}: {
  desiredFps?: number;
  framesSentToWriter: number;
  schedulerMissedDeadlines?: number;
  segmentStartedAtMs?: number;
  totalComposeDurationMs: number;
  updatedAtMs: number;
}): RecorderPerformanceStats {
  return {
    desiredFps,
    segmentStartedAtMs,
    state: 'recording',
    timing: {
      composedFrames: framesSentToWriter,
      cpuReadableFramesReceived: framesSentToWriter,
      dmabufFramesReadBackToCpu: 0,
      dmabufFramesReceived: 0,
      framesReceivedBySource: {
        'monitor-1': framesSentToWriter,
      },
      framesSentToWriter,
      lastComposedTimestampMs: updatedAtMs,
      latestFrameStorageBySource: {
        'monitor-1': 'cpu-readable',
      },
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

test('low FPS measures four frame intervals and rollover preserves cumulative history', () => {
  const previous = stats({
    desiredFps: 0.2,
    framesSentToWriter: 0,
    totalComposeDurationMs: 0,
    updatedAtMs: 0,
  });
  expect(
    deriveRecorderPerformanceDisplay(
      stats({
        desiredFps: 0.2,
        framesSentToWriter: 3,
        totalComposeDurationMs: 30,
        updatedAtMs: 15_000,
      }),
      [previous],
    ).actualFps,
  ).toBeNull();
  const current = stats({
    desiredFps: 0.2,
    framesSentToWriter: 4,
    totalComposeDurationMs: 40,
    updatedAtMs: 20_000,
    segmentStartedAtMs: 10_000,
  });
  expect(deriveRecorderPerformanceDisplay(current, [previous]).actualFps).toBe(
    0.2,
  );
  expect(nextRecorderPerformanceStatsHistory(current, [previous])).toEqual([
    previous,
    current,
  ]);
});
