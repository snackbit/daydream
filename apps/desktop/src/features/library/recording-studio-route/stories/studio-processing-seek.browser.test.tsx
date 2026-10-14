import type { ReactNode } from 'react';
import type * as TauriCore from '@tauri-apps/api/core';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { useSettingsStore } from '@/features/settings/settings-store';
import '@/index.css';
import { RecordingStudioScreen } from '@/screens/RecordingStudioScreen';
import { useStudioResultWindowStore } from '../../components/studio/results/studio-result-window-store';
import {
  workflowAssets,
  workflowDetail,
  workflowSegment,
} from '../../components/studio/testing/studio-workflow-fixture';
import {
  chunk,
  state,
  visualFrame,
} from '../../components/studio/timeline/processing/processing-test-fixtures';
import { useStudioTimelineStore } from '../../components/studio/timeline/state/studio-timeline-store';

const native = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: native.listen }));
vi.mock('@tanstack/react-router', () => ({
  Link: ({ children, to }: { children: ReactNode; to: string }) => (
    <a href={to}>{children}</a>
  ),
  useNavigate: () => vi.fn(),
  useParams: () => ({ recordingId: 'studio-story' }),
  useSearch: () => ({ evidenceId: null, source: null, seek: 175000 }),
}));
afterEach(() => {
  cleanup();
  useSettingsStore.setState(useSettingsStore.getInitialState(), true);
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
  useStudioResultWindowStore.setState(
    useStudioResultWindowStore.getInitialState(),
    true,
  );
  vi.resetAllMocks();
});

test('seeking across Review pages keeps combined screen indicator boundaries and operation labels fixed without starting work', async () => {
  await page.viewport(1440, 960);
  const detail = {
    ...workflowDetail,
    durationMs: 600000,
    endedAtMs: 601000,
    segments: [workflowSegment({ durationMs: 600000, endedAtMs: 601000 })],
  };
  // The short, synthetic media represents the native stream starting at each
  // seek offset; this workflow verifies read/display behavior, not decoding.
  const assets = { ...workflowAssets(detail), canSeekInStream: false };
  const processing = {
    ...state([
      chunk({
        id: 'ocr',
        recordingId: detail.id,
        segmentId: 'segment',
        sourceKind: 'ocr',
        trackId: 'video-0',
        endOffsetMs: 600000,
        status: 'running',
      }),
      chunk({
        id: 'vlm',
        recordingId: detail.id,
        segmentId: 'segment',
        sourceKind: 'vlm',
        trackId: 'video-0',
        startOffsetMs: 180000,
        endOffsetMs: 300000,
      }),
    ]),
    recordingId: detail.id,
    transcript: { recordingId: detail.id, tracks: [] },
    visualFrames: [
      visualFrame({
        id: 'done-start',
        recordingId: detail.id,
        segmentId: 'segment',
      }),
      visualFrame({
        id: 'running',
        recordingId: detail.id,
        segmentId: 'segment',
        frameTimestampMs: 180000,
        status: 'running',
        progress: 0.5,
      }),
      visualFrame({
        id: 'done-later',
        recordingId: detail.id,
        segmentId: 'segment',
        frameTimestampMs: 300000,
      }),
      visualFrame({
        id: 'queued',
        recordingId: detail.id,
        segmentId: 'segment',
        frameTimestampMs: 580000,
        status: 'queued',
        progress: 0,
      }),
    ],
  };
  processing.job = { ...processing.job!, recordingId: detail.id };
  native.listen.mockResolvedValue(() => undefined);
  // Only the native boundary is substituted; keep the screen, controller,
  // typed client, visibility, window publication and range model real.
  native.invoke.mockImplementation(async (command: string) => {
    switch (command) {
      case 'getLibraryRecording':
        return detail;
      case 'createLibraryPlaybackSession':
        return assets;
      case 'closeLibraryPlaybackSession':
        return;
      case 'getRecordingAiProcessingState':
        return structuredClone(processing);
      case 'getRecordingActivity':
        return {
          recordingId: detail.id,
          captureStates: [],
          events: [],
          intervals: [],
          windowSnapshots: [],
        };
      case 'getRecordingVisualObservations':
        return {
          recordingId: detail.id,
          ocrRegions: [],
          vlmObservations: [],
          vlmState: {
            status: 'disabled',
            providerStatus: 'disabled',
            errorCode: null,
          },
        };
      case 'getOcrTimelineFrames':
        return { recordingId: detail.id, frames: [], nextCursor: null };
      case 'getRecordingWaveformTiles':
        return { recordingId: detail.id, binMs: 10, tracks: [], error: null };
      default:
        throw new Error(`Unexpected native boundary: ${command}`);
    }
  });
  useSettingsStore.setState({
    loadStatus: 'ready',
    draft: null,
    settingsState: null,
  });
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  useStudioTimelineStore.getState().setTimeRangeSelection({
    laneIds: ['video-0'],
    startMs: 160000,
    endMs: 200000,
  });
  render(<RecordingStudioScreen />);
  await userEvent.click(await screen.findByRole('button', { name: 'Zoom In' }));
  await userEvent.click(screen.getByRole('tab', { name: 'Review' }));
  const label = 'Screen · OCR · Processing · Screen observations · Queued';
  await screen.findByRole('progressbar', { name: label });
  const reads = () =>
    native.invoke.mock.calls.filter(
      ([command]) => command === 'getRecordingAiProcessingState',
    );
  await waitFor(() =>
    expect(reads().at(-1)?.[1].input.bucketMs).toBeUndefined(),
  );
  const indicator = () => {
    const bar = screen.getByRole('progressbar', { name: label });
    const range = bar.closest('[data-processing-range]');
    if (!(range instanceof HTMLElement))
      throw new Error('Processing range missing');
    return {
      label: bar.getAttribute('aria-label'),
      status: bar.getAttribute('aria-valuetext'),
      title: range.title,
      left: range.style.left,
      width: range.style.width,
    };
  };
  const before = indicator();
  expect(before.title).toBe(`${label} · 3:00–5:00`);
  const viewport = { ...useStudioResultWindowStore.getState().range };
  const slider = screen.getByRole('slider', { name: 'Playback position' });
  await waitFor(() =>
    expect(slider).toHaveAttribute('aria-valuenow', '175000'),
  );
  slider.focus();
  await userEvent.keyboard('{ArrowRight}');
  await waitFor(() =>
    expect(reads().at(-1)?.[1].input.reviewStartOffsetMs).toBe(90000),
  );
  expect(slider).toHaveAttribute('aria-valuenow', '180000');
  expect(indicator()).toEqual(before);
  expect(useStudioResultWindowStore.getState().range).toEqual(viewport);
  await userEvent.keyboard('{ArrowLeft}');
  await waitFor(() =>
    expect(reads().at(-1)?.[1].input.reviewStartOffsetMs).toBe(0),
  );
  expect(indicator()).toEqual(before);
  expect(slider).toHaveAttribute('aria-valuenow', '175000');
  expect(useStudioResultWindowStore.getState().range).toEqual(viewport);
  expect(native.invoke.mock.calls.map(([command]) => command)).not.toContain(
    'startRecordingAiProcessing',
  );
  expect(native.invoke.mock.calls.map(([command]) => command)).not.toContain(
    'retryProcessingJob',
  );
  await page.screenshot({ path: '__screenshots__/studio-processing-seek.png' });
});
