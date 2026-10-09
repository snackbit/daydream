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
afterEach(cleanupCaptureFixture);
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
  expect(client.startRecording).toHaveBeenCalledTimes(1);
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
