import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { Button } from '@/components/ui/button';
import type * as NotificationsClient from '@/features/notifications/notifications-client';
import type * as ProcessingClient from '@/features/processing/processing-client';
import { useRecordingProcessingActions } from '@/features/processing/use-recording-processing-actions';
import type * as RecorderClient from '@/features/recorder/recorder-client';
import { SettingsScreen } from '@/screens/SettingsScreen';
import { useSettingsStore } from '../settings-store';
import './settings-workspace-harness';
import {
  setupWorkspace,
  cleanupWorkspace,
  settings,
  saved,
  maintenance,
  selectSetting,
  blocker,
  ai,
  recorder,
} from './settings-workspace-harness';

const processingControl = vi.hoisted(() => vi.fn());
vi.mock('@/features/processing/processing-client', async (importOriginal) => ({
  ...(await importOriginal<typeof ProcessingClient>()),
  controlRecordingProcessingGroups: processingControl,
}));

const refreshedProcessing = async () => {};
function LocalPresetAction() {
  const { perform, pending, error } = useRecordingProcessingActions(
    'synthetic-recording',
    refreshedProcessing,
  );
  return (
    <>
      <Button
        disabled={pending}
        onClick={() => void perform({ action: 'optimize-visual' })}
      >
        Use fast local processing
      </Button>
      {error ? <p role="alert">{error}</p> : null}
    </>
  );
}

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
test.each([true, false])(
  'local preset stays saved after reopening settings and an unrelated edit (admitted: %s)',
  async (admitted) => {
    const nativeState = structuredClone(saved);
    nativeState.settings.ai.ocr.engine = 'vision';
    nativeState.settings.ai.processing.resourceMode = 'background';
    nativeState.settings.ai.processing.visualSamplingMode = 'full';
    settings.getAppSettings.mockImplementation(async () =>
      structuredClone(nativeState),
    );
    settings.saveAppSettings.mockImplementation(async (input) => {
      nativeState.settings = structuredClone(input.settings);
      return structuredClone(nativeState);
    });
    const initial = render(<SettingsScreen />);
    await screen.findByRole('switch', { name: 'Enable debug tools' });
    initial.unmount();
    processingControl.mockImplementation(async () => {
      nativeState.settings.ai.ocr.engine = 'tesseract';
      nativeState.settings.ai.processing.resourceMode = 'fast';
      nativeState.settings.ai.processing.visualSamplingMode = 'adaptive-v1';
      return {
        results: [
          {
            jobId: 'synthetic-job',
            succeeded: admitted,
            errorCode: admitted ? null : 'ai_reprocessing_preview_changed',
          },
        ],
      };
    });
    const action = render(<LocalPresetAction />);
    await page
      .getByRole('button', { name: 'Use fast local processing' })
      .click();
    await waitFor(() =>
      expect(
        useSettingsStore.getState().draft?.ai.processing.resourceMode,
      ).toBe('fast'),
    );
    await expect(
      page.getByRole('button', { name: 'Use fast local processing' }),
    ).not.toBeDisabled();
    if (!admitted) await screen.findByRole('alert');
    action.unmount();
    render(<SettingsScreen />);
    await screen.findByRole('switch', { name: 'Enable debug tools' });
    await page.getByRole('switch', { name: 'Enable debug tools' }).click();
    await waitFor(() => expect(nativeState.settings.debug?.enabled).toBe(true));
    expect(nativeState.settings.ai.ocr.engine).toBe('tesseract');
    expect(nativeState.settings.ai.processing).toMatchObject({
      resourceMode: 'fast',
      visualSamplingMode: 'adaptive-v1',
    });
  },
);
test('SET-03 switches autosave without cleanup and survive reopening settings', async () => {
  const view = render(<SettingsScreen />);
  await screen.findByRole('switch', { name: 'Enable expert mode' });
  await page.getByRole('switch', { name: 'Enable expert mode' }).click();
  await page.getByRole('switch', { name: 'Enable debug tools' }).click();
  await waitFor(() =>
    expect(
      saved.settings.expert?.enabled && saved.settings.debug?.enabled,
    ).toBe(true),
  );
  expect(settings.previewAppSettingsStorageCleanup).not.toHaveBeenCalled();
  for (const [input] of settings.saveAppSettings.mock.calls)
    expect(input.applyStorageCleanup).toBe(false);
  expect(screen.queryByRole('button', { name: /^Save$/ })).toBeNull();
  view.unmount();
  render(<SettingsScreen />);
  expect(
    await screen.findByRole('switch', { name: 'Enable expert mode' }),
  ).toHaveAttribute('aria-checked', 'true');
});
test('SET-04 reset cancellation keeps saved preferences and never executes maintenance', async () => {
  render(<SettingsScreen />);
  await screen.findByRole('switch', { name: 'Enable debug tools' });
  await page.getByRole('switch', { name: 'Enable debug tools' }).click();
  await waitFor(() => expect(saved.settings.debug?.enabled).toBe(true));
  await page
    .getByRole('button', { name: 'Reset preferences', exact: true })
    .click();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  expect(maintenance.executeMaintenance).not.toHaveBeenCalled();
  expect(saved.settings.debug?.enabled).toBe(true);
});
test('recording numbers commit on Enter and reject invalid input before autosave', async () => {
  render(<SettingsScreen section="recording" />);
  await screen.findByRole('spinbutton', { name: 'Seconds per frame' });
  await page.getByRole('spinbutton', { name: 'Seconds per frame' }).fill('10');
  expect(settings.saveAppSettings).not.toHaveBeenCalled();
  await userEvent.keyboard('{Enter}');
  await waitFor(() => expect(saved.settings.record.fps).toBe(0.1));
  await page.getByRole('spinbutton', { name: 'Seconds per frame' }).fill('0');
  await userEvent.keyboard('{Enter}');
  await screen.findByRole('alert');
  expect(saved.settings.record.fps).toBe(0.1);
});
test('SET-05 storage cancellation discards the policy; confirmation authorizes cleanup once', async () => {
  settings.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 2,
    sizeBytes: 1024,
  });
  render(<SettingsScreen section="storage" />);
  await screen.findByRole('spinbutton', { name: 'Recording retention' });
  await page.getByRole('spinbutton', { name: 'Recording retention' }).fill('1');
  await userEvent.keyboard('{Enter}');
  await screen.findByRole('dialog', { name: 'Delete old recordings?' });
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  expect(settings.saveAppSettings).not.toHaveBeenCalled();
  expect(saved.settings.storage.retentionDays).toBe(7);
  await page.getByRole('spinbutton', { name: 'Recording retention' }).fill('2');
  await userEvent.keyboard('{Enter}');
  await screen.findByRole('dialog', { name: 'Delete old recordings?' });
  await page.getByRole('button', { name: 'Delete and save' }).click();
  await waitFor(() => expect(saved.settings.storage.retentionDays).toBe(2));
  expect(settings.saveAppSettings).toHaveBeenCalledExactlyOnceWith(
    expect.objectContaining({ applyStorageCleanup: true }),
  );
});
test('SET-06 protection validates complete rules before committing and supports removal', async () => {
  render(<SettingsScreen section="privacy" />);
  await screen.findByRole('button', { name: /Set up protection/ });
  await page.getByRole('button', { name: /Set up protection/ }).click();
  await page
    .getByRole('checkbox', { name: 'Enable protected windows' })
    .click();
  await page.getByRole('button', { name: 'Add rule', exact: true }).click();
  await page
    .getByRole('textbox', { name: 'Label', exact: true })
    .fill('Synthetic private app');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await screen.findByText(
    'Enabled protected window rules need an app, process, or title matcher.',
  );
  expect(settings.saveAppSettings).not.toHaveBeenCalled();
  await page
    .getByRole('textbox', { name: 'App identity', exact: true })
    .fill('org.example.Synthetic');
  await selectSetting('Title matcher', 'Regex');
  await page.getByRole('textbox', { name: 'Title value' }).fill('[');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await screen.findByText('Protected window title regex is invalid.');
  await page.getByRole('textbox', { name: 'Title value' }).fill('^Synthetic');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await waitFor(() =>
    expect(saved.settings.record.windowProtection.rules).toHaveLength(1),
  );
  await page.getByRole('button', { name: /1 active rule/ }).click();
  await page
    .getByRole('button', { name: 'Delete Synthetic private app' })
    .click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await waitFor(() =>
    expect(saved.settings.record.windowProtection.rules).toHaveLength(0),
  );
});
test('SET-07 window picker retries an unavailable provider and saves the selected app', async () => {
  recorder.listProtectableWindows
    .mockResolvedValueOnce({
      windows: [],
      provider: 'linux-x11',
      status: 'provider-unavailable',
    })
    .mockResolvedValue({
      windows: [
        {
          appIdentity: 'org.example.Synthetic',
          processName: 'synthetic',
          title: 'Synthetic app',
          id: '1',
          processId: 1,
          platform: 'linux',
        },
      ],
      provider: 'linux-x11',
      status: 'available',
    });
  render(<SettingsScreen section="privacy" detail="protection" />);
  await screen.findByRole('button', { name: 'Pick window' });
  await page.getByRole('button', { name: 'Pick window' }).click();
  await screen.findByRole('button', { name: /Refresh/ });
  await page.getByRole('button', { name: /Refresh/ }).click();
  await page.getByRole('button', { name: /Synthetic app/ }).click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await waitFor(() =>
    expect(saved.settings.record.windowProtection.rules[0]?.appIdentity).toBe(
      'org.example.Synthetic',
    ),
  );
});
test('SET-08 processing timing and resource mode autosave independently', async () => {
  saved.settings.expert = { enabled: true };
  render(<SettingsScreen section="ai" />);
  await screen.findByRole('combobox', { name: 'Processing timing' });
  await selectSetting('Processing timing', 'Manually');
  await selectSetting('Processing resource use', 'Fast');
  await waitFor(() =>
    expect(saved.settings.ai.processing).toMatchObject({
      autoStartAfterRecording: false,
      resourceMode: 'fast',
    }),
  );
  await selectSetting('Processing timing', 'During recording');
  await waitFor(() =>
    expect(saved.settings.ai.processing).toMatchObject({
      autoStartAfterRecording: true,
      timing: 'during-recording',
    }),
  );
  await page.getByRole('button', { name: 'Advanced processing' }).click();
  const adaptive = await screen.findByRole('switch', {
    name: 'Adaptive screen sampling',
  });
  const wasEnabled = adaptive.getAttribute('aria-checked') === 'true';
  await page.getByRole('switch', { name: 'Adaptive screen sampling' }).click();
  expect(
    saved.settings.ai.processing.visualSamplingMode === 'adaptive-v1',
  ).toBe(wasEnabled);
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await waitFor(() =>
    expect(saved.settings.ai.processing).toMatchObject({
      visualSamplingMode: wasEnabled ? 'full' : 'adaptive-v1',
      resourceMode: 'fast',
      timing: 'during-recording',
    }),
  );
  await page.getByRole('button', { name: 'Advanced processing' }).click();
  expect(
    await screen.findByRole('switch', { name: 'Adaptive screen sampling' }),
  ).toHaveAttribute('aria-checked', String(!wasEnabled));
  await page.getByRole('switch', { name: 'Adaptive screen sampling' }).click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await waitFor(() =>
    expect(
      saved.settings.ai.processing.visualSamplingMode === 'adaptive-v1',
    ).toBe(wasEnabled),
  );
});
test('SET-10 transcription saves model, language and backend together; CUDA remains unavailable', async () => {
  saved.settings.expert = { enabled: true };
  ai.listAiModels.mockResolvedValue({
    models: [
      {
        id: 'tiny',
        label: 'Whisper tiny',
        provider: 'local-whisper-cpp',
        downloadStatus: 'available',
      },
    ],
  });
  render(<SettingsScreen section="ai" />);
  await screen.findByRole('button', { name: 'Configure Transcription' });
  await page.getByRole('button', { name: 'Configure Transcription' }).click();
  await selectSetting('Whisper model', 'Whisper tiny');
  await selectSetting('Transcription language', 'German');
  await page.getByRole('button', { name: 'Advanced', exact: true }).click();
  await page.getByRole('combobox', { name: 'Local backend' }).click();
  expect(await screen.findByRole('option', { name: 'CUDA' })).toHaveAttribute(
    'aria-disabled',
    'true',
  );
  await page.getByRole('option', { name: 'VULKAN' }).click();
  expect(settings.saveAppSettings).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await waitFor(() =>
    expect(saved.settings.ai.transcription).toMatchObject({
      localModelId: 'tiny',
      localBackend: 'vulkan',
      language: 'de',
    }),
  );
});
test('failed autosave retains edits; Retry persists them and Revert restores the saved state', async () => {
  settings.saveAppSettings.mockRejectedValueOnce(
    new Error('Synthetic disk failure.'),
  );
  render(<SettingsScreen />);
  await screen.findByRole('switch', { name: 'Enable expert mode' });
  await page.getByRole('switch', { name: 'Enable expert mode' }).click();
  await screen.findByText('Synthetic disk failure.');
  expect(
    screen.getByRole('switch', { name: 'Enable expert mode' }),
  ).toHaveAttribute('aria-checked', 'true');
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await waitFor(() => expect(saved.settings.expert?.enabled).toBe(true));
  settings.saveAppSettings.mockRejectedValueOnce(
    new Error('Synthetic disk failure.'),
  );
  await page.getByRole('switch', { name: 'Enable debug tools' }).click();
  await screen.findByText('Synthetic disk failure.');
  await page.getByRole('button', { name: 'Revert changes' }).click();
  expect(
    screen.getByRole('switch', { name: 'Enable debug tools' }),
  ).toHaveAttribute('aria-checked', 'false');
});
test('failed edits can stay, discard or retry before leaving settings', async () => {
  settings.saveAppSettings.mockRejectedValueOnce(
    new Error('Synthetic failure.'),
  );
  const view = render(<SettingsScreen />);
  await screen.findByRole('switch', { name: 'Enable expert mode' });
  await page.getByRole('switch', { name: 'Enable expert mode' }).click();
  await screen.findByText('Synthetic failure.');
  blocker.status = 'blocked';
  view.rerender(<SettingsScreen />);
  await page.getByRole('button', { name: 'Stay', exact: true }).click();
  expect(blocker.reset).toHaveBeenCalledOnce();
  await page.getByRole('button', { name: 'Discard changes' }).click();
  expect(blocker.proceed).toHaveBeenCalledOnce();
  expect(useSettingsStore.getState().draft?.expert?.enabled).toBe(false);
});
test('SET-14 a failed initial load recovers through Try again', async () => {
  settings.getAppSettings.mockRejectedValueOnce(
    new Error('Synthetic database unavailable.'),
  );
  render(<SettingsScreen />);
  await screen.findByText('Synthetic database unavailable.');
  await page.getByRole('button', { name: 'Try again' }).click();
  await screen.findByRole('switch', { name: 'Enable expert mode' });
  expect(screen.queryByRole('alert')).toBeNull();
});
