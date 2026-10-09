import type {
  AiModelAsset,
  AppSettingsState,
  InputActivityStatus,
  RecordWindowProtectionSettings,
} from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { useNotificationsStore } from '@/features/notifications/notifications-store';
import type * as RecorderClientModule from '@/features/recorder/recorder-client';
import { useSettingsStore } from '@/features/settings/settings-store';
import {
  defaultAiSettings,
  defaultWindowProtection,
  defaultSettingsState,
  scene,
  aiModels,
  availableInputActivityStatus,
} from '@/features/settings/testing/settings-browser-fixtures';
import '@/index.css';
import { SettingsScreen } from './SettingsScreen';

const aiClient = vi.hoisted(() => ({
  cancelAiModelDownload: vi.fn(),
  deleteAiModel: vi.fn(),
  deleteAiProviderSecret: vi.fn(),
  downloadAiModel: vi.fn(),
  getAiProviderStatus: vi.fn(),
  listenToAiModelsChanged: vi.fn(),
  listAiModels: vi.fn(),
  saveAiProviderSecret: vi.fn(),
  selfTestAiModel: vi.fn(),
}));
const transferClient = vi.hoisted(() => ({
  pauseAiModelDownload: vi.fn(),
  resumeAiModelDownload: vi.fn(),
  removeAiModelPartial: vi.fn(),
}));
const settingsClient = vi.hoisted(() => ({
  getAppSettings: vi.fn(),
  getInputActivityStatus: vi.fn(),
  previewAppSettingsStorageCleanup: vi.fn(),
  repairInputActivityPermissions: vi.fn(),
  resetAppSettings: vi.fn(),
  saveAppSettings: vi.fn(),
}));
const maintenanceClient = vi.hoisted(() => ({
  getMaintenanceStatus: vi.fn(),
  previewMaintenance: vi.fn(),
  executeMaintenance: vi.fn(),
  acknowledgeMaintenanceThemeReset: vi.fn(),
}));
vi.mock('@/features/settings/maintenance-client', () => maintenanceClient);
const recorderClient = vi.hoisted(() => ({
  getRecorderStatus: vi.fn(),
  getRecorderPreviewStatus: vi.fn(),
  getCaptureSources: vi.fn(),
  getAudioSources: vi.fn(),
  getWindowProtectionStatus: vi.fn(),
  getRecorderScene: vi.fn(),
  listProtectableWindows: vi.fn(),
}));
const recallClient = vi.hoisted(() => ({
  getRecallStatus: vi.fn(),
  listenToRecallStateChanged: vi.fn(),
  testRecallProvider: vi.fn(),
}));
const router = vi.hoisted(() => {
  const blocker = {
    action: undefined,
    current: undefined,
    next: undefined,
    proceed: vi.fn(),
    reset: vi.fn(),
    status: 'idle' as 'blocked' | 'idle',
  };
  const state = {
    blocker,
    blockerOptions: null as unknown,
  };

  return {
    state,
    useBlocker: vi.fn((options: unknown) => {
      state.blockerOptions = options;
      return blocker;
    }),
  };
});

vi.mock('@/features/ai/ai-client', () => aiClient);
vi.mock('@/features/ai/models/model-transfer-client', () => transferClient);
vi.mock('@/features/settings/settings-client', () => settingsClient);
vi.mock('@/features/recorder/recorder-client', async (importOriginal) => ({
  ...(await importOriginal<typeof RecorderClientModule>()),
  ...recorderClient,
}));
vi.mock('@/features/recall/recall-client', () => recallClient);
vi.mock('@tanstack/react-router', () => ({
  useBlocker: router.useBlocker,
}));

async function openAiSettingsTab() {
  await screen.findByRole('tablist', { name: 'Settings sections' });
  const aiTab = screen.getByRole('tab', { name: 'AI/LLM' });
  fireEvent.mouseDown(aiTab, { button: 0, ctrlKey: false });
  fireEvent.mouseUp(aiTab);
  fireEvent.click(aiTab);
  await waitFor(() => expect(aiTab.getAttribute('aria-selected')).toBe('true'));
  await screen.findByText('Transcription');
}

function embeddingModel(id: string, label: string): AiModelAsset {
  return {
    id,
    label,
    provider: 'local-embedding',
    createdAtMs: 0,
    updatedAtMs: 0,
    checksum: null,
    checksumAlgorithm: 'sha256',
    downloadedBytes: 0,
    totalBytes: 100,
    sizeBytes: 100,
    downloadStatus: 'missing',
    etag: null,
    errorCode: null,
    localPath: null,
    sourceUrl: 'https://synthetic.example/model.gguf',
    selfTestStatus: 'not-run',
    verificationStatus: 'unknown',
  };
}

test('E5 admission failure stays beside its model across focus refresh and BGE actions, then its retry clears it', async () => {
  const e5 = embeddingModel(
    'multilingual-e5-small-q8_0',
    'Multilingual E5 Small',
  );
  const bge = embeddingModel('bge-small-en-v1.5-q8_0', 'BGE Small English');
  let models = [...aiModels, e5, bge];
  aiClient.listAiModels.mockImplementation(async () => ({ models }));
  aiClient.downloadAiModel
    .mockRejectedValueOnce({
      code: 'ai_storage_failed',
      message: 'SQLite CHECK constraint failed: private technical details',
    })
    .mockImplementation(async ({ modelId }: { modelId: string }) => {
      models = models.map((model) =>
        model.id === modelId
          ? { ...model, downloadStatus: 'downloading' }
          : model,
      );
    });
  recallClient.getRecallStatus.mockResolvedValue({
    privacyRevision: 0,
    indexedEvidenceCount: 0,
    pendingEvidenceCount: 0,
    embeddingReady: false,
    generationReady: false,
    embeddingUnavailableReason: 'recall_embedding_model_missing',
    generationUnavailableReason: 'recall_generation_disabled',
    jobs: [],
  });
  render(<SettingsScreen tab="ai" />);
  await page
    .getByRole('group', { name: e5.label, exact: true })
    .getByRole('button', { name: 'Download', exact: true })
    .click();
  const e5Group = await screen.findByRole('group', { name: e5.label });
  expect(await within(e5Group).findByRole('alert')).toHaveTextContent(
    'The download could not be saved',
  );
  expect(screen.queryByText(/SQLite CHECK/)).toBeNull();
  const readsBeforeFocus = aiClient.listAiModels.mock.calls.length;
  fireEvent(window, new Event('focus'));
  await waitFor(() =>
    expect(aiClient.listAiModels.mock.calls.length).toBeGreaterThan(
      readsBeforeFocus,
    ),
  );
  expect(within(e5Group).getByRole('alert')).toHaveTextContent(
    'The download could not be saved',
  );
  await page
    .getByRole('group', { name: bge.label, exact: true })
    .getByRole('button', { name: 'Download', exact: true })
    .click();
  await waitFor(() =>
    expect(aiClient.downloadAiModel).toHaveBeenCalledWith({ modelId: bge.id }),
  );
  await within(screen.getByRole('group', { name: bge.label })).findByRole(
    'button',
    { name: 'Pause' },
  );
  expect(within(e5Group).getByRole('alert')).toHaveTextContent(
    'The download could not be saved',
  );
  expect(screen.queryByText('recall_embedding_model_missing')).toBeNull();
  expect(screen.queryByText('recall_generation_disabled')).toBeNull();
  await page
    .getByRole('group', { name: e5.label, exact: true })
    .getByRole('button', { name: 'Download', exact: true })
    .click();
  await waitFor(() => expect(within(e5Group).queryByRole('alert')).toBeNull());
  await within(e5Group).findByRole('button', { name: 'Pause' });
});

test('Settings pauses an E5 transfer until native acknowledgement, resumes saved bytes and removes only partial data', async () => {
  let e5: AiModelAsset = {
    ...embeddingModel('multilingual-e5-small-q8_0', 'Multilingual E5 Small'),
    downloadStatus: 'downloading',
    downloadedBytes: 25,
    transfer: {
      attempt: 1,
      phase: 'transferring',
      bytesPerSecond: 20,
      etaMs: 3750,
    },
  };
  aiClient.listAiModels.mockImplementation(async () => ({
    models: [...aiModels, e5],
  }));
  let acknowledgePause: (() => void) | undefined;
  transferClient.pauseAiModelDownload.mockImplementation(
    () =>
      new Promise((resolve) => {
        acknowledgePause = () => {
          e5 = {
            ...e5,
            downloadStatus: 'paused',
            transfer: {
              attempt: 1,
              phase: 'paused',
              bytesPerSecond: null,
              etaMs: null,
            },
          };
          resolve({ models: [e5] });
        };
      }),
  );
  transferClient.resumeAiModelDownload.mockImplementation(async () => {
    e5 = {
      ...e5,
      downloadStatus: 'queued',
      transfer: {
        attempt: 2,
        phase: 'queued',
        bytesPerSecond: null,
        etaMs: null,
      },
    };
  });
  transferClient.removeAiModelPartial.mockImplementation(async () => {
    e5 = { ...e5, downloadStatus: 'missing', downloadedBytes: 0 };
  });
  render(<SettingsScreen tab="ai" />);
  const row = await screen.findByRole('group', { name: e5.label });
  await page
    .getByRole('group', { name: e5.label, exact: true })
    .getByRole('button', { name: 'Pause', exact: true })
    .click();
  expect(within(row).getByRole('button', { name: 'Pause' })).toBeDisabled();
  expect(transferClient.pauseAiModelDownload).toHaveBeenCalledTimes(1);
  if (!acknowledgePause) throw new Error('Pause command was not admitted');
  acknowledgePause();
  await within(row).findByRole('button', { name: 'Resume' });
  expect(within(row).getByText(/25 B of 100 B/)).toBeTruthy();
  await page
    .getByRole('group', { name: e5.label, exact: true })
    .getByRole('button', { name: 'Resume', exact: true })
    .click();
  await within(row).findByRole('button', { name: 'Pause' });
  expect(transferClient.resumeAiModelDownload).toHaveBeenCalledWith({
    modelId: e5.id,
  });
  await page
    .getByRole('group', { name: e5.label, exact: true })
    .getByRole('button', { name: 'Pause', exact: true })
    .click();
  if (!acknowledgePause)
    throw new Error('Second pause command was not admitted');
  acknowledgePause();
  await within(row).findByRole('button', { name: 'Remove partial data' });
  await page
    .getByRole('group', { name: e5.label, exact: true })
    .getByRole('button', { name: 'Remove partial data', exact: true })
    .click();
  await within(row).findByRole('button', { name: 'Download' });
  expect(transferClient.removeAiModelPartial).toHaveBeenCalledWith({
    modelId: e5.id,
  });
  expect(aiClient.deleteAiModel).not.toHaveBeenCalled();
});

async function openRecordSettingsTab() {
  await screen.findByRole('tablist', { name: 'Settings sections' });
  const recordTab = screen.getByRole('tab', { name: 'Record' });
  fireEvent.mouseDown(recordTab, { button: 0, ctrlKey: false });
  fireEvent.mouseUp(recordTab);
  fireEvent.click(recordTab);
  await waitFor(() =>
    expect(recordTab.getAttribute('aria-selected')).toBe('true'),
  );
  await screen.findByText('Frame cadence');
}

async function selectOption(label: string, option: string) {
  const trigger = screen.getByRole('combobox', { name: label });
  trigger.focus();
  fireEvent.keyDown(trigger, { key: 'ArrowDown' });
  await userEvent.click(await screen.findByRole('option', { name: option }));
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  expect(document.activeElement).toBe(trigger);
}

async function selectRemoteTranscriptionProvider() {
  await selectOption('Provider', 'OpenAI-compatible');
  await screen.findByLabelText('API key');
}

function settingsWithVision({
  observationsEnabled,
}: {
  observationsEnabled: boolean;
}): AppSettingsState {
  return {
    ...defaultSettingsState,
    settings: {
      ...defaultSettingsState.settings,
      ai: {
        ...defaultSettingsState.settings.ai,
        ocr: {
          ...defaultSettingsState.settings.ai.ocr,
          engine: 'vision',
        },
        vision: {
          ...defaultSettingsState.settings.ai.vision,
          apiKeyRef: 'ai.secret.vision.saved',
          baseUrl: 'https://localhost:11434/v1',
          model: 'vision-model',
          observationsEnabled,
          provider: 'openai-compatible',
        },
      },
    },
  };
}

beforeEach(() => {
  vi.resetAllMocks();
  recorderClient.getRecorderStatus.mockResolvedValue({
    state: 'idle',
    activeRecordingId: null,
    activeSceneId: null,
    activeSegmentId: null,
    cleanupDisposition: 'ready',
    finalization: null,
    lastError: null,
    statusRevision: 1,
    updatedAtMs: 1,
  });
  recorderClient.getRecorderPreviewStatus.mockResolvedValue({
    state: 'idle',
    sceneId: null,
    sourceIds: [],
    error: null,
    updatedAtMs: 1,
  });
  recorderClient.getCaptureSources.mockResolvedValue({
    sources: [],
    error: null,
  });
  recorderClient.getAudioSources.mockResolvedValue({
    sources: [],
    error: null,
  });
  maintenanceClient.getMaintenanceStatus.mockResolvedValue(null);
  maintenanceClient.previewMaintenance.mockImplementation(
    async ({ action }) => ({
      action,
      token: 'synthetic-impact',
      recordingCount: 2,
      libraryRowCount: 5,
      modelCount: 1,
      fileCount: 0,
      sizeBytes: 0,
    }),
  );
  maintenanceClient.executeMaintenance.mockImplementation(
    async ({ action }) => ({
      action,
      operationId: 'synthetic-operation',
      phase: 'awaiting-theme-reset',
      error: null,
    }),
  );
  maintenanceClient.acknowledgeMaintenanceThemeReset.mockResolvedValue({
    action: 'reset-preferences',
    operationId: 'synthetic-operation',
    phase: 'completed',
    error: null,
  });
  recallClient.listenToRecallStateChanged.mockResolvedValue(vi.fn());
  recallClient.getRecallStatus.mockResolvedValue({
    privacyRevision: 0,
    indexedEvidenceCount: 0,
    pendingEvidenceCount: 0,
    embeddingReady: false,
    generationReady: false,
    embeddingUnavailableReason: 'Model not downloaded',
    generationUnavailableReason: 'Generation disabled',
    jobs: [],
  });
  recallClient.testRecallProvider.mockResolvedValue({
    kind: 'generation',
    status: 'succeeded',
    errorCode: null,
    elapsedMs: 12,
  });
  settingsClient.getAppSettings.mockResolvedValue(defaultSettingsState);
  settingsClient.saveAppSettings.mockImplementation(async ({ settings }) => ({
    ...defaultSettingsState,
    settings,
  }));
  settingsClient.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 0,
    sizeBytes: 0,
  });
  settingsClient.getInputActivityStatus.mockResolvedValue(
    availableInputActivityStatus,
  );
  settingsClient.repairInputActivityPermissions.mockResolvedValue({
    message: 'Input activity permissions are already available.',
    status: availableInputActivityStatus,
    success: true,
  });
  settingsClient.resetAppSettings.mockResolvedValue(defaultSettingsState);
  recorderClient.getRecorderScene.mockResolvedValue(scene);
  recorderClient.getWindowProtectionStatus.mockResolvedValue({
    message: 'Focused-window detection is available through X11.',
    platform: 'linux',
    provider: 'linux-x11',
    status: 'available',
  });
  aiClient.listAiModels.mockResolvedValue({ models: aiModels });
  aiClient.listenToAiModelsChanged.mockResolvedValue(vi.fn());
  aiClient.getAiProviderStatus.mockResolvedValue({
    localTranscription: {
      availableLanguages: [
        { id: 'en', label: 'English' },
        { id: 'de', label: 'German' },
      ],
      availableBackends: [
        { available: true, id: 'cpu', reason: null },
        {
          available: false,
          id: 'vulkan',
          reason: 'Vulkan backend is not compiled in this build.',
        },
        {
          available: false,
          id: 'cuda',
          reason: 'CUDA backend is not compiled in this build.',
        },
      ],
      checksumStatus: 'unknown',
      localModelId: 'small',
      modelDownloadStatus: 'missing',
      modelPath: null,
      selectedBackend: 'cpu',
      selfTestStatus: 'not-run',
    },
    remoteTranscription: 'not-configured',
    vision: 'disabled',
  });
  aiClient.saveAiProviderSecret.mockResolvedValue({
    secretRef: 'ai.secret.transcription.saved',
  });
  router.state.blocker.status = 'idle';
  router.state.blocker.proceed.mockReset();
  router.state.blocker.reset.mockReset();
  router.state.blockerOptions = null;
  router.useBlocker.mockClear();
  useSettingsStore.setState({
    draft: null,
    error: null,
    isDirty: false,
    loadStatus: 'idle',
    mutationStatus: 'idle',
    savedScene: null,
    settingsState: null,
    successMessage: null,
  });
  useNotificationsStore.setState({
    error: null,
    notifications: [],
    status: 'idle',
  });
});

afterEach(async () => {
  cleanup();
  delete document.documentElement.dataset.theme;
  await page.viewport(1440, 960);
});

test.each([
  ['light', 1440],
  ['dark', 1440],
  ['light', 1024],
  ['dark', 1024],
  ['light', 768],
  ['dark', 768],
  ['light', 390],
  ['dark', 390],
  ['light', 320],
  ['dark', 320],
] as const)(
  'Vision settings align and remain reachable in %s at %i pixels',
  async (theme, width) => {
    await page.viewport(width, 900);
    document.documentElement.dataset.theme = theme;
    settingsClient.getAppSettings.mockResolvedValue({
      ...defaultSettingsState,
      settings: {
        ...defaultSettingsState.settings,
        ai: {
          ...defaultSettingsState.settings.ai,
          ocr: { ...defaultSettingsState.settings.ai.ocr, engine: 'vision' },
          vision: {
            ...defaultSettingsState.settings.ai.vision,
            provider: 'openai-compatible',
            baseUrl: 'http://localhost:8080/v1',
            model: 'synthetic-vision',
            authMode: 'none',
            observationsEnabled: true,
          },
        },
      },
    });
    render(<SettingsScreen tab="ai" />);
    const model = await screen.findByRole('textbox', { name: 'Vision model' });
    const save = screen.getByRole('button', { name: 'Save' });
    const probe = screen.getByRole('button', { name: /test vision/i });
    for (const control of [
      model,
      save,
      probe,
      screen.getByRole('combobox', { name: 'Embedding provider' }),
      screen.getByRole('button', { name: 'Test generation provider' }),
      screen.getByRole('combobox', { name: 'Authentication' }),
    ]) {
      const bounds = control.getBoundingClientRect();
      expect(bounds.width).toBeGreaterThan(0);
      expect(bounds.left).toBeGreaterThanOrEqual(0);
      expect(bounds.right).toBeLessThanOrEqual(width);
    }
    expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(width);
    const visionSection = document.getElementById('ai-vision');
    expect(visionSection).not.toBeNull();
    if (!visionSection) throw new Error('Vision settings are not visible');
    expect(
      Math.abs(
        visionSection.getBoundingClientRect().left -
          screen
            .getByRole('heading', { name: 'Settings' })
            .getBoundingClientRect().left,
      ),
    ).toBeLessThan(1);
    expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(width);
    await page.screenshot({ element: visionSection });
    await page.screenshot({
      element: screen.getByRole('tablist', { name: 'Settings sections' }),
    });
    const recallSection = document.getElementById('ai-recall');
    if (!recallSection) throw new Error('Recall settings are not visible');
    await page.screenshot({ element: recallSection });
  },
);

test('SettingsScreen edits record and storage settings then resets them', async () => {
  render(<SettingsScreen />);

  await openRecordSettingsTab();
  expect(screen.getByText('1 frame every 5 seconds')).toBeTruthy();
  expect(screen.getByText('2560 x 1440')).toBeTruthy();
  expect(screen.getByText('712.39 MiB')).toBeTruthy();
  expect(
    screen
      .getByRole('radio', { name: /one frame every/i })
      .getAttribute('aria-checked'),
  ).toBe('true');

  fireEvent.click(screen.getByText('Frames per second'));
  await waitFor(() =>
    expect(
      screen
        .getByRole('radio', { name: /frames per second/i })
        .getAttribute('aria-checked'),
    ).toBe('true'),
  );
  fireEvent.change(
    screen.getByRole('spinbutton', { name: 'Frames per second' }),
    {
      target: { value: '2' },
    },
  );
  fireEvent.change(screen.getByLabelText('Video bitrate'), {
    target: { value: '2000' },
  });
  const storageTab = screen.getByRole('tab', { name: 'Storage/Library' });
  fireEvent.mouseDown(storageTab, { button: 0, ctrlKey: false });
  fireEvent.mouseUp(storageTab);
  fireEvent.click(storageTab);
  await screen.findByLabelText('Recording retention');
  fireEvent.change(screen.getByLabelText('Recording retention'), {
    target: { value: '30' },
  });
  fireEvent.change(screen.getByLabelText('Storage size limit'), {
    target: { value: '60' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(
      settingsClient.previewAppSettingsStorageCleanup,
    ).toHaveBeenCalledWith({
      settings: {
        ai: defaultAiSettings,
        debug: defaultSettingsState.settings.debug,
        expert: defaultSettingsState.settings.expert,
        record: {
          fps: 2,
          videoBitrateKbps: 2_000,
          windowProtection: defaultWindowProtection,
        },
        storage: {
          retentionDays: 30,
          sizeLimitBytes: 60 * 1024 * 1024 * 1024,
        },
      },
    }),
  );
  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      settings: {
        ai: defaultAiSettings,
        debug: defaultSettingsState.settings.debug,
        expert: defaultSettingsState.settings.expert,
        record: {
          fps: 2,
          videoBitrateKbps: 2_000,
          windowProtection: defaultWindowProtection,
        },
        storage: {
          retentionDays: 30,
          sizeLimitBytes: 60 * 1024 * 1024 * 1024,
        },
      },
    }),
  );
  expect(await screen.findByText('Settings saved.')).toBeTruthy();

  recorderClient.getRecorderScene.mockResolvedValue(null);
  await page
    .getByRole('button', { name: 'Reset preferences', exact: true })
    .click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  expect(maintenanceClient.executeMaintenance).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Confirm', exact: true }).click();
  await waitFor(() =>
    expect(
      maintenanceClient.executeMaintenance,
    ).toHaveBeenCalledExactlyOnceWith({
      action: 'reset-preferences',
      previewToken: 'synthetic-impact',
      confirmation: 'RESET PREFERENCES',
    }),
  );
  expect(settingsClient.resetAppSettings).not.toHaveBeenCalled();
  expect(
    settingsClient.previewAppSettingsStorageCleanup,
  ).toHaveBeenCalledOnce();
  expect(await screen.findByText('Reset preferences completed.')).toBeTruthy();
});

test('SettingsScreen shows protected-window provider failures', async () => {
  const message =
    'Focused-window detection requires a Wayland compositor that exposes wlr-foreign-toplevel-management or KDE KWin scripting.';
  const windowProtection: RecordWindowProtectionSettings = {
    ...defaultWindowProtection,
    enabled: true,
    rules: [
      {
        action: 'inherit',
        appIdentity: 'org.example.Secret',
        enabled: true,
        id: 'protected-rule-1',
        label: 'Secret app',
        processName: null,
        titleMatcher: null,
      },
    ],
  };
  settingsClient.getAppSettings.mockResolvedValue({
    ...defaultSettingsState,
    settings: {
      ...defaultSettingsState.settings,
      record: {
        ...defaultSettingsState.settings.record,
        windowProtection,
      },
    },
  });
  recorderClient.getWindowProtectionStatus.mockResolvedValue({
    message,
    platform: 'linux',
    provider: 'linux-wayland',
    status: 'provider-unavailable',
  });
  recorderClient.listProtectableWindows.mockResolvedValue({
    provider: 'linux-wayland',
    status: 'provider-unavailable',
    windows: [],
  });

  render(<SettingsScreen />);

  await openRecordSettingsTab();
  expect(
    await screen.findByText('Protected-window automation unavailable'),
  ).toBeTruthy();
  expect(screen.getByText(message)).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: 'Pick window' }));

  const pickerDialog = screen.getByRole('dialog', { name: 'Pick a window' });
  expect(
    (await within(pickerDialog).findByRole('alert')).textContent,
  ).toContain(message);
});

test('SettingsScreen confirms cleanup before saving deletion-causing storage settings', async () => {
  settingsClient.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 2,
    sizeBytes: 3 * 1024 * 1024 * 1024,
  });
  render(<SettingsScreen />);

  await openRecordSettingsTab();
  expect(screen.getByText('1 frame every 5 seconds')).toBeTruthy();
  fireEvent.change(screen.getByLabelText('Video bitrate'), {
    target: { value: '2000' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  expect(await screen.findByText('Delete old recordings?')).toBeTruthy();
  expect(screen.getByText(/delete 2 sessions/i)).toBeTruthy();
  expect(settingsClient.saveAppSettings).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole('button', { name: /^delete and save$/i }));

  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalled(),
  );
});

test('SettingsScreen does not save remote API keys before cleanup confirmation', async () => {
  settingsClient.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 2,
    sizeBytes: 3 * 1024 * 1024 * 1024,
  });
  render(<SettingsScreen />);

  await openAiSettingsTab();
  await selectRemoteTranscriptionProvider();
  fireEvent.change(screen.getByLabelText('API key'), {
    target: { value: 'sk-private' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  expect(await screen.findByText('Delete old recordings?')).toBeTruthy();
  expect(aiClient.saveAiProviderSecret).not.toHaveBeenCalled();
  expect(settingsClient.saveAppSettings).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole('button', { name: /^cancel$/i }));

  await waitFor(() =>
    expect(screen.queryByText('Delete old recordings?')).toBeNull(),
  );
  expect(aiClient.saveAiProviderSecret).not.toHaveBeenCalled();
  expect(settingsClient.saveAppSettings).not.toHaveBeenCalled();
  expect(screen.getByDisplayValue('sk-private')).toBeTruthy();
});

test('SettingsScreen saves remote transcription settings and the new key in one request', async () => {
  render(<SettingsScreen />);
  await openAiSettingsTab();
  await selectRemoteTranscriptionProvider();
  fireEvent.change(await screen.findByLabelText('Base URL'), {
    target: { value: 'https://localhost:11434/v1' },
  });
  fireEvent.change(screen.getByLabelText('API key'), {
    target: { value: 'synthetic-key' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));
  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      credentials: { transcriptionKey: 'synthetic-key' },
      settings: expect.objectContaining({
        ai: expect.objectContaining({
          transcription: expect.objectContaining({
            provider: 'openai-compatible',
            remoteBaseUrl: 'https://localhost:11434/v1',
          }),
        }),
      }),
    }),
  );
  expect(aiClient.saveAiProviderSecret).not.toHaveBeenCalled();
  await waitFor(() =>
    expect(screen.queryByDisplayValue('synthetic-key')).toBeNull(),
  );
});

test('SettingsScreen saves a first Vision profile and its new key together', async () => {
  render(<SettingsScreen />);
  await openAiSettingsTab();
  await selectOption('Text recognition engine', 'Vision model');
  expect(screen.getByText(/You can save this setup now/)).toBeTruthy();
  await selectOption('Vision provider', 'OpenAI-compatible');
  fireEvent.change(await screen.findByLabelText('Base URL'), {
    target: { value: 'http://localhost:8080/v1' },
  });
  fireEvent.change(screen.getByLabelText('Vision model'), {
    target: { value: 'synthetic-vision' },
  });
  fireEvent.change(screen.getByLabelText('API key'), {
    target: { value: 'synthetic-key' },
  });
  expect(screen.getByText('New key, not saved.')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));
  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      credentials: { visionKey: 'synthetic-key' },
      settings: expect.objectContaining({
        ai: expect.objectContaining({
          ocr: expect.objectContaining({ engine: 'vision', enabled: true }),
          vision: expect.objectContaining({
            provider: 'openai-compatible',
            model: 'synthetic-vision',
            baseUrl: 'http://localhost:8080/v1',
          }),
        }),
      }),
    }),
  );
  expect(aiClient.saveAiProviderSecret).not.toHaveBeenCalled();
  await waitFor(() =>
    expect(screen.queryByDisplayValue('synthetic-key')).toBeNull(),
  );
});

test('turning off screen observations keeps the Vision OCR endpoint and saved key', async () => {
  const savedVision = settingsWithVision({ observationsEnabled: true });
  settingsClient.getAppSettings.mockResolvedValue(savedVision);

  render(<SettingsScreen />);
  await openAiSettingsTab();

  const observations = screen.getByRole('checkbox', {
    name: /Generate screen observations/,
  });
  expect(observations.getAttribute('aria-checked')).toBe('true');
  fireEvent.click(observations);
  expect(observations.getAttribute('aria-checked')).toBe('false');
  expect(screen.getByPlaceholderText('Saved key')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      settings: expect.objectContaining({
        ai: expect.objectContaining({
          ocr: expect.objectContaining({ engine: 'vision', enabled: true }),
          vision: expect.objectContaining({
            apiKeyRef: 'ai.secret.vision.saved',
            observationsEnabled: false,
            provider: 'openai-compatible',
          }),
        }),
      }),
    }),
  );
  expect(aiClient.deleteAiProviderSecret).not.toHaveBeenCalled();
  expect(aiClient.saveAiProviderSecret).not.toHaveBeenCalled();
});

test('switching OCR back to Tesseract leaves the configured Vision endpoint intact', async () => {
  const savedVision = settingsWithVision({ observationsEnabled: false });
  settingsClient.getAppSettings.mockResolvedValue(savedVision);

  render(<SettingsScreen />);
  await openAiSettingsTab();
  await selectOption('Text recognition engine', 'Tesseract (local)');
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      settings: expect.objectContaining({
        ai: expect.objectContaining({
          ocr: expect.objectContaining({ engine: 'tesseract', enabled: true }),
          vision: expect.objectContaining({
            apiKeyRef: 'ai.secret.vision.saved',
            observationsEnabled: false,
            provider: 'openai-compatible',
          }),
        }),
      }),
    }),
  );
  expect(aiClient.deleteAiProviderSecret).not.toHaveBeenCalled();
});

test('incomplete Vision setup saves without blocking unrelated settings', async () => {
  render(<SettingsScreen />);
  await openAiSettingsTab();
  await selectOption('Text recognition engine', 'Vision model');
  expect(screen.getByText(/You can save this setup now/)).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));
  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      settings: expect.objectContaining({
        ai: expect.objectContaining({
          ocr: expect.objectContaining({ engine: 'vision' }),
        }),
      }),
    }),
  );
});

test('SettingsScreen toggles checkbox fields from their description text', async () => {
  render(<SettingsScreen />);

  await openAiSettingsTab();
  await selectOption('Language', 'German');
  fireEvent.click(
    screen.getByText(
      'Providers may omit word timestamps; Daydream keeps segment timestamps either way.',
    ),
  );
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      settings: {
        ai: {
          ...defaultAiSettings,
          transcription: {
            ...defaultAiSettings.transcription,
            language: 'de',
            wordTimestampsEnabled: false,
          },
        },
        debug: defaultSettingsState.settings.debug,
        expert: defaultSettingsState.settings.expert,
        record: defaultSettingsState.settings.record,
        storage: defaultSettingsState.settings.storage,
      },
    }),
  );
});

test('SettingsScreen reveals expert transcription controls from General settings', async () => {
  render(<SettingsScreen />);

  await screen.findByRole('tablist', { name: 'Settings sections' });
  expect(screen.queryByText('Silence threshold')).toBeNull();

  fireEvent.click(screen.getByText('Enable expert mode'));
  await openAiSettingsTab();

  expect(screen.getByText('Silence threshold')).toBeTruthy();
  expect(screen.getByText('Minimum speech')).toBeTruthy();
  expect(screen.getByText('Merge close speech gaps')).toBeTruthy();
  expect(screen.getByText('Whisper initial timestamp cap')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      settings: expect.objectContaining({
        expert: {
          enabled: true,
        },
      }),
    }),
  );
});

test('SettingsScreen repairs Linux input activity permissions from AI settings', async () => {
  const deniedStatus = {
    ...availableInputActivityStatus,
    available: false,
    message:
      'Keyboard and mouse activity capture cannot read Linux input devices.',
    permissionDeniedDeviceCount: 2,
    readableDeviceCount: 0,
    reason: 'permission-denied',
    repairAvailable: true,
    userInInputGroup: false,
  } satisfies InputActivityStatus;
  const repairedStatus = {
    ...deniedStatus,
    message:
      'Input activity permission repair completed. Log out and back in, then reopen Daydream.',
    repairAvailable: false,
    userInInputGroup: true,
  } satisfies InputActivityStatus;
  settingsClient.getInputActivityStatus.mockResolvedValue(deniedStatus);
  settingsClient.repairInputActivityPermissions.mockResolvedValue({
    message:
      'Input activity permission repair completed. Log out and back in, then reopen Daydream.',
    status: repairedStatus,
    success: true,
  });

  render(<SettingsScreen />);

  await openAiSettingsTab();
  expect(await screen.findByText('Input activity unavailable')).toBeTruthy();
  expect(screen.getByText(/0\/2 input devices readable/i)).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Repair permissions' }));

  await waitFor(() =>
    expect(settingsClient.repairInputActivityPermissions).toHaveBeenCalled(),
  );
  expect(
    await screen.findByText('Log out and back in, then reopen Daydream.'),
  ).toBeTruthy();
});

test('SettingsScreen saves AI slider values', async () => {
  render(<SettingsScreen />);

  await screen.findByRole('tablist', { name: 'Settings sections' });
  fireEvent.click(screen.getByText('Enable expert mode'));
  await openAiSettingsTab();
  const parallelJobs = screen.getByRole('slider', { name: 'Parallel jobs' });
  const chunksPerJob = screen.getByRole('slider', { name: 'Chunks per job' });
  const fullFrameInterval = screen.getByRole('slider', {
    name: 'Full-frame interval',
  });
  const deltaTileSize = screen.getByRole('slider', {
    name: 'Delta tile size',
  });
  const keyboardActivityWindow = screen.getByRole('slider', {
    name: 'Keyboard activity window',
  });
  const mouseActivityWindow = screen.getByRole('slider', {
    name: 'Mouse activity window',
  });
  const windowActivity = screen.getByRole('checkbox', {
    name: /Use window activity/i,
  });

  expect(screen.getByText('1 frame every 5 seconds')).toBeTruthy();
  expect(
    screen.getByText(
      'Range: 1 frame every 60 seconds to 5 frames per second. Step: 0.05 fps.',
    ),
  ).toBeTruthy();
  expect(screen.getByText('1 minute')).toBeTruthy();
  expect(
    screen.getByText('Range: 5 seconds to 10 minutes. Step: 0.5 seconds.'),
  ).toBeTruthy();
  expect(parallelJobs.getAttribute('aria-valuenow')).toBe('1');
  expect(chunksPerJob.getAttribute('aria-valuenow')).toBe('1');
  expect(fullFrameInterval.getAttribute('aria-valuenow')).toBe('60000');
  expect(deltaTileSize.getAttribute('aria-valuenow')).toBe('64');
  expect(keyboardActivityWindow.getAttribute('aria-valuenow')).toBe('30000');
  expect(mouseActivityWindow.getAttribute('aria-valuenow')).toBe('5000');
  expect(windowActivity.getAttribute('aria-checked')).toBe('true');

  parallelJobs.focus();
  fireEvent.keyDown(parallelJobs, { key: 'ArrowRight' });
  chunksPerJob.focus();
  fireEvent.keyDown(chunksPerJob, { key: 'ArrowRight' });
  fullFrameInterval.focus();
  fireEvent.keyDown(fullFrameInterval, { key: 'ArrowRight' });
  deltaTileSize.focus();
  fireEvent.keyDown(deltaTileSize, { key: 'ArrowRight' });
  keyboardActivityWindow.focus();
  fireEvent.keyDown(keyboardActivityWindow, { key: 'ArrowRight' });
  mouseActivityWindow.focus();
  fireEvent.keyDown(mouseActivityWindow, { key: 'ArrowRight' });
  fireEvent.click(windowActivity);

  await waitFor(() =>
    expect(fullFrameInterval.getAttribute('aria-valuenow')).toBe('60500'),
  );
  expect(deltaTileSize.getAttribute('aria-valuenow')).toBe('80');
  expect(keyboardActivityWindow.getAttribute('aria-valuenow')).toBe('31000');
  expect(mouseActivityWindow.getAttribute('aria-valuenow')).toBe('6000');
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      settings: {
        ai: {
          ...defaultAiSettings,
          activity: {
            ...defaultAiSettings.activity,
            keyboard: {
              ...defaultAiSettings.activity.keyboard,
              windowMs: 31_000,
            },
            mouse: {
              ...defaultAiSettings.activity.mouse,
              windowMs: 6_000,
            },
            windows: {
              enabled: false,
            },
          },
          processing: {
            ...defaultAiSettings.processing,
            maxParallelChunksPerJob: 2,
            maxParallelJobs: 2,
            periodicFullFrameIntervalMs: 60_500,
            visualDeltaTileSizePx: 80,
          },
        },
        debug: defaultSettingsState.settings.debug,
        expert: {
          enabled: true,
        },
        record: defaultSettingsState.settings.record,
        storage: defaultSettingsState.settings.storage,
      },
    }),
  );
});

test('SettingsScreen retains the key draft when the atomic save fails', async () => {
  settingsClient.saveAppSettings.mockRejectedValue(
    new Error('Settings backend unavailable.'),
  );
  render(<SettingsScreen />);
  await openAiSettingsTab();
  await selectRemoteTranscriptionProvider();
  fireEvent.change(screen.getByLabelText('API key'), {
    target: { value: 'synthetic-key' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));
  await screen.findByText('Settings backend unavailable.');
  expect(screen.getByDisplayValue('synthetic-key')).toBeTruthy();
  expect(aiClient.saveAiProviderSecret).not.toHaveBeenCalled();
  expect(aiClient.deleteAiProviderSecret).not.toHaveBeenCalled();
});

test('SettingsScreen retains edits when native credential encryption is unavailable', async () => {
  settingsClient.saveAppSettings.mockRejectedValue(
    new Error('The API key could not be encrypted.'),
  );
  render(<SettingsScreen />);
  await openAiSettingsTab();
  await selectRemoteTranscriptionProvider();
  fireEvent.change(screen.getByLabelText('API key'), {
    target: { value: 'synthetic-key' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));
  await screen.findByText('The API key could not be encrypted.');
  expect(screen.getByDisplayValue('synthetic-key')).toBeTruthy();
  expect(aiClient.saveAiProviderSecret).not.toHaveBeenCalled();
});

test('SettingsScreen blocks dirty leave and lets the user stay', async () => {
  const view = render(<SettingsScreen />);

  await openRecordSettingsTab();
  expect(screen.getByText('1 frame every 5 seconds')).toBeTruthy();
  fireEvent.change(screen.getByLabelText('Video bitrate'), {
    target: { value: '2000' },
  });
  router.state.blocker.status = 'blocked';
  view.rerender(<SettingsScreen />);

  const dialog = await screen.findByRole('dialog', {
    name: 'Do you want to save?',
  });
  const options = router.state.blockerOptions as {
    enableBeforeUnload: boolean;
    shouldBlockFn: (input: {
      current: { pathname: string };
      next: { pathname: string };
    }) => boolean;
  };

  expect(options.enableBeforeUnload).toBe(true);
  expect(
    options.shouldBlockFn({
      current: { pathname: '/settings' },
      next: { pathname: '/library' },
    }),
  ).toBe(true);

  fireEvent.click(within(dialog).getByRole('button', { name: 'Stay' }));

  expect(router.state.blocker.reset).toHaveBeenCalled();
  expect(router.state.blocker.proceed).not.toHaveBeenCalled();
  expect(settingsClient.saveAppSettings).not.toHaveBeenCalled();
});

test('SettingsScreen discards dirty settings before leaving', async () => {
  const view = render(<SettingsScreen />);

  await openRecordSettingsTab();
  expect(screen.getByText('1 frame every 5 seconds')).toBeTruthy();
  fireEvent.change(screen.getByLabelText('Video bitrate'), {
    target: { value: '2000' },
  });
  expect(useSettingsStore.getState().isDirty).toBe(true);
  router.state.blocker.status = 'blocked';
  view.rerender(<SettingsScreen />);

  const dialog = await screen.findByRole('dialog', {
    name: 'Do you want to save?',
  });
  fireEvent.click(
    within(dialog).getByRole('button', { name: 'Discard changes' }),
  );

  expect(useSettingsStore.getState().isDirty).toBe(false);
  expect(router.state.blocker.proceed).toHaveBeenCalled();
  expect(settingsClient.saveAppSettings).not.toHaveBeenCalled();
});

test('SettingsScreen saves dirty settings before leaving', async () => {
  const view = render(<SettingsScreen />);

  await openRecordSettingsTab();
  expect(screen.getByText('1 frame every 5 seconds')).toBeTruthy();
  fireEvent.change(screen.getByLabelText('Video bitrate'), {
    target: { value: '2000' },
  });
  router.state.blocker.status = 'blocked';
  view.rerender(<SettingsScreen />);

  const dialog = await screen.findByRole('dialog', {
    name: 'Do you want to save?',
  });
  fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      settings: {
        ai: defaultAiSettings,
        debug: defaultSettingsState.settings.debug,
        expert: defaultSettingsState.settings.expert,
        record: {
          fps: 0.2,
          videoBitrateKbps: 2_000,
          windowProtection: defaultWindowProtection,
        },
        storage: defaultSettingsState.settings.storage,
      },
    }),
  );
  expect(router.state.blocker.proceed).toHaveBeenCalled();
});

test('Recall generation requires its own consent and tests anonymous profiles with synthetic text', async () => {
  render(<SettingsScreen />);
  await openAiSettingsTab();
  const testButton = screen.getByRole('button', {
    name: 'Test generation provider',
  });
  expect(testButton).toBeDisabled();
  expect(
    screen.getByText(/This consent is separate from Vision and embeddings/),
  ).toBeTruthy();
  fireEvent.click(
    screen.getByRole('checkbox', { name: /^Enable external generation/ }),
  );
  fireEvent.change(screen.getByLabelText('Generation endpoint URL'), {
    target: { value: 'http://localhost:8000/v1' },
  });
  fireEvent.change(screen.getByLabelText('Generation model name'), {
    target: { value: 'synthetic-model' },
  });
  fireEvent.click(screen.getByLabelText('Generation authentication'));
  fireEvent.click(await screen.findByRole('option', { name: 'Anonymous' }));
  expect(screen.queryByLabelText('Generation API key')).toBeNull();
  fireEvent.click(testButton);
  expect(
    await screen.findByText('Synthetic test succeeded (12 ms).'),
  ).toBeTruthy();
  expect(recallClient.testRecallProvider).toHaveBeenCalledWith(
    expect.objectContaining({
      kind: 'generation',
      keyDraft: null,
      settings: expect.objectContaining({
        generation: expect.objectContaining({
          enabled: true,
          authMode: 'none',
          model: 'synthetic-model',
        }),
      }),
    }),
  );
  expect(screen.getByText('Unsaved changes')).toBeTruthy();
  expect(settingsClient.saveAppSettings).not.toHaveBeenCalled();
});

test('SettingsScreen waits for cleanup confirmation before leaving after save', async () => {
  settingsClient.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 2,
    sizeBytes: 3 * 1024 * 1024 * 1024,
  });
  const view = render(<SettingsScreen />);

  await openRecordSettingsTab();
  expect(screen.getByText('1 frame every 5 seconds')).toBeTruthy();
  fireEvent.change(screen.getByLabelText('Video bitrate'), {
    target: { value: '2000' },
  });
  router.state.blocker.status = 'blocked';
  view.rerender(<SettingsScreen />);

  const dialog = await screen.findByRole('dialog', {
    name: 'Do you want to save?',
  });
  fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

  expect(await screen.findByText('Delete old recordings?')).toBeTruthy();
  expect(router.state.blocker.proceed).not.toHaveBeenCalled();
  expect(settingsClient.saveAppSettings).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole('button', { name: /^delete and save$/i }));

  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalled(),
  );
  expect(router.state.blocker.proceed).toHaveBeenCalled();
});

test('Recall provider test failure preserves draft edits and remains retryable', async () => {
  recallClient.testRecallProvider.mockResolvedValueOnce({
    kind: 'generation',
    status: 'failed',
    errorCode: 'recall_authentication_failed',
    elapsedMs: 8,
  });
  render(<SettingsScreen tab="ai" />);
  fireEvent.click(
    await screen.findByRole('checkbox', {
      name: /^Enable external generation/,
    }),
  );
  fireEvent.change(screen.getByLabelText('Generation endpoint URL'), {
    target: { value: 'https://synthetic.example/v1' },
  });
  fireEvent.change(screen.getByLabelText('Generation API key'), {
    target: { value: 'synthetic-test-key' },
  });
  fireEvent.click(
    screen.getByRole('button', { name: 'Test generation provider' }),
  );
  expect(
    await screen.findByText(
      'Synthetic test failed: recall_authentication_failed.',
    ),
  ).toBeTruthy();
  expect(screen.getByLabelText('Generation API key')).toHaveValue(
    'synthetic-test-key',
  );
  expect(
    screen.getByRole('button', { name: 'Test generation provider' }),
  ).not.toBeDisabled();
  fireEvent.click(
    screen.getByRole('button', { name: 'Test generation provider' }),
  );
  expect(
    await screen.findByText('Synthetic test succeeded (12 ms).'),
  ).toBeTruthy();
  expect(settingsClient.saveAppSettings).not.toHaveBeenCalled();
});
