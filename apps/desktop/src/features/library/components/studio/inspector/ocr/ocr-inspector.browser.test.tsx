import type {
  OcrDocument,
  OcrHistoryPage,
  OcrHistoryCursor,
  OcrDocumentSelection,
  ProcessingJob,
} from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import { VisualSourceInspector } from '../visual-source-inspector';

const ipc = vi.hoisted(() => ({
  invoke: vi.fn(),
  copy: vi.fn(),
  listen: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: ipc.listen }));
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
  ipc.listen.mockResolvedValue(() => undefined);
  ipc.copy.mockResolvedValue(undefined);
  const navigatorMock = Object.create(navigator);
  Object.defineProperty(navigatorMock, 'clipboard', {
    value: { writeText: ipc.copy },
  });
  vi.stubGlobal('navigator', navigatorMock);
  ipc.invoke.mockImplementation((command: string) =>
    command === 'getOcrDocument'
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

test('a dense OCR frame is readable, searchable, copyable and source linked without a metadata dump', async () => {
  await page.viewport(620, 960);
  const seek = vi.fn();
  render(
    <VisualSourceInspector
      context={context}
      frame={frame}
      onClear={vi.fn()}
      onSeek={seek}
    />,
  );
  const text = await screen.findByRole('tabpanel', { name: 'Text' });
  expect(
    (await within(text).findByLabelText('Recognized text')).textContent,
  ).toContain('Word0 Word1 Word2');
  expect(screen.queryByText('Chunk ID')).toBeNull();
  expect(screen.queryByText('Frame reference')).toBeNull();
  expect(screen.getByText(/1 uncertain word/i)).toBeTruthy();
  fireEvent.change(screen.getByRole('textbox', { name: 'Find OCR text' }), {
    target: { value: 'Word499' },
  });
  expect(await screen.findByText(/1 match/i)).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Copy OCR text' }));
  await waitFor(() => expect(ipc.copy).toHaveBeenCalledWith(document.text));
  fireEvent.click(screen.getByRole('button', { name: 'Jump to frame' }));
  expect(seek).toHaveBeenCalledWith(500);
  fireEvent.click(screen.getByRole('button', { name: 'Expand OCR text' }));
  expect(
    await screen.findByRole('dialog', { name: 'OCR reading view' }),
  ).toBeTruthy();
  const readingText = screen.getByLabelText('OCR reading text');
  await waitFor(() => expect(window.document.activeElement).toBe(readingText));
  expect(screen.queryByRole('tooltip')).toBeNull();
  const dialog = screen.getByRole('dialog', { name: 'OCR reading view' });
  const lastWord = within(dialog).getByText('Word499', { selector: 'mark' });
  await expect.element(lastWord).not.toBeInViewport();
  await userEvent.wheel(within(dialog).getByTestId('ocr-reading-scroll'), {
    direction: 'down',
    times: 20,
  });
  await expect.element(lastWord).toBeInViewport();
  await userEvent.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(window.document.activeElement).toBe(
    screen.getByRole('button', { name: 'Expand OCR text' }),
  );
  const uncertain = screen.getByRole('button', { name: 'Uncertain OCR words' });
  uncertain.focus();
  expect(await screen.findByRole('tooltip')).toBeTruthy();
});

function openTab(name: string) {
  const tab = screen.getByRole('tab', { name });
  fireEvent.mouseDown(tab, { button: 0 });
  fireEvent.click(tab);
}
function textDocument(
  text: string,
  overrides: Partial<OcrDocument> = {},
): OcrDocument {
  return {
    ...document,
    blocks: [
      {
        confidence: null,
        id: 'literal-block',
        order: 0,
        rawRegionIds: [],
        region: null,
        text,
        words: [],
      },
    ],
    text,
    ...overrides,
  };
}
function deferred<T>() {
  let resolve: (value: T) => void = () => {
    throw new Error('Promise not initialized');
  };
  let reject: (failure: Error) => void = () => {
    throw new Error('Promise not initialized');
  };
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, resolve, reject };
}

test('Changes preserves a changed digit, groups supporting samples and selects source-linked paginated revisions', async () => {
  const current = textDocument('Meeting 18: Alice agreed.', {
    id: 'current',
    frameTimestampMs: 2_500,
    observedStartMs: 2_500,
    observedEndMs: 4_500,
    supportTimestampsMs: [2_500, 3_500, 4_500],
    occurrenceCount: 3,
  });
  const previous = textDocument('Meeting 17: Alice agreed.', {
    id: 'previous',
    frameTimestampMs: 100,
    observedStartMs: 100,
    observedEndMs: 100,
    supportTimestampsMs: [100],
    occurrenceCount: 1,
    provenance: 'reconstructed',
    engine: 'legacy',
  });
  const comparablePrevious = {
    ...previous,
    engine: current.engine,
    provenance: current.provenance,
  };
  const older = textDocument('Meeting 16: Alice agreed.', {
    id: 'older',
    frameTimestampMs: 50,
    observedStartMs: 50,
    observedEndMs: 50,
    supportTimestampsMs: [50],
    occurrenceCount: 1,
  });
  const seek = vi.fn();
  ipc.invoke.mockImplementation(
    (
      command: string,
      args: {
        input: { documentId?: string | null; cursor?: OcrHistoryCursor | null };
      },
    ) =>
      command === 'getOcrDocument'
        ? Promise.resolve({
            document:
              args.input.documentId === previous.id ? previous : current,
            recordingId: context.recordingId,
            job: null,
            sourceTrackId: context.sourceTrackId,
          })
        : Promise.resolve(
            args.input.cursor
              ? { items: [older], nextCursor: null }
              : {
                  items: [current, comparablePrevious],
                  nextCursor: { id: previous.id, timestampMs: 100 },
                },
          ),
  );
  render(
    <VisualSourceInspector
      context={context}
      frame={frame}
      onClear={vi.fn()}
      onSeek={seek}
    />,
  );
  await screen.findByLabelText('Recognized text');
  expect(
    ipc.invoke.mock.calls.filter(([command]) => command === 'getOcrHistory'),
  ).toHaveLength(0);
  openTab('Changes');
  const changes = await screen.findByRole('tabpanel', { name: 'Changes' });
  expect(await within(changes).findByText(/3 supporting samples/)).toBeTruthy();
  expect(within(changes).getByText('7', { selector: 'del' })).toBeTruthy();
  expect(within(changes).getByText('8', { selector: 'ins' })).toBeTruthy();
  fireEvent.click(within(changes).getByRole('button', { name: /Meeting 17/ }));
  expect(seek).toHaveBeenCalledWith(100);
  openTab('Text');
  await waitFor(() =>
    expect(screen.getByLabelText('Recognized text').textContent).toBe(
      previous.text,
    ),
  );
  expect(screen.getByText(/Reconstructed from legacy OCR/)).toBeTruthy();
  openTab('Changes');
  fireEvent.click(
    await screen.findByRole('button', { name: 'Load older revisions' }),
  );
  expect(
    await screen.findByRole('button', { name: /Meeting 16/ }),
  ).toBeTruthy();
});

test('literal Vision OCR keeps unknown confidence honest and exposes raw details only through explicit Debug', async () => {
  const vision = textDocument('Invoice 123.50 — Alice', {
    engine: 'vision',
    provider: 'openai-compatible',
    model: 'synthetic-vision-model',
  });
  ipc.invoke.mockResolvedValue({
    document: vision,
    recordingId: context.recordingId,
    job: {
      createdAtMs: 0,
      errorCode: 'ai_transcription_provider_failed',
      id: 'mixed-parent',
      kind: 'ai-processing',
      progress: 1,
      recordingId: context.recordingId,
      status: 'failed',
      updatedAtMs: 1,
    } satisfies ProcessingJob,
    sourceTrackId: context.sourceTrackId,
  });
  const { rerender } = render(
    <VisualSourceInspector
      context={context}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  expect((await screen.findByLabelText('Recognized text')).textContent).toBe(
    vision.text,
  );
  expect(screen.getByText('Confidence unknown')).toBeTruthy();
  expect(screen.queryByText(/OCR processing failed/)).toBeNull();
  expect(screen.queryByText('Provider')).toBeNull();
  openTab('Details');
  expect(screen.getAllByText('Provider')).toHaveLength(1);
  expect(
    screen.queryByRole('button', { name: 'Show raw OCR details' }),
  ).toBeNull();
  rerender(
    <VisualSourceInspector
      context={{ ...context, debugEnabled: true }}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  expect(screen.queryByText('Frame reference')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Show raw OCR details' }));
  expect(screen.getByText('Frame reference')).toBeTruthy();
  expect(screen.getByText('opaque-frame-reference')).toBeTruthy();
});

test('changing privacy hides open reading text immediately and late reads or failed reloads cannot restore it', async () => {
  const stale = deferred<OcrDocumentSelection>();
  ipc.invoke
    .mockImplementationOnce(() => stale.promise)
    .mockRejectedValue(new Error('Reload unavailable'));
  const { rerender } = render(
    <VisualSourceInspector
      context={context}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  await waitFor(() => expect(ipc.invoke).toHaveBeenCalledTimes(1));
  rerender(
    <VisualSourceInspector
      context={{ ...context, privacyRevision: 'cut' }}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  stale.resolve({
    document,
    recordingId: context.recordingId,
    job: null,
    sourceTrackId: context.sourceTrackId,
  });
  expect(await screen.findByRole('alert')).toBeTruthy();
  expect(screen.queryByLabelText('Recognized text')).toBeNull();
  expect(screen.queryByText(/Word42/)).toBeNull();
});

test('a failed Vision document retries real processing and preserves the blocked setup instead of showing an empty success', async () => {
  const failed = textDocument('', {
    engine: 'vision',
    status: 'failed',
    errorCode: 'ai_ocr_vision_unconfigured',
  });
  ipc.invoke.mockImplementation((command: string) =>
    command === 'reprocessRecordingOcr'
      ? Promise.reject({
          message: 'Configure Vision OCR before retrying.',
          code: 'ai_ocr_vision_unconfigured',
        })
      : Promise.resolve({
          document: failed,
          recordingId: context.recordingId,
          job: null,
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
  expect(await screen.findByRole('alert')).toBeTruthy();
  expect(screen.queryByLabelText('Recognized text')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Retry processing' }));
  expect(
    await screen.findByText('Configure Vision OCR before retrying.'),
  ).toBeTruthy();
  expect(ipc.invoke).toHaveBeenCalledWith('reprocessRecordingOcr', {
    input: { recordingId: context.recordingId },
  });
  expect(screen.queryByText('No text detected in this frame.')).toBeNull();
});

test('successful empty OCR and processing gaps are distinct visible states', async () => {
  ipc.invoke
    .mockResolvedValueOnce({
      document: textDocument(''),
      recordingId: context.recordingId,
      job: null,
      sourceTrackId: context.sourceTrackId,
    })
    .mockResolvedValue({
      document: textDocument('', { status: 'gap' }),
      recordingId: context.recordingId,
      job: null,
      sourceTrackId: context.sourceTrackId,
    });
  const { rerender } = render(
    <VisualSourceInspector
      context={context}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  expect(
    await screen.findByText('No text detected in this frame.'),
  ).toBeTruthy();
  rerender(
    <VisualSourceInspector
      context={context}
      frame={{ ...frame, frameTimestampMs: 1_500 }}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  expect(
    await screen.findByText(/OCR is unavailable for this sample/),
  ).toBeTruthy();
  expect(screen.queryByText('No text detected in this frame.')).toBeNull();
});

test('a privacy change closes copied reading content and clears selected history and diffs before failed native reloads', async () => {
  const current = textDocument('Private meeting 18', { id: 'private-current' });
  const previous = textDocument('Private meeting 17', {
    id: 'private-previous',
  });
  ipc.invoke.mockImplementation((command: string) =>
    command === 'getOcrHistory'
      ? Promise.resolve({ items: [current, previous], nextCursor: null })
      : Promise.resolve({
          document: current,
          job: null,
          recordingId: context.recordingId,
          sourceTrackId: context.sourceTrackId,
        }),
  );
  const { rerender } = render(
    <VisualSourceInspector
      context={context}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  await screen.findByLabelText('Recognized text');
  openTab('Changes');
  fireEvent.click(
    await screen.findByRole('button', { name: /Private meeting 17/ }),
  );
  expect(await screen.findByText(/Revision at/)).toBeTruthy();
  openTab('Text');
  fireEvent.click(
    await screen.findByRole('button', { name: 'Expand OCR text' }),
  );
  expect(await screen.findByRole('dialog')).toBeTruthy();
  ipc.invoke.mockRejectedValue(new Error('Private results unavailable'));
  rerender(
    <VisualSourceInspector
      context={{ ...context, privacyRevision: 'cut' }}
      frame={frame}
      onClear={vi.fn()}
      onSeek={vi.fn()}
    />,
  );
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(screen.queryByText(/Private meeting/)).toBeNull();
  expect(screen.queryByText(/Revision at/)).toBeNull();
  expect(await screen.findByRole('alert')).toBeTruthy();
});

test('pre-frame OCR job phases refresh from processing events and retry the actual failed job without changing engines', async () => {
  const job: ProcessingJob = {
    createdAtMs: 0,
    errorCode: null,
    id: 'scoped-ocr-job',
    kind: 'ocr',
    progress: 0,
    recordingId: context.recordingId,
    status: 'running',
    updatedAtMs: 1,
  };
  let currentJob = job;
  let processingChanged: (event: {
    payload: { recordingId: string };
  }) => void = () => {
    throw new Error('Listener missing');
  };
  ipc.listen.mockImplementation(
    (_event: string, callback: typeof processingChanged) => {
      processingChanged = callback;
      return Promise.resolve(() => undefined);
    },
  );
  ipc.invoke.mockImplementation((command: string) =>
    command === 'retryProcessingJob'
      ? Promise.reject({
          message: 'Configure the saved Vision OCR profile before retrying.',
        })
      : Promise.resolve({
          document: null,
          job: currentJob,
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
  expect(await screen.findByText(/OCR processing is running/)).toBeTruthy();
  expect(screen.queryByText('No text detected in this frame.')).toBeNull();
  currentJob = {
    ...job,
    errorCode: 'ai_ocr_vision_unconfigured',
    status: 'failed',
    updatedAtMs: 2,
  };
  processingChanged({ payload: { recordingId: context.recordingId } });
  expect(await screen.findByRole('alert')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Retry processing' }));
  expect(
    await screen.findByText(
      'Configure the saved Vision OCR profile before retrying.',
    ),
  ).toBeTruthy();
  expect(ipc.invoke).toHaveBeenCalledWith('retryProcessingJob', {
    input: { jobId: job.id },
  });
  expect(
    ipc.invoke.mock.calls.some(
      ([command]) => command === 'reprocessRecordingOcr',
    ),
  ).toBe(false);
});
