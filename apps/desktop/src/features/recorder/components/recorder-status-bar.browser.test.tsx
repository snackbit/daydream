import type { RecorderPerformanceStats } from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { RecorderStatusBar } from './recorder-status-bar';

const client = vi.hoisted(() => ({
  getRecorderPerformanceStats: vi.fn(),
}));

vi.mock('@/features/recorder/recorder-client', () => client);

beforeEach(() => {
  client.getRecorderPerformanceStats.mockReset();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

test('recorder status bar renders idle recorder stats', async () => {
  client.getRecorderPerformanceStats.mockResolvedValue({
    desiredFps: null,
    segmentStartedAtMs: null,
    state: 'idle',
    timing: null,
    updatedAtMs: 1000,
  } satisfies RecorderPerformanceStats);

  render(<RecorderStatusBar />);

  expect(await screen.findByText('Recorder idle')).toBeDefined();
  expect(screen.getByText('FPS --')).toBeDefined();
  expect(screen.getByText('Capture cost --')).toBeDefined();
  expect(screen.getByLabelText('Recorder performance: Idle')).toBeDefined();
});

test('recorder status bar reports unavailable stats fetches', async () => {
  client.getRecorderPerformanceStats.mockRejectedValue(
    new Error('Stats unavailable'),
  );

  render(<RecorderStatusBar />);

  expect(await screen.findByText('Capture stats unavailable')).toBeDefined();
});

test('recorder status bar smooths alternating visible fps samples', async () => {
  const samples = [
    recordingStats({
      framesSentToWriter: 0,
      totalComposeDurationMs: 0,
      updatedAtMs: 0,
    }),
    recordingStats({
      framesSentToWriter: 2,
      totalComposeDurationMs: 5,
      updatedAtMs: 500,
    }),
    recordingStats({
      framesSentToWriter: 5,
      totalComposeDurationMs: 10,
      updatedAtMs: 1000,
    }),
    recordingStats({
      framesSentToWriter: 7,
      totalComposeDurationMs: 15,
      updatedAtMs: 1500,
    }),
    recordingStats({
      framesSentToWriter: 10,
      totalComposeDurationMs: 20,
      updatedAtMs: 2000,
    }),
    recordingStats({
      framesSentToWriter: 12,
      totalComposeDurationMs: 25,
      updatedAtMs: 2500,
    }),
    recordingStats({
      framesSentToWriter: 15,
      totalComposeDurationMs: 30,
      updatedAtMs: 3000,
    }),
  ];
  let sampleIndex = 0;
  client.getRecorderPerformanceStats.mockImplementation(() =>
    Promise.resolve(samples[Math.min(sampleIndex++, samples.length - 1)]),
  );

  render(<RecorderStatusBar />);

  expect(await screen.findByText('Recording')).toBeDefined();
  await waitFor(
    () =>
      expect(client.getRecorderPerformanceStats).toHaveBeenCalledTimes(
        samples.length,
      ),
    { timeout: 5000 },
  );

  expect(screen.getByText('FPS 5.0 / 5.0')).toBeDefined();
  expect(screen.getByLabelText('Recorder performance: On track')).toBeDefined();
});

function recordingStats({
  framesSentToWriter,
  totalComposeDurationMs,
  updatedAtMs,
}: {
  framesSentToWriter: number;
  totalComposeDurationMs: number;
  updatedAtMs: number;
}): RecorderPerformanceStats {
  return {
    desiredFps: 5,
    segmentStartedAtMs: 0,
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
      schedulerMissedDeadlines: 0,
      schedulerTicks: framesSentToWriter,
      sourceTimestampMaxMs: updatedAtMs,
      sourceTimestampMinMs: 0,
      totalComposeDurationMs,
    },
    updatedAtMs,
  };
}
