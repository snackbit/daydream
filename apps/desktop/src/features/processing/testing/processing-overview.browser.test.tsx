import { useState } from 'react';
import {
  createRootRoute,
  createRoute,
  createRouter,
  createMemoryHistory,
  RouterProvider,
} from '@tanstack/react-router';
import type { BackgroundTasksChanged } from '@daydream/shared';
import {
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { Button } from '@/components/ui/button';
import { useStudioSelectionDismissal } from '@/features/library/components/studio/interactions/use-studio-selection-dismissal';
import { useStudioTimelineStore } from '@/features/library/components/studio/studio-timeline-store';
import { detail } from '@/features/library/components/studio/timeline/processing/processing-test-fixtures';
import '@/index.css';
import { ProcessingScreen } from '@/screens/ProcessingScreen';
import { StudioProcessingOverview } from '../components/studio-processing-overview';
import { validateProcessingLocation } from '../processing-location';
import { useRecordingProcessingStore } from '../recording-processing-store';
import {
  activity,
  group,
  groupPage,
  lane,
} from './processing-browser-fixtures';

const client = vi.hoisted(() => ({
  listRecordingProcessingGroups: vi.fn(),
  listenToBackgroundTasksChanged: vi.fn(),
  controlRecordingProcessingGroups: vi.fn(),
  getRecordingProcessingLane: vi.fn(),
  listBackgroundTasks: vi.fn(),
  getBackgroundTask: vi.fn(),
  retryProcessingUnits: vi.fn(),
  reprocessRecordingTranscription: vi.fn(),
}));
vi.mock('../processing-client', () => client);
let changed: ((event: BackgroundTasksChanged) => void) | undefined;
let studioIds = ['recording-b', 'recording-a'];
const clearSelection = vi.fn();

test('an empty Studio scope cannot open unrelated work; a single recording has a scoped description', async () => {
  studioIds = [];
  const view = mount(true);
  const trigger = await screen.findByRole('button', {
    name: 'Processing overview',
  });
  expect((trigger as HTMLButtonElement).disabled).toBe(true);
  expect(client.listRecordingProcessingGroups).not.toHaveBeenCalled();
  view.unmount();
  studioIds = ['recording-a'];
  mount(true);
  await page.getByRole('button', { name: 'Processing overview' }).click();
  await screen.findByText('Current and queued work for this recording.');
});

test.each(['disabled', 'no-eligible-input', 'available'] as const)(
  'completed pipelines retain source availability and %s search readiness without a fabricated estimate',
  async (applicability) => {
    const empty = (
      kind: Parameters<typeof lane>[0],
      patch: Parameters<typeof lane>[1] = {},
    ) =>
      lane(kind, {
        status: null,
        jobIds: [],
        total: 0,
        completed: 0,
        running: 0,
        queued: 0,
        progress: null,
        applicability: 'no-source',
        ...patch,
      });
    const finished = group({
      status: 'succeeded',
      startedAtMs: null,
      etaMs: null,
      jobIds: [],
      lanes: [
        empty('vlm', { applicability: 'disabled' }),
        empty('ocr', { applicability: 'available' }),
        empty('audio'),
        empty('activity', { applicability: 'protected' }),
        empty('waveform'),
      ],
      indexCoverage: { applicability, indexed: 0, total: 0, pending: 0 },
    });
    client.listRecordingProcessingGroups.mockResolvedValue(
      groupPage([finished]),
    );
    mount(false, '?recordings=synthetic-recording');
    await page.getByRole('button', { name: 'Inspect OCR' }).click();
    expect(screen.getByText('Processing finished')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Details' })).toBeNull();
    await page.getByRole('button', { name: /Search readiness/ }).click();
    expect(screen.getByRole('link', { name: 'Recall settings' })).toBeTruthy();
    if (applicability === 'disabled')
      expect(screen.getByText(/Configure Recall embeddings/)).toBeTruthy();
    else if (applicability === 'available')
      expect(
        screen.getByText(/Current eligible passages are ready/),
      ).toBeTruthy();
    else
      expect(
        screen.getByText(/Eligible OCR and transcript results appear here/),
      ).toBeTruthy();
    await page
      .getByRole('button', { name: 'Inspect Activity analysis' })
      .click();
    expect(
      screen.getByRole('link', { name: 'Review recording protection' }),
    ).toBeTruthy();
    await page
      .getByRole('button', { name: 'Inspect Activity analysis' })
      .click();
    expect(screen.queryByLabelText('Selected stage details')).toBeNull();
  },
);

test('paused work exposes scoped retry and recording resume while retaining skipped and cancelled outcomes', async () => {
  const waveform = lane('waveform', {
    status: 'paused',
    completed: 2,
    failed: 0,
    skipped: 1,
    cancelled: 1,
    running: 0,
    queued: 0,
    total: 4,
    progress: null,
    nextRetryAtMs: Date.now() + 10000,
  });
  const paused = group({
    status: 'paused',
    etaMs: null,
    lanes: [
      lane('audio'),
      lane('ocr'),
      lane('vlm'),
      lane('activity'),
      waveform,
    ],
  });
  client.listRecordingProcessingGroups.mockResolvedValue(groupPage([paused]));
  mount(false, '?recordings=synthetic-recording');
  await page.getByRole('button', { name: 'Inspect Waveforms' }).click();
  const selected = within(screen.getByLabelText('Selected stage details'));
  expect(selected.getByText('1 skipped')).toBeTruthy();
  expect(selected.getByText('1 cancelled')).toBeTruthy();
  expect(selected.getByText(/Automatic retry scheduled/)).toBeTruthy();
  expect(
    selected.queryByRole('button', { name: 'Use current settings' }),
  ).toBeNull();
  await page
    .getByRole('button', { name: 'Resume recording processing' })
    .click();
  await waitFor(() =>
    expect(client.controlRecordingProcessingGroups).toHaveBeenCalledWith({
      recordingId: 'synthetic-recording',
      action: 'resume',
    }),
  );
  await page.getByRole('button', { name: 'Retry failed', exact: true }).click();
  await waitFor(() =>
    expect(client.controlRecordingProcessingGroups).toHaveBeenLastCalledWith({
      recordingId: 'synthetic-recording',
      action: 'retry',
      sourceKind: 'waveform',
      useCurrentSettings: false,
    }),
  );
});
function Studio() {
  useStudioSelectionDismissal(clearSelection);
  const selected = useStudioTimelineStore((state) => state.selection.laneIds);
  const [position, setPosition] = useState('11:26');
  return (
    <main>
      <label htmlFor="position">Review position</label>
      <input id="position" value={position} onChange={setPositionFromInput} />
      <Button data-studio-preserve-selection onClick={selectMonitor}>
        Select monitor
      </Button>
      <p aria-label="Selected sources">{selected.length} sources selected</p>
      <StudioProcessingOverview recordingIds={studioIds} />
    </main>
  );
  function setPositionFromInput(event: React.ChangeEvent<HTMLInputElement>) {
    setPosition(event.target.value);
  }
  function selectMonitor() {
    useStudioTimelineStore.getState().initializeFromDetail(detail);
    useStudioTimelineStore.getState().selectLane('screen-a');
  }
}
function mount(studio = false, search = '') {
  const root = createRootRoute();
  const index = createRoute({
    getParentRoute: () => root,
    path: '/',
    component: Studio,
  });
  const processing = createRoute({
    getParentRoute: () => root,
    path: '/processing',
    validateSearch: validateProcessingLocation,
    component: ProcessingScreen,
  });
  const library = createRoute({
    getParentRoute: () => root,
    path: '/library/$recordingId',
  });
  const settings = createRoute({
    getParentRoute: () => root,
    path: '/settings',
  });
  return render(
    <RouterProvider
      router={createRouter({
        routeTree: root.addChildren([index, processing, library, settings]),
        history: createMemoryHistory({
          initialEntries: [studio ? '/' : `/processing${search}`],
        }),
      })}
    />,
  );
}
beforeEach(() => {
  vi.resetAllMocks();
  changed = undefined;
  studioIds = ['recording-b', 'recording-a'];
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
  useRecordingProcessingStore.setState(
    useRecordingProcessingStore.getInitialState(),
    true,
  );
  client.listenToBackgroundTasksChanged.mockImplementation(async (handler) => {
    changed = handler;
    return () => {
      changed = undefined;
    };
  });
  client.listBackgroundTasks.mockResolvedValue({ tasks: [], nextCursor: null });
  client.controlRecordingProcessingGroups.mockResolvedValue({
    results: [{ jobId: 'job', succeeded: true, errorCode: null }],
  });
  client.listRecordingProcessingGroups.mockResolvedValue(
    groupPage(
      [
        group({
          lanes: [
            lane('audio'),
            lane('activity'),
            lane('waveform'),
            lane('ocr', {
              activity: [
                activity({
                  phaseStartedAtMs: Date.now() - 60000,
                  lastProgressAtMs: Date.now() - 30000,
                }),
              ],
            }),
            lane('vlm', {
              running: 0,
              queued: 6,
              activity: [activity({ status: 'queued', waitingReason: 'ocr' })],
            }),
          ],
        }),
      ],
      {
        counts: {
          all: 50,
          active: 20,
          attention: 3,
          history: 30,
          running: 4,
          queued: 100,
        },
      },
    ),
  );
});
afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});

test('stage details explain the current source and dependent work, and scoped recovery refreshes without losing selection', async () => {
  mount();
  await screen.findByRole('button', { name: 'Inspect OCR' });
  expect(screen.getByLabelText('Processing totals').textContent).toContain(
    '100 queued',
  );
  await page.getByRole('button', { name: 'Inspect OCR' }).click();
  expect(
    within(
      screen.getByRole('list', { name: 'Current and next work' }),
    ).getByText('Recognizing screen text'),
  ).toBeTruthy();
  expect(screen.getByText('Monitor · 0:00–0:01')).toBeTruthy();
  expect(screen.getByText(/Last progress:/)).toBeTruthy();
  await page
    .getByRole('button', { name: 'Inspect Screen observations' })
    .click();
  expect(
    within(screen.getByLabelText('Selected stage details')).getAllByText(
      'Waiting for OCR in this section',
    ),
  ).toHaveLength(2);
  client.listRecordingProcessingGroups.mockResolvedValue(
    groupPage([
      group({
        lanes: group().lanes.map((value) =>
          value.kind === 'vlm'
            ? lane('vlm', {
                activity: [activity({ phase: 'observing-screen' })],
              })
            : value,
        ),
      }),
    ]),
  );
  changed?.({ recordingId: 'synthetic-recording', updatedAtMs: 3000 });
  await within(
    screen.getByRole('list', { name: 'Current and next work' }),
  ).findByText('Describing screen changes');
  expect(
    screen
      .getByRole('button', { name: 'Inspect Screen observations' })
      .getAttribute('aria-pressed'),
  ).toBe('true');
});

test('a lost event is reconciled on focus; failed refresh preserves details and exposes recovery', async () => {
  mount();
  await page.getByRole('button', { name: 'Inspect OCR' }).click();
  client.listRecordingProcessingGroups.mockRejectedValueOnce({
    code: 'ai_processing_storage_failed',
  });
  window.dispatchEvent(new Event('focus'));
  await screen.findByRole('alert');
  expect(
    within(
      screen.getByRole('list', { name: 'Current and next work' }),
    ).getByText('Recognizing screen text'),
  ).toBeTruthy();
  await page.getByRole('button', { name: 'Refresh', exact: true }).click();
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
  expect(
    screen
      .getByRole('button', { name: 'Inspect OCR' })
      .getAttribute('aria-pressed'),
  ).toBe('true');
});

test('Studio sheet preserves local review state, keyboard focus and day scope through full-page navigation', async () => {
  mount(true);
  await screen.findByLabelText('Review position');
  await userEvent.clear(screen.getByLabelText('Review position'));
  await userEvent.type(screen.getByLabelText('Review position'), '11:28');
  await page.getByRole('button', { name: 'Select monitor' }).click();
  expect(screen.getByLabelText('Selected sources').textContent).toBe(
    '1 sources selected',
  );
  await page.getByRole('button', { name: 'Processing overview' }).click();
  await screen.findByRole('button', { name: 'Inspect OCR' });
  expect(client.listRecordingProcessingGroups).toHaveBeenCalledWith(
    expect.objectContaining({ recordingIds: ['recording-a', 'recording-b'] }),
  );
  await userEvent.keyboard('{Escape}');
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole('button', { name: 'Processing overview' }),
    ),
  );
  expect(
    (screen.getByLabelText('Review position') as HTMLInputElement).value,
  ).toBe('11:28');
  expect(screen.getByLabelText('Selected sources').textContent).toBe(
    '1 sources selected',
  );
  await page.getByRole('button', { name: 'Processing overview' }).click();
  await page.getByRole('link', { name: 'Open full Processing page' }).click();
  await screen.findByText('Showing 2 selected recordings.');
  await page.getByRole('link', { name: 'Show all recordings' }).click();
  await screen.findByRole('heading', { name: 'Processing' });
  await waitFor(() =>
    expect(client.listRecordingProcessingGroups).toHaveBeenLastCalledWith(
      expect.not.objectContaining({ recordingIds: expect.anything() }),
    ),
  );
});

test.each(
  ['light', 'dark'].flatMap((theme) =>
    [1440, 390].map((width) => ({ theme, width })),
  ),
)(
  'pipeline layout and source details stay usable in $theme at $width pixels',
  async ({ theme, width }) => {
    await page.viewport(width, 900);
    document.documentElement.dataset.theme = theme;
    const view = mount();
    await page.getByRole('button', { name: 'Inspect OCR' }).click();
    await screen.findByRole('list', { name: 'Current and next work' });
    expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(width);
    await page.screenshot({
      element: view.container,
      path: `__screenshots__/overview-${theme}-${width}.png`,
    });
  },
);

test('activity distinguishes an unowned worker, unknown timing, protection and a scheduled retry without exposing hidden source details', async () => {
  client.listRecordingProcessingGroups.mockResolvedValue(
    groupPage([
      group({
        lanes: group().lanes.map((value) =>
          value.kind === 'ocr'
            ? lane('ocr', {
                activity: [
                  activity({
                    chunkId: 'stopped',
                    workerActive: false,
                    phaseStartedAtMs: null,
                    lastProgressAtMs: null,
                    sourceLabel: null,
                    startOffsetMs: null,
                    endOffsetMs: null,
                  }),
                  activity({
                    chunkId: 'retry',
                    status: 'queued',
                    waitingReason: 'retry',
                    nextRetryAtMs: Date.now() + 10000,
                    sourceLabel: null,
                    endOffsetMs: null,
                  }),
                  activity({
                    chunkId: 'protected',
                    status: 'queued',
                    waitingReason: 'protection',
                    sourceLabel: null,
                    startOffsetMs: null,
                    endOffsetMs: null,
                  }),
                ],
              })
            : value,
        ),
      }),
    ]),
  );
  mount();
  await page.getByRole('button', { name: 'Inspect OCR' }).click();
  const work = within(
    screen.getByRole('list', { name: 'Current and next work' }),
  );
  expect(work.getByText('Worker stopped · reconciling state')).toBeTruthy();
  expect(work.getByText('No measured progress yet')).toBeTruthy();
  expect(work.getByText('Waiting for recording protection')).toBeTruthy();
  expect(work.getByText(/Retry at/)).toBeTruthy();
  expect(work.queryByText('Monitor')).toBeNull();
});
