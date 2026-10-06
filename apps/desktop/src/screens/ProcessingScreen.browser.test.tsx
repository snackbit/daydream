import type { AnchorHTMLAttributes, ReactNode } from 'react';
import type { BackgroundTask } from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import { useBackgroundTaskStore } from '@/features/processing/background-task-store';
import '@/index.css';
import { ProcessingScreen } from './ProcessingScreen';

const client = vi.hoisted(() => ({
  listBackgroundTasks: vi.fn(),
  getBackgroundTask: vi.fn(),
  retryProcessingUnits: vi.fn(),
  reprocessRecordingTranscription: vi.fn(),
  listenToBackgroundTasksChanged: vi.fn(),
}));
const ai = vi.hoisted(() => ({
  pauseProcessingJob: vi.fn(),
  resumeProcessingJob: vi.fn(),
  cancelProcessingJob: vi.fn(),
  downloadAiModel: vi.fn(),
  cancelAiModelDownload: vi.fn(),
}));
vi.mock('@/features/processing/processing-client', () => client);
vi.mock('@/features/ai/ai-client', () => ai);
vi.mock('@tanstack/react-router', () => ({
  Link: ({
    to,
    search,
    params,
    children,
    ...props
  }: AnchorHTMLAttributes<HTMLAnchorElement> & {
    to: string;
    search?: { tab: string; focus: string | null };
    params?: { recordingId: string };
    children: ReactNode;
  }) => (
    <a
      {...props}
      href={
        search
          ? `${to}?tab=${search.tab}&focus=${search.focus ?? ''}`
          : params
            ? `/library/${params.recordingId}`
            : to
      }
    >
      {children}
    </a>
  ),
}));

function task(patch: Partial<BackgroundTask> = {}): BackgroundTask {
  return {
    id: 'synthetic-job',
    kind: 'ai-processing',
    title: 'Synthetic recording',
    recordingId: 'synthetic-recording',
    modelId: null,
    status: 'running',
    phase: 'recognizing-text',
    progress: 0.45,
    elapsedMs: 3_000,
    etaMs: 5_000,
    nextRetryAtMs: null,
    inputOpen: false,
    errorCode: null,
    updatedAtMs: 3_000,
    stages: [
      {
        kind: 'ocr',
        total: 10,
        completed: 4,
        failed: 0,
        skipped: 0,
        running: 1,
        queued: 5,
        progress: 0.45,
      },
    ],
    ...patch,
  };
}
beforeEach(() => {
  vi.clearAllMocks();
  useBackgroundTaskStore.setState({
    tasks: [],
    nextCursor: null,
    status: 'idle',
    error: null,
    loading: false,
    refreshQueued: false,
  });
  client.listBackgroundTasks.mockResolvedValue({
    tasks: [task()],
    nextCursor: null,
  });
  client.retryProcessingUnits.mockResolvedValue({
    id: 'synthetic-job',
    status: 'queued',
  });
});
afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});

test('shows measured progress and allows pausing an active job', async () => {
  render(<ProcessingScreen />);
  const progress = await screen.findByRole('progressbar', {
    name: 'Synthetic recording progress',
  });
  expect(progress.getAttribute('aria-valuenow')).toBe('45');
  await fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
  await waitFor(() =>
    expect(ai.pauseProcessingJob).toHaveBeenCalledWith({
      jobId: 'synthetic-job',
    }),
  );
});

test('failed work links to its configuration and retries with current settings', async () => {
  client.listBackgroundTasks
    .mockResolvedValueOnce({
      tasks: [
        task({
          status: 'completed-with-errors',
          errorCode: 'ai_ocr_vision_response_invalid',
        }),
      ],
      nextCursor: null,
    })
    .mockResolvedValue({
      tasks: [
        task({
          status: 'queued',
          phase: 'queued',
          progress: 0,
          errorCode: null,
        }),
      ],
      nextCursor: null,
    });
  render(<ProcessingScreen />);
  const link = await screen.findByRole('link', {
    name: 'Review vision settings',
  });
  expect(link.getAttribute('href')).toBe('/settings?tab=ai&focus=vision');
  await fireEvent.click(
    screen.getByRole('button', { name: 'Retry with current settings' }),
  );
  await waitFor(() =>
    expect(client.retryProcessingUnits).toHaveBeenCalledWith({
      jobId: 'synthetic-job',
      useCurrentSettings: true,
    }),
  );
  await waitFor(() =>
    expect(
      screen.queryByRole('button', { name: 'Retry with current settings' }),
    ).toBeNull(),
  );
});

test('loading failures remain recoverable through refresh', async () => {
  client.listBackgroundTasks.mockRejectedValueOnce(
    new Error('Synthetic IPC disconnected'),
  );
  render(<ProcessingScreen />);
  await screen.findByRole('alert');
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await screen.findByRole('progressbar', {
    name: 'Synthetic recording progress',
  });
});

test('older recordings use the new speech detector only through explicit audio reprocessing', async () => {
  client.listBackgroundTasks.mockResolvedValue({
    tasks: [task({ status: 'succeeded', progress: 1 })],
    nextCursor: null,
  });
  render(<ProcessingScreen />);
  await fireEvent.click(
    await screen.findByRole('button', { name: 'Reprocess audio tracks' }),
  );
  await waitFor(() =>
    expect(client.reprocessRecordingTranscription).toHaveBeenCalledWith({
      recordingId: 'synthetic-recording',
    }),
  );
});

test('task details retry only the selected failed frame while completed sections stay available', async () => {
  const current = task({
    status: 'completed-with-errors',
    errorCode: 'ai_ocr_vision_response_invalid',
  });
  client.listBackgroundTasks.mockResolvedValue({
    tasks: [current],
    nextCursor: null,
  });
  client.getBackgroundTask.mockResolvedValue({
    task: current,
    units: [
      {
        id: 'failed-frame',
        chunkId: 'ocr-chunk',
        sourceKind: 'ocr',
        startOffsetMs: 1_000,
        endOffsetMs: null,
        status: 'failed',
        progress: 1,
        errorCode: 'ai_ocr_vision_response_invalid',
        attemptCount: 2,
      },
      {
        id: 'completed-frame',
        chunkId: 'ocr-chunk',
        sourceKind: 'ocr',
        startOffsetMs: 2_000,
        endOffsetMs: null,
        status: 'succeeded',
        progress: 1,
        errorCode: null,
        attemptCount: 1,
      },
    ],
  });
  render(<ProcessingScreen />);
  await fireEvent.click(await screen.findByRole('button', { name: 'Details' }));
  const sections = await screen.findByRole('list', {
    name: 'Processing sections',
  });
  expect(within(sections).getAllByRole('listitem')).toHaveLength(2);
  expect(
    within(sections).getAllByRole('button', { name: 'Retry section' }),
  ).toHaveLength(1);
  await fireEvent.click(
    within(sections).getByRole('button', { name: 'Retry section' }),
  );
  await waitFor(() =>
    expect(client.retryProcessingUnits).toHaveBeenCalledWith({
      jobId: 'synthetic-job',
      chunkId: 'ocr-chunk',
      unitId: 'failed-frame',
      useCurrentSettings: false,
    }),
  );
});

test.each([
  ['light', 1440],
  ['dark', 1440],
  ['light', 420],
  ['dark', 420],
] as const)(
  'processing recovery controls fit %s at %i pixels',
  async (theme, width) => {
    await page.viewport(width, 900);
    document.documentElement.dataset.theme = theme;
    client.listBackgroundTasks.mockResolvedValue({
      tasks: [
        task({
          status: 'completed-with-errors',
          errorCode: 'ai_ocr_vision_response_invalid',
          phase: 'completed-with-errors',
          etaMs: null,
          progress: 1,
        }),
      ],
      nextCursor: null,
    });
    render(<ProcessingScreen />);
    const retry = await screen.findByRole('button', {
      name: 'Retry with current settings',
    });
    const settings = screen.getByRole('link', {
      name: 'Review vision settings',
    });
    for (const control of [retry, settings]) {
      const bounds = control.getBoundingClientRect();
      expect(bounds.width).toBeGreaterThan(0);
      expect(bounds.right).toBeLessThanOrEqual(width);
    }
    expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(width);
    await page.screenshot();
  },
);
