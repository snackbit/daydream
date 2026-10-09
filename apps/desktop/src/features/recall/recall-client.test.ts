import type { Event } from '@tauri-apps/api/event';
import { beforeEach, expect, test, vi } from 'vitest';
import { listenToRecallStateChanged } from './recall-client';

const { listenMock } = vi.hoisted(() => ({ listenMock: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }));
let receive: (event: Event<unknown>) => void;
const cleanup = vi.fn();
beforeEach(() => {
  vi.clearAllMocks();
  receive = () => {
    throw new Error('listener not registered');
  };
  listenMock.mockImplementation(
    async (_name: string, handler: (event: Event<unknown>) => void) => {
      receive = handler;
      return cleanup;
    },
  );
});
test('malformed privacy events notify invalidation without forwarding fabricated metadata', async () => {
  const handler = vi.fn();
  const onInvalid = vi.fn();
  const unlisten = await listenToRecallStateChanged(handler, onInvalid);
  receive({
    event: 'recall-state-changed',
    id: 1,
    payload: {
      dayId: null,
      privacyRevision: -1,
      privateText: 'Synthetic sensitive text',
    },
  });
  expect(handler).not.toHaveBeenCalled();
  expect(onInvalid).toHaveBeenCalledTimes(1);
  const error: unknown = onInvalid.mock.calls[0]?.[0];
  expect(error).toBeInstanceOf(Error);
  expect(String(error)).not.toContain('Synthetic sensitive text');
  receive({
    event: 'recall-state-changed',
    id: 2,
    payload: { dayId: '2026-01-01', privacyRevision: 2 },
  });
  expect(handler).toHaveBeenCalledWith({
    dayId: '2026-01-01',
    privacyRevision: 2,
  });
  expect(onInvalid).toHaveBeenCalledTimes(1);
  unlisten();
  expect(cleanup).toHaveBeenCalledTimes(1);
});
test('errors from a valid event consumer are not misreported as malformed native metadata', async () => {
  const onInvalid = vi.fn();
  const consumerError = new Error('synthetic consumer failure');
  await listenToRecallStateChanged(() => {
    throw consumerError;
  }, onInvalid);
  expect(() =>
    receive({
      event: 'recall-state-changed',
      id: 1,
      payload: { dayId: null, privacyRevision: 1 },
    }),
  ).toThrow(consumerError);
  expect(onInvalid).not.toHaveBeenCalled();
});
