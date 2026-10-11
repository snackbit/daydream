import { useState } from 'react';
import type { AppSettings } from '@daydream/shared';
import {
  cleanup,
  act,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { userEvent, page } from 'vitest/browser';
import '@/index.css';
import { defaultSettingsState } from '../testing/settings-browser-fixtures';
import { emptyRegistry } from './model-registry';
import { ModelSelector } from './model-selector';
import { ModelsSection } from './models-section';
import { SubscriptionControls } from './subscription-controls';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => native);
beforeEach(async () => {
  await page.viewport(1280, 960);
  native.invoke.mockReset().mockImplementation(async (command: string) => {
    if (command === 'test_configured_model')
      return {
        fingerprint: 'synthetic',
        checks: [
          {
            capability: 'text',
            status: 'succeeded',
            message: 'Synthetic test passed.',
            elapsedMs: 12,
            dimensions: null,
          },
        ],
      };
    if (command === 'cancel_model_request') return;
    if (command === 'log_frontend_failure') return;
    throw new Error('Unexpected boundary');
  });
});
afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});
function Setup() {
  const [draft, setDraft] = useState<AppSettings>({
    ...defaultSettingsState.settings,
    ai: { ...defaultSettingsState.settings.ai, modelRegistry: emptyRegistry() },
  });
  const [keys, setKeys] = useState<Record<string, string>>({});
  const update = (updater: (draft: AppSettings) => AppSettings) =>
    setDraft(updater);
  return (
    <>
      <ModelsSection
        draft={draft}
        keyDrafts={keys}
        onKeyChange={(id, key) =>
          setKeys((current) => ({ ...current, [id]: key }))
        }
        onUpdateDraft={update}
      />
      <ModelSelector
        draft={draft}
        feature="summary"
        capability="text"
        label="Summary model"
        onUpdateDraft={update}
      />
      <ModelSelector
        draft={draft}
        feature="embedding"
        capability="embedding"
        label="Embedding model"
        onUpdateDraft={update}
      />
      <p role="status">
        {draft.ai.recall.generation.enabled ? 'Summaries on' : 'Summaries off'}
      </p>
    </>
  );
}
test('adds and tests an unsaved model, reuses it for summaries, and keeps remote consent off', async () => {
  render(<Setup />);
  await userEvent.click(screen.getByRole('button', { name: 'Add connection' }));
  await userEvent.fill(
    screen.getByLabelText('Connection name'),
    'Synthetic OpenAI',
  );
  await userEvent.fill(screen.getByLabelText('API key'), 'synthetic-key');
  await userEvent.click(screen.getByRole('button', { name: 'Add model' }));
  await userEvent.fill(
    screen.getByLabelText('Model label'),
    'Synthetic reasoning model',
  );
  await userEvent.fill(
    screen.getByLabelText('Model identifier'),
    'synthetic-reasoning',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Test model' }));
  await screen.findByText('text: Synthetic test passed.');
  expect(native.invoke).toHaveBeenCalledWith('test_configured_model', {
    input: expect.objectContaining({
      keyDraft: 'synthetic-key',
      model: expect.objectContaining({ model: 'synthetic-reasoning' }),
      connection: expect.objectContaining({ protocol: 'responses' }),
    }),
  });
  await userEvent.click(screen.getByLabelText('Summary model'));
  await userEvent.click(
    screen.getByRole('option', {
      name: 'Synthetic reasoning model · Synthetic OpenAI',
    }),
  );
  await waitFor(() =>
    expect(screen.getByRole('status').textContent).toBe('Summaries off'),
  );
  expect(screen.getByRole('button', { name: 'Remove model' })).toBeDisabled();
  await userEvent.click(screen.getByLabelText('Embedding model'));
  expect(
    screen.queryByRole('option', {
      name: 'Synthetic reasoning model · Synthetic OpenAI',
    }),
  ).toBeNull();
  await userEvent.keyboard('{Escape}');
});
test('a failed capability test is logged and a retry replaces the visible failure', async () => {
  let attempts = 0;
  native.invoke.mockImplementation(async (command: string) => {
    if (command !== 'test_configured_model') return undefined;
    attempts++;
    return {
      fingerprint: 'synthetic',
      checks: [
        {
          capability: 'text',
          status: attempts === 1 ? 'failed' : 'succeeded',
          message:
            attempts === 1
              ? 'Synthetic validation failed. Check Diagnostics.'
              : 'Synthetic test passed.',
          errorCode: attempts === 1 ? 'model_response_invalid' : null,
          elapsedMs: 12,
          dimensions: null,
        },
      ],
    };
  });
  render(<Setup />);
  await userEvent.click(screen.getByRole('button', { name: 'Add connection' }));
  await userEvent.click(screen.getByRole('button', { name: 'Add model' }));
  await userEvent.fill(
    screen.getByLabelText('Model identifier'),
    'synthetic-model',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Test model' }));
  await screen.findByText(
    'text: Synthetic validation failed. Check Diagnostics.',
  );
  await waitFor(() =>
    expect(native.invoke).toHaveBeenCalledWith('log_frontend_failure', {
      input: expect.objectContaining({
        operation: 'test_configured_model',
        stage: 'execute',
        errorCode: 'model_response_invalid',
        requestId: expect.any(String),
      }),
    }),
  );
  await userEvent.click(screen.getByRole('button', { name: 'Test model' }));
  await screen.findByText('text: Synthetic test passed.');
  expect(screen.queryByText(/Synthetic validation failed/)).toBeNull();
  expect(screen.getByText('Summaries off')).toBeVisible();
});
test('an IPC rejection remains visible and retry succeeds with a correlated diagnostic', async () => {
  let attempts = 0;
  native.invoke.mockImplementation(async (command: string) => {
    if (command !== 'test_configured_model') return undefined;
    attempts++;
    if (attempts === 1) throw 'model_timeout';
    return {
      fingerprint: 'synthetic',
      checks: [
        {
          capability: 'text',
          status: 'succeeded',
          message: 'Synthetic test passed.',
          elapsedMs: 12,
          dimensions: null,
        },
      ],
    };
  });
  render(<Setup />);
  await userEvent.click(screen.getByRole('button', { name: 'Add connection' }));
  await userEvent.click(screen.getByRole('button', { name: 'Add model' }));
  await userEvent.fill(
    screen.getByLabelText('Model identifier'),
    'synthetic-model',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Test model' }));
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'The request timed out. Try again.',
  );
  const request = native.invoke.mock.calls.find(
    ([command]) => command === 'test_configured_model',
  )?.[1].input.requestId;
  await waitFor(() =>
    expect(native.invoke).toHaveBeenCalledWith('log_frontend_failure', {
      input: expect.objectContaining({
        requestId: request,
        stage: 'invoke',
        errorCode: 'model_timeout',
      }),
    }),
  );
  await userEvent.click(screen.getByRole('button', { name: 'Test model' }));
  await screen.findByText('text: Synthetic test passed.');
  expect(screen.queryByRole('alert')).toBeNull();
});
test('malformed native results identify the decoder and retain the request for diagnostics', async () => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'test_configured_model')
      return {
        fingerprint: 123,
        privateText: 'synthetic-private-native-data',
        checks: [],
      };
    return undefined;
  });
  render(<Setup />);
  await userEvent.click(screen.getByRole('button', { name: 'Add connection' }));
  await userEvent.click(screen.getByRole('button', { name: 'Add model' }));
  await userEvent.fill(
    screen.getByLabelText('Model identifier'),
    'synthetic-model',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Test model' }));
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Check Diagnostics',
  );
  const requestId = native.invoke.mock.calls.find(
    ([command]) => command === 'test_configured_model',
  )?.[1].input.requestId;
  await waitFor(() =>
    expect(native.invoke).toHaveBeenCalledWith('log_frontend_failure', {
      input: expect.objectContaining({
        operation: 'test_configured_model',
        stage: 'decode',
        requestId,
        errorCode: 'model_response_invalid',
      }),
    }),
  );
  expect(
    JSON.stringify(
      native.invoke.mock.calls.filter(
        ([command]) => command === 'log_frontend_failure',
      ),
    ),
  ).not.toContain('synthetic-private');
});
test('rejects late test results after the model changes and cancels the native owner', async () => {
  let finish!: (value: unknown) => void;
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'test_configured_model')
      return new Promise((resolve) => {
        finish = resolve;
      });
    return undefined;
  });
  render(<Setup />);
  await userEvent.click(screen.getByRole('button', { name: 'Add connection' }));
  await userEvent.click(screen.getByRole('button', { name: 'Add model' }));
  await userEvent.fill(
    screen.getByLabelText('Model identifier'),
    'synthetic-before',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Test model' }));
  await userEvent.fill(
    screen.getByLabelText('Model identifier'),
    'synthetic-after',
  );
  finish({
    fingerprint: 'old',
    checks: [
      {
        capability: 'text',
        status: 'succeeded',
        message: 'Stale success',
        elapsedMs: 1,
        dimensions: null,
      },
    ],
  });
  await waitFor(() =>
    expect(native.invoke).toHaveBeenCalledWith(
      'cancel_model_request',
      expect.anything(),
    ),
  );
  expect(screen.queryByText(/Stale success/)).toBeNull();
});
test('subscription sign-in stays independent from feature consent and excludes embeddings', async () => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'prepare_chatgpt_connection')
      return '00000000-0000-4000-8000-000000000001';
    if (command === 'sign_in_chatgpt' || command === 'get_chatgpt_connection')
      return {
        id: '00000000-0000-4000-8000-000000000001',
        label: 'Synthetic account',
        status: 'connected',
        models: [
          {
            slug: 'synthetic-subscription',
            displayName: 'Synthetic subscription model',
          },
        ],
      };
    return undefined;
  });
  render(<Setup />);
  await userEvent.click(screen.getByRole('button', { name: 'Add connection' }));
  await userEvent.click(screen.getByLabelText('Authentication'));
  await userEvent.click(
    screen.getByRole('option', { name: 'ChatGPT subscription' }),
  );
  await userEvent.click(
    await screen.findByRole('button', { name: 'Sign in with ChatGPT' }),
  );
  await screen.findByText('Connected as Synthetic account');
  await userEvent.click(
    screen.getByRole('button', { name: 'Add Synthetic subscription model' }),
  );
  expect(screen.getByLabelText('Embeddings')).toBeDisabled();
  expect(screen.getByText('Summaries off')).toBeTruthy();
  expect(screen.queryByLabelText('API key')).toBeNull();
});
test.each(['state', 'error'])(
  'a late initial account %s cannot overwrite successful sign-in',
  async (outcome) => {
    const id = '00000000-0000-4000-8000-000000000001';
    let finishRead!: (value: unknown) => void;
    let failRead!: (error: unknown) => void;
    native.invoke.mockImplementation(async (command: string) => {
      if (command === 'get_chatgpt_connection')
        return new Promise((resolve, reject) => {
          finishRead = resolve;
          failRead = reject;
        });
      if (command === 'sign_in_chatgpt')
        return {
          id,
          label: 'Synthetic account',
          status: 'connected',
          models: [],
        };
      return undefined;
    });
    render(
      <SubscriptionControls
        id={id}
        onRegistered={vi.fn()}
        onConnected={vi.fn()}
        onAddModel={vi.fn()}
      />,
    );
    await waitFor(() => expect(finishRead).toBeTypeOf('function'));
    await userEvent.click(
      screen.getByRole('button', { name: 'Sign in with ChatGPT' }),
    );
    await screen.findByText('Connected as Synthetic account');
    await act(async () => {
      if (outcome === 'state')
        finishRead({
          id,
          label: 'ChatGPT',
          status: 'reconnect-required',
          models: [],
        });
      else failRead('model_subscription_reconnect');
    });
    expect(screen.getByText('Connected as Synthetic account')).toBeVisible();
    expect(screen.queryByRole('alert')).toBeNull();
  },
);
test.each(['light', 'dark'] as const)(
  'model setup keeps its controls within a narrow %s viewport',
  async (theme) => {
    await page.viewport(390, 844);
    document.documentElement.dataset.theme = theme;
    render(<Setup />);
    await userEvent.click(
      screen.getByRole('button', { name: 'Add connection' }),
    );
    await userEvent.click(screen.getByRole('button', { name: 'Add model' }));
    const section = screen.getByRole('group', {
      name: 'Connection New connection',
    });
    expect(section.getBoundingClientRect().right).toBeLessThanOrEqual(390);
    expect(within(section).getByLabelText('Model identifier')).toBeVisible();
    await page.screenshot({
      path: `./__screenshots__/models-narrow-${theme}.png`,
    });
  },
);
test('retains a registration after failed sign-in and disconnects before changing authentication', async () => {
  const id = '00000000-0000-4000-8000-000000000001';
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'prepare_chatgpt_connection') return id;
    if (command === 'sign_in_chatgpt')
      throw 'model_subscription_sign_in_denied';
    if (command === 'get_chatgpt_connection')
      return { id, label: 'ChatGPT', status: 'signed-out', models: [] };
    if (command === 'sign_out_chatgpt') return false;
    return undefined;
  });
  render(<Setup />);
  await userEvent.click(screen.getByRole('button', { name: 'Add connection' }));
  await userEvent.click(screen.getByLabelText('Authentication'));
  await userEvent.click(
    screen.getByRole('option', { name: 'ChatGPT subscription' }),
  );
  await userEvent.click(
    await screen.findByRole('button', { name: 'Sign in with ChatGPT' }),
  );
  await screen.findByText('Sign-in was declined. You can try again.');
  await userEvent.click(
    screen.getByRole('button', { name: 'Sign in with ChatGPT' }),
  );
  await waitFor(() =>
    expect(
      native.invoke.mock.calls.filter(
        ([command]) => command === 'sign_in_chatgpt',
      ),
    ).toHaveLength(2),
  );
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'prepare_chatgpt_connection',
    ),
  ).toHaveLength(1);
  expect(native.invoke).toHaveBeenCalledWith('sign_in_chatgpt', {
    subscriptionId: id,
    requestId: expect.any(String),
  });
  await screen.findByText('Sign-in was declined. You can try again.');
  await userEvent.click(screen.getByLabelText('Authentication'));
  await userEvent.click(screen.getByRole('option', { name: 'API key' }));
  await screen.findByLabelText('API key');
  expect(native.invoke).toHaveBeenCalledWith('sign_out_chatgpt', {
    subscriptionId: id,
  });
  expect(screen.getByRole('alert').textContent).toMatch(/Signed out locally/);
});
