import type { AiVisionProviderSettings } from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { VisionProviderTest } from './vision-provider-test';

const client = vi.hoisted(() => ({ testVisionProvider: vi.fn() }));
vi.mock('../../vision-provider-client', () => client);
const initial: AiVisionProviderSettings = {
  provider: 'openai-compatible',
  authMode: 'api-key',
  baseUrl: 'http://127.0.0.1:8080/v1',
  model: 'synthetic-vision',
  apiKeyRef: null,
  observationsEnabled: false,
};
beforeEach(() => {
  vi.resetAllMocks();
  client.testVisionProvider.mockResolvedValue({
    checks: [
      { kind: 'ocr', status: 'succeeded', errorCode: null, elapsedMs: 12 },
    ],
  });
});
afterEach(cleanup);
test('tests an unsaved Vision profile and displays its capability result', async () => {
  const vision = { ...initial, model: 'unsaved-model' };
  render(
    <VisionProviderTest
      vision={vision}
      keyDraft="synthetic-key"
      ocr
      observations={false}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Test Vision' }));
  await waitFor(() =>
    expect(client.testVisionProvider).toHaveBeenCalledWith({
      vision,
      keyDraft: 'synthetic-key',
      ocr: true,
      observations: false,
    }),
  );
  await screen.findByText(/Text recognition: Connected/);
});
test('anonymous provider tests omit a previously entered key draft', async () => {
  render(
    <VisionProviderTest
      vision={{ ...initial, authMode: 'none' }}
      keyDraft="obsolete-key"
      ocr
      observations={false}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Test Vision' }));
  await waitFor(() =>
    expect(client.testVisionProvider).toHaveBeenCalledWith(
      expect.objectContaining({
        keyDraft: null,
        vision: expect.objectContaining({ authMode: 'none' }),
      }),
    ),
  );
});
test('changing the profile during a test hides its stale response', async () => {
  let resolve: (value: unknown) => void = () => undefined;
  client.testVisionProvider.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const view = render(
    <VisionProviderTest
      vision={initial}
      keyDraft="synthetic-key"
      ocr
      observations={false}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Test Vision' }));
  view.rerender(
    <VisionProviderTest
      vision={{ ...initial, model: 'changed-model' }}
      keyDraft="synthetic-key"
      ocr
      observations={false}
    />,
  );
  resolve({
    checks: [
      { kind: 'ocr', status: 'succeeded', errorCode: null, elapsedMs: 12 },
    ],
  });
  await waitFor(() =>
    expect(
      screen.getByRole<HTMLButtonElement>('button', { name: 'Test Vision' })
        .disabled,
    ).toBe(false),
  );
  expect(screen.queryByText(/Connected/)).toBeNull();
});
