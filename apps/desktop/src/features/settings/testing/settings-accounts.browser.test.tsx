import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import type * as NotificationsClient from '@/features/notifications/notifications-client';
import type * as RecorderClient from '@/features/recorder/recorder-client';
import { SettingsScreen } from '@/screens/SettingsScreen';
import './settings-workspace-harness';
import {
  setupWorkspace,
  cleanupWorkspace,
  configureAccounts,
  settings,
  saved,
  accounts,
  ai,
  transfer,
  localAsset,
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
test('account list opens one editor; cancelling a new connection leaves settings untouched', async () => {
  configureAccounts();
  render(<SettingsScreen section="models" />);
  await screen.findByRole('button', { name: 'Manage Synthetic API' });
  expect(screen.queryByRole('textbox', { name: 'Connection name' })).toBeNull();
  await page.getByRole('button', { name: 'Add connection' }).click();
  await page
    .getByRole('button', { name: 'OpenAI API key', exact: true })
    .click();
  await page
    .getByLabelText('API key', { exact: true })
    .fill('synthetic-secret');
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  expect(settings.saveAppSettings).not.toHaveBeenCalled();
  expect(saved.settings.ai.modelRegistry?.connections).toHaveLength(1);
});
test('API connection and complete model commit together with encrypted key references', async () => {
  render(<SettingsScreen section="models" />);
  await screen.findByRole('button', { name: 'Add connection' });
  await page.getByRole('button', { name: 'Add connection' }).click();
  await page
    .getByRole('button', { name: 'OpenAI API key', exact: true })
    .click();
  await page
    .getByLabelText('API key', { exact: true })
    .fill('synthetic-secret');
  await page.getByRole('button', { name: 'Add model', exact: true }).click();
  await page
    .getByRole('textbox', { name: 'Model label' })
    .fill('Synthetic text');
  await page
    .getByRole('textbox', { name: 'Model identifier' })
    .fill('synthetic-text');
  await page.getByRole('button', { name: 'Test model', exact: true }).click();
  await screen.findByText(/text: Passed/);
  await page.getByRole('button', { name: 'Done with model' }).click();
  expect(settings.saveAppSettings).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await waitFor(() =>
    expect(saved.settings.ai.modelRegistry?.models).toHaveLength(1),
  );
  expect(settings.saveAppSettings).toHaveBeenCalledOnce();
  expect(saved.settings.ai.modelRegistry?.connections[0]?.apiKeyRef).toMatch(
    /^synthetic-encrypted-/,
  );
  expect(saved.settings.ai.recall.generation.enabled).toBe(false);
  expect(saved.settings.ai.vision.provider).toBe('disabled');
});
test('ChatGPT sign-in and model selection remain local until Done; account setup grants no remote consent', async () => {
  render(<SettingsScreen section="models" />);
  await screen.findByRole('button', { name: 'Add connection' });
  await page.getByRole('button', { name: 'Add connection' }).click();
  await page
    .getByRole('button', { name: 'ChatGPT subscription', exact: true })
    .click();
  await page.getByRole('button', { name: 'Sign in with ChatGPT' }).click();
  await screen.findByText('Connected as Synthetic account');
  await page.getByRole('button', { name: 'Add Synthetic text' }).click();
  await page.getByRole('button', { name: 'Done with model' }).click();
  expect(settings.saveAppSettings).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await waitFor(() =>
    expect(
      saved.settings.ai.modelRegistry?.connections[0]?.subscriptionId,
    ).toBe('synthetic-subscription'),
  );
  expect(saved.settings.ai.recall.generation.enabled).toBe(false);
  expect(saved.settings.ai.vision.provider).toBe('disabled');
  expect(accounts.signOutChatGpt).not.toHaveBeenCalled();
});
test('changing an active account destination asks for consent before saving', async () => {
  configureAccounts();
  saved.settings.ai.recall.generation.enabled = true;
  render(<SettingsScreen section="models" />);
  await screen.findByRole('button', { name: 'Manage Synthetic API' });
  await page.getByRole('button', { name: 'Manage Synthetic API' }).click();
  await page
    .getByRole('textbox', { name: 'API base URL' })
    .fill('https://other.example.test/v1');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await screen.findByRole('alertdialog', { name: 'Allow remote processing?' });
  expect(screen.getByText(/Synthetic API.*other.example.test/)).toBeTruthy();
  expect(settings.saveAppSettings).not.toHaveBeenCalled();
  await page
    .getByRole('alertdialog')
    .getByRole('button', { name: 'Cancel', exact: true })
    .click();
  expect(saved.settings.ai.modelRegistry?.connections[0]?.baseUrl).toBe(
    'https://example.test/v1',
  );
});
test('used models cannot be removed; transfer errors stay beside their model across refresh and unrelated actions', async () => {
  configureAccounts();
  ai.listAiModels.mockResolvedValue({
    models: [localAsset('multilingual-e5-small-q8_0', 'Multilingual E5')],
  });
  ai.downloadAiModel.mockRejectedValueOnce({ code: 'ai_storage_failed' });
  render(<SettingsScreen section="models" />);
  await screen.findByRole('button', { name: 'Manage Synthetic API' });
  await page.getByRole('button', { name: 'Manage Synthetic API' }).click();
  await page.getByRole('button', { name: 'Edit Synthetic model' }).click();
  expect(screen.getByRole('button', { name: 'Remove model' })).toBeDisabled();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await page
    .getByRole('group', { name: 'Multilingual E5' })
    .getByRole('button', { name: 'Download', exact: true })
    .click();
  await screen.findByText(/available disk space/i);
  window.dispatchEvent(new Event('focus'));
  await waitFor(() => expect(ai.listAiModels).toHaveBeenCalledTimes(2));
  expect(screen.getByText(/available disk space/i)).toBeTruthy();
  ai.downloadAiModel.mockResolvedValue({});
  await page
    .getByRole('group', { name: 'Multilingual E5' })
    .getByRole('button', { name: 'Download', exact: true })
    .click();
  await waitFor(() =>
    expect(screen.queryByText(/available disk space/i)).toBeNull(),
  );
});
test('pause, resume and partial removal use native transfer acknowledgements', async () => {
  ai.listAiModels.mockResolvedValue({
    models: [
      {
        ...localAsset(
          'multilingual-e5-small-q8_0',
          'Multilingual E5',
          'downloading',
        ),
        downloadedBytes: 1024,
      },
    ],
  });
  let acknowledge: (() => void) | undefined;
  transfer.pauseAiModelDownload.mockImplementation(
    () =>
      new Promise<void>((resolve) => {
        acknowledge = resolve;
      }),
  );
  render(<SettingsScreen section="models" />);
  await screen.findByRole('button', { name: 'Pause' });
  await page
    .getByRole('group', { name: 'Multilingual E5' })
    .getByRole('button', { name: 'Pause', exact: true })
    .click();
  expect(screen.getByRole('button', { name: 'Pause' })).toBeDisabled();
  ai.listAiModels.mockResolvedValue({
    models: [
      {
        ...localAsset(
          'multilingual-e5-small-q8_0',
          'Multilingual E5',
          'paused',
        ),
        downloadedBytes: 1024,
      },
    ],
  });
  acknowledge?.();
  await screen.findByRole('button', { name: 'Resume' });
  await page
    .getByRole('group', { name: 'Multilingual E5' })
    .getByRole('button', { name: 'Resume', exact: true })
    .click();
  await waitFor(() =>
    expect(transfer.resumeAiModelDownload).toHaveBeenCalledWith({
      modelId: 'multilingual-e5-small-q8_0',
    }),
  );
  await page
    .getByRole('group', { name: 'Multilingual E5' })
    .getByRole('button', { name: 'Remove partial data', exact: true })
    .click();
  await waitFor(() =>
    expect(transfer.removeAiModelPartial).toHaveBeenCalledWith({
      modelId: 'multilingual-e5-small-q8_0',
    }),
  );
});
