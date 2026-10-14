import {
  createRootRoute,
  createRoute,
  createRouter,
  createMemoryHistory,
  Link,
  Outlet,
  RouterProvider,
} from '@tanstack/react-router';
import type { CaptureSetup } from '@daydream/shared';
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import {
  chooseSelectOption,
  waitForSelectDismissal,
} from '@/components/ui/testing/select-interactions';
import { useAppearanceStore } from '@/features/appearance/appearance-store';
import { useRecorderWorkspaceStore } from '@/features/recorder/recorder-workspace-store';
import '@/index.css';
import { CaptureScreen } from './CaptureScreen';
import {
  cleanupCaptureFixture,
  client,
  deferred,
  resetCaptureFixture,
  savedScene,
  savedSetup,
  setupClient,
} from './capture-browser-fixture';

vi.mock('@/features/recorder/recorder-client');
vi.mock('@/features/recorder/source-capture-client');
beforeEach(resetCaptureFixture);
afterEach(() => {
  cleanupCaptureFixture();
  useAppearanceStore.getState().setMode('light');
});

function NavigationFixture() {
  return (
    <>
      <nav aria-label="Test navigation">
        <Link to="/capture">Capture</Link>
        <Link to="/settings">Settings</Link>
      </nav>
      <Outlet />
    </>
  );
}
function SettingsDestination() {
  return <h1>Settings destination</h1>;
}
function mountWithNavigation() {
  const root = createRootRoute({ component: NavigationFixture });
  const capture = createRoute({
    getParentRoute: () => root,
    path: '/capture',
    component: CaptureScreen,
  });
  const settings = createRoute({
    getParentRoute: () => root,
    path: '/settings',
    component: SettingsDestination,
  });
  return render(
    <RouterProvider
      router={createRouter({
        routeTree: root.addChildren([capture, settings]),
        history: createMemoryHistory({ initialEntries: ['/capture'] }),
      })}
    />,
  );
}

test.each([
  { theme: 'light', width: 1440 },
  { theme: 'dark', width: 960 },
] as const)(
  'pending cleanup and cancelled chooser preserve sources and navigation at $width in $theme',
  async ({ theme, width }) => {
    await page.viewport(width, 960);
    useAppearanceStore.getState().setMode(theme);
    const stop =
      deferred<Awaited<ReturnType<typeof client.stopRecorderPreview>>>();
    const selection =
      deferred<Awaited<ReturnType<typeof client.selectCaptureSources>>>();
    client.stopRecorderPreview.mockReturnValueOnce(stop.promise);
    client.selectCaptureSources.mockReturnValueOnce(selection.promise);
    mountWithNavigation();
    await screen.findByText('Preview: live');
    await userEvent.click(
      screen.getByRole('button', { name: 'Add monitor sources' }),
    );
    expect(
      screen.getByRole('button', { name: 'Add monitor sources' }),
    ).toBeDisabled();
    expect(
      screen.getByRole('button', { name: 'Focus Synthetic monitor' }),
    ).toBeTruthy();
    expect(client.selectCaptureSources).not.toHaveBeenCalled();
    await screen.findByText('Selecting monitor sources…');
    await page.screenshot({
      path: `__screenshots__/capture-pending-${theme}-${width}.png`,
    });
    await userEvent.click(screen.getByRole('link', { name: 'Settings' }));
    await screen.findByRole('heading', { name: 'Settings destination' });
    stop.resolve({
      error: null,
      sceneId: null,
      sourceIds: [],
      state: 'idle',
      updatedAtMs: 3,
    });
    await waitFor(() =>
      expect(client.selectCaptureSources).toHaveBeenCalledOnce(),
    );
    await userEvent.click(screen.getByRole('link', { name: 'Capture' }));
    await screen.findByText('Selecting monitor sources…');
    expect(client.startRecorderPreview).toHaveBeenCalledOnce();
    selection.reject(
      Object.assign(new Error('Synthetic picker cancellation'), {
        code: 'recorder_portal_permission_denied',
        category: 'permission',
        message: 'Source selection cancelled.',
      }),
    );
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Add monitor sources' }),
      ).toBeEnabled(),
    );
    expect(
      screen.getByRole('button', { name: 'Focus Synthetic monitor' }),
    ).toBeTruthy();
    expect(screen.queryByRole('alert')).toBeNull();
    await waitFor(() =>
      expect(client.startRecorderPreview).toHaveBeenCalledTimes(2),
    );
    useAppearanceStore.getState().setMode('light');
  },
);

test('deleting the last monitor and adding it again displays a decoded preview after pending startup', async () => {
  const selection =
    deferred<Awaited<ReturnType<typeof client.selectCaptureSources>>>();
  const preview =
    deferred<Awaited<ReturnType<typeof client.startRecorderPreview>>>();
  let publishFrame:
    Parameters<typeof client.listenToRecorderPreviewFrames>[0] | undefined;
  client.listenToRecorderPreviewFrames.mockImplementation(async (listener) => {
    publishFrame = listener;
    return () => undefined;
  });
  render(<CaptureScreen />);
  await screen.findByText('Preview: live');
  await userEvent.click(
    screen.getByRole('button', { name: 'Delete Synthetic monitor' }),
  );
  await screen.findByText('No monitor sources selected.');
  await waitFor(() =>
    expect(useRecorderWorkspaceStore.getState().previewStopInFlight).toBe(
      false,
    ),
  );
  client.selectCaptureSources.mockReturnValueOnce(selection.promise);
  client.startRecorderPreview.mockReturnValueOnce(preview.promise);
  await userEvent.click(
    screen.getByRole('button', { name: 'Add monitor sources' }),
  );
  await screen.findByText('Selecting monitor sources…');
  selection.resolve({
    sources: [
      {
        id: 'replacement-monitor',
        displayName: 'Replacement monitor',
        kind: 'monitor',
        width: 1440,
        height: 900,
        logicalWidth: 1440,
        logicalHeight: 900,
        scaleFactor: 1,
      },
    ],
  });
  await screen.findByRole('button', { name: 'Focus Replacement monitor' });
  await screen.findByText('Preview: starting');
  expect(client.startRecorderPreview).toHaveBeenCalledTimes(2);
  const source = useRecorderWorkspaceStore.getState().scene?.sources[0];
  if (!source) throw new Error('Expected the selected replacement source');
  preview.resolve({
    error: null,
    sceneId: savedScene.id,
    sourceIds: ['replacement-monitor'],
    state: 'live',
    updatedAtMs: 4,
  });
  await screen.findByText('Preview: live');
  publishFrame?.({
    sceneSourceId: source.id,
    sourceId: source.sourceId,
    width: 320,
    height: 180,
    timestampMs: 5,
    dataUrl:
      'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aZWQAAAAASUVORK5CYII=',
  });
  const image = await screen.findByRole('img', {
    name: 'Replacement monitor preview',
  });
  expect((image as HTMLImageElement).naturalWidth).toBeGreaterThan(0);
});

test('navigation during startup fences a late same-scene preview response', async () => {
  const obsolete =
    deferred<Awaited<ReturnType<typeof client.startRecorderPreview>>>();
  const current =
    deferred<Awaited<ReturnType<typeof client.startRecorderPreview>>>();
  client.startRecorderPreview
    .mockReturnValueOnce(obsolete.promise)
    .mockReturnValueOnce(current.promise);
  mountWithNavigation();
  await screen.findByText('Preview: starting');
  await userEvent.click(screen.getByRole('link', { name: 'Settings' }));
  await screen.findByRole('heading', { name: 'Settings destination' });
  await waitFor(() =>
    expect(client.stopRecorderPreview).toHaveBeenCalledOnce(),
  );
  await userEvent.click(screen.getByRole('link', { name: 'Capture' }));
  await waitFor(() =>
    expect(client.startRecorderPreview).toHaveBeenCalledTimes(2),
  );
  obsolete.resolve({
    error: null,
    sceneId: savedScene.id,
    sourceIds: ['obsolete'],
    state: 'live',
    updatedAtMs: 2,
  });
  await act(async () => {
    await obsolete.promise;
  });
  expect(screen.getByText('Preview: starting')).toBeTruthy();
  current.resolve({
    error: null,
    sceneId: savedScene.id,
    sourceIds: ['synthetic-monitor'],
    state: 'live',
    updatedAtMs: 4,
  });
  await screen.findByText('Preview: live');
});
test('dirty Start submits one reviewed snapshot and retains edits when persistence fails', async () => {
  const pending = deferred<Awaited<ReturnType<typeof client.startRecording>>>();
  client.startRecording.mockReturnValue(pending.promise);
  render(<CaptureScreen />);
  const bitrate = await screen.findByRole('spinbutton', {
    name: 'Video bitrate per source (kbps)',
  });
  await waitFor(() => expect(bitrate).toHaveValue(2000));
  await userEvent.fill(bitrate, '4500');
  await chooseSelectOption(
    screen.getByRole('combobox', { name: 'Output scale' }),
    '50%',
  );
  const start = screen.getByRole('button', { name: 'Start' });
  await userEvent.click(start);
  expect(start).toBeDisabled();
  fireEvent.click(start);
  await waitFor(() => expect(client.startRecording).toHaveBeenCalledTimes(1));
  expect(client.startRecording).toHaveBeenCalledWith(
    expect.objectContaining({
      setup: expect.objectContaining({ videoBitrateKbps: 4500, fps: 0.2 }),
    }),
  );
  expect(setupClient.saveCaptureSetup).not.toHaveBeenCalled();
  pending.reject(
    Object.assign(new Error('Synthetic setup persistence failure'), {
      category: 'filesystem',
      code: 'recorder_storage_error',
    }),
  );
  await screen.findByRole('alert');
  expect(bitrate).toHaveValue(4500);
  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(true);
  expect(start).toBeEnabled();
});
test('permission failure retains the setup saved by the native Start owner', async () => {
  let saved: ((setup: CaptureSetup) => void) | undefined;
  setupClient.listenToCaptureSetupSaved.mockImplementation(async (handler) => {
    saved = handler;
    return () => undefined;
  });
  client.startRecording.mockImplementation(async (input) => {
    if (!input.setup) throw new Error('Reviewed setup missing');
    act(() => saved?.(input.setup!));
    throw {
      category: 'permission',
      code: 'recorder_portal_permission_denied',
      message: 'Screen permission expired. Reselect monitor sources.',
      recoverable: true,
    };
  });
  render(<CaptureScreen />);
  const bitrate = await screen.findByRole('spinbutton', {
    name: 'Video bitrate per source (kbps)',
  });
  await waitFor(() => expect(bitrate).toHaveValue(2000));
  await userEvent.fill(bitrate, '4500');
  await chooseSelectOption(
    screen.getByRole('combobox', { name: 'Output scale' }),
    '50%',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Start' }));
  await screen.findByText(
    'Screen permission expired. Reselect monitor sources.',
  );
  expect(bitrate).toHaveValue(4500);
  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(false);
});

test('CAP-SETUP-01 typed crop and custom output survive save and a fresh workspace', async () => {
  const view = render(<CaptureScreen />);
  await chooseSelectOption(
    await screen.findByRole('combobox', { name: 'Capture area' }),
    'Region',
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Precision crop controls' }),
  );
  await userEvent.fill(
    screen.getByRole('spinbutton', { name: 'Crop width' }),
    '1000',
  );
  await userEvent.fill(
    screen.getByRole('spinbutton', { name: 'Crop height' }),
    '600',
  );
  await userEvent.fill(
    screen.getByRole('spinbutton', { name: 'Crop X' }),
    '100',
  );
  await userEvent.fill(
    screen.getByRole('spinbutton', { name: 'Crop Y' }),
    '50',
  );
  await chooseSelectOption(
    screen.getByRole('combobox', { name: 'Output scale' }),
    'Custom size',
  );
  await userEvent.fill(
    screen.getByRole('spinbutton', { name: 'Output width' }),
    '500',
  );
  await screen.findByText('Saved video: 500 × 300');
  await userEvent.fill(
    screen.getByRole('spinbutton', { name: 'Frame interval (seconds)' }),
    '2',
  );
  await userEvent.fill(
    screen.getByRole('spinbutton', { name: 'Video bitrate per source (kbps)' }),
    '4000',
  );
  expect(screen.getByRole('button', { name: 'Start' })).toBeEnabled();
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText('Source setup saved');
  const persisted = savedSetup();
  expect(persisted).toMatchObject({
    fps: 0.5,
    videoBitrateKbps: 4000,
    scene: {
      sources: [
        {
          crop: { left: 100, top: 50, right: 340, bottom: 250 },
          rect: { width: 500, height: 300 },
        },
      ],
    },
  });
  view.unmount();
  useRecorderWorkspaceStore.setState(
    useRecorderWorkspaceStore.getInitialState(),
    true,
  );
  client.getRecorderScene.mockResolvedValue(persisted.scene);
  setupClient.getCaptureSetup.mockResolvedValue(persisted);
  render(<CaptureScreen />);
  await screen.findByText('Saved video: 500 × 300');
  await userEvent.click(
    screen.getByRole('button', { name: 'Precision crop controls' }),
  );
  expect(screen.getByRole('spinbutton', { name: 'Crop X' })).toHaveValue(100);
  expect(
    screen.getByRole('spinbutton', { name: 'Frame interval (seconds)' }),
  ).toHaveValue(2);
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Start' })).toBeEnabled(),
  );
  await chooseSelectOption(
    screen.getByRole('combobox', { name: 'Capture area' }),
    'Full source',
  );
  expect(screen.queryByRole('spinbutton', { name: 'Crop X' })).toBeNull();
  await userEvent.click(
    screen.getByRole('button', { name: 'Reset crop and scale' }),
  );
  await screen.findByText('Saved video: 1440 × 900');
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText('Source setup saved');
  expect(savedSetup().scene?.sources[0]).toMatchObject({
    crop: { left: 0, top: 0, right: 0, bottom: 0 },
    rect: { width: 1440, height: 900 },
  });
});

test('CAP-SETUP-02 failed save retains edits, locks duplicate submission and recovers on retry', async () => {
  const pending =
    deferred<Awaited<ReturnType<typeof setupClient.saveCaptureSetup>>>();
  setupClient.saveCaptureSetup.mockReturnValueOnce(pending.promise);
  render(<CaptureScreen />);
  const bitrate = await screen.findByRole('spinbutton', {
    name: 'Video bitrate per source (kbps)',
  });
  await waitFor(() => expect(bitrate).toHaveValue(2000));
  await userEvent.fill(bitrate, '4500');
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  expect(bitrate).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Start' })).toBeDisabled();
  expect(setupClient.saveCaptureSetup).toHaveBeenCalledTimes(1);
  pending.reject(new Error('Synthetic settings disk is full'));
  await screen.findByRole('alert');
  expect(bitrate).toHaveValue(4500);
  expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled();
  expect(screen.getByRole('button', { name: 'Start' })).toBeEnabled();
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText('Source setup saved');
  expect(screen.queryByRole('alert')).toBeNull();
  expect(savedSetup().videoBitrateKbps).toBe(4500);
  expect(setupClient.saveCaptureSetup).toHaveBeenCalledTimes(2);
});

test('CAP-SETUP-03 settings load failure has a visible retry that restores editable defaults', async () => {
  setupClient.getCaptureSetup.mockRejectedValueOnce(
    new Error('Synthetic settings unavailable'),
  );
  render(<CaptureScreen />);
  await screen.findByRole('alert');
  expect(
    screen.getByRole('spinbutton', { name: 'Frame interval (seconds)' }),
  ).toBeDisabled();
  await userEvent.click(
    screen.getByRole('button', { name: 'Retry recording settings' }),
  );
  await waitFor(() =>
    expect(
      screen.getByRole('spinbutton', { name: 'Frame interval (seconds)' }),
    ).toHaveValue(5),
  );
  expect(screen.queryByRole('alert')).toBeNull();
  await userEvent.fill(
    screen.getByRole('spinbutton', { name: 'Frame interval (seconds)' }),
    '10',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText('Source setup saved');
  expect(savedSetup().fps).toBe(0.1);
});

test('CAP-SETUP-04 narrow keyboard output sheet restores focus and saves source edits', async () => {
  await page.viewport(768, 960);
  render(<CaptureScreen />);
  const trigger = await screen.findByRole('button', { name: 'Source output' });
  trigger.focus();
  await userEvent.keyboard('{Enter}');
  await screen.findByRole('dialog', { name: 'Source output settings' });
  const scale = screen.getByRole('combobox', { name: 'Output scale' });
  await userEvent.click(scale);
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole('option', { name: '100%' }),
    ),
  );
  await userEvent.keyboard('{ArrowDown}');
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole('option', { name: '50%' }),
    ),
  );
  await userEvent.keyboard('{Enter}');
  await waitForSelectDismissal(scale);
  await screen.findByText('Saved video: 720 × 450');
  await userEvent.keyboard('{Escape}');
  await waitFor(() => expect(document.activeElement).toBe(trigger));
  expect(screen.queryByRole('dialog')).toBeNull();
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText('Source setup saved');
  expect(savedSetup().scene?.sources[0]?.rect).toMatchObject({
    width: 720,
    height: 450,
  });
});

test('CAP-SETUP-05 switching selected monitors keeps each output independent and deleting selected falls back', async () => {
  const scene = structuredClone(savedScene);
  const first = scene.sources[0];
  if (!first) throw new Error('Expected synthetic source');
  scene.sources.push({
    ...first,
    id: 'synthetic-second-source',
    sourceId: 'synthetic-second-monitor',
    displayName: 'Second synthetic monitor',
    zIndex: 1,
  });
  client.getRecorderScene.mockResolvedValue(scene);
  client.getCaptureSources.mockResolvedValue({ sources: [] });
  render(<CaptureScreen />);
  await chooseSelectOption(
    await screen.findByRole('combobox', { name: 'Output scale' }),
    '50%',
  );
  await screen.findByText('Saved video: 720 × 450');
  await userEvent.click(
    screen.getByRole('button', { name: 'Focus Second synthetic monitor' }),
  );
  expect(
    screen.getByRole('button', { name: 'Focus Second synthetic monitor' }),
  ).toHaveAttribute('aria-pressed', 'true');
  await screen.findByText('Saved video: 1440 × 900');
  await chooseSelectOption(
    screen.getByRole('combobox', { name: 'Output scale' }),
    'Custom size',
  );
  await userEvent.fill(
    screen.getByRole('spinbutton', { name: 'Output width' }),
    '960',
  );
  await screen.findByText('Saved video: 960 × 600');
  await userEvent.click(
    screen.getByRole('button', { name: 'Focus Synthetic monitor' }),
  );
  await screen.findByText('Saved video: 720 × 450');
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText('Source setup saved');
  expect(
    savedSetup().scene?.sources.map((source) => source.rect.width),
  ).toEqual([720, 960]);
  await userEvent.click(
    screen.getByRole('button', { name: 'Delete Synthetic monitor' }),
  );
  await screen.findByText('Saved video: 960 × 600');
  expect(
    screen.getByRole('button', { name: 'Focus Second synthetic monitor' }),
  ).toHaveAttribute('aria-pressed', 'true');
  expect(
    screen.queryByRole('button', { name: 'Focus Synthetic monitor' }),
  ).toBeNull();
});
