import type {
  AppSettingsState,
  InputActivityStatus,
  RecorderScene,
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
import { useNotificationsStore } from '@/features/notifications/notifications-store';
import { useSettingsStore } from '@/features/settings/settings-store';
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
const settingsClient = vi.hoisted(() => ({
  getAppSettings: vi.fn(),
  getInputActivityStatus: vi.fn(),
  previewAppSettingsStorageCleanup: vi.fn(),
  repairInputActivityPermissions: vi.fn(),
  resetAppSettings: vi.fn(),
  saveAppSettings: vi.fn(),
}));
const recorderClient = vi.hoisted(() => ({
  getRecorderScene: vi.fn(),
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
vi.mock('@/features/settings/settings-client', () => settingsClient);
vi.mock('@/features/recorder/recorder-client', () => recorderClient);
vi.mock('@tanstack/react-router', () => ({
  useBlocker: router.useBlocker,
}));

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
    frameRecognitionFps: 0.2,
    frameRecognitionSyncWithRecording: true,
    frameSimilarityEnabled: true,
    maxFullFrameCheckpointsPerMinute: 2,
    maxParallelChunksPerJob: 1,
    maxParallelJobs: 1,
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
    provider: 'disabled',
  },
} as const;

const defaultSettingsState = {
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
      fps: 0.2,
      videoBitrateKbps: 1_500,
    },
    storage: {
      retentionDays: 7,
      sizeLimitBytes: 50 * 1024 * 1024 * 1024,
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

const aiModels = [
  {
    checksum: '55356645c2b361a969dfd0ef2c5a50d530afd8d5',
    checksumAlgorithm: 'sha1',
    createdAtMs: 0,
    downloadedBytes: 0,
    downloadStatus: 'missing',
    etag: null,
    errorCode: null,
    id: 'small',
    label: 'Whisper small',
    localPath: null,
    provider: 'local-whisper-cpp',
    selfTestStatus: 'not-run',
    sizeBytes: null,
    sourceUrl:
      'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin',
    totalBytes: null,
    updatedAtMs: 0,
    verificationStatus: 'unknown',
  },
] as const;

const availableInputActivityStatus = {
  available: true,
  candidateDeviceCount: 2,
  currentUser: 'felixf',
  message: 'Keyboard and mouse activity capture can read Linux input devices.',
  permissionDeniedDeviceCount: 0,
  platform: 'linux',
  readableDeviceCount: 2,
  reason: 'available',
  repairAvailable: false,
  userInInputGroup: true,
} satisfies InputActivityStatus;

async function openAiSettingsTab() {
  await screen.findByRole('tablist', { name: 'Settings sections' });
  const aiTab = screen.getByRole('tab', { name: 'AI/LLM' });
  fireEvent.mouseDown(aiTab, { button: 0, ctrlKey: false });
  fireEvent.mouseUp(aiTab);
  fireEvent.click(aiTab);
  await waitFor(() => expect(aiTab.getAttribute('aria-selected')).toBe('true'));
  await screen.findByText('Transcription');
}

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
  fireEvent.click(await screen.findByRole('option', { name: option }));
}

async function selectRemoteTranscriptionProvider() {
  await selectOption('Provider', 'OpenAI-compatible');
  await screen.findByLabelText('API key');
}

beforeEach(() => {
  vi.resetAllMocks();
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

afterEach(() => {
  cleanup();
});

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
        },
        storage: {
          retentionDays: 30,
          sizeLimitBytes: 60 * 1024 * 1024 * 1024,
        },
      },
    }),
  );
  expect(await screen.findByText('Settings saved.')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /^reset$/i }));
  fireEvent.click(screen.getByRole('button', { name: /^reset settings$/i }));

  await waitFor(() =>
    expect(
      settingsClient.previewAppSettingsStorageCleanup,
    ).toHaveBeenCalledWith({
      settings: defaultSettingsState.defaults,
    }),
  );
  await waitFor(() =>
    expect(settingsClient.resetAppSettings).toHaveBeenCalled(),
  );
  expect(await screen.findByText('Settings reset.')).toBeTruthy();
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

  expect(screen.queryByText('Delete old recordings?')).toBeNull();
  expect(aiClient.saveAiProviderSecret).not.toHaveBeenCalled();
  expect(settingsClient.saveAppSettings).not.toHaveBeenCalled();
  expect(screen.getByDisplayValue('sk-private')).toBeTruthy();
});

test('SettingsScreen saves remote transcription settings through encrypted key refs', async () => {
  render(<SettingsScreen />);

  await openAiSettingsTab();
  await selectRemoteTranscriptionProvider();
  fireEvent.change(await screen.findByLabelText('Base URL'), {
    target: { value: 'https://localhost:11434/v1' },
  });
  fireEvent.change(screen.getByLabelText('API key'), {
    target: { value: 'sk-private' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(settingsClient.previewAppSettingsStorageCleanup).toHaveBeenCalled(),
  );
  await waitFor(() =>
    expect(aiClient.saveAiProviderSecret).toHaveBeenCalledWith({
      plaintext: 'sk-private',
      role: 'transcription',
    }),
  );
  await waitFor(() =>
    expect(settingsClient.saveAppSettings).toHaveBeenCalledWith({
      settings: {
        ai: {
          ...defaultAiSettings,
          transcription: {
            ...defaultAiSettings.transcription,
            provider: 'openai-compatible',
            remoteApiKeyRef: 'ai.secret.transcription.saved',
            remoteBaseUrl: 'https://localhost:11434/v1',
          },
        },
        debug: defaultSettingsState.settings.debug,
        expert: defaultSettingsState.settings.expert,
        record: defaultSettingsState.settings.record,
        storage: defaultSettingsState.settings.storage,
      },
    }),
  );
  expect(screen.queryByDisplayValue('sk-private')).toBeNull();
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

test('SettingsScreen deletes newly-created AI secrets when the final save fails', async () => {
  settingsClient.saveAppSettings.mockRejectedValue(
    new Error('Settings backend unavailable.'),
  );
  render(<SettingsScreen />);

  await openAiSettingsTab();
  await selectRemoteTranscriptionProvider();
  fireEvent.change(screen.getByLabelText('API key'), {
    target: { value: 'sk-private' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(aiClient.saveAiProviderSecret).toHaveBeenCalledWith({
      plaintext: 'sk-private',
      role: 'transcription',
    }),
  );
  await waitFor(() =>
    expect(aiClient.deleteAiProviderSecret).toHaveBeenCalledWith({
      secretRef: 'ai.secret.transcription.saved',
    }),
  );
  expect(screen.getByDisplayValue('sk-private')).toBeTruthy();
  expect(await screen.findByText('Settings backend unavailable.')).toBeTruthy();
});

test('SettingsScreen surfaces platform-not-implemented secret errors without saving settings', async () => {
  aiClient.saveAiProviderSecret.mockRejectedValue(
    new Error('Encrypted settings are not implemented on this platform yet.'),
  );
  render(<SettingsScreen />);

  await openAiSettingsTab();
  await selectRemoteTranscriptionProvider();
  fireEvent.change(screen.getByLabelText('API key'), {
    target: { value: 'sk-private' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  expect(
    await screen.findByText(
      'Encrypted settings are not implemented on this platform yet.',
    ),
  ).toBeTruthy();
  expect(settingsClient.saveAppSettings).not.toHaveBeenCalled();
  expect(screen.getByDisplayValue('sk-private')).toBeTruthy();
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
        },
        storage: defaultSettingsState.settings.storage,
      },
    }),
  );
  expect(router.state.blocker.proceed).toHaveBeenCalled();
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
