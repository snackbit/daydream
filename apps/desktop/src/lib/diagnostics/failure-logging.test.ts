import { beforeEach, afterEach, expect, test, vi } from 'vitest';
import { reportFrontendFailure } from './failure-logging';
import { invoke, decodeResponse } from './invoke';
import { listen } from './listen';

const boundary = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: boundary.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: boundary.listen }));
beforeEach(() => {
  boundary.invoke.mockReset();
  boundary.listen.mockReset();
  boundary.invoke.mockResolvedValue(undefined);
});
afterEach(() => vi.restoreAllMocks());
function reports() {
  return boundary.invoke.mock.calls
    .filter(([command]) => command === 'log_frontend_failure')
    .map(([, args]) => args.input);
}
async function flush() {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

test('a failed command retains its original rejection and reports safe correlated metadata', async () => {
  const requestId = crypto.randomUUID();
  const failure = {
    code: 'model_auth_failed',
    message: 'synthetic-private-provider-body',
  };
  boundary.invoke.mockImplementation(async (command) => {
    if (command !== 'log_frontend_failure') throw failure;
  });
  const input = {
    requestId,
    keyDraft: 'synthetic-secret-token',
    prompt: 'synthetic-private-prompt',
  };
  await expect(invoke('test_configured_model', { input })).rejects.toBe(
    failure,
  );
  await flush();
  expect(reports()).toEqual([
    expect.objectContaining({
      requestId,
      operation: 'test_configured_model',
      stage: 'invoke',
      errorCode: 'model_auth_failed',
      errorKind: 'native-code',
      elapsedMs: expect.any(Number),
    }),
  ]);
  const log = JSON.stringify(reports());
  expect(log).not.toContain('synthetic-private');
  expect(log).not.toContain('synthetic-secret');
});
test('decoder failures are distinct from command failures and preserve the thrown error', async () => {
  const failure = new SyntaxError('synthetic-private-output');
  const decode = () => {
    throw failure;
  };
  expect(() =>
    decodeResponse(
      'getRecallDay',
      { text: 'synthetic-private-source' },
      decode,
    ),
  ).toThrow(failure);
  await flush();
  expect(reports()).toEqual([
    expect.objectContaining({
      operation: 'getRecallDay',
      stage: 'decode',
      errorKind: 'SyntaxError',
      errorCode: null,
    }),
  ]);
  expect(JSON.stringify(reports())).not.toContain('synthetic-private');
});
test('one propagated failure is reported once while independent command attempts remain visible', async () => {
  const failure = new Error('synthetic repeated failure');
  boundary.invoke.mockImplementation(async (command) => {
    if (command !== 'log_frontend_failure') throw failure;
  });
  await expect(invoke('saveAppSettings')).rejects.toBe(failure);
  reportFrontendFailure('route_error', 'render', failure);
  await expect(invoke('saveAppSettings')).rejects.toBe(failure);
  await flush();
  expect(reports()).toHaveLength(2);
  expect(reports()[0].requestId).not.toBe(reports()[1].requestId);
});
test('a broken logging bridge cannot replace the original failure or recurse', async () => {
  const failure = new Error('synthetic-private-original');
  const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
  boundary.invoke.mockImplementation((command) => {
    throw new Error(
      command === 'log_frontend_failure'
        ? 'synthetic-private-logging'
        : failure.message,
    );
  });
  await expect(invoke('getAppSettings')).rejects.toThrow(
    'synthetic-private-original',
  );
  await flush();
  expect(reports()).toHaveLength(1);
  expect(warn).toHaveBeenCalledWith(
    'frontend_failure_logging_unavailable',
    expect.objectContaining({ operation: 'getAppSettings', stage: 'invoke' }),
  );
  expect(JSON.stringify(warn.mock.calls)).not.toContain('synthetic-private');
});
test('unknown codes and operation names never become diagnostic content', async () => {
  reportFrontendFailure(
    'synthetic-private-operation',
    'execute',
    { code: 'synthetic-private-code', message: 'synthetic-private-message' },
    { deduplicate: false },
  );
  await flush();
  expect(reports()[0]).toMatchObject({
    operation: 'frontend_action',
    errorCode: null,
    errorKind: 'other',
  });
  expect(JSON.stringify(reports())).not.toContain('synthetic-private');
});
test('hostile error metadata cannot replace the original invocation failure', async () => {
  const error = Object.defineProperty({}, 'code', {
    get: () => {
      throw new Error('synthetic-private-getter');
    },
  });
  const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
  boundary.invoke.mockRejectedValue(error);
  await expect(invoke('getAppSettings')).rejects.toBe(error);
  expect(warn).toHaveBeenCalledWith('frontend_failure_metadata_unavailable');
  expect(JSON.stringify(warn.mock.calls)).not.toContain('synthetic-private');
});
test('event subscription and consumer failures are logged without swallowing either', async () => {
  const subscriptionFailure = new Error('synthetic-private-subscription');
  boundary.listen.mockRejectedValueOnce(subscriptionFailure);
  await expect(listen('recall-state-changed', () => undefined)).rejects.toBe(
    subscriptionFailure,
  );
  let receive:
    | ((event: { event: string; id: number; payload: unknown }) => void)
    | undefined;
  const unlisten = vi.fn();
  boundary.listen.mockImplementation(async (_event, callback) => {
    receive = callback;
    return unlisten;
  });
  const consumerFailure = new Error('synthetic-private-consumer');
  await listen('recall-state-changed', () => {
    throw consumerFailure;
  });
  expect(() =>
    receive?.({
      event: 'recall-state-changed',
      id: 1,
      payload: { text: 'synthetic-private-event' },
    }),
  ).toThrow(consumerFailure);
  await flush();
  expect(reports().map((report) => report.stage)).toEqual([
    'subscribe',
    'callback',
  ]);
  expect(JSON.stringify(reports())).not.toContain('synthetic-private');
});
