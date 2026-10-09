import type { AppSettingsState } from '@daydream/shared';
import {
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import type * as RecorderClientModule from '@/features/recorder/recorder-client';
import { useSettingsStore } from '@/features/settings/settings-store';
import { SettingsScreen } from '@/screens/SettingsScreen';
import {
  defaultSettingsState,
  scene,
  aiModels,
  availableInputActivityStatus,
} from './settings-browser-fixtures';

const settings = vi.hoisted(() => ({
  getAppSettings: vi.fn(),
  saveAppSettings: vi.fn(),
  resetAppSettings: vi.fn(),
  previewAppSettingsStorageCleanup: vi.fn(),
  getInputActivityStatus: vi.fn(),
  repairInputActivityPermissions: vi.fn(),
}));
const recorder = vi.hoisted(() => ({
  getRecorderScene: vi.fn(),
  getWindowProtectionStatus: vi.fn(),
  listProtectableWindows: vi.fn(),
}));
const ai = vi.hoisted(() => ({
  listAiModels: vi.fn(),
  getAiProviderStatus: vi.fn(),
  listenToAiModelsChanged: vi.fn(),
  downloadAiModel: vi.fn(),
  cancelAiModelDownload: vi.fn(),
  deleteAiModel: vi.fn(),
  deleteAiProviderSecret: vi.fn(),
}));
const recall = vi.hoisted(() => ({
  getRecallStatus: vi.fn(),
  listenToRecallStateChanged: vi.fn(),
  testRecallProvider: vi.fn(),
}));
const maintenance = vi.hoisted(() => ({
  getMaintenanceStatus: vi.fn(),
  previewMaintenance: vi.fn(),
  executeMaintenance: vi.fn(),
  acknowledgeMaintenanceThemeReset: vi.fn(),
}));
vi.mock('@/features/settings/maintenance-client', () => maintenance);
vi.mock('@/features/settings/settings-client', () => settings);
vi.mock('@/features/recorder/recorder-client', async (importOriginal) => ({
  ...(await importOriginal<typeof RecorderClientModule>()),
  ...recorder,
}));
vi.mock('@/features/ai/ai-client', () => ai);
vi.mock('@/features/recall/recall-client', () => recall);
vi.mock('@tanstack/react-router', () => ({
  useBlocker: () => ({ status: 'idle' }),
}));

let saved: AppSettingsState;
beforeEach(() => {
  vi.resetAllMocks();
  maintenance.getMaintenanceStatus.mockResolvedValue(null);
  maintenance.previewMaintenance.mockImplementation(async ({ action }) => ({
    action,
    token: 'synthetic-impact',
    recordingCount: 2,
    libraryRowCount: 5,
    modelCount: 1,
    fileCount: 0,
    sizeBytes: 0,
  }));
  saved = structuredClone(defaultSettingsState);
  useSettingsStore.setState(useSettingsStore.getInitialState(), true);
  settings.getAppSettings.mockImplementation(async () => saved);
  settings.saveAppSettings.mockImplementation(async ({ settings: next }) => {
    saved = { ...saved, settings: next };
    return saved;
  });
  settings.resetAppSettings.mockImplementation(async () => {
    saved = { ...saved, settings: saved.defaults };
    return saved;
  });
  settings.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 0,
    sizeBytes: 0,
  });
  settings.getInputActivityStatus.mockResolvedValue(
    availableInputActivityStatus,
  );
  recorder.getRecorderScene.mockResolvedValue(scene);
  recorder.getWindowProtectionStatus.mockResolvedValue({
    message: 'Synthetic window provider available.',
    platform: 'linux',
    provider: 'linux-x11',
    status: 'available',
  });
  ai.listAiModels.mockResolvedValue({ models: aiModels });
  ai.getAiProviderStatus.mockResolvedValue(null);
  ai.listenToAiModelsChanged.mockResolvedValue(() => undefined);
  recall.listenToRecallStateChanged.mockResolvedValue(() => undefined);
});
afterEach(cleanup);

async function selectSetting(name: string, value: string) {
  await page.getByRole('combobox', { name, exact: true }).click();
  await page.getByRole('option', { name: value, exact: true }).click();
}

test('SET-03 expert and debug preferences survive saving and reopening settings', async () => {
  const view = render(<SettingsScreen />);
  await screen.findByRole('checkbox', { name: /Enable expert mode/ });
  await page.getByRole('checkbox', { name: /Enable expert mode/ }).click();
  await page.getByRole('checkbox', { name: /Enable debug tools/ }).click();
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await screen.findByText('Settings saved.');
  view.unmount();
  useSettingsStore.setState(useSettingsStore.getInitialState(), true);
  render(<SettingsScreen />);
  await waitFor(() =>
    expect(
      screen
        .getByRole('checkbox', { name: /Enable expert mode/ })
        .getAttribute('aria-checked'),
    ).toBe('true'),
  );
  expect(
    screen
      .getByRole('checkbox', { name: /Enable debug tools/ })
      .getAttribute('aria-checked'),
  ).toBe('true');
});

test('SET-04 cancelling reset keeps draft preferences and does not request cleanup or reset', async () => {
  render(<SettingsScreen />);
  await screen.findByRole('checkbox', { name: /Enable debug tools/ });
  await page.getByRole('checkbox', { name: /Enable debug tools/ }).click();
  await page
    .getByRole('button', { name: 'Reset preferences', exact: true })
    .click();
  await screen.findByRole('alertdialog', { name: 'Reset preferences' });
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  expect(screen.queryByRole('alertdialog')).toBeNull();
  expect(
    screen
      .getByRole('checkbox', { name: /Enable debug tools/ })
      .getAttribute('aria-checked'),
  ).toBe('true');
  expect(settings.resetAppSettings).not.toHaveBeenCalled();
  expect(settings.previewAppSettingsStorageCleanup).not.toHaveBeenCalled();
  expect(maintenance.executeMaintenance).not.toHaveBeenCalled();
});

test('SET-05 cancelling storage cleanup keeps the draft until an explicit confirmed save', async () => {
  settings.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 1,
    sizeBytes: 100,
  });
  render(<SettingsScreen tab="storage" />);
  await screen.findByRole('spinbutton', { name: 'Recording retention' });
  await page.getByRole('spinbutton', { name: 'Recording retention' }).fill('1');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await screen.findByRole('dialog', { name: 'Delete old recordings?' });
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  expect(settings.saveAppSettings).not.toHaveBeenCalled();
  expect(
    screen.getByRole('spinbutton', { name: 'Recording retention' }),
  ).toHaveProperty('value', '1');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await screen.findByRole('dialog', { name: 'Delete old recordings?' });
  await page
    .getByRole('button', { name: 'Delete and save', exact: true })
    .click();
  await screen.findByText('Settings saved.');
  expect(settings.saveAppSettings).toHaveBeenCalledOnce();
});

test('SET-06 protected rules block saving without a matcher then persist valid rules and their removal', async () => {
  const view = render(<SettingsScreen tab="record" />);
  await screen.findByRole('button', { name: 'Add rule' });
  await page
    .getByRole('checkbox', { name: 'Enable protected windows' })
    .click();
  await page.getByRole('button', { name: 'Add rule', exact: true }).click();
  await page
    .getByRole('textbox', { name: 'Label', exact: true })
    .fill('Synthetic private app');
  expect(screen.getByRole('button', { name: 'Save' })).toHaveProperty(
    'disabled',
    true,
  );
  expect(
    screen.getByText(
      'Enabled protected window rules need an app, process, or title matcher.',
    ),
  ).toBeTruthy();
  await page
    .getByRole('textbox', { name: 'App identity', exact: true })
    .fill('org.example.Synthetic');
  await page.getByRole('spinbutton', { name: 'Focus dwell' }).fill('3000');
  await page.getByRole('spinbutton', { name: 'Merge gap' }).fill('7000');
  await selectSetting('Action', 'Pause');
  await selectSetting('Title matcher', 'Regex');
  await page.getByRole('textbox', { name: 'Title value' }).fill('[');
  expect(screen.getByRole('button', { name: 'Save' })).toHaveProperty(
    'disabled',
    true,
  );
  expect(
    screen.getByText('Protected window title regex is invalid.'),
  ).toBeTruthy();
  await page.getByRole('textbox', { name: 'Title value' }).fill('^Synthetic');
  await page.getByRole('radio', { name: /Record then redact/ }).click();
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await screen.findByText('Settings saved.');
  view.unmount();
  useSettingsStore.setState(useSettingsStore.getInitialState(), true);
  render(<SettingsScreen tab="record" />);
  await screen.findByDisplayValue('org.example.Synthetic');
  expect(
    screen.getByRole('spinbutton', { name: 'Focus dwell' }),
  ).toHaveProperty('value', '3000');
  expect(screen.getByRole('spinbutton', { name: 'Merge gap' })).toHaveProperty(
    'value',
    '7000',
  );
  expect(screen.getByRole('textbox', { name: 'Title value' })).toHaveProperty(
    'value',
    '^Synthetic',
  );
  expect(
    screen.getByRole('combobox', { name: 'Action' }).textContent,
  ).toContain('Pause');
  expect(
    screen
      .getByRole('checkbox', { name: 'Enable protected windows' })
      .getAttribute('aria-checked'),
  ).toBe('true');
  expect(
    screen
      .getByRole('radio', { name: /Record then redact/ })
      .getAttribute('aria-checked'),
  ).toBe('true');
  await page
    .getByRole('button', { name: 'Delete Synthetic private app' })
    .click();
  expect(screen.queryByRole('textbox', { name: 'App identity' })).toBeNull();
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await screen.findByText('Settings saved.');
  expect(saved.settings.record.windowProtection.rules).toEqual([]);
});

test('SET-07 an unavailable window picker recovers through refresh and adds the selected app', async () => {
  recorder.listProtectableWindows
    .mockResolvedValueOnce({
      provider: null,
      status: 'provider-unavailable',
      windows: [],
    })
    .mockResolvedValueOnce({
      provider: 'linux-x11',
      status: 'available',
      windows: [
        {
          appIdentity: 'org.example.Synthetic',
          id: 'synthetic-window',
          platform: 'linux',
          processId: 42,
          processName: 'synthetic-app',
          title: 'Synthetic workspace',
        },
      ],
    });
  render(<SettingsScreen tab="record" />);
  await screen.findByRole('button', { name: 'Pick window' });
  await page.getByRole('button', { name: 'Pick window', exact: true }).click();
  const pickerDialog = await screen.findByRole('dialog', {
    name: 'Pick a window',
  });
  expect(
    (await within(pickerDialog).findByRole('alert')).textContent,
  ).toContain('unavailable');
  await page
    .getByRole('button', { name: 'Refresh windows', exact: true })
    .click();
  await screen.findByRole('button', {
    name: /Synthetic workspace.*synthetic-app.*org\.example\.Synthetic.*PID 42/,
  });
  await page
    .getByRole('button', {
      name: /Synthetic workspace.*synthetic-app.*org\.example\.Synthetic.*PID 42/,
    })
    .click();
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(screen.getByRole('textbox', { name: 'App identity' })).toHaveProperty(
    'value',
    'org.example.Synthetic',
  );
  expect(screen.getByRole('textbox', { name: 'Process' })).toHaveProperty(
    'value',
    'synthetic-app',
  );
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await screen.findByText('Settings saved.');
  expect(saved.settings.record.windowProtection.rules).toHaveLength(1);
});

test('SET-08 processing presets save manual timing and resource use then recover automatic timing', async () => {
  recall.getRecallStatus.mockResolvedValue({
    privacyRevision: 0,
    indexedEvidenceCount: 0,
    pendingEvidenceCount: 0,
    embeddingReady: false,
    generationReady: false,
    embeddingUnavailableReason: 'Not downloaded',
    generationUnavailableReason: 'Disabled',
    jobs: [],
  });
  render(<SettingsScreen tab="ai" />);
  await screen.findByRole('combobox', { name: 'Processing timing' });
  await selectSetting('Processing timing', 'Manual');
  await selectSetting('Processing resource use', 'Fast');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await screen.findByText('Settings saved.');
  expect(saved.settings.ai.processing.autoStartAfterRecording).toBe(false);
  expect(saved.settings.ai.processing.timing).toBe('manual');
  expect(saved.settings.ai.processing.resourceMode).toBe('fast');
  await selectSetting('Processing timing', 'After stop');
  await selectSetting('Processing resource use', 'Background');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await waitFor(() =>
    expect(settings.saveAppSettings).toHaveBeenCalledTimes(2),
  );
  expect(saved.settings.ai.processing.autoStartAfterRecording).toBe(true);
  expect(saved.settings.ai.processing.timing).toBe('after-stop');
  expect(saved.settings.ai.processing.resourceMode).toBe('background');
  await selectSetting('Processing timing', 'During recording');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await waitFor(() =>
    expect(settings.saveAppSettings).toHaveBeenCalledTimes(3),
  );
  expect(saved.settings.ai.processing.timing).toBe('during-recording');
});

test('SET-09 remote embeddings require independent consent and save their profile without enabling generation', async () => {
  recall.getRecallStatus.mockResolvedValue({
    privacyRevision: 0,
    indexedEvidenceCount: 0,
    pendingEvidenceCount: 0,
    embeddingReady: false,
    generationReady: false,
    embeddingUnavailableReason: 'Not downloaded',
    generationUnavailableReason: 'Disabled',
    jobs: [],
  });
  recall.testRecallProvider.mockResolvedValue({
    kind: 'embedding',
    status: 'succeeded',
    errorCode: null,
    elapsedMs: 5,
  });
  render(<SettingsScreen tab="ai" />);
  await screen.findByRole('combobox', { name: 'Embedding provider' });
  await selectSetting('Embedding provider', 'OpenAI-compatible API');
  expect(
    screen.getByRole('button', { name: 'Test embedding provider' }),
  ).toHaveProperty('disabled', true);
  await page
    .getByRole('checkbox', { name: /^Allow remote embeddings/ })
    .click();
  await page
    .getByRole('textbox', { name: 'Embedding endpoint URL' })
    .fill('http://localhost:8000/v1');
  await page
    .getByRole('textbox', { name: 'Embedding model name' })
    .fill('synthetic-embedding');
  await selectSetting('Embedding authentication', 'Anonymous');
  await page
    .getByRole('spinbutton', { name: 'Embedding dimensions' })
    .fill('384');
  await selectSetting('Embedding convention', 'E5 query/passage prefixes');
  await page.getByRole('button', { name: 'Test embedding provider' }).click();
  await screen.findByText('Synthetic test succeeded (5 ms).');
  expect(recall.testRecallProvider).toHaveBeenCalledWith(
    expect.objectContaining({ kind: 'embedding', keyDraft: null }),
  );
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await screen.findByText('Settings saved.');
  expect(saved.settings.ai.recall.embedding).toMatchObject({
    provider: 'openai-compatible',
    enabled: true,
    authMode: 'none',
    model: 'synthetic-embedding',
    dimensions: 384,
    convention: 'e5',
  });
  expect(saved.settings.ai.recall.generation.enabled).toBe(false);
});

test('SET-10 transcription model, language and available backend save while unavailable CUDA cannot be selected', async () => {
  ai.listAiModels.mockResolvedValue({
    models: [
      ...aiModels,
      { ...aiModels[0], id: 'tiny', label: 'Whisper tiny' },
    ],
  });
  ai.getAiProviderStatus.mockResolvedValue({
    localTranscription: {
      availableLanguages: [{ id: 'de', label: 'German' }],
      availableBackends: [
        { id: 'cpu', available: true, reason: null },
        { id: 'vulkan', available: true, reason: null },
        { id: 'cuda', available: false, reason: 'Not compiled.' },
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
  render(<SettingsScreen tab="ai" />);
  await screen.findByRole('combobox', { name: 'Local model' });
  await selectSetting('Local model', 'Whisper tiny');
  await page.getByRole('combobox', { name: 'Local backend' }).click();
  expect(
    await screen.findByRole('option', { name: 'CUDA unavailable' }),
  ).toHaveProperty('ariaDisabled', 'true');
  await page.getByRole('option', { name: 'Vulkan', exact: true }).click();
  await selectSetting('Language', 'German');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await screen.findByText('Settings saved.');
  expect(saved.settings.ai.transcription).toMatchObject({
    localModelId: 'tiny',
    localBackend: 'vulkan',
    language: 'de',
  });
});

test('SET-11 local Recall model and backend save and readiness can recover after a failed refresh', async () => {
  const status = {
    privacyRevision: 0,
    indexedEvidenceCount: 3,
    pendingEvidenceCount: 0,
    embeddingReady: true,
    generationReady: false,
    embeddingUnavailableReason: null,
    generationUnavailableReason: 'Disabled',
    jobs: [],
  };
  recall.getRecallStatus
    .mockResolvedValueOnce(status)
    .mockRejectedValueOnce(new Error('Readiness worker is busy.'))
    .mockResolvedValueOnce(status);
  render(<SettingsScreen tab="ai" />);
  await screen.findByRole('combobox', { name: 'Embedding model' });
  await selectSetting('Embedding model', 'BGE Small English v1.5 Q8_0');
  await selectSetting('Embedding backend', 'Auto');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await screen.findByText('Settings saved.');
  expect(saved.settings.ai.recall.embedding).toMatchObject({
    localModelId: 'bge-small-en-v1.5-q8_0',
    localBackend: 'auto',
    provider: 'local',
  });
  await page.getByRole('button', { name: 'Check readiness' }).click();
  await screen.findByText('Could not check readiness. Try again.');
  await page.getByRole('button', { name: 'Check readiness' }).click();
  await screen.findByText('Embeddings: Ready');
  expect(recall.getRecallStatus).toHaveBeenCalledTimes(3);
});

test('SET-14 failed initial settings loading preserves the screen and recovers through retry', async () => {
  settings.getAppSettings.mockRejectedValueOnce(
    new Error('Synthetic settings database is busy.'),
  );
  render(<SettingsScreen />);
  await screen.findByText('Synthetic settings database is busy.');
  expect(screen.getByRole('heading', { name: 'Settings' })).toBeTruthy();
  await screen.findByRole('button', { name: 'Try again' });
  await page.getByRole('button', { name: 'Try again', exact: true }).click();
  expect(
    await screen.findByRole('checkbox', { name: /Enable expert mode/ }),
  ).toBeTruthy();
  expect(screen.queryByRole('alert')).toBeNull();
  expect(settings.getAppSettings).toHaveBeenCalledTimes(2);
});
