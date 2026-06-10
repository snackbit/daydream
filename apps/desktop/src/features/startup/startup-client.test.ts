import { beforeEach, expect, test, vi } from 'vitest';
import {
  appStartupStatusChangedEvent,
  getAppStartupStatus,
  listenToAppStartupStatusChanged,
  retryAppStartup,
} from './startup-client';

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockResolvedValue({});
  listenMock.mockReset();
});

test('startup client uses app startup command names', async () => {
  await getAppStartupStatus();
  await retryAppStartup();

  expect(invokeMock).toHaveBeenNthCalledWith(1, 'getAppStartupStatus');
  expect(invokeMock).toHaveBeenNthCalledWith(2, 'retryAppStartup');
});

test('startup client listens for startup status changes', async () => {
  const unlisten = vi.fn();
  const handler = vi.fn();
  listenMock.mockImplementation((_eventName, listener) => {
    listener({ payload: { phase: 'ready' } });
    return Promise.resolve(unlisten);
  });

  await expect(listenToAppStartupStatusChanged(handler)).resolves.toBe(
    unlisten,
  );

  expect(listenMock).toHaveBeenCalledWith(
    appStartupStatusChangedEvent,
    expect.any(Function),
  );
  expect(handler).toHaveBeenCalledWith({ phase: 'ready' });
});
