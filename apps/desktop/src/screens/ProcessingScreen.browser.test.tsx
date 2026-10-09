import {
  createRootRoute,
  createRoute,
  createRouter,
  createMemoryHistory,
  RouterProvider,
} from '@tanstack/react-router';
import type { AiModelAsset } from '@daydream/shared';
import {
  cleanup,
  render,
  screen,
  within,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import type * as AiClient from '@/features/ai/ai-client';
import { useBackgroundTaskStore } from '@/features/processing/background-task-store';
import { useRecordingProcessingStore } from '@/features/processing/recording-processing-store';
import {
  group,
  groupPage,
  lane,
  task,
} from '@/features/processing/testing/processing-browser-fixtures';
import { useSettingsStore } from '@/features/settings/settings-store';
import '@/index.css';
import { ProcessingScreen } from './ProcessingScreen';

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
const recall = vi.hoisted(() => ({ controlRecallJob: vi.fn() }));
const models = vi.hoisted(() => ({
  listAiModels: vi.fn(),
  listenToAiModelsChanged: vi.fn(),
  cancelAiModelDownload: vi.fn(),
}));
vi.mock('@/features/processing/processing-client', () => client);
vi.mock('@/features/recall/recall-client', () => recall);
vi.mock('@/features/ai/ai-client', async (importOriginal) => ({
  ...(await importOriginal<typeof AiClient>()),
  ...models,
}));
function mount() {
  const root = createRootRoute();
  const processing = createRoute({
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
  return render(
    <RouterProvider
      router={createRouter({
        routeTree: root.addChildren([processing, library, settings]),
        history: createMemoryHistory({ initialEntries: ['/'] }),
      })}
    />,
  );
}
beforeEach(() => {
  vi.resetAllMocks();
  useRecordingProcessingStore.setState(
    useRecordingProcessingStore.getInitialState(),
    true,
  );
  useBackgroundTaskStore.setState(
    useBackgroundTaskStore.getInitialState(),
    true,
  );
  useSettingsStore.setState({
    loadStatus: 'ready',
    draft: null,
    settingsState: null,
  });
  client.listRecordingProcessingGroups.mockResolvedValue(groupPage());
  client.listBackgroundTasks.mockResolvedValue({ tasks: [], nextCursor: null });
  client.listenToBackgroundTasksChanged.mockResolvedValue(() => undefined);
  client.controlRecordingProcessingGroups.mockResolvedValue({
    results: [{ jobId: 'synthetic-job', succeeded: true, errorCode: null }],
  });
  models.listenToAiModelsChanged.mockResolvedValue(() => undefined);
});
afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});

test('one recording owns all lanes and lane retry preserves sibling work and configuration choice', async () => {
  const failed = group({
    status: 'completed-with-errors',
    etaMs: null,
    lanes: [
      lane('audio', {
        status: 'succeeded',
        completed: 10,
        running: 0,
        queued: 0,
        progress: 1,
      }),
      lane('ocr', {
        status: 'completed-with-errors',
        completed: 9,
        failed: 1,
        running: 0,
        queued: 0,
        progress: 1,
        errorCode: 'ai_ocr_vision_response_invalid',
      }),
      lane('vlm'),
      lane('activity'),
    ],
  });
  client.listRecordingProcessingGroups.mockResolvedValue(groupPage([failed]));
  mount();
  const list = await screen.findByRole('list', {
    name: 'Recording processing',
  });
  expect(within(list).getAllByRole('heading')).toHaveLength(1);
  const ocr = within(list).getByRole('listitem', { name: 'OCR' });
  await page
    .getByRole('listitem', { name: 'OCR', exact: true })
    .getByRole('button', { name: 'Retry failed', exact: true })
    .click();
  await waitFor(() =>
    expect(client.controlRecordingProcessingGroups).toHaveBeenCalledWith({
      recordingId: 'synthetic-recording',
      action: 'retry',
      sourceKind: 'ocr',
      useCurrentSettings: false,
    }),
  );
  await page
    .getByRole('listitem', { name: 'OCR', exact: true })
    .getByRole('button', { name: 'Use current settings' })
    .click();
  await waitFor(() =>
    expect(client.controlRecordingProcessingGroups).toHaveBeenLastCalledWith({
      recordingId: 'synthetic-recording',
      action: 'retry',
      sourceKind: 'ocr',
      useCurrentSettings: true,
    }),
  );
  expect(
    within(ocr)
      .getByRole('link', { name: 'Review vision settings' })
      .getAttribute('href'),
  ).toContain('/settings');
  expect(screen.queryByText('ai_ocr_vision_response_invalid')).toBeNull();
  expect(
    screen.queryByRole('button', { name: 'Technical details' }),
  ).toBeNull();
});

test('live input reports caught-up work without a fixed completion estimate', async () => {
  client.listRecordingProcessingGroups.mockResolvedValue(
    groupPage([
      group({
        inputOpen: true,
        etaMs: null,
        lanes: ['audio', 'ocr', 'vlm', 'activity'].map((kind) =>
          lane(
            kind === 'audio' || kind === 'ocr' || kind === 'vlm'
              ? kind
              : 'activity',
            {
              inputOpen: true,
              status: 'succeeded',
              total: 1,
              completed: 1,
              running: 0,
              queued: 0,
              progress: 1,
            },
          ),
        ),
      }),
    ]),
  );
  mount();
  expect(
    await screen.findByText('Live recording · no fixed finish time'),
  ).toBeTruthy();
  expect(screen.getAllByText('Caught up · live recording')).toHaveLength(4);
  expect(screen.queryByText(/About .* remaining/)).toBeNull();
});

test('actual recall-index work stays in the collapsed shared section and uses shared controls', async () => {
  client.listBackgroundTasks.mockResolvedValue({
    tasks: [task()],
    nextCursor: null,
  });
  recall.controlRecallJob.mockResolvedValue({});
  mount();
  const disclosure = await screen.findByRole('button', { name: /Shared work/ });
  expect(disclosure.getAttribute('aria-expanded')).toBe('false');
  await page.getByRole('button', { name: /Shared work/ }).click();
  await page
    .getByRole('list', { name: 'Shared background tasks' })
    .getByRole('button', { name: 'Cancel', exact: true })
    .click();
  await waitFor(() =>
    expect(recall.controlRecallJob).toHaveBeenCalledWith({
      jobId: 'shared-recall',
      action: 'cancel',
    }),
  );
  expect(client.controlRecordingProcessingGroups).not.toHaveBeenCalled();
});

test('a shared model cancellation targets the model without changing recording jobs', async () => {
  const model: AiModelAsset = {
    id: 'synthetic-model',
    label: 'Whisper model',
    provider: 'local-whisper-cpp',
    checksum: null,
    checksumAlgorithm: null,
    createdAtMs: 0,
    downloadedBytes: 25,
    downloadStatus: 'downloading',
    etag: null,
    errorCode: null,
    localPath: null,
    selfTestStatus: 'not-run',
    sizeBytes: 100,
    sourceUrl: 'https://example.test/synthetic.bin',
    totalBytes: 100,
    updatedAtMs: 1,
    verificationStatus: 'pending',
  };
  models.listAiModels.mockResolvedValue({ models: [model] });
  models.cancelAiModelDownload.mockResolvedValue(undefined);
  client.listBackgroundTasks.mockResolvedValue({
    tasks: [
      task({
        id: 'model-download:synthetic-model',
        modelId: 'synthetic-model',
        kind: 'model-download',
        title: 'Whisper model',
      }),
    ],
    nextCursor: null,
  });
  mount();
  await page.getByRole('button', { name: /Shared work/ }).click();
  await screen.findByText('Whisper model');
  await page
    .getByRole('list', { name: 'Shared background tasks' })
    .getByRole('button', { name: 'Cancel', exact: true })
    .click();
  await waitFor(() =>
    expect(models.cancelAiModelDownload).toHaveBeenCalledWith({
      modelId: 'synthetic-model',
    }),
  );
  expect(client.controlRecordingProcessingGroups).not.toHaveBeenCalled();
  expect(client.retryProcessingUnits).not.toHaveBeenCalled();
});

test.each([
  ['light', 1440],
  ['dark', 1440],
  ['light', 390],
  ['dark', 390],
  ['light', 320],
  ['dark', 320],
] as const)(
  'lane recovery and shared disclosure work in %s at %i pixels',
  async (theme, width) => {
    await page.viewport(width, 900);
    document.documentElement.dataset.theme = theme;
    client.listRecordingProcessingGroups.mockResolvedValue(
      groupPage([
        group({
          lanes: [
            lane('audio'),
            lane('ocr', {
              failed: 1,
              completed: 3,
              errorCode: 'ai_ocr_vision_auth_failed',
            }),
            lane('vlm'),
            lane('activity'),
          ],
        }),
      ]),
    );
    mount();
    await screen.findByRole('listitem', { name: 'OCR' });
    await page
      .getByRole('listitem', { name: 'OCR', exact: true })
      .getByRole('button', { name: 'Use current settings' })
      .click();
    await waitFor(() =>
      expect(client.controlRecordingProcessingGroups).toHaveBeenCalled(),
    );
    await page.getByRole('button', { name: /Shared work/ }).click();
    expect(
      screen
        .getByRole('button', { name: /Shared work/ })
        .getAttribute('aria-expanded'),
    ).toBe('true');
    await page.screenshot();
  },
);
