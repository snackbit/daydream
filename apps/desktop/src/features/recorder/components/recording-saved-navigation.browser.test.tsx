import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Link,
  Outlet,
  RouterProvider,
} from '@tanstack/react-router';
import type { RecordingSaved } from '@daydream/shared';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, afterEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import '@/index.css';
import {
  readyStatus,
  resetCaptureFixture,
  cleanupCaptureFixture,
} from '@/screens/capture-browser-fixture';
import * as recorderClient from '../recorder-client';
import { useRecorderWorkspaceStore } from '../recorder-workspace-store';
import { RecordingSavedNavigation } from './recording-saved-navigation';

vi.mock('../recorder-client');
vi.mock('../source-capture-client');
const client = vi.mocked(recorderClient);
let saved: ((event: RecordingSaved) => void) | null;
beforeEach(async () => {
  await resetCaptureFixture();
  saved = null;
  client.listenToRecordingSaved.mockImplementation(async (handler) => {
    saved = handler;
    return () => {
      saved = null;
    };
  });
  const status = {
    ...readyStatus,
    state: 'recording' as const,
    activeRecordingId: 'synthetic-recording',
    updatedAtMs: 10,
  };
  useRecorderWorkspaceStore.setState({ status });
  client.stopRecording.mockResolvedValue({
    ...status,
    state: 'stopping',
    updatedAtMs: 11,
  });
});
afterEach(() => {
  cleanup();
  cleanupCaptureFixture();
});
function Capture() {
  return (
    <>
      <h1>Capture test</h1>
      <button
        onClick={() => {
          void useRecorderWorkspaceStore.getState().stop();
        }}
      >
        Stop
      </button>
      <Link to="/search" search={{ q: '', source: 'all', date: null }}>
        Elsewhere
      </Link>
    </>
  );
}
function setup() {
  const root = createRootRoute({
    component: () => (
      <>
        <RecordingSavedNavigation />
        <Outlet />
      </>
    ),
  });
  const capture = createRoute({
    getParentRoute: () => root,
    path: '/capture',
    component: Capture,
  });
  const elsewhere = createRoute({
    getParentRoute: () => root,
    path: '/search',
    component: () => <h1>Elsewhere test</h1>,
  });
  const studio = createRoute({
    getParentRoute: () => root,
    path: '/library/$recordingId',
    component: () => <h1>Studio test</h1>,
  });
  const router = createRouter({
    routeTree: root.addChildren([capture, elsewhere, studio]),
    history: createMemoryHistory({ initialEntries: ['/capture'] }),
  });
  render(<RouterProvider router={router} />);
  return router;
}
test('Stop opens Studio only after durable saved completion, once', async () => {
  const router = setup();
  await userEvent.click(await screen.findByRole('button', { name: 'Stop' }));
  await waitFor(() => expect(client.stopRecording).toHaveBeenCalledTimes(1));
  expect(screen.getByRole('heading', { name: 'Capture test' })).toBeVisible();
  const navigate = vi.spyOn(router, 'navigate');
  act(() => saved?.({ recordingId: 'synthetic-recording', savedAtMs: 12 }));
  await screen.findByRole('heading', { name: 'Studio test' });
  act(() => saved?.({ recordingId: 'synthetic-recording', savedAtMs: 12 }));
  expect(navigate).toHaveBeenCalledTimes(1);
});
test('Deliberate navigation while saving offers an Open recording notification', async () => {
  setup();
  await userEvent.click(await screen.findByRole('button', { name: 'Stop' }));
  await userEvent.click(screen.getByRole('link', { name: 'Elsewhere' }));
  await screen.findByRole('heading', { name: 'Elsewhere test' });
  act(() => saved?.({ recordingId: 'synthetic-recording', savedAtMs: 12 }));
  expect(screen.getByRole('heading', { name: 'Elsewhere test' })).toBeVisible();
  await userEvent.click(
    await screen.findByRole('link', { name: 'Open recording' }),
  );
  await screen.findByRole('heading', { name: 'Studio test' });
});
test('Failed finalization does not navigate or announce a saved recording', async () => {
  client.stopRecording.mockResolvedValue({
    ...readyStatus,
    updatedAtMs: 12,
    lastError: {
      category: 'filesystem',
      code: 'synthetic_finalization_failed',
      message: 'Synthetic finalization failed',
    },
  });
  setup();
  await userEvent.click(await screen.findByRole('button', { name: 'Stop' }));
  await waitFor(() =>
    expect(useRecorderWorkspaceStore.getState().status?.lastError?.code).toBe(
      'synthetic_finalization_failed',
    ),
  );
  expect(screen.getByRole('heading', { name: 'Capture test' })).toBeVisible();
  expect(screen.queryByRole('link', { name: 'Open recording' })).toBeNull();
});
