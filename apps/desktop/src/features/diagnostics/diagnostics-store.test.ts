import { beforeEach, expect, test, vi } from 'vitest';
import { useDiagnosticsStore } from './diagnostics-store';

const client = vi.hoisted(() => ({
  getDiagnostics: vi.fn(),
}));

vi.mock('./diagnostics-client', () => client);

beforeEach(() => {
  vi.resetAllMocks();
  useDiagnosticsStore.setState(useDiagnosticsStore.getInitialState(), true);
});

test('load records diagnostics failures and rejects for error boundaries', async () => {
  client.getDiagnostics.mockRejectedValue(new Error('diagnostics unavailable'));

  await expect(useDiagnosticsStore.getState().load()).rejects.toThrow(
    'diagnostics unavailable',
  );

  expect(useDiagnosticsStore.getState()).toMatchObject({
    error: 'diagnostics unavailable',
    status: 'error',
  });
});
