import {
  createMemoryHistory,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { TooltipProvider } from '@/components/ui/tooltip';
import { useNotificationsStore } from '@/features/notifications/notifications-store';
import { recorderWorkspaceInitialState } from '@/features/recorder/recorder-workspace-initial-state';
import { useRecorderWorkspaceStore } from '@/features/recorder/recorder-workspace-store';
import { useSettingsStore } from '@/features/settings/settings-store';
import '@/index.css';
import { router as applicationRouter } from '@/routes';
import { createRecallJourneyBackend } from './recall-journey-backend';

const native = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  handlers: new Map<string, Set<(event: { payload: unknown }) => void>>(),
}));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: native.invoke,
  convertFileSrc: (path: string) => `asset://${path}`,
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: native.listen }));
let backend: ReturnType<typeof createRecallJourneyBackend>;
let routeErrors: ReturnType<typeof vi.spyOn>;

beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  routeErrors = vi.spyOn(console, 'error');
  native.handlers.clear();
  vi.setSystemTime(new Date('2026-10-07T12:00:00'));
  backend = createRecallJourneyBackend();
  native.invoke.mockImplementation(backend.invoke);
  native.listen.mockImplementation(
    async (name: string, handler: (event: { payload: unknown }) => void) => {
      const listeners = native.handlers.get(name) ?? new Set();
      listeners.add(handler);
      native.handlers.set(name, listeners);
      return () => listeners.delete(handler);
    },
  );
  useSettingsStore.setState({
    draft: null,
    settingsState: null,
    savedScene: null,
    loadStatus: 'idle',
    isDirty: false,
    error: null,
  });
  useRecorderWorkspaceStore.setState(recorderWorkspaceInitialState);
  useNotificationsStore.setState({
    status: 'idle',
    notifications: [],
    error: null,
  });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  const lazyFailures = routeErrors.mock.calls.filter((args: unknown[]) =>
    args.some(
      (value) =>
        (typeof value === 'string' &&
          value.includes('This library called use() to suspend')) ||
        (value instanceof Error &&
          value.message.includes('This library called use() to suspend')),
    ),
  );
  vi.restoreAllMocks();
  expect(lazyFailures).toEqual([]);
});
async function open(path = '/day/2026-10-05') {
  const router = createRouter({
    routeTree: applicationRouter.routeTree,
    history: createMemoryHistory({ initialEntries: [path] }),
  });
  await router.load();
  await act(async () => {
    render(
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>,
    );
  });
  if (path.startsWith('/day/')) await openDayReview();
  return router;
}
async function openDayReview() {
  await userEvent.click(
    await screen.findByRole('button', { name: 'Day review' }),
  );
}

test('REC-01 session corrections survive real-route reloads and save recovery', async () => {
  const router = await open();
  await expect
    .element(page.getByRole('button', { name: 'Correct this session' }))
    .toBeVisible();
  await userEvent.click(
    screen.getByRole('button', { name: 'Correct this session' }),
  );
  await userEvent.fill(
    screen.getByLabelText('Session title'),
    'Keyboard planning',
  );
  await userEvent.fill(screen.getByLabelText('Session category'), 'Planning');
  await userEvent.fill(
    screen.getByLabelText('Local correction'),
    'The synthetic meeting concerned keyboard behavior.',
  );
  backend.state.failures.add('setRecallSessionCorrection');
  await userEvent.click(
    screen.getByRole('button', { name: 'Save correction' }),
  );
  await screen.findByRole('alert');
  expect(screen.getByLabelText('Session title')).toHaveValue(
    'Keyboard planning',
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Save correction' }),
  );
  await screen.findByRole('heading', { name: 'Keyboard planning' });
  expect(native.invoke).toHaveBeenCalledWith('setRecallSessionCorrection', {
    input: {
      dayId: '2026-10-05',
      sessionId: 'session-1',
      titleCorrection: 'Keyboard planning',
      categoryCorrection: 'Planning',
      correction: 'The synthetic meeting concerned keyboard behavior.',
    },
  });
  expect(screen.queryByLabelText('Feedback or correction')).toBeNull();
  await act(async () => router.navigate({ to: '/library' }));
  await screen.findByRole('link', { name: /today/i });
  await userEvent.click(
    await screen.findByRole('link', { name: /open.*october 5/i }),
  );
  await openDayReview();
  await screen.findByRole('heading', { name: 'Keyboard planning' });
  expect(screen.getByText('Planning')).toBeTruthy();
  expect(screen.queryByLabelText('Feedback or correction')).toBeNull();
  await userEvent.click(
    screen.getByRole('button', { name: 'Correct this session' }),
  );
  await userEvent.fill(
    screen.getByLabelText('Session title'),
    'Accessibility research',
  );
  await userEvent.fill(screen.getByLabelText('Session category'), 'Research');
  await userEvent.fill(screen.getByLabelText('Local correction'), '');
  await userEvent.click(
    screen.getByRole('button', { name: 'Save correction' }),
  );
  await screen.findByRole('heading', { name: 'Accessibility research' });
  expect(screen.queryByText(/Your correction:/)).toBeNull();
});

test('REC-02 accepted tasks edit, complete, reopen, dismiss and withhold without losing decisions', async () => {
  const router = await open();
  await userEvent.click(
    await screen.findByRole('button', { name: 'Accept as local task' }),
  );
  await userEvent.click(await screen.findByRole('button', { name: 'Edit' }));
  await userEvent.fill(
    screen.getByLabelText('Task title'),
    'Unsaved local draft',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Edit' }));
  expect(screen.queryByLabelText('Task title')).toBeNull();
  expect(
    screen.getByRole('heading', { name: 'Review focus behavior' }),
  ).toBeTruthy();
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === 'updateRecallTask',
    ),
  ).toBe(false);
  await userEvent.click(screen.getByRole('button', { name: 'Edit' }));
  await userEvent.fill(
    screen.getByLabelText('Task title'),
    'Check keyboard behavior',
  );
  await userEvent.fill(
    screen.getByLabelText('Notes'),
    'Local synthetic action note',
  );
  backend.state.failures.add('updateRecallTask');
  await userEvent.click(screen.getByRole('button', { name: 'Save task' }));
  await screen.findByRole('alert');
  expect(screen.getByLabelText('Notes')).toHaveValue(
    'Local synthetic action note',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Save task' }));
  await screen.findByRole('heading', { name: 'Check keyboard behavior' });
  await userEvent.click(screen.getByRole('button', { name: 'Mark done' }));
  await screen.findByText('done');
  await userEvent.click(screen.getByRole('button', { name: 'Reopen' }));
  await screen.findByText('open');
  await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
  await screen.findByText('dismissed');
  await act(async () => router.navigate({ to: '/library' }));
  await userEvent.click(await screen.findByRole('link', { name: /open day/i }));
  await openDayReview();
  await screen.findByRole('heading', { name: 'Check keyboard behavior' });
  expect(screen.getByRole('button', { name: 'Dismiss' })).toBeDisabled();
  backend.state.day.privacyRevision = 2;
  backend.state.day.day.privacyRevision = 2;
  backend.state.status.privacyRevision = 2;
  const savedTasks = structuredClone(backend.state.day.tasks);
  backend.state.day.day.status = 'invalidated';
  backend.state.day.day.summary = null;
  backend.state.day.sessions = [];
  backend.state.day.suggestions = [];
  backend.state.day.tasks = backend.state.day.tasks.map((task) => ({
    ...task,
    title: '',
    notes: '',
    sourceWithheld: true,
    citations: [],
  }));
  act(() =>
    native.handlers
      .get('recall-state-changed')
      ?.forEach((handler) =>
        handler({ payload: { dayId: '2026-10-05', privacyRevision: 2 } }),
      ),
  );
  await openDayReview();
  await screen.findByText(/source content withheld after a privacy change/i);
  expect(screen.queryByText('Check keyboard behavior')).toBeNull();
  expect(screen.getByText('dismissed')).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
  backend.state.day.privacyRevision = 3;
  backend.state.day.day.privacyRevision = 3;
  backend.state.status.privacyRevision = 3;
  backend.state.day.day.status = 'partial';
  backend.state.day.tasks = savedTasks;
  act(() =>
    native.handlers
      .get('recall-state-changed')
      ?.forEach((handler) =>
        handler({ payload: { dayId: '2026-10-05', privacyRevision: 3 } }),
      ),
  );
  await openDayReview();
  await screen.findByRole('heading', { name: 'Check keyboard behavior' });
  expect(screen.getByText('Local synthetic action note')).toBeTruthy();
  expect(screen.getByText('dismissed')).toBeTruthy();
});

test('REC-03 generation blockers, explicit regeneration and review/unreview transitions remain visible', async () => {
  backend.state.status.generationReady = false;
  await open();
  await screen.findByRole('link', { name: 'Summary settings' });
  expect(screen.queryByRole('button', { name: 'Generate summary' })).toBeNull();
  expect(
    screen.getByRole('link', { name: 'Summary settings' }).getAttribute('href'),
  ).toContain('focus=recall-generation');
  backend.state.status.generationReady = true;
  act(() =>
    native.handlers
      .get('recall-state-changed')
      ?.forEach((handler) =>
        handler({ payload: { dayId: '2026-10-05', privacyRevision: 1 } }),
      ),
  );
  await waitFor(
    () =>
      expect(
        screen.getByRole('button', { name: 'Generate summary' }),
      ).not.toBeDisabled(),
    { timeout: 2500 },
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Generate summary' }),
  );
  await waitFor(() =>
    expect(native.invoke).toHaveBeenCalledWith('regenerateRecallDay', {
      input: expect.objectContaining({ dayId: '2026-10-05' }),
    }),
  );
  await userEvent.click(screen.getByRole('button', { name: 'Mark reviewed' }));
  await userEvent.click(
    await screen.findByRole('button', { name: 'Mark unreviewed' }),
  );
  await screen.findByRole('button', { name: 'Mark reviewed' });
  backend.state.day.day.evidenceCount = 0;
  backend.state.day.day.summary = null;
  backend.state.day.day.status = 'empty';
  act(() =>
    native.handlers
      .get('recall-state-changed')
      ?.forEach((handler) =>
        handler({ payload: { dayId: '2026-10-05', privacyRevision: 1 } }),
      ),
  );
  await screen.findByText(
    'Recorded content is not available for a summary yet.',
  );
  expect(screen.queryByRole('button', { name: 'Generate summary' })).toBeNull();
});

test('NAV-01 root redirects to Days and actual shell navigation preserves a Search query across history', async () => {
  const router = await open('/');
  await screen.findByRole('link', { name: /today/i });
  expect(router.state.location.pathname).toBe('/library');
  await userEvent.click(await screen.findByRole('link', { name: /open day/i }));
  await openDayReview();
  await screen.findByRole('button', { name: 'Mark reviewed' });
  expect(router.state.location.pathname).toBe('/day/2026-10-05');
  await userEvent.keyboard('{Escape}');
  await waitFor(() =>
    expect(screen.queryByRole('dialog', { name: 'Day review' })).toBeNull(),
  );
  await userEvent.click(screen.getByRole('link', { name: 'Search' }));
  await userEvent.fill(
    await screen.findByLabelText('Find a moment'),
    'keyboard planning',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Search' }));
  await screen.findByRole('link', { name: /screen text/i });
  expect(
    screen.queryByText('Synthetic keyboard accessibility research notes.'),
  ).toBeNull();
  expect(router.state.location.search.q).toBe('keyboard planning');
  await userEvent.click(screen.getByRole('link', { name: 'Library' }));
  await screen.findByRole('link', { name: /today/i });
  await act(async () => {
    router.history.back();
    await router.load();
  });
  await screen.findByRole('link', { name: /screen text/i });
  expect(
    screen.queryByText('Synthetic keyboard accessibility research notes.'),
  ).toBeNull();
  expect(screen.getByLabelText('Find a moment')).toHaveValue(
    'keyboard planning',
  );
  expect(native.invoke).toHaveBeenCalledWith('searchRecallEvidence', {
    input: expect.objectContaining({ query: 'keyboard planning' }),
  });
});

test('NAV-02 disabled debug routes to General Settings without reading diagnostics', async () => {
  const router = await open('/diagnostics');
  await screen.findByText('Diagnostics are hidden until Debug is enabled.');
  expect(screen.queryByRole('link', { name: 'Diagnostics' })).toBeNull();
  await userEvent.click(screen.getByRole('link', { name: 'Open Settings' }));
  await expect
    .element(page.getByRole('button', { name: 'General', exact: true }))
    .toBeVisible();
  expect(screen.getByRole('button', { name: 'General' })).toHaveAttribute(
    'aria-current',
    'page',
  );
  expect(router.state.location.pathname).toBe('/settings');
  expect(
    native.invoke.mock.calls.some(([command]) => command === 'getDiagnostics'),
  ).toBe(false);
});

test('SRCH-04 previous results and mode changes reset the actual route query pagination safely', async () => {
  backend.state.search.nextCursor = 'page-two';
  backend.state.searchNext.hits = backend.state.search.hits.map(
    ({ evidence }) => ({
      evidence: {
        ...evidence,
        evidenceId: 'evidence-2',
        documentId: 'document-2',
        sourceFrameRef: 'frame-2',
        text: 'Synthetic second-page source wording.',
      },
      score: 0.8,
    }),
  );
  await open('/search?q=keyboard');
  await screen.findByRole('link', { name: /screen text/i });
  expect(
    screen.queryByText('Synthetic keyboard accessibility research notes.'),
  ).toBeNull();
  expect(
    screen.getByRole('button', { name: 'Previous results' }),
  ).toBeDisabled();
  await userEvent.click(screen.getByRole('button', { name: 'Next results' }));
  await screen.findByText('Page 2');
  expect(
    screen.getByRole('link', { name: /screen text/i }).getAttribute('href'),
  ).toContain('evidenceId=evidence-2');
  expect(
    screen.queryByText('Synthetic second-page source wording.'),
  ).toBeNull();
  expect(screen.getByText('Page 2')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Next results' })).toBeDisabled();
  await userEvent.click(
    screen.getByRole('button', { name: 'Previous results' }),
  );
  await screen.findByRole('link', { name: /screen text/i });
  expect(
    screen.queryByText('Synthetic keyboard accessibility research notes.'),
  ).toBeNull();
  expect(screen.getByText('Page 1')).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Next results' }));
  await screen.findByText('Page 2');
  expect(
    screen.getByRole('link', { name: /screen text/i }).getAttribute('href'),
  ).toContain('evidenceId=evidence-2');
  expect(
    screen.queryByText('Synthetic second-page source wording.'),
  ).toBeNull();
  await userEvent.click(screen.getByRole('combobox', { name: 'Match' }));
  await userEvent.click(screen.getByRole('option', { name: 'Literal words' }));
  await screen.findByRole('link', { name: /screen text/i });
  expect(
    screen.queryByText('Synthetic keyboard accessibility research notes.'),
  ).toBeNull();
  expect(screen.getByText('Page 1')).toBeTruthy();
  expect(native.invoke).toHaveBeenCalledWith('searchRecallEvidence', {
    input: expect.objectContaining({
      query: 'keyboard',
      mode: 'lexical',
      cursor: null,
    }),
  });
});

test('SRCH-05 stale pagination recovery restarts on the first page after privacy invalidation', async () => {
  backend.state.search.nextCursor = 'page-two';
  await open('/search?q=keyboard');
  await screen.findByText('Page 1');
  await userEvent.click(screen.getByRole('button', { name: 'Next results' }));
  await screen.findByText('Page 2');
  const invoke = backend.invoke;
  native.invoke.mockImplementation(async (command: string, args: unknown) => {
    if (
      command === 'searchRecallEvidence' &&
      typeof args === 'object' &&
      args !== null &&
      'input' in args
    ) {
      const input = args.input;
      if (
        typeof input === 'object' &&
        input !== null &&
        'cursor' in input &&
        input.cursor !== null
      )
        throw new Error('Recall cursor is stale; restart the query');
    }
    return invoke(command, args);
  });
  backend.state.search.privacyRevision = 2;
  backend.state.status.privacyRevision = 2;
  await act(async () => {
    native.handlers
      .get('recall-state-changed')
      ?.forEach((handler) =>
        handler({ payload: { dayId: null, privacyRevision: 2 } }),
      );
  });
  await screen.findByRole('alert');
  await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
  await expect.element(page.getByText('Page 1', { exact: true })).toBeVisible();
  expect(
    screen.getByRole('button', { name: 'Previous results' }),
  ).toBeDisabled();
});
