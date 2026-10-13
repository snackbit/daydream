import { invoke } from '@tauri-apps/api/core';
import type {
  RecordingAiProcessingState,
  RecordingReprocessingScope,
  VisualFrameProcessingState,
} from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { TooltipProvider } from '@/components/ui/tooltip';
import '@/index.css';
import { ReprocessingDialog } from '../../reprocessing/reprocessing-dialog';
import { useStudioReprocessing } from '../../reprocessing/use-studio-reprocessing';
import type { StudioInspectorSelection } from '../../studio-inspector-selection';
import { TrackLane } from '../../track-lane';
import {
  chunk,
  detail,
  microphone,
  monitor,
  state,
} from './processing-test-fixtures';
import { TimelineProcessingOverlay } from './timeline-processing-overlay';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
  delete document.documentElement.dataset.theme;
});
const scope: RecordingReprocessingScope = {
  id: 'scope',
  trackId: 'mic',
  sourceLabel: 'Microphone',
  operation: 'transcription',
  startOffsetMs: 1000,
  endOffsetMs: 4000,
};
const preview = {
  recordingId: detail.id,
  planFingerprint: 'synthetic-plan',
  blockedReason: null,
  scopes: [scope],
  operations: [
    {
      operation: 'transcription',
      enabled: true,
      remote: false,
      providerLabel: 'Synthetic local',
      blockedReason: null,
    },
  ],
};

function Lane({
  processing,
  video = false,
  plannedScopes = [],
  onInspectTranscript = vi.fn(),
}: {
  processing: RecordingAiProcessingState | null;
  video?: boolean;
  plannedScopes?: readonly RecordingReprocessingScope[];
  onInspectTranscript?: (selection: StudioInspectorSelection) => void;
}) {
  const track = video ? monitor : microphone;
  return (
    <TooltipProvider>
      <div
        className="bg-background group/lane grid"
        style={{ gridTemplateColumns: '216px 1000px', width: 1216 }}
      >
        <TrackLane
          track={track}
          lane={{
            trackId: track.id,
            kind: track.kind,
            label: track.label,
            description: track.description,
            colorId: 'rose',
          }}
          detail={detail}
          aiProcessingState={processing}
          plannedProcessingScopes={plannedScopes}
          clipDrafts={{}}
          dragHandleProps={{}}
          mutedTrackIds={new Set()}
          onSelectClip={vi.fn()}
          onInspectTranscript={onInspectTranscript}
          onSelectLane={vi.fn()}
          onSelectRedaction={vi.fn()}
          onToggleTrackMuted={vi.fn()}
          playbackAssets={null}
          selectedClipIds={new Set()}
          selectedLane={false}
          selectedRedactionIds={new Set()}
          selectedVisualFrameIds={new Set()}
          thumbnailStatus="idle"
          timelineWidthPx={1000}
          timeRanges={[]}
          totalDurationMs={12000}
          visibleRange={{ startTimelineMs: 0, endTimelineMs: 12000 }}
          visualFrames={[]}
          waveformTiles={null}
          onSelectVisualFrame={vi.fn()}
        />
      </div>
    </TooltipProvider>
  );
}
function Admission({
  processing = null,
}: {
  processing?: RecordingAiProcessingState | null;
}) {
  const controller = useStudioReprocessing(detail.id, processing?.job ?? null);
  function open() {
    controller.open([
      { trackId: 'mic', ranges: [{ startOffsetMs: 1000, endOffsetMs: 4000 }] },
    ]);
  }
  return (
    <>
      <button type="button" onClick={open}>
        Reprocess microphone
      </button>
      <Lane processing={processing} plannedScopes={controller.plannedScopes} />
      <ReprocessingDialog controller={controller} />
    </>
  );
}

test('audio processing has one curtain across neighboring chunks and media fragments', () => {
  render(
    <Lane
      processing={state([
        chunk({ id: 'first', endOffsetMs: 2500 }),
        chunk({ id: 'second', startOffsetMs: 2500 }),
        chunk({
          id: 'third',
          segmentId: 'segment-1',
          startOffsetMs: 7000,
          endOffsetMs: 12000,
        }),
      ])}
    />,
  );
  expect(screen.getAllByRole('progressbar')).toHaveLength(1);
});

test('a fresh segment crossing the publication edge remains underneath the retreating curtain', () => {
  const fresh = {
    id: 'fresh-crossing',
    startOffsetMs: 500,
    endOffsetMs: 2500,
    text: 'Synthetic crossing sentence',
    confidence: null,
    model: null,
    provider: 'synthetic',
    segmentId: 'segment-0',
    words: [],
  };
  render(
    <Lane
      processing={{
        ...state([chunk({ status: 'running' })]),
        transcript: {
          recordingId: detail.id,
          tracks: [
            { trackId: 'mic', sourceRole: 'microphone', segments: [fresh] },
          ],
        },
        transcriptionProgress: [
          {
            jobId: 'job',
            trackId: 'mic',
            sourceRole: 'microphone',
            status: 'running',
            ranges: [{ startOffsetMs: 0, endOffsetMs: 5000 }],
            publishedRanges: [{ startOffsetMs: 0, endOffsetMs: 2000 }],
          },
        ],
      }}
    />,
  );
  expect(screen.getByText(fresh.text)).toBeTruthy();
});

test('reprocessing removes stale transcript actions and reveals fresh text only after publication', async () => {
  const inspect = vi.fn();
  const old = {
    id: 'old',
    startOffsetMs: 1000,
    endOffsetMs: 4000,
    text: 'Synthetic old words',
    confidence: null,
    model: null,
    provider: 'synthetic',
    segmentId: 'segment-0',
    words: [],
  };
  const processing = {
    ...state([chunk({ status: 'running' })]),
    transcript: {
      recordingId: detail.id,
      tracks: [
        { trackId: 'mic', sourceRole: 'microphone' as const, segments: [old] },
      ],
    },
    transcriptionProgress: [
      {
        jobId: 'job',
        trackId: 'mic',
        sourceRole: 'microphone' as const,
        status: 'running' as const,
        ranges: [{ startOffsetMs: 0, endOffsetMs: 5000 }],
        publishedRanges: [] as Array<{
          startOffsetMs: number;
          endOffsetMs: number;
        }>,
      },
    ],
  };
  const view = render(
    <Lane processing={processing} onInspectTranscript={inspect} />,
  );
  expect(screen.queryByText(old.text)).toBeNull();
  const fresh = {
    ...old,
    id: 'fresh',
    startOffsetMs: 500,
    endOffsetMs: 1500,
    text: 'Synthetic fresh words',
  };
  view.rerender(
    <Lane
      onInspectTranscript={inspect}
      processing={{
        ...processing,
        transcript: {
          ...processing.transcript,
          tracks: [
            // Native publication withholds the old overlapping segment.
            { ...processing.transcript.tracks[0]!, segments: [fresh] },
          ],
        },
        transcriptionProgress: [
          {
            ...processing.transcriptionProgress[0]!,
            publishedRanges: [{ startOffsetMs: 0, endOffsetMs: 2000 }],
          },
        ],
      }}
    />,
  );
  expect(screen.queryByText(old.text)).toBeNull();
  const action = await screen.findByRole('button', {
    name: /^Inspect Microphone transcript/,
  });
  await waitFor(() => expect(action.hasAttribute('disabled')).toBe(false));
  await userEvent.click(action);
  expect(inspect).toHaveBeenCalledWith(
    expect.objectContaining({ text: fresh.text }),
  );
  expect(screen.getAllByRole('progressbar')).toHaveLength(1);
});

test('a reviewed reprocessing request marks its source immediately during admission and removes the mark when admission fails', async () => {
  let reject!: (reason: Error) => void;
  vi.mocked(invoke)
    .mockResolvedValueOnce(preview)
    .mockImplementationOnce(
      () =>
        new Promise((_resolve, fail) => {
          reject = fail;
        }),
    );
  render(<Admission />);
  await userEvent.click(
    screen.getByRole('button', { name: 'Reprocess microphone' }),
  );
  await screen.findByText(/Microphone · Transcription · 0:01/);
  expect(screen.queryByRole('progressbar', { hidden: true })).toBeNull();
  await userEvent.click(screen.getByRole('button', { name: 'Reprocess' }));
  expect(
    (await screen.findByRole('progressbar', { hidden: true })).getAttribute(
      'aria-label',
    ),
  ).toBe('Microphone · Transcription · Preparing');
  reject(new Error('Synthetic admission failure'));
  await screen.findByRole('alert');
  expect(screen.queryByRole('progressbar', { hidden: true })).toBeNull();
});

test('accepted work stays visible until native state arrives and never revives after a later job', async () => {
  const job = { ...state([]).job!, createdAtMs: 1000 };
  vi.mocked(invoke).mockResolvedValueOnce(preview).mockResolvedValueOnce(job);
  const view = render(<Admission />);
  await userEvent.click(
    screen.getByRole('button', { name: 'Reprocess microphone' }),
  );
  await screen.findByText(/Microphone · Transcription · 0:01/);
  await userEvent.click(screen.getByRole('button', { name: 'Reprocess' }));
  await screen.findByRole('progressbar', {
    name: 'Microphone · Transcription · Queued',
  });
  expect(screen.queryByRole('dialog')).toBeNull();
  const finished = {
    ...state([chunk({ status: 'succeeded' })]),
    job: { ...job, status: 'succeeded' as const },
  };
  view.rerender(<Admission processing={finished} />);
  await waitFor(() => expect(screen.queryByRole('progressbar')).toBeNull());
  view.rerender(
    <Admission
      processing={{
        ...finished,
        job: { ...job, id: 'later-job', createdAtMs: 2000 },
      }}
    />,
  );
  await waitFor(() => expect(screen.queryByRole('progressbar')).toBeNull());
});

test('automatic audio work advances, stops on pause, and clears on completion without requiring a selected range', async () => {
  const processing = state([chunk()]);
  const view = render(<Lane processing={processing} />);
  expect(
    screen
      .getByRole('progressbar', { name: 'Microphone · Transcription · Queued' })
      .hasAttribute('aria-valuenow'),
  ).toBe(false);
  view.rerender(
    <Lane processing={state([chunk({ status: 'running', progress: 0.4 })])} />,
  );
  expect(
    screen
      .getByRole('progressbar', {
        name: 'Microphone · Transcription · Processing',
      })
      .getAttribute('aria-valuenow'),
  ).toBeNull();
  const paused = {
    ...processing,
    chunks: [chunk({ status: 'running', progress: 0.4 })],
    job: { ...processing.job!, status: 'paused' as const },
  };
  view.rerender(<Lane processing={paused} />);
  expect(
    screen
      .getByRole('progressbar', {
        name: 'Microphone · Transcription · Paused',
      })
      .getAttribute('aria-valuenow'),
  ).toBeNull();
  view.rerender(
    <Lane processing={state([chunk({ status: 'succeeded', progress: 1 })])} />,
  );
  await waitFor(() => expect(screen.queryByRole('progressbar')).toBeNull());
});

test('video sample completion clears only its own work while the other visual operation continues', () => {
  const processing = state([
    chunk({
      id: 'ocr',
      sourceKind: 'ocr',
      trackId: 'screen-a',
      status: 'running',
    }),
    chunk({ id: 'vlm', sourceKind: 'vlm', trackId: 'screen-a' }),
  ]);
  const frames: VisualFrameProcessingState[] = [0, 2500].map(
    (frameTimestampMs, index) => ({
      id: `frame-${index}`,
      chunkId: 'ocr',
      jobId: 'job',
      recordingId: detail.id,
      segmentId: 'segment-0',
      sourceKind: 'ocr',
      sourceFrameRef: `synthetic:${index}`,
      frameTimestampMs,
      status: 'queued',
      progress: 0,
      errorCode: null,
      outputCount: 0,
      region: { x: 0, y: 0, width: 1, height: 1 },
      createdAtMs: 0,
      updatedAtMs: 0,
    }),
  );
  const view = render(
    <Lane processing={{ ...processing, visualFrames: frames }} video />,
  );
  expect(
    screen.getAllByRole('progressbar', {
      name: 'Monitor · OCR · Queued · Screen observations · Queued',
    }),
  ).toHaveLength(1);
  view.rerender(
    <Lane
      processing={{
        ...processing,
        visualFrames: frames.map((frame, index) => ({
          ...frame,
          status: index === 0 ? 'succeeded' : 'running',
        })),
      }}
      video
    />,
  );
  expect(
    screen.queryByRole('progressbar', { name: 'Monitor · OCR · Queued' }),
  ).toBeNull();
  expect(
    screen.getByRole('progressbar', {
      name: 'Monitor · OCR · Processing · Screen observations · Queued',
    }),
  ).toBeTruthy();
  view.rerender(
    <Lane
      processing={{
        ...processing,
        visualFrames: frames.map((frame) => ({
          ...frame,
          status: 'succeeded',
        })),
      }}
      video
    />,
  );
  expect(
    screen.queryByRole('progressbar', { name: /Monitor · OCR/ }),
  ).toBeNull();
  expect(
    screen.getByRole('progressbar', {
      name: 'Monitor · Screen observations · Queued',
    }),
  ).toBeTruthy();
});

test('a failed screen section stays discoverable in one bar while its adjacent section continues', () => {
  render(
    <Lane
      video
      processing={state([
        chunk({
          id: 'failed',
          sourceKind: 'ocr',
          trackId: 'screen-a',
          status: 'failed',
          endOffsetMs: 2500,
        }),
        chunk({
          id: 'running',
          sourceKind: 'ocr',
          trackId: 'screen-a',
          status: 'running',
          startOffsetMs: 2500,
        }),
      ])}
    />,
  );
  expect(screen.getAllByRole('progressbar')).toHaveLength(1);
  expect(
    screen.getByRole('progressbar', {
      name: 'Monitor · OCR · Processing · Needs attention',
    }),
  ).toBeTruthy();
});

test('the legacy transcription overlay fills its available row and reports queued work without a percentage', () => {
  const view = render(
    <div style={{ position: 'relative', height: 200 }}>
      <TimelineProcessingOverlay
        ranges={[
          {
            id: 'legacy',
            operation: 'transcription',
            status: 'queued',
            startOffsetMs: 0,
            endOffsetMs: 5000,
            progress: 0,
          },
        ]}
        trackLabel="Legacy audio"
        video={false}
        timelineWidthPx={1000}
        totalDurationMs={12000}
        visibleRange={{ startTimelineMs: 0, endTimelineMs: 12000 }}
      />
    </div>,
  );
  const bar = screen.getByRole('progressbar', {
    name: 'Legacy audio · Transcription · Queued',
  });
  expect(bar.getAttribute('aria-valuenow')).toBeNull();
  const overlay = view.container.querySelector('[data-timeline-processing]');
  expect(bar.getBoundingClientRect().height).toBe(
    overlay?.getBoundingClientRect().height,
  );
});

test.each(
  ['light', 'dark'].flatMap((theme) =>
    [1440, 390].map((width) => ({ theme, width })),
  ),
)(
  'video and audio processing fill source ranges at $width px in $theme',
  async ({ theme, width }) => {
    await page.viewport(width, 600);
    document.documentElement.dataset.theme = theme;
    const processing = state([
      chunk({ status: 'running', progress: 0.4 }),
      chunk({ id: 'ocr', sourceKind: 'ocr', trackId: 'screen-a' }),
      chunk({
        id: 'vlm',
        sourceKind: 'vlm',
        trackId: 'screen-a',
        status: 'running',
      }),
    ]);
    const view = render(
      <div
        className="bg-background h-screen overflow-auto"
        aria-label="Timeline viewport"
      >
        <Lane processing={processing} video />
        <Lane processing={processing} />
      </div>,
    );
    expect(
      screen.getByRole('progressbar', {
        name: 'Monitor · OCR · Queued · Screen observations · Processing',
      }),
    ).toBeTruthy();
    expect(screen.getAllByRole('progressbar')).toHaveLength(2);
    const viewport = screen.getByLabelText('Timeline viewport');
    viewport.scrollLeft = 120;
    await page.screenshot({
      element: view.container,
      path: `__screenshots__/processing-${theme}-${width}.png`,
    });
    await userEvent.click(
      screen.getByRole('button', { name: 'Select Microphone' }),
    );
  },
);
