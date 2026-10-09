import type { AppStartupStatus } from '@daydream/shared';
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import { useStartupStore } from '../startup-store';
import { AppStartupGate } from './app-startup-gate';
import { StartupWarningBanner } from './startup-warning-banner';

const client = vi.hoisted(() => ({
  getAppStartupStatus: vi.fn(),
  listenToAppStartupStatusChanged: vi.fn(),
  retryAppStartup: vi.fn(),
}));
vi.mock('../startup-client', () => client);

function status(phase: AppStartupStatus['phase']): AppStartupStatus {
  return {
    activeStageId: null,
    phase,
    error:
      phase === 'error'
        ? {
            code: 'startup_failed',
            message: 'Synthetic local storage unavailable.',
            stageId: 'core',
          }
        : null,
    stages: [
      {
        id: 'core',
        label: 'Core state',
        description: 'Synthetic startup stage',
        status: phase === 'ready' ? 'ready' : 'error',
      },
    ],
    warnings: [],
  };
}
beforeEach(() => {
  vi.resetAllMocks();
  useStartupStore.setState(useStartupStore.getInitialState(), true);
  client.listenToAppStartupStatusChanged.mockResolvedValue(() => undefined);
});
afterEach(cleanup);

test('SET-12 startup retry uses the real store and reveals the workspace after native recovery', async () => {
  client.getAppStartupStatus.mockResolvedValue(status('error'));
  client.retryAppStartup
    .mockRejectedValueOnce(new Error('Synthetic retry unavailable.'))
    .mockResolvedValueOnce(status('ready'));
  render(
    <AppStartupGate>
      <h1>Synthetic workspace</h1>
    </AppStartupGate>,
  );
  await screen.findByText('Synthetic local storage unavailable.');
  expect(
    screen.queryByRole('heading', { name: 'Synthetic workspace' }),
  ).toBeNull();
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await screen.findByText('Synthetic retry unavailable.');
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  expect(
    await screen.findByRole('heading', { name: 'Synthetic workspace' }),
  ).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Retry' })).toBeNull();
  expect(client.retryAppStartup).toHaveBeenCalledTimes(2);
});

test('SET-13 startup warnings allow the workspace to open and remain visible for review', async () => {
  client.getAppStartupStatus.mockResolvedValue({
    ...status('ready'),
    warnings: [
      {
        code: 'synthetic_model_unavailable',
        message: 'Synthetic optional model is unavailable.',
        stageId: 'transcription',
      },
    ],
  });
  render(
    <AppStartupGate>
      <h1>Synthetic workspace</h1>
      <StartupWarningBanner />
    </AppStartupGate>,
  );
  expect(
    await screen.findByRole('heading', { name: 'Synthetic workspace' }),
  ).toBeTruthy();
  expect(
    screen.getByText('Synthetic optional model is unavailable.'),
  ).toBeTruthy();
});
