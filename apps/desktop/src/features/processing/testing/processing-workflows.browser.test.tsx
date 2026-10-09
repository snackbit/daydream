import {
  createRootRoute,
  createRoute,
  createRouter,
  createMemoryHistory,
  RouterProvider,
} from '@tanstack/react-router';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import { useSettingsStore } from '@/features/settings/settings-store';
import { ProcessingScreen } from '@/screens/ProcessingScreen';
import { useBackgroundTaskStore } from '../background-task-store';
import { useRecordingProcessingStore } from '../recording-processing-store';
import { group, groupPage, lane } from './processing-browser-fixtures';

const client = vi.hoisted(() => ({
  listRecordingProcessingGroups: vi.fn(),
  getRecordingProcessingLane: vi.fn(),
  controlRecordingProcessingGroups: vi.fn(),
  listBackgroundTasks: vi.fn(),
  getBackgroundTask: vi.fn(),
  retryProcessingUnits: vi.fn(),
  reprocessRecordingTranscription: vi.fn(),
  listenToBackgroundTasksChanged: vi.fn(),
}));
vi.mock('../processing-client', () => client);
function mount() {
  const root = createRootRoute();
  const index = createRoute({
    getParentRoute: () => root,
    path: '/',
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
  render(
    <RouterProvider
      router={createRouter({
        routeTree: root.addChildren([index, library, settings]),
        history: createMemoryHistory({ initialEntries: ['/'] }),
      })}
    />,
  );
}
beforeEach(() => {
  vi.resetAllMocks();
  useBackgroundTaskStore.setState(
    useBackgroundTaskStore.getInitialState(),
    true,
  );
  useRecordingProcessingStore.setState(
    useRecordingProcessingStore.getInitialState(),
    true,
  );
  useSettingsStore.setState({
    loadStatus: 'ready',
    draft: null,
    settingsState: null,
  });
  client.listBackgroundTasks.mockResolvedValue({ tasks: [], nextCursor: null });
  client.listenToBackgroundTasksChanged.mockResolvedValue(() => undefined);
  client.listRecordingProcessingGroups.mockResolvedValue(groupPage());
});
afterEach(cleanup);

test('group filters request complete native groups and failed pagination retries the same cursor', async () => {
  const cursor = { updatedAtMs: 1000, id: 'synthetic-recording' };
  client.listRecordingProcessingGroups
    .mockResolvedValueOnce(groupPage([group()], { nextCursor: cursor }))
    .mockRejectedValueOnce({ code: 'ai_processing_storage_failed' })
    .mockResolvedValueOnce(
      groupPage([
        group({
          recordingId: 'older-recording',
          title: 'Earlier recording',
          updatedAtMs: 500,
        }),
      ]),
    )
    .mockResolvedValueOnce(groupPage([]));
  mount();
  await page.getByRole('button', { name: 'Load earlier recordings' }).click();
  await screen.findByRole('alert');
  expect(
    screen.getByRole('heading', { name: 'Synthetic recording' }),
  ).toBeTruthy();
  await page.getByRole('button', { name: 'Load earlier recordings' }).click();
  await screen.findByRole('heading', { name: 'Earlier recording' });
  expect(client.listRecordingProcessingGroups).toHaveBeenNthCalledWith(3, {
    filter: 'all',
    cursor,
    knownRecordingIds: [],
  });
  await page.getByRole('tab', { name: 'Needs attention' }).click();
  await waitFor(() =>
    expect(client.listRecordingProcessingGroups).toHaveBeenLastCalledWith({
      filter: 'attention',
      cursor: null,
      knownRecordingIds: [],
    }),
  );
  await screen.findByText('No recordings in this view.');
});

test('partial recording actions show failure and retain successful lane work for recovery', async () => {
  client.controlRecordingProcessingGroups
    .mockResolvedValueOnce({
      results: [
        { jobId: 'synthetic-job', succeeded: true, errorCode: null },
        {
          jobId: 'other-job',
          succeeded: false,
          errorCode: 'ai_processing_resume_pending',
        },
      ],
    })
    .mockResolvedValueOnce({
      results: [{ jobId: 'other-job', succeeded: true, errorCode: null }],
    });
  mount();
  await page
    .getByRole('button', { name: 'Pause recording processing' })
    .click();
  await screen.findByRole('alert');
  expect(client.controlRecordingProcessingGroups).toHaveBeenCalledWith({
    recordingId: 'synthetic-recording',
    action: 'pause',
  });
  expect(
    screen.getByRole('heading', { name: 'Synthetic recording' }),
  ).toBeTruthy();
  await page
    .getByRole('button', { name: 'Pause recording processing' })
    .click();
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
});

test('failed lane details recover and paginate sections while retry targets the selected unit', async () => {
  const ocr = lane('ocr', {
    failed: 1,
    completed: 3,
    errorCode: 'ai_ocr_vision_response_invalid',
  });
  client.listRecordingProcessingGroups.mockResolvedValue(
    groupPage([
      group({ lanes: [lane('audio'), ocr, lane('vlm'), lane('activity')] }),
    ]),
  );
  const unit = {
    id: 'frame-1',
    chunkId: 'ocr-chunk',
    jobId: 'synthetic-job',
    sourceKind: 'ocr',
    sourceTrackId: 'source-screen-1',
    sourceRole: null,
    startOffsetMs: 1000,
    endOffsetMs: null,
    status: 'failed',
    progress: 1,
    errorCode: null,
    attemptCount: 2,
  };
  client.getRecordingProcessingLane
    .mockRejectedValueOnce({ code: 'ai_processing_storage_failed' })
    .mockResolvedValueOnce({
      lane: ocr,
      units: [unit],
      nextCursor: { chunkId: 'ocr-chunk', unitId: 'frame-1' },
    })
    .mockResolvedValueOnce({
      lane: ocr,
      units: [{ ...unit, id: 'frame-2', status: 'succeeded' }],
      nextCursor: null,
    })
    .mockResolvedValue({
      lane: ocr,
      units: [{ ...unit, status: 'queued' }],
      nextCursor: null,
    });
  client.retryProcessingUnits.mockResolvedValue({
    id: 'synthetic-job',
    status: 'queued',
  });
  mount();
  await page
    .getByRole('listitem', { name: 'OCR', exact: true })
    .getByRole('button', { name: 'Details' })
    .click();
  await screen.findByRole('button', { name: 'Retry loading sections' });
  await page.getByRole('button', { name: 'Retry loading sections' }).click();
  await screen.findByRole('button', { name: 'Load more sections' });
  await page.getByRole('button', { name: 'Load more sections' }).click();
  await waitFor(() =>
    expect(client.getRecordingProcessingLane).toHaveBeenNthCalledWith(3, {
      recordingId: 'synthetic-recording',
      sourceKind: 'ocr',
      cursor: { chunkId: 'ocr-chunk', unitId: 'frame-1' },
    }),
  );
  await page
    .getByRole('dialog')
    .getByRole('button', { name: 'Retry section', exact: true })
    .click();
  await waitFor(() =>
    expect(client.retryProcessingUnits).toHaveBeenCalledWith({
      jobId: 'synthetic-job',
      chunkId: 'ocr-chunk',
      unitId: 'frame-1',
      useCurrentSettings: false,
    }),
  );
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: 'Retry section' })).toBeNull(),
  );
  await page
    .getByRole('button', { name: 'Close', exact: true })
    .first()
    .click();
  expect(screen.queryByRole('dialog')).toBeNull();
});
