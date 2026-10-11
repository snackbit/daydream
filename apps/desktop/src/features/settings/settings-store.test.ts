import type {
  AppSettingsState,
  RecorderScene,
  RecordWindowProtectionSettings,
} from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import { recallSettingsFixture } from '../recall/recall-test-fixtures';
import { createSettingsStore, useSettingsStore } from './settings-store';

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
  recall: recallSettingsFixture(),
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
    windows: {
      enabled: true,
    },
  },
  ocr: {
    enabled: true,
    engine: 'tesseract',
  },
  processing: {
    autoStartAfterRecording: true,
    frameRecognitionFps: 0.2,
    frameRecognitionSyncWithRecording: true,
    frameSimilarityEnabled: true,
    maxFullFrameCheckpointsPerMinute: 2,
    maxParallelChunksPerJob: 1,
    maxParallelJobs: 1,
    maxParallelOcrJobs: 1,
    maxParallelVlmJobs: 1,
    periodicFullFrameIntervalMs: 60_000,
    visualDeltaTileSizePx: 64,
  },
  transcription: {
    language: 'auto',
    localBackend: 'cpu',
    localModelId: 'small',
    maxInitialTimestampSeconds: 3,
    mergeGapMs: 700,
    minSpeechMs: 250,
    provider: 'local-whisper-cpp',
    remoteApiKeyRef: null,
    remoteBaseUrl: null,
    remoteModel: null,
    silenceGateEnabled: true,
    silenceThresholdDb: -45,
    wordTimestampsEnabled: true,
  },
  vision: {
    apiKeyRef: null,
    baseUrl: null,
    model: null,
    observationsEnabled: false,
    provider: 'disabled',
  },
} as const;

const defaultWindowProtection: RecordWindowProtectionSettings = {
  defaultAction: 'pause',
  dwellMs: 2_000,
  enabled: false,
  mergeGapMs: 5_000,
  rules: [],
};

const settingsState = {
  defaults: {
    ai: defaultAiSettings,
    debug: {
      enabled: false,
    },
    expert: {
      enabled: false,
    },
    record: {
      fps: 0.2,
      videoBitrateKbps: 1_500,
      windowProtection: defaultWindowProtection,
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
        frameRecognitionFpsMax: 5,
        frameRecognitionFpsMin: 1 / 60,
        maxFullFrameCheckpointsPerMinuteMax: 60,
        maxFullFrameCheckpointsPerMinuteMin: 1,
        maxParallelChunksPerJobMax: 8,
        maxParallelChunksPerJobMin: 1,
        maxParallelJobsMax: 4,
        maxParallelJobsMin: 1,
        maxParallelOcrJobsMax: 8,
        maxParallelOcrJobsMin: 1,
        maxParallelVlmJobsMax: 8,
        maxParallelVlmJobsMin: 1,
        periodicFullFrameIntervalMsMax: 600_000,
        periodicFullFrameIntervalMsMin: 5_000,
        visualDeltaTileSizePxMax: 256,
        visualDeltaTileSizePxMin: 32,
      },
      transcription: {
        maxInitialTimestampSecondsMax: 10,
        maxInitialTimestampSecondsMin: 1,
        mergeGapMsMax: 5_000,
        mergeGapMsMin: 0,
        minSpeechMsMax: 2_000,
        minSpeechMsMin: 50,
        silenceThresholdDbMax: -20,
        silenceThresholdDbMin: -80,
      },
    },
    record: {
      fpsMax: 5,
      fpsMin: 1 / 60,
      videoBitrateKbpsMax: 10_000,
      videoBitrateKbpsMin: 250,
      windowProtectionDwellMsMax: 30_000,
      windowProtectionDwellMsMin: 250,
      windowProtectionMergeGapMsMax: 120_000,
      windowProtectionMergeGapMsMin: 0,
      windowProtectionRuleCountMax: 100,
      windowProtectionTextMaxChars: 160,
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
    debug: {
      enabled: false,
    },
    expert: {
      enabled: false,
    },
    record: {
      fps: 0.5,
      videoBitrateKbps: 2_500,
      windowProtection: defaultWindowProtection,
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

test('preference invalidation prevents a late settings read restoring removed scenes', async () => {
  let finish: ((settings: AppSettingsState) => void) | undefined;
  settingsClient.getAppSettings
    .mockReturnValueOnce(
      new Promise<AppSettingsState>((resolve) => {
        finish = resolve;
      }),
    )
    .mockResolvedValueOnce(settingsState);
  recorderClient.getRecorderScene
    .mockResolvedValueOnce(scene)
    .mockResolvedValueOnce(null);
  const store = createSettingsStore();
  const old = store.getState().load();
  store.getState().invalidatePreferences();
  await store.getState().load();
  finish?.(settingsState);
  await old;
  expect(store.getState().savedScene).toBeNull();
  expect(store.getState().loadStatus).toBe('ready');
});

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

test('saving preserves newer edits and keeps a single settings request in flight', async () => {
  await useSettingsStore.getState().load();
  const submitted = {
    ...settingsState.settings,
    storage: { ...settingsState.settings.storage, retentionDays: 30 },
  };
  const newer = {
    ...submitted,
    storage: { ...submitted.storage, retentionDays: 45 },
  };
  let finishSave: (result: AppSettingsState) => void = () => {
    throw new Error('save not started');
  };
  settingsClient.saveAppSettings.mockImplementation(
    () =>
      new Promise<AppSettingsState>((resolve) => {
        finishSave = resolve;
      }),
  );
  useSettingsStore.getState().setDraft(submitted);
  const saving = useSettingsStore.getState().save();
  useSettingsStore.getState().setDraft(newer);
  expect(useSettingsStore.getState().mutationStatus).toBe('loading');
  await expect(useSettingsStore.getState().save()).rejects.toThrow(/already/i);
  expect(settingsClient.saveAppSettings).toHaveBeenCalledTimes(1);
  finishSave({ ...settingsState, settings: submitted });
  await saving;
  expect(useSettingsStore.getState()).toMatchObject({
    draft: newer,
    isDirty: true,
    settingsState: { settings: submitted },
  });
});

test('autosave serializes newer edits and never requests storage cleanup', async () => {
  const store = createSettingsStore();
  await store.getState().load();
  let finish!: (result: AppSettingsState) => void;
  settingsClient.saveAppSettings
    .mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    )
    .mockImplementation(async ({ settings }) => ({
      ...settingsState,
      settings,
    }));
  const first = store
    .getState()
    .commit((current) => ({ ...current, debug: { enabled: true } }));
  const second = store
    .getState()
    .commit((current) => ({ ...current, expert: { enabled: true } }));
  expect(settingsClient.saveAppSettings).toHaveBeenCalledTimes(1);
  finish({
    ...settingsState,
    settings: settingsClient.saveAppSettings.mock.calls[0]?.[0].settings,
  });
  await Promise.all([first, second]);
  expect(settingsClient.saveAppSettings).toHaveBeenCalledTimes(2);
  expect(settingsClient.saveAppSettings.mock.calls[1]?.[0]).toMatchObject({
    applyStorageCleanup: false,
    settings: { debug: { enabled: true }, expert: { enabled: true } },
  });
  expect(store.getState().isDirty).toBe(false);
});

test('failed autosave retains credentials for explicit retry', async () => {
  const store = createSettingsStore();
  await store.getState().load();
  settingsClient.saveAppSettings.mockRejectedValueOnce(
    new Error('Synthetic save failure'),
  );
  expect(
    await store
      .getState()
      .commit((current) => ({ ...current, debug: { enabled: true } }), {
        transcriptionKey: 'synthetic-key',
      }),
  ).toBe(false);
  expect(store.getState().isDirty).toBe(true);
  settingsClient.saveAppSettings.mockImplementation(async ({ settings }) => ({
    ...settingsState,
    settings,
  }));
  expect(await store.getState().retryAutosave()).toBe(true);
  expect(settingsClient.saveAppSettings.mock.calls[1]?.[0].credentials).toEqual(
    { transcriptionKey: 'synthetic-key' },
  );
  expect(store.getState().isDirty).toBe(false);
});
test('preference invalidation abandons queued edits and credentials after an in-flight save', async () => {
  const store = createSettingsStore();
  await store.getState().load();
  let finish: ((value: AppSettingsState) => void) | undefined;
  settingsClient.saveAppSettings.mockImplementationOnce(
    () =>
      new Promise<AppSettingsState>((resolve) => {
        finish = resolve;
      }),
  );
  const first = store
    .getState()
    .commit((current) => ({ ...current, debug: { enabled: true } }));
  const queued = store
    .getState()
    .commit((current) => ({ ...current, expert: { enabled: true } }), {
      transcriptionKey: 'synthetic-key',
    });
  store.getState().invalidatePreferences();
  finish?.(settingsState);
  await Promise.all([first, queued]);
  expect(settingsClient.saveAppSettings).toHaveBeenCalledOnce();
  expect(store.getState().draft).toBeNull();
  expect(await store.getState().retryAutosave()).toBe(true);
  expect(settingsClient.saveAppSettings).toHaveBeenCalledOnce();
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

test('recall profile and both credentials use the same save while newer draft edits survive', async () => {
  await useSettingsStore.getState().load();
  const submitted = {
    ...settingsState.settings,
    ai: {
      ...settingsState.settings.ai,
      recall: {
        ...recallSettingsFixture(),
        generation: {
          ...recallSettingsFixture().generation,
          enabled: true,
          model: 'synthetic-model',
        },
      },
    },
  };
  const newer = {
    ...submitted,
    ai: {
      ...submitted.ai,
      recall: { ...submitted.ai.recall, autoGenerate: false },
    },
  };
  let settle: (result: AppSettingsState) => void = () => {
    throw new Error('save not started');
  };
  settingsClient.saveAppSettings.mockImplementation(
    () =>
      new Promise<AppSettingsState>((resolve) => {
        settle = resolve;
      }),
  );
  useSettingsStore.getState().setDraft(submitted);
  const credentials = {
    recallEmbeddingKey: 'synthetic-embedding-key',
    recallGenerationKey: 'synthetic-generation-key',
  };
  const saving = useSettingsStore.getState().save(submitted, credentials);
  useSettingsStore.getState().setDraft(newer);
  settle({ ...settingsState, settings: submitted });
  await saving;
  expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
    settings: submitted,
    credentials,
  });
  expect(useSettingsStore.getState()).toMatchObject({
    draft: newer,
    isDirty: true,
    settingsState: { settings: submitted },
  });
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
