import type { AppStartupStatus } from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import { useStartupStore } from './startup-store';

const client = vi.hoisted(() => ({
  getAppStartupStatus: vi.fn(),
  listenToAppStartupStatusChanged: vi.fn(),
  retryAppStartup: vi.fn(),
}));

vi.mock('./startup-client', () => client);

beforeEach(() => {
  client.getAppStartupStatus.mockReset();
  client.listenToAppStartupStatusChanged.mockReset();
  client.retryAppStartup.mockReset();
  useStartupStore.setState(useStartupStore.getInitialState(), true);
});

test('initialize keeps ready startup event when initial snapshot resolves stale', async () => {
  const listenerRef: {
    current: ((status: AppStartupStatus) => void) | null;
  } = { current: null };
  const resolveStatusRef: {
    current: ((status: AppStartupStatus) => void) | null;
  } = { current: null };
  const unlisten = vi.fn();

  client.listenToAppStartupStatusChanged.mockImplementation(
    (handler: (status: AppStartupStatus) => void) => {
      listenerRef.current = handler;
      return Promise.resolve(unlisten);
    },
  );
  client.getAppStartupStatus.mockReturnValue(
    new Promise<AppStartupStatus>((resolve) => {
      resolveStatusRef.current = resolve;
    }),
  );

  const initialize = useStartupStore.getState().initialize();
  await Promise.resolve();

  const listener = listenerRef.current;
  const resolveStatus = resolveStatusRef.current;

  if (!listener || !resolveStatus) {
    throw new Error('startup initialization did not subscribe before fetching');
  }

  listener(startupStatus('ready'));
  resolveStatus(startupStatus('loading'));
  await initialize;

  expect(useStartupStore.getState()).toMatchObject({
    error: null,
    status: startupStatus('ready'),
    subscriptionStatus: 'listening',
  });
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
    ],
    warnings: [],
  };
}
