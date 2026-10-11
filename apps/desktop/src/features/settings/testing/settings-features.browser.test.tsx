import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import type * as NotificationsClient from '@/features/notifications/notifications-client';
import type * as RecorderClient from '@/features/recorder/recorder-client';
import { SettingsScreen } from '@/screens/SettingsScreen';
import { useSettingsStore } from '../settings-store';
import './settings-workspace-harness';
import {
  setupWorkspace,
  cleanupWorkspace,
  configureAccounts,
  settings,
  saved,
  selectSetting,
  recall,
} from './settings-workspace-harness';

vi.mock(
  '@/features/settings/maintenance-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks'))
      .maintenance,
);
vi.mock(
  '@/features/settings/settings-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks'))
      .settings,
);
vi.mock('@/features/recorder/recorder-client', async (importOriginal) => ({
  ...(await importOriginal<typeof RecorderClient>()),
  ...(await import('@/features/settings/testing/settings-workspace-mocks'))
    .recorder,
}));
vi.mock(
  '@/features/ai/ai-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks')).ai,
);
vi.mock(
  '@/features/ai/models/model-transfer-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks'))
      .transfer,
);
vi.mock(
  '@/features/recall/recall-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks'))
      .recall,
);
vi.mock(
  '@/features/settings/models/model-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks'))
      .accounts,
);
vi.mock('@tanstack/react-router', async () => {
  const { blocker } =
    await import('@/features/settings/testing/settings-workspace-mocks');
  return { useBlocker: () => blocker };
});
vi.mock(
  '@/features/notifications/notifications-client',
  async (importOriginal) => ({
    ...(await importOriginal<typeof NotificationsClient>()),
    getNotifications: async () => ({ notifications: [] }),
  }),
);

beforeEach(setupWorkspace);
afterEach(cleanupWorkspace);
test('selecting Local clears a configured remote embedding binding without requesting remote consent', async () => {
  configureAccounts();
  saved.settings.ai.recall.embedding.enabled = true;
  render(<SettingsScreen section="ai" />);
  await screen.findByRole('button', { name: 'Configure Search' });
  await page.getByRole('button', { name: 'Configure Search' }).click();
  await selectSetting('Search processing', 'Local');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await waitFor(() => expect(settings.saveAppSettings).toHaveBeenCalledOnce());
  expect(saved.settings.ai.modelRegistry?.bindings).toEqual({
    embedding: null,
    summary: 'synthetic-model',
    vision: 'synthetic-model',
  });
  expect(saved.settings.ai.recall.embedding).toMatchObject({
    provider: 'local',
    enabled: true,
  });
  expect(
    screen.queryByRole('alertdialog', { name: 'Allow remote processing?' }),
  ).toBeNull();
  await page.getByRole('button', { name: 'Configure Search' }).click();
  expect(
    await screen.findByRole('combobox', { name: 'Search processing' }),
  ).toHaveTextContent('Local');
});
test('Cancel after a failed compound save discards the profile and credentials', async () => {
  settings.saveAppSettings.mockRejectedValueOnce(
    new Error('Synthetic encryption failure.'),
  );
  render(<SettingsScreen section="ai" />);
  await screen.findByRole('button', { name: 'Configure Transcription' });
  await page.getByRole('button', { name: 'Configure Transcription' }).click();
  await selectSetting('Transcription processing', 'Remote · API');
  await page
    .getByRole('textbox', { name: 'API URL', exact: true })
    .fill('https://example.test/v1');
  await page
    .getByRole('textbox', { name: 'Model', exact: true })
    .fill('synthetic-audio');
  await page
    .getByLabelText('API key', { exact: true })
    .fill('synthetic-secret');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.getByRole('button', { name: 'Allow and save' }).click();
  await screen.findByText(
    'Could not save changes. Review the connection and try again.',
  );
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  expect(useSettingsStore.getState().draft?.ai.transcription.provider).toBe(
    'local-whisper-cpp',
  );
  expect(useSettingsStore.getState().isDirty).toBe(false);
  await useSettingsStore
    .getState()
    .commit((current) => ({ ...current, debug: { enabled: true } }));
  expect(
    settings.saveAppSettings.mock.lastCall?.[0].credentials,
  ).toBeUndefined();
  expect(saved.settings.ai.transcription.provider).toBe('local-whisper-cpp');
});
test('feature editor Cancel and Escape discard edits and return focus to the row', async () => {
  configureAccounts();
  render(<SettingsScreen section="ai" />);
  await screen.findByRole('button', { name: 'Configure Day summaries' });
  await page.getByRole('button', { name: 'Configure Day summaries' }).click();
  await page.getByRole('switch', { name: 'Generate automatically' }).click();
  await page.getByRole('button', { name: 'Close', exact: true }).click();
  await screen.findByRole('alertdialog', { name: 'Discard changes?' });
  await page.getByRole('button', { name: 'Keep editing' }).click();
  expect(
    await screen.findByRole('dialog', { name: 'Day summaries' }),
  ).toBeTruthy();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(settings.saveAppSettings).not.toHaveBeenCalled();
});
test('SET-09 remote search consent names the destination and does not enable summaries or screen processing', async () => {
  configureAccounts();
  render(<SettingsScreen section="ai" />);
  await screen.findByRole('button', { name: 'Configure Search' });
  await page.getByRole('button', { name: 'Configure Search' }).click();
  await selectSetting('Search processing', 'Remote');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await screen.findByRole('alertdialog', { name: 'Allow remote processing?' });
  expect(screen.getByText(/Synthetic API.*example.test/)).toBeTruthy();
  expect(
    screen.getByText('Recognized text, audio transcripts and search queries.'),
  ).toBeTruthy();
  expect(settings.saveAppSettings).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Allow and save' }).click();
  await waitFor(() =>
    expect(saved.settings.ai.recall.embedding).toMatchObject({
      provider: 'openai-compatible',
      enabled: true,
    }),
  );
  expect(saved.settings.ai.recall.generation.enabled).toBe(false);
  expect(saved.settings.ai.vision.provider).toBe('disabled');
});
test('summary consent cancellation keeps remote processing off, then explicit approval enables only summaries', async () => {
  configureAccounts();
  render(<SettingsScreen section="ai" />);
  await screen.findByRole('button', { name: 'Configure Day summaries' });
  await page.getByRole('button', { name: 'Configure Day summaries' }).click();
  await page.getByRole('switch', { name: 'Enable day summaries' }).click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page
    .getByRole('alertdialog')
    .getByRole('button', { name: 'Cancel', exact: true })
    .click();
  expect(saved.settings.ai.recall.generation.enabled).toBe(false);
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.getByRole('button', { name: 'Allow and save' }).click();
  await waitFor(() =>
    expect(saved.settings.ai.recall.generation.enabled).toBe(true),
  );
  expect(saved.settings.ai.recall.embedding.enabled).toBe(false);
  expect(saved.settings.ai.vision.provider).toBe('disabled');
});
test('screen consent is shared while disabling observations preserves Vision OCR and account credentials', async () => {
  configureAccounts();
  render(<SettingsScreen section="ai" />);
  await screen.findByRole('button', { name: 'Configure Text recognition' });
  await page
    .getByRole('button', { name: 'Configure Text recognition' })
    .click();
  await selectSetting('Recognition method', 'Remote · Screen model');
  await page
    .getByRole('switch', { name: 'Allow remote screen processing' })
    .click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await screen.findByText(
    /Sampled screen images and recognized text for text recognition/,
  );
  await page.getByRole('button', { name: 'Allow and save' }).click();
  await waitFor(() => expect(saved.settings.ai.ocr.engine).toBe('vision'));
  await page
    .getByRole('button', { name: 'Configure Screen observations' })
    .click();
  await page
    .getByRole('switch', { name: 'Enable screen observations' })
    .click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.getByRole('button', { name: 'Allow and save' }).click();
  await waitFor(() =>
    expect(saved.settings.ai.vision.observationsEnabled).toBe(true),
  );
  await page
    .getByRole('button', { name: 'Configure Screen observations' })
    .click();
  await page
    .getByRole('switch', { name: 'Enable screen observations' })
    .click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await waitFor(() =>
    expect(saved.settings.ai.vision.observationsEnabled).toBe(false),
  );
  expect(saved.settings.ai.vision.provider).toBe('openai-compatible');
  expect(saved.settings.ai.vision.baseUrl).toBe('https://example.test/v1');
  expect(saved.settings.ai.ocr.engine).toBe('vision');
});
test('remote transcription saves its key and profile atomically, retaining the key after encryption failure', async () => {
  settings.saveAppSettings.mockRejectedValueOnce({
    code: 'settings_secret_encryption_unavailable',
    message: 'Encryption unavailable.',
  });
  render(<SettingsScreen section="ai" />);
  await screen.findByRole('button', { name: 'Configure Transcription' });
  await page.getByRole('button', { name: 'Configure Transcription' }).click();
  await selectSetting('Transcription processing', 'Remote · API');
  await page
    .getByRole('textbox', { name: 'API URL', exact: true })
    .fill('https://example.test/v1');
  await page
    .getByRole('textbox', { name: 'Model', exact: true })
    .fill('synthetic-audio');
  await page
    .getByLabelText('API key', { exact: true })
    .fill('synthetic-secret');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.getByRole('button', { name: 'Allow and save' }).click();
  await screen.findByText(
    'Could not save changes. Review the connection and try again.',
  );
  expect(screen.getByLabelText('API key', { exact: true })).toHaveValue(
    'synthetic-secret',
  );
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.getByRole('button', { name: 'Allow and save' }).click();
  await waitFor(() =>
    expect(saved.settings.ai.transcription.remoteApiKeyRef).toBe(
      'synthetic-encrypted-ref',
    ),
  );
  expect(settings.saveAppSettings).toHaveBeenLastCalledWith(
    expect.objectContaining({
      credentials: { transcriptionKey: 'synthetic-secret' },
      applyStorageCleanup: false,
      settings: expect.objectContaining({
        ai: expect.objectContaining({
          transcription: expect.objectContaining({
            remoteBaseUrl: 'https://example.test/v1',
            remoteModel: 'synthetic-audio',
          }),
        }),
      }),
    }),
  );
});
test('SET-11 local search tuning and saved readiness recover after a failed check', async () => {
  saved.settings.expert = { enabled: true };
  render(<SettingsScreen section="ai" />);
  await screen.findByRole('button', { name: 'Configure Search' });
  await page.getByRole('button', { name: 'Configure Search' }).click();
  await selectSetting('Local embedding model', 'BGE English · Download needed');
  await page.getByRole('button', { name: 'Advanced', exact: true }).click();
  await selectSetting('Embedding backend', 'Auto');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await waitFor(() =>
    expect(saved.settings.ai.recall.embedding).toMatchObject({
      localModelId: 'bge-small-en-v1.5-q8_0',
      localBackend: 'auto',
      provider: 'local',
    }),
  );
  await page.getByRole('button', { name: 'Configure Search' }).click();
  await page
    .getByRole('button', { name: 'Saved configuration', exact: true })
    .click();
  await screen.findByRole('button', { name: 'Check readiness' });
  recall.getRecallStatus.mockRejectedValueOnce(
    new Error('Synthetic worker busy.'),
  );
  await page.getByRole('button', { name: 'Check readiness' }).click();
  await screen.findByText('Could not check readiness. Try again.');
  await page.getByRole('button', { name: 'Check readiness' }).click();
  await screen.findByText(/Embeddings:/);
});
