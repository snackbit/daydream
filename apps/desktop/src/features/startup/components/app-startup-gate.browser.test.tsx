import type { AppStartupStatus } from '@daydream/shared';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { AppStartupGate } from './app-startup-gate';

const startupStore = vi.hoisted(() => ({
  error: null as string | null,
  initialize: vi.fn(),
  retry: vi.fn(),
  status: null as AppStartupStatus | null,
}));

vi.mock('../startup-store', () => ({
  useStartupStore: (selector: (state: typeof startupStore) => unknown) =>
    selector(startupStore),
}));

beforeEach(() => {
  startupStore.error = null;
  startupStore.initialize.mockReset();
  startupStore.retry.mockReset();
  startupStore.status = null;
});

afterEach(() => {
  cleanup();
});

test('startup gate renders loading state before the app shell', () => {
  startupStore.status = startupStatus('loading');

  render(
    <AppStartupGate>
      <div>App shell</div>
    </AppStartupGate>,
  );

  expect(
    screen.getByRole('heading', { name: 'Preparing your workspace' }),
  ).toBeDefined();
  expect(screen.queryByText('App shell')).toBeNull();
  expect(startupStore.initialize).toHaveBeenCalled();
});

test('startup gate reveals children when startup is ready', () => {
  startupStore.status = startupStatus('ready');

  render(
    <AppStartupGate>
      <div>App shell</div>
    </AppStartupGate>,
  );

  expect(screen.getByText('App shell')).toBeDefined();
  expect(
    screen.queryByRole('heading', { name: 'Preparing your workspace' }),
  ).toBeNull();
});

test('startup gate keeps hard startup failures on the error screen', () => {
  startupStore.status = {
    ...startupStatus('error'),
    error: {
      code: 'startup_failed',
      message: 'Database migration failed.',
      stageId: 'core',
    },
  };

  render(
    <AppStartupGate>
      <div>App shell</div>
    </AppStartupGate>,
  );

  expect(
    screen.getByRole('heading', { name: 'Could not finish startup' }),
  ).toBeDefined();
  expect(screen.getByText('Database migration failed.')).toBeDefined();
  expect(screen.queryByText('App shell')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
  expect(startupStore.retry).toHaveBeenCalledOnce();
});

function startupStatus(phase: AppStartupStatus['phase']): AppStartupStatus {
  return {
    activeStageId: phase === 'loading' ? 'core' : null,
    error: null,
    phase,
    stages: [
      {
        description: 'Loading persisted settings and local metadata.',
        id: 'core',
        label: 'Core app state',
        status: phase === 'loading' ? 'loading' : 'ready',
      },
      {
        description: 'Preparing local transcription.',
        id: 'transcription',
        label: 'Local transcription',
        status: phase === 'loading' ? 'pending' : 'ready',
      },
    ],
    warnings: [],
  };
}
