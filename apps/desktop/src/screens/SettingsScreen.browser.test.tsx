import type { AppSettingsState, RecorderScene } from '@daydream/shared';
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
  deleteAiModel: vi.fn(),
  deleteAiProviderSecret: vi.fn(),
  downloadAiModel: vi.fn(),
  getAiProviderStatus: vi.fn(),
  listAiModels: vi.fn(),
  saveAiProviderSecret: vi.fn(),
  selfTestAiModel: vi.fn(),
}));
const settingsClient = vi.hoisted(() => ({
  getAppSettings: vi.fn(),
  previewAppSettingsStorageCleanup: vi.fn(),
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
    frameSimilarityEnabled: true,
    maxParallelChunksPerJob: 1,
    maxParallelJobs: 1,
    periodicFullFrameIntervalMs: 300_000,
  },
  transcription: {
    localModelId: 'small',
    provider: 'local-whisper-cpp',
    remoteApiKeyRef: null,
    remoteBaseUrl: null,
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
    createdAtMs: 0,
    downloadStatus: 'missing',
    etag: null,
    id: 'small',
    label: 'Whisper small',
    localPath: null,
    provider: 'local-whisper-cpp',
    selfTestStatus: 'not-run',
    sha256: null,
    sizeBytes: null,
    updatedAtMs: 0,
  },
] as const;

async function openAiSettingsTab() {
  await screen.findByRole('tablist', { name: 'Settings sections' });
  const recordTab = screen.getByRole('tab', { name: 'Record' });
  const aiTab = screen.getByRole('tab', { name: 'AI/LLM' });
  recordTab.focus();
  fireEvent.keyDown(recordTab, { key: 'ArrowRight' });
  fireEvent.keyDown(screen.getByRole('tab', { name: 'Storage/Library' }), {
    key: 'ArrowRight',
  });
  await waitFor(() => expect(aiTab.getAttribute('aria-selected')).toBe('true'));
  await screen.findByText('Transcription');
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
  settingsClient.resetAppSettings.mockResolvedValue(defaultSettingsState);
  recorderClient.getRecorderScene.mockResolvedValue(scene);
  aiClient.listAiModels.mockResolvedValue({ models: aiModels });
  aiClient.getAiProviderStatus.mockResolvedValue({
    localTranscription: 'missing',
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

  expect(await screen.findByText('1 frame every 5 seconds')).toBeTruthy();
  expect(screen.getByText('2560 x 1440')).toBeTruthy();
  expect(screen.getByText('712.39 MiB')).toBeTruthy();

  fireEvent.change(screen.getByLabelText('Seconds per frame'), {
    target: { value: '10' },
  });
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
        record: {
          fps: 0.1,
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
        record: {
          fps: 0.1,
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

  expect(await screen.findByText('1 frame every 5 seconds')).toBeTruthy();
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
        record: defaultSettingsState.settings.record,
        storage: defaultSettingsState.settings.storage,
      },
    }),
  );
  expect(screen.queryByDisplayValue('sk-private')).toBeNull();
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

  expect(await screen.findByText('1 frame every 5 seconds')).toBeTruthy();
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

  expect(await screen.findByText('1 frame every 5 seconds')).toBeTruthy();
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

  expect(await screen.findByText('1 frame every 5 seconds')).toBeTruthy();
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

  expect(await screen.findByText('1 frame every 5 seconds')).toBeTruthy();
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
