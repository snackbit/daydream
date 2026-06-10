import type { AppSettingsState, RecorderScene } from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import { useSettingsStore } from './settings-store';

const settingsClient = vi.hoisted(() => ({
  getAppSettings: vi.fn(),
  previewAppSettingsStorageCleanup: vi.fn(),
  resetAppSettings: vi.fn(),
  saveAppSettings: vi.fn(),
}));
const recorderClient = vi.hoisted(() => ({
  getRecorderScene: vi.fn(),
}));

vi.mock('./settings-client', () => settingsClient);
vi.mock('@/features/recorder/recorder-client', () => recorderClient);

const defaultAiSettings = {
  activity: {
    frameDeltas: {
      enabled: true,
    },
    keyboard: {
      enabled: false,
      windowMs: 30_000,
    },
    mouse: {
      enabled: false,
      windowMs: 5_000,
    },
  },
  ocr: {
    enabled: true,
    engine: 'tesseract',
  },
  processing: {
    autoStartAfterRecording: true,
    frameSimilarityEnabled: true,
    maxParallelChunksPerJob: 1,
    maxParallelJobs: 1,
    periodicFullFrameIntervalMs: 300_000,
  },
  transcription: {
    localBackend: 'cpu',
    localModelId: 'small',
    provider: 'local-whisper-cpp',
    remoteApiKeyRef: null,
    remoteBaseUrl: null,
    remoteModel: null,
    wordTimestampsEnabled: true,
  },
  vision: {
    apiKeyRef: null,
    baseUrl: null,
    model: null,
    provider: 'disabled',
  },
} as const;

const settingsState = {
  defaults: {
    ai: defaultAiSettings,
    record: {
      fps: 0.2,
      videoBitrateKbps: 1_500,
    },
    storage: {
      retentionDays: 7,
      sizeLimitBytes: 50 * 1024 * 1024 * 1024,
    },
  },
  limits: {
    ai: {
      activity: {
        keyboardWindowMsMax: 300_000,
        keyboardWindowMsMin: 1_000,
        mouseWindowMsMax: 300_000,
        mouseWindowMsMin: 1_000,
      },
      processing: {
        maxParallelChunksPerJobMax: 8,
        maxParallelChunksPerJobMin: 1,
        maxParallelJobsMax: 4,
        maxParallelJobsMin: 1,
        periodicFullFrameIntervalMsMax: 3_600_000,
        periodicFullFrameIntervalMsMin: 30_000,
      },
    },
    record: {
      fpsMax: 5,
      fpsMin: 1 / 60,
      videoBitrateKbpsMax: 10_000,
      videoBitrateKbpsMin: 250,
    },
    storage: {
      retentionDaysMax: 365,
      retentionDaysMin: 1,
      sizeLimitBytesMax: null,
      sizeLimitBytesMin: 1024 * 1024 * 1024,
    },
  },
  settings: {
    ai: defaultAiSettings,
    record: {
      fps: 0.5,
      videoBitrateKbps: 2_500,
    },
    storage: {
      retentionDays: 14,
      sizeLimitBytes: 75 * 1024 * 1024 * 1024,
    },
  },
} satisfies AppSettingsState;

const scene = {
  canvasHeight: 1440,
  canvasWidth: 2560,
  desktopAudioSourceId: 'desktop',
  id: 'scene-1',
  layoutMode: 'horizontal',
  microphoneAudioSourceId: 'mic',
  name: 'Scene',
  sources: [],
} satisfies RecorderScene;

beforeEach(() => {
  vi.resetAllMocks();
  settingsClient.getAppSettings.mockResolvedValue(settingsState);
  settingsClient.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 0,
    sizeBytes: 0,
  });
  settingsClient.saveAppSettings.mockResolvedValue(settingsState);
  settingsClient.resetAppSettings.mockResolvedValue({
    ...settingsState,
    settings: settingsState.defaults,
  });
  recorderClient.getRecorderScene.mockResolvedValue(scene);
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
});

test('settings store loads settings with the saved recorder scene', async () => {
  await useSettingsStore.getState().load();

  expect(settingsClient.getAppSettings).toHaveBeenCalledWith();
  expect(recorderClient.getRecorderScene).toHaveBeenCalledWith();
  expect(useSettingsStore.getState()).toMatchObject({
    draft: settingsState.settings,
    isDirty: false,
    loadStatus: 'ready',
    savedScene: scene,
    settingsState,
  });
});

test('settings store saves dirty draft and resets to defaults', async () => {
  await useSettingsStore.getState().load();
  useSettingsStore.getState().setDraft({
    ...settingsState.settings,
    record: {
      ...settingsState.settings.record,
      videoBitrateKbps: 3_000,
    },
  });

  expect(useSettingsStore.getState().isDirty).toBe(true);

  await useSettingsStore.getState().save();

  expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
    settings: {
      ...settingsState.settings,
      record: {
        ...settingsState.settings.record,
        videoBitrateKbps: 3_000,
      },
    },
  });
  expect(useSettingsStore.getState().successMessage).toBe('Settings saved.');

  await useSettingsStore.getState().reset();

  expect(settingsClient.resetAppSettings).toHaveBeenCalledWith();
  expect(useSettingsStore.getState().draft).toEqual(settingsState.defaults);
});

test('settings store treats ai edits as dirty settings', async () => {
  await useSettingsStore.getState().load();
  useSettingsStore.getState().setDraft({
    ...settingsState.settings,
    ai: {
      ...settingsState.settings.ai,
      processing: {
        ...settingsState.settings.ai.processing,
        maxParallelJobs: 2,
      },
    },
  });

  expect(useSettingsStore.getState().isDirty).toBe(true);
});

test('settings store previews storage cleanup and summarizes cleanup results', async () => {
  settingsClient.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 2,
    sizeBytes: 3,
  });
  settingsClient.saveAppSettings.mockResolvedValue({
    ...settingsState,
    storageCleanup: {
      deletedRecordingCount: 2,
      deletedSizeBytes: 3,
      failedRecordingCount: 1,
    },
  });

  await useSettingsStore.getState().load();
  const preview = await useSettingsStore
    .getState()
    .previewStorageCleanup(settingsState.settings);
  await useSettingsStore.getState().save();

  expect(preview).toEqual({
    recordingCount: 2,
    sizeBytes: 3,
  });
  expect(settingsClient.previewAppSettingsStorageCleanup).toHaveBeenCalledWith({
    settings: settingsState.settings,
  });
  expect(useSettingsStore.getState().successMessage).toBe(
    'Settings saved. Deleted 2 sessions. 1 cleanup deletion failed.',
  );
});
