import type {
  OcrDocument,
  OcrHistoryPage,
  ProcessingJob,
} from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { useSettingsStore } from '@/features/settings/settings-store';
import {
  defaultSettingsState,
  scene,
} from '@/features/settings/testing/settings-browser-fixtures';
import '@/index.css';
import { VisualSourceInspector } from '../details/visual-source-inspector';
import { OcrDocumentInspector } from './ocr-document-inspector';

const ipc = vi.hoisted(() => ({
  invoke: vi.fn(),
  copy: vi.fn(),
  listen: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: ipc.listen }));
vi.mock('@tanstack/react-router', () => ({
  Link: ({ children, to }: { children: React.ReactNode; to: string }) => (
    <a href={to}>{children}</a>
  ),
}));
const words = Array.from({ length: 500 }, (_, index) => ({
  confidence: index === 42 ? 0.3 : 0.95,
  hierarchy: null,
  id: `word-${index}`,
  rawRegionId: `raw-${index}`,
  region: {
    x: (index % 25) * 20,
    y: Math.floor(index / 25) * 20,
    width: 18,
    height: 18,
  },
  text: `Word${index}`,
}));
const document: OcrDocument = {
  blocks: Array.from({ length: 20 }, (_, order) => ({
    confidence: null,
    id: `block-${order}`,
    order,
    rawRegionIds: [],
    region: null,
    text: words
      .slice(order * 25, order * 25 + 25)
      .map((word) => word.text)
      .join(' '),
    words: words.slice(order * 25, order * 25 + 25),
  })),
  cacheHit: false,
  configFingerprint: 'synthetic-config',
  contentId: 'content-1',
  engine: 'tesseract',
  errorCode: null,
  frameTimestampMs: 500,
  height: 1080,
  id: 'document-1',
  model: 'tesseract',
  observedEndMs: 2_500,
  observedStartMs: 500,
  occurrenceCount: 3,
  provenance: 'native',
  provider: 'tesseract',
  recordingId: 'recording-1',
  segmentId: 'segment-1',
  sourceFrameRef: 'opaque-frame-reference',
  sourceTrackId: 'video-0',
  status: 'succeeded',
  supportTimestampsMs: [500, 1_500, 2_500],
  text: Array.from({ length: 20 }, (_, index) =>
    words
      .slice(index * 25, index * 25 + 25)
      .map((word) => word.text)
      .join(' '),
  ).join('\n\n'),
  width: 1920,
};
const frame = {
  endTimestampMs: 1_000,
  frameTimestampMs: 500,
  id: 'frame:500',
  ocrRegions: words.map((word) => ({
    chunkId: 'chunk-1',
    confidence: word.confidence,
    frameTimestampMs: 500,
    id: word.id,
    jobId: 'job-1',
    model: 'tesseract',
    provider: 'tesseract',
    recordingId: 'recording-1',
    region: word.region,
    sourceFrameRef: 'opaque-frame-reference',
    text: word.text,
  })),
  sourceFrameRef: 'opaque-frame-reference',
  sourceFrameRefs: ['opaque-frame-reference'],
  vlmObservations: [],
};
const context = {
  debugEnabled: false,
  privacyRevision: 'original',
  recordingId: 'recording-1',
  sourceTrackId: 'video-0',
};
beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  useSettingsStore.setState(useSettingsStore.getInitialState(), true);
  ipc.listen.mockResolvedValue(() => undefined);
  ipc.copy.mockResolvedValue(undefined);
  const navigatorMock = Object.create(navigator);
  Object.defineProperty(navigatorMock, 'clipboard', {
    value: { writeText: ipc.copy },
  });
  vi.stubGlobal('navigator', navigatorMock);
  ipc.invoke.mockImplementation((command: string) =>
    command === 'getAppSettings'
      ? Promise.resolve(structuredClone(defaultSettingsState))
      : command === 'getRecorderScene'
        ? Promise.resolve(structuredClone(scene))
        : command === 'getOcrDocument'
          ? Promise.resolve({
              document,
              recordingId: context.recordingId,
              job: null,
              sourceTrackId: context.sourceTrackId,
            })
          : command === 'getOcrHistory'
            ? Promise.resolve({
                items: [document],
                nextCursor: null,
              } satisfies OcrHistoryPage)
            : Promise.reject(new Error(`Unexpected command ${command}`)),
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

test.each([false, true])(
  'dense OCR stays internal with debug=%s while frame navigation works',
  async (debugEnabled) => {
    await page.viewport(620, 960);
    const seek = vi.fn();
    render(
      <VisualSourceInspector
        context={{ ...context, debugEnabled }}
        frame={frame}
        onClear={vi.fn()}
        onSeek={seek}
      />,
    );
    await screen.findByText(/Screen recognition complete/);
    expect(screen.queryByText(/Word42/)).toBeNull();
    expect(screen.queryByRole('button', { name: 'Read OCR text' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Copy OCR text' })).toBeNull();
    expect(screen.queryByRole('tab', { name: 'Changes' })).toBeNull();
    expect(
      screen.queryByRole('button', { name: 'Show raw OCR details' }),
    ).toBeNull();
    await userEvent.click(
      screen.getByRole('button', { name: 'Jump to frame' }),
    );
    expect(seek).toHaveBeenCalledWith(500);
    expect(
      ipc.invoke.mock.calls.some(([command]) => command === 'getOcrHistory'),
    ).toBe(false);
    expect(ipc.copy).not.toHaveBeenCalled();
  },
);

test('privacy invalidation discards late recognition metadata and exposes a failed read', async () => {
  let resolve: (value: unknown) => void = () => {
    throw new Error('Not initialized');
  };
  ipc.invoke
    .mockImplementationOnce(
      () =>
        new Promise((next) => {
          resolve = next;
        }),
    )
    .mockRejectedValue(new Error('Synthetic withheld source'));
  const view = render(
    <VisualSourceInspector
      context={context}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  await waitFor(() => expect(ipc.invoke).toHaveBeenCalledTimes(1));
  view.rerender(
    <VisualSourceInspector
      context={{ ...context, privacyRevision: 'cut' }}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  resolve({
    document,
    job: null,
    recordingId: context.recordingId,
    sourceTrackId: context.sourceTrackId,
  });
  await screen.findByRole('alert');
  expect(screen.queryByText(/Screen recognition complete/)).toBeNull();
  expect(screen.queryByText(/Word42/)).toBeNull();
});

test('a failed Vision document retries processing and preserves safe setup guidance', async () => {
  ipc.invoke.mockImplementation((command: string) =>
    command === 'getAppSettings'
      ? Promise.resolve(structuredClone(defaultSettingsState))
      : command === 'getRecorderScene'
        ? Promise.resolve(structuredClone(scene))
        : command === 'reprocessRecordingOcr'
          ? Promise.reject({
              code: 'ai_ocr_vision_unconfigured',
              message: 'Synthetic private provider body',
            })
          : Promise.resolve({
              document: {
                ...document,
                status: 'failed',
                engine: 'vision',
                errorCode: 'ai_ocr_vision_unconfigured',
              },
              job: null,
              recordingId: context.recordingId,
              sourceTrackId: context.sourceTrackId,
            }),
  );
  render(
    <VisualSourceInspector
      context={context}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  await screen.findAllByRole('alert');
  await userEvent.click(
    screen.getByRole('button', { name: 'Retry processing' }),
  );
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Retry processing' }),
    ).toBeEnabled(),
  );
  expect(
    screen.getByRole('link', { name: 'Review vision settings' }),
  ).toBeTruthy();
  expect(screen.queryByText('Synthetic private provider body')).toBeNull();
  expect(screen.queryByText('ai_ocr_vision_unconfigured')).toBeNull();
  expect(ipc.invoke).toHaveBeenCalledWith('reprocessRecordingOcr', {
    input: { recordingId: context.recordingId },
  });
});

test.each(['gap', 'succeeded'] as const)(
  'recognition %s metadata stays distinct without showing raw text',
  async (status) => {
    ipc.invoke.mockResolvedValue({
      document: { ...document, status, text: '' },
      job: null,
      recordingId: context.recordingId,
      sourceTrackId: context.sourceTrackId,
    });
    render(
      <VisualSourceInspector
        context={context}
        frame={frame}
        onClear={vi.fn()}
        onSeek={vi.fn()}
      />,
    );
    await screen.findByText(
      status === 'gap'
        ? /OCR is unavailable for this sample/
        : /Screen recognition complete/,
    );
    expect(screen.queryByRole('button', { name: 'Read OCR text' })).toBeNull();
    expect(screen.queryByText(/Word42/)).toBeNull();
  },
);

test('selecting a screen source without a frame keeps its timestamp and recognition metadata', async () => {
  render(
    <OcrDocumentInspector
      context={context}
      timestampMs={500}
      onSeek={vi.fn()}
    />,
  );
  await screen.findByText(/Screen recognition complete/);
  expect(screen.getByText('0:00.500')).toBeTruthy();
  expect(screen.getByText('video-0')).toBeTruthy();
  expect(screen.queryByText(/Word42/)).toBeNull();
});

test('live update failure remains visible beside eligible recognition metadata', async () => {
  ipc.listen.mockRejectedValue(new Error('Synthetic subscription unavailable'));
  render(
    <VisualSourceInspector
      context={context}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  await screen.findByText(/Live OCR updates are unavailable/);
  await screen.findByText(/Screen recognition complete/);
  expect(screen.queryByText(/Word42/)).toBeNull();
});

test.each(['queued', 'running'] as const)(
  'recognition %s is visible while the document is pending',
  async (status) => {
    const job: ProcessingJob = {
      id: 'recognition-job',
      createdAtMs: 1,
      updatedAtMs: 1,
      kind: 'ocr',
      progress: 0,
      recordingId: context.recordingId,
      status,
      errorCode: null,
    };
    ipc.invoke.mockResolvedValue({
      document: null,
      job,
      recordingId: context.recordingId,
      sourceTrackId: context.sourceTrackId,
    });
    render(
      <VisualSourceInspector
        context={context}
        frame={frame}
        onClear={vi.fn()}
        onSeek={vi.fn()}
      />,
    );
    await screen.findByText(
      status === 'queued'
        ? /OCR processing is queued/
        : /OCR processing is running/,
    );
    expect(screen.queryByText(/Screen recognition complete/)).toBeNull();
  },
);

test.each(['paused', 'cancelled', 'failed', 'completed-with-errors'] as const)(
  'recognition %s recovers through its existing job and refreshes the source',
  async (status) => {
    const job: ProcessingJob = {
      id: 'recognition-job',
      createdAtMs: 1,
      updatedAtMs: 1,
      kind: 'ocr',
      progress: 0,
      recordingId: context.recordingId,
      status,
      errorCode: status === 'failed' ? 'ai_processing_failed' : null,
    };
    let release: () => void = () => {
      throw new Error('Not initialized');
    };
    const operation = new Promise<void>((resolve) => {
      release = resolve;
    });
    let resumed = false;
    ipc.invoke.mockImplementation(async (command: string) => {
      if (command === 'getAppSettings')
        return structuredClone(defaultSettingsState);
      if (command === 'getRecorderScene') return structuredClone(scene);
      if (command === 'getOcrDocument')
        return {
          document: null,
          job: resumed ? { ...job, status: 'queued' } : job,
          recordingId: context.recordingId,
          sourceTrackId: context.sourceTrackId,
        };
      if (
        command === 'resumeProcessingJob' ||
        command === 'retryProcessingJob'
      ) {
        await operation;
        resumed = true;
        return { ...job, status: 'queued' };
      }
      if (command === 'reportFrontendFailure') return undefined;
      throw new Error('Unexpected synthetic command');
    });
    render(
      <VisualSourceInspector
        context={context}
        frame={frame}
        onClear={vi.fn()}
        onSeek={vi.fn()}
      />,
    );
    await userEvent.click(
      await screen.findByRole('button', {
        name: status === 'paused' ? 'Resume processing' : 'Retry processing',
      }),
    );
    expect(
      screen.getByRole('button', { name: 'Starting OCR…' }),
    ).toBeDisabled();
    release();
    await screen.findByText(/OCR processing is queued/);
    expect(ipc.invoke).toHaveBeenCalledWith(
      status === 'paused' ? 'resumeProcessingJob' : 'retryProcessingJob',
      { input: { jobId: job.id } },
    );
    expect(screen.queryByText(/Word42/)).toBeNull();
  },
);

test('a failed recognition read retries and recovers without exposing OCR', async () => {
  ipc.invoke.mockRejectedValueOnce(new Error('Synthetic read failure'));
  render(
    <VisualSourceInspector
      context={context}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  await userEvent.click(
    await screen.findByRole('button', { name: 'Retry loading OCR' }),
  );
  await screen.findByText(/Screen recognition complete/);
  expect(screen.queryByRole('alert')).toBeNull();
  expect(screen.queryByText(/Word42/)).toBeNull();
});
