import {
  createBrowserHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
  useNavigate,
} from '@tanstack/react-router';
import type { AppSettings } from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import type { SettingsScreenTabsActions } from '../screen/settings-screen-tabs';
import { useSettingsStore } from '../settings-store';
import { FeatureEditor } from '../workspace/feature-editor';
import { defaultSettingsState } from './settings-browser-fixtures';

const save = vi.fn(async (_settings: AppSettings) => true);
const discard = vi.fn();
let originalUrl: string;
let originalState: unknown;
let browserHistory: ReturnType<typeof createBrowserHistory> | undefined;
const actions: SettingsScreenTabsActions = {
  cancelAiModelDownload: vi.fn(),
  changeCadenceMode: vi.fn(),
  changeTranscriptionKey: vi.fn(),
  changeVisionKey: vi.fn(),
  changeRecallEmbeddingKey: vi.fn(),
  changeRecallGenerationKey: vi.fn(),
  deleteAiModel: vi.fn(),
  downloadAiModel: vi.fn(),
  refreshInputActivityStatus: vi.fn(),
  repairInputActivity: vi.fn(),
  updateDraft: vi.fn(),
};
function EditorRoute() {
  const navigate = useNavigate();
  return (
    <FeatureEditor
      detail="processing"
      model={{
        draft: defaultSettingsState.settings,
        limits: defaultSettingsState.limits,
        aiModels: [],
        aiProviderStatus: null,
        cadenceMode: null,
        inputActivityRepairMessage: null,
        inputActivityRepairing: false,
        inputActivityStatus: null,
        savedScene: null,
        transcriptionKeyDraft: '',
        visionKeyDraft: '',
        recallEmbeddingKeyDraft: '',
        recallGenerationKeyDraft: '',
      }}
      actions={actions}
      onClose={() => {
        void navigate({ to: '/library' });
      }}
      onDiscard={discard}
      onSave={save}
    />
  );
}
function mount() {
  const root = createRootRoute({ component: Outlet });
  const settings = createRoute({
    getParentRoute: () => root,
    path: '/settings',
    component: EditorRoute,
  });
  const library = createRoute({
    getParentRoute: () => root,
    path: '/library',
    component: () => <h1>Library destination</h1>,
  });
  window.history.replaceState(null, '', '/library');
  const history = createBrowserHistory();
  browserHistory = history;
  history.push('/settings');
  history.flush();
  const router = createRouter({
    routeTree: root.addChildren([settings, library]),
    history,
  });
  render(<RouterProvider router={router} />);
  return { router, history };
}
beforeEach(() => {
  originalUrl = window.location.href;
  originalState = window.history.state;
  vi.clearAllMocks();
  useSettingsStore.setState({
    draft: defaultSettingsState.settings,
    settingsState: defaultSettingsState,
    isDirty: false,
  });
});
afterEach(() => {
  cleanup();
  browserHistory?.destroy();
  window.history.replaceState(originalState, '', originalUrl);
});

test('Back protects an edited feature; Keep editing retains it and Discard leaves without saving', async () => {
  const { router, history } = mount();
  await screen.findByRole('heading', { name: 'Processing' });
  await page.getByRole('spinbutton', { name: 'Parallel jobs' }).fill('2');
  await userEvent.keyboard('{Tab}');
  history.back();
  await screen.findByRole('alertdialog');
  await page.getByRole('button', { name: 'Keep editing' }).click();
  await waitFor(() => expect(window.location.pathname).toBe('/settings'));
  expect(router.state.location.pathname).toBe('/settings');
  expect(
    await screen.findByRole('spinbutton', { name: 'Parallel jobs' }),
  ).toHaveValue(2);
  history.back();
  await screen.findByRole('alertdialog');
  await page.getByRole('button', { name: 'Discard changes' }).click();
  await screen.findByRole('heading', { name: 'Library destination' });
  expect(save).not.toHaveBeenCalled();
  expect(discard).toHaveBeenCalledOnce();
});

test('Done commits the edited feature and leaves without a discard prompt', async () => {
  mount();
  await screen.findByRole('heading', { name: 'Processing' });
  await page.getByRole('spinbutton', { name: 'Parallel jobs' }).fill('2');
  await userEvent.keyboard('{Tab}');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await screen.findByRole('heading', { name: 'Library destination' });
  await waitFor(() => expect(save).toHaveBeenCalledOnce());
  expect(save.mock.calls[0]?.[0]).toMatchObject({
    ai: { processing: { maxParallelJobs: 2 } },
  });
  expect(screen.queryByRole('alertdialog')).toBeNull();
});

test.each([true, false])(
  'feature controls stay locked during a pending save (success: %s)',
  async (succeeded) => {
    const pending = Promise.withResolvers<boolean>();
    save.mockImplementationOnce(() => pending.promise);
    mount();
    await screen.findByRole('spinbutton', { name: 'Parallel jobs' });
    await page.getByRole('spinbutton', { name: 'Parallel jobs' }).fill('2');
    await userEvent.keyboard('{Tab}');
    await page.getByRole('button', { name: 'Done', exact: true }).click();
    await waitFor(() => expect(save).toHaveBeenCalledOnce());
    const jobs = screen.getByRole('spinbutton', { name: 'Parallel jobs' });
    try {
      expect(jobs).toBeDisabled();
      expect(screen.getByRole('button', { name: /^Cancel$/ })).toBeDisabled();
    } finally {
      pending.resolve(succeeded);
    }
    expect(save.mock.calls[0]?.[0].ai.processing.maxParallelJobs).toBe(2);
    if (succeeded) {
      await screen.findByRole('heading', { name: 'Library destination' });
    } else {
      await screen.findByRole('alert');
      expect(jobs).toBeEnabled();
      expect(jobs).toHaveValue(2);
      await page.getByRole('spinbutton', { name: 'Parallel jobs' }).fill('3');
      await userEvent.keyboard('{Tab}');
      await page.getByRole('button', { name: 'Done', exact: true }).click();
      await screen.findByRole('heading', { name: 'Library destination' });
      expect(save.mock.lastCall?.[0].ai.processing.maxParallelJobs).toBe(3);
    }
  },
);
