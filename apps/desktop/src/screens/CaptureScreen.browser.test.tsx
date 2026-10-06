import type {
  AudioSourceList,
  CaptureSourceList,
  RecorderMediaWriterCapabilities,
  RecorderPreviewFrame,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import { useRecorderWorkspaceStore } from '@/features/recorder/recorder-workspace-store';
import '@/index.css';
import { CaptureScreen } from './CaptureScreen';

const client = vi.hoisted(() => ({
  getAudioSources: vi.fn(),
  getCaptureSources: vi.fn(),
  getRecorderPreviewStatus: vi.fn(),
  getRecorderRecordingStats: vi.fn(),
  getRecorderScene: vi.fn(),
  getRecorderStatus: vi.fn(),
  listenToRecorderPreviewFrames: vi.fn(),
  listenToRecorderStatusChanged: vi.fn(),
  pauseRecording: vi.fn(),
  resumeRecording: vi.fn(),
  saveRecorderScene: vi.fn(),
  selectCaptureSources: vi.fn(),
  startRecorderPreview: vi.fn(),
  startRecording: vi.fn(),
  stopRecorderPreview: vi.fn(),
  stopRecording: vi.fn(),
}));

vi.mock('@/features/recorder/recorder-client', () => client);
const setupClient = vi.hoisted(() => ({
  getCaptureSetup: vi.fn(),
  saveCaptureSetup: vi.fn(),
}));
vi.mock('@/features/recorder/source-capture-client', () => setupClient);

const unavailableMediaWriterCapabilities = {
  appsrc: false,
  audioConvert: false,
  audioResample: false,
  fileSink: false,
  h264Parse: false,
  matroskaMuxer: false,
  opusEncoder: false,
  pulseSource: false,
  queue: false,
  videoConvert: false,
  x264Encoder: false,
} satisfies RecorderMediaWriterCapabilities;

const status: RecorderStatus = {
  activeRecordingId: null,
  activeSceneId: null,
  activeSegmentId: null,
  backendHealth: {
    desktopAudio: {
      available: true,
      defaultSourceId: 'desktop',
      error: null,
    },
    mediaWriter: {
      available: false,
      capabilities: unavailableMediaWriterCapabilities,
      error: 'Media writer is not wired yet.',
      version: null,
      writer: null,
    },
    gpuCapture: {
      available: false,
      cleanupError: null,
      dmabufImportAvailable: false,
      eglAvailable: false,
      eglClientApis: [],
      eglPlatform: null,
      eglVendor: null,
      eglVersion: null,
      error: null,
      modifierImportAvailable: null,
      openglAvailable: false,
      renderer: null,
      vendor: null,
      version: null,
    },
    microphone: {
      available: true,
      defaultSourceId: 'mic',
      error: null,
    },
    pipewire: {
      available: true,
      error: null,
    },
    portal: {
      available: true,
      cursorModes: ['embedded'],
      error: null,
      screenCastVersion: 5,
      sourceTypes: ['monitor'],
    },
  },
  finalization: null,
  lastError: null,
  state: 'idle',
  updatedAtMs: 1,
};

const captureSources: CaptureSourceList = {
  sources: [
    {
      displayName: 'Monitor 1',
      height: null,
      id: 'main',
      kind: 'monitor',
      logicalHeight: 900,
      logicalWidth: 1440,
      scaleFactor: null,
      width: null,
    },
    {
      displayName: 'Monitor 2',
      height: null,
      id: 'secondary',
      kind: 'monitor',
      logicalHeight: 900,
      logicalWidth: 1440,
      scaleFactor: null,
      width: null,
    },
  ],
};

const audioSources: AudioSourceList = {
  sources: [
    {
      displayName: 'Desktop monitor',
      id: 'desktop',
      isDefault: true,
      kind: 'monitor',
      roles: ['desktop-audio'],
    },
    {
      displayName: 'Microphone',
      id: 'mic',
      isDefault: true,
      kind: 'input',
      roles: ['microphone'],
    },
  ],
};

beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  setupClient.getCaptureSetup.mockResolvedValue({
    scene: null,
    fps: 0.2,
    videoBitrateKbps: 2000,
  });
  setupClient.saveCaptureSetup.mockImplementation(async (input) => ({
    ...input,
    scene: { ...input.scene, independentVideo: true },
  }));
  useRecorderWorkspaceStore.setState(
    useRecorderWorkspaceStore.getInitialState(),
    true,
  );
  client.getRecorderStatus.mockResolvedValue(status);
  client.getRecorderPreviewStatus.mockResolvedValue({
    error: null,
    sceneId: null,
    sourceIds: [],
    state: 'idle',
    updatedAtMs: 1,
  });
  client.getRecorderRecordingStats.mockResolvedValue(null);
  client.getCaptureSources.mockResolvedValue(captureSources);
  client.getRecorderScene.mockResolvedValue(null);
  client.getAudioSources.mockResolvedValue(audioSources);
  client.listenToRecorderPreviewFrames.mockResolvedValue(vi.fn());
  client.listenToRecorderStatusChanged.mockResolvedValue(vi.fn());
  client.selectCaptureSources.mockResolvedValue(captureSources);
  client.startRecorderPreview.mockImplementation(({ sceneId }) =>
    Promise.resolve({
      error: null,
      sceneId,
      sourceIds: ['main'],
      state: 'live',
      updatedAtMs: 2,
    }),
  );
  client.stopRecorderPreview.mockResolvedValue({
    error: null,
    sceneId: null,
    sourceIds: [],
    state: 'idle',
    updatedAtMs: 3,
  });
});

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});

test.each([
  ['light', 1440],
  ['dark', 1440],
  ['light', 420],
  ['dark', 420],
] as const)(
  'source crop controls remain reachable in %s at %i pixels',
  async (theme, width) => {
    await page.viewport(width, 900);
    document.documentElement.dataset.theme = theme;
    client.getRecorderScene.mockResolvedValue(
      sceneFixture(['main', 'secondary']),
    );
    const view = render(<CaptureScreen />);
    const scale = await screen.findByRole('combobox', { name: 'Output scale' });
    const crop = screen.getByRole('combobox', { name: 'Capture area' });
    fireEvent.click(crop);
    fireEvent.click(await screen.findByRole('option', { name: 'Region' }));
    const x = await screen.findByRole('spinbutton', { name: 'Crop X' });
    fireEvent.change(screen.getByRole('spinbutton', { name: 'Crop width' }), {
      target: { value: '1000' },
    });
    fireEvent.change(x, { target: { value: '20' } });
    for (const control of [
      scale,
      crop,
      x,
      screen.getByRole('button', { name: 'Save' }),
      screen.getByRole('spinbutton', { name: 'Frame interval (seconds)' }),
    ]) {
      const bounds = control.getBoundingClientRect();
      expect(bounds.width).toBeGreaterThan(0);
      expect(bounds.left).toBeGreaterThanOrEqual(0);
      expect(bounds.right).toBeLessThanOrEqual(width);
    }
    expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(width);
    await page.screenshot({ element: view.container });
    if (width === 420) {
      const output = scale.closest('aside');
      const shared = screen.getByRole('region', {
        name: 'Shared recording settings',
      });
      if (!output) throw new Error('Source output controls are not visible');
      await page.screenshot({ element: output });
      await page.screenshot({ element: shared });
    }
  },
);

async function addFirstAvailableAudioSource() {
  fireEvent.click(screen.getByRole('button', { name: /add audio source/i }));
  const dialog = await screen.findByRole('dialog', {
    name: /add audio source/i,
  });
  fireEvent.click(within(dialog).getByRole('button', { name: /^add$/i }));
  await waitFor(() =>
    expect(
      screen.queryByRole('dialog', { name: /add audio source/i }),
    ).toBeNull(),
  );
}

test('CaptureScreen reviews selected sources and saves independent output settings explicitly', async () => {
  render(<CaptureScreen />);
  await screen.findAllByText('Monitor 1');
  expect(screen.queryByLabelText(/canvas preset/i)).toBeNull();
  expect(screen.getByText(/Review the source crops/)).toBeTruthy();
  await waitFor(() =>
    expect(screen.getByLabelText('Frame interval (seconds)')).toHaveValue(5),
  );
  expect(setupClient.saveCaptureSetup).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));
  await waitFor(() =>
    expect(setupClient.saveCaptureSetup).toHaveBeenCalledWith(
      expect.objectContaining({
        fps: 0.2,
        videoBitrateKbps: 2000,
        scene: expect.objectContaining({
          independentVideo: true,
          layoutMode: 'custom',
        }),
      }),
    ),
  );
});

test('CaptureScreen explains the media writer blocker for a saved scene', async () => {
  client.getRecorderScene.mockResolvedValue(sceneFixture(['saved-main']));

  render(<CaptureScreen />);

  expect(
    await screen.findByText('Media writer is not wired yet.'),
  ).toBeTruthy();
  expect(
    (screen.getByRole('button', { name: /^start$/i }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
});

test('CaptureScreen asks before refreshing a duplicate-looking monitor', async () => {
  render(<CaptureScreen />);

  await screen.findAllByText('Monitor 1');
  client.selectCaptureSources.mockResolvedValue({
    sources: [
      {
        displayName: 'Monitor 1',
        height: null,
        id: 'main-new-portal-id',
        kind: 'monitor',
        logicalHeight: 900,
        logicalWidth: 1440,
        scaleFactor: null,
        width: null,
      },
    ],
  });

  fireEvent.click(screen.getByRole('button', { name: /add monitor sources/i }));

  const dialog = await screen.findByRole('dialog', {
    name: /duplicate monitor/i,
  });
  expect(within(dialog).getByText(/looks like Monitor 1/i)).toBeTruthy();
  expect(
    within(dialog).queryByRole('button', { name: /add anyway/i }),
  ).toBeNull();

  fireEvent.click(
    within(dialog).getByRole('button', { name: /refresh existing/i }),
  );

  await waitFor(() =>
    expect(
      screen.queryByRole('dialog', { name: /duplicate monitor/i }),
    ).toBeNull(),
  );
  expect(useRecorderWorkspaceStore.getState().scene?.sources).toHaveLength(2);
  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.sourceId).toBe(
    'main-new-portal-id',
  );
});

test('CaptureScreen saves shared frame interval and quality without canvas controls', async () => {
  client.getRecorderScene.mockResolvedValue(sceneFixture(['main']));
  render(<CaptureScreen />);
  const interval = await screen.findByLabelText('Frame interval (seconds)');
  await waitFor(() => expect(interval).toHaveValue(5));
  fireEvent.change(interval, { target: { value: '10' } });
  fireEvent.change(screen.getByLabelText('Video bitrate per source (kbps)'), {
    target: { value: '4000' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));
  await waitFor(() =>
    expect(setupClient.saveCaptureSetup).toHaveBeenCalledWith(
      expect.objectContaining({ fps: 0.1, videoBitrateKbps: 4000 }),
    ),
  );
  expect(screen.queryByLabelText(/canvas preset/i)).toBeNull();
});

test('CaptureScreen records an ultrawide source at half its width and height', async () => {
  const wide = sceneFixture(['main']);
  const source = sceneSource(wide);
  source.sourceWidth = 5120;
  source.sourceHeight = 1440;
  source.rect = { x: 0, y: 0, width: 5120, height: 1440 };
  client.getRecorderScene.mockResolvedValue(wide);
  render(<CaptureScreen />);
  fireEvent.click(
    await screen.findByRole('combobox', { name: 'Output scale' }),
  );
  fireEvent.click(await screen.findByRole('option', { name: '50%' }));
  await screen.findByText('Saved video: 2560 × 720');
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));
  await waitFor(() =>
    expect(setupClient.saveCaptureSetup).toHaveBeenCalledWith(
      expect.objectContaining({
        scene: expect.objectContaining({
          sources: [
            expect.objectContaining({
              rect: { x: 0, y: 0, width: 2560, height: 720 },
            }),
          ],
        }),
      }),
    ),
  );
});

test('CaptureScreen crops the right half of an ultrawide source in physical pixels', async () => {
  const wide = sceneFixture(['main']);
  const source = sceneSource(wide);
  source.sourceWidth = 5120;
  source.sourceHeight = 1440;
  source.rect = { x: 0, y: 0, width: 5120, height: 1440 };
  client.getRecorderScene.mockResolvedValue(wide);
  render(<CaptureScreen />);
  fireEvent.click(
    await screen.findByRole('combobox', { name: 'Capture area' }),
  );
  fireEvent.click(await screen.findByRole('option', { name: 'Region' }));
  fireEvent.change(screen.getByLabelText('Crop width'), {
    target: { value: '2560' },
  });
  fireEvent.change(screen.getByLabelText('Crop X'), {
    target: { value: '2560' },
  });
  await screen.findByText('Saved video: 2560 × 1440');
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));
  await waitFor(() =>
    expect(setupClient.saveCaptureSetup).toHaveBeenCalledWith(
      expect.objectContaining({
        scene: expect.objectContaining({
          sources: [
            expect.objectContaining({
              crop: { left: 2560, top: 0, right: 0, bottom: 0 },
              rect: { x: 0, y: 0, width: 2560, height: 1440 },
            }),
          ],
        }),
      }),
    ),
  );
});

test('CaptureScreen keeps an invalid crop editable and restores valid settings through reset', async () => {
  client.getRecorderScene.mockResolvedValue(sceneFixture(['main']));
  render(<CaptureScreen />);
  fireEvent.click(
    await screen.findByRole('combobox', { name: 'Capture area' }),
  );
  fireEvent.click(await screen.findByRole('option', { name: 'Region' }));
  fireEvent.change(screen.getByLabelText('Crop X'), {
    target: { value: '2000' },
  });
  await screen.findByRole('alert');
  expect(
    screen.getByRole<HTMLButtonElement>('button', { name: /^save$/i }).disabled,
  ).toBe(true);
  fireEvent.click(screen.getByRole('button', { name: 'Reset crop and scale' }));
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
  expect(
    screen.getByRole<HTMLButtonElement>('button', { name: /^save$/i }).disabled,
  ).toBe(false);
});

test('CaptureScreen renders a saved scene on startup without adding sources', async () => {
  const savedScene = sceneFixture(['saved-main']);
  const savedSource = sceneSource(savedScene);
  savedSource.displayName = 'Saved monitor';
  savedSource.rect.x = 24;
  client.getRecorderScene.mockResolvedValue(savedScene);

  render(<CaptureScreen />);

  expect(await screen.findAllByText('Saved monitor')).toHaveLength(2);
  expect(client.selectCaptureSources).not.toHaveBeenCalled();
  expect(screen.getByText('Source setup saved')).toBeTruthy();
});

test('CaptureScreen shows stale screen permission error without opening source picker on start', async () => {
  const savedScene = sceneFixture(['saved-main']);
  client.getRecorderScene.mockResolvedValue(savedScene);
  client.getRecorderStatus.mockResolvedValue({
    ...status,
    backendHealth: {
      ...status.backendHealth,
      mediaWriter: {
        ...status.backendHealth.mediaWriter,
        available: true,
        error: null,
      },
    },
  });
  client.startRecording.mockRejectedValue({
    category: 'permission',
    code: 'recorder_portal_restore_token_invalid',
    message: 'Screen permission expired. Reselect monitor sources.',
  });

  render(<CaptureScreen />);

  const startButton = await screen.findByRole('button', { name: /^start$/i });
  await waitFor(() =>
    expect((startButton as HTMLButtonElement).disabled).toBe(false),
  );
  fireEvent.click(startButton);

  await screen.findByText(
    'Screen permission expired. Reselect monitor sources.',
  );
  expect(client.startRecording).toHaveBeenCalledWith(
    expect.objectContaining({ sceneId: savedScene.id }),
  );
  expect(client.selectCaptureSources).not.toHaveBeenCalled();
});

test('CaptureScreen renders live preview frames for a saved scene', async () => {
  const savedScene = sceneFixture(['main']);
  let previewFrameHandler: ((frame: RecorderPreviewFrame) => void) | undefined;
  client.getRecorderScene.mockResolvedValue(savedScene);
  client.startRecorderPreview.mockResolvedValue({
    error: null,
    sceneId: savedScene.id,
    sourceIds: ['main'],
    state: 'live',
    updatedAtMs: 2,
  });
  client.listenToRecorderPreviewFrames.mockImplementation((handler) => {
    previewFrameHandler = handler;
    return Promise.resolve(vi.fn());
  });

  render(<CaptureScreen />);

  await waitFor(() =>
    expect(client.startRecorderPreview).toHaveBeenCalledWith({
      sceneId: savedScene.id,
    }),
  );
  previewFrameHandler?.({
    dataUrl: 'data:image/png;base64,abc',
    height: 180,
    sceneSourceId: savedScene.sources[0]?.id ?? '',
    sourceId: 'main',
    timestampMs: 3,
    width: 320,
  });

  const preview = await screen.findByRole('img', {
    name: /main preview/i,
  });
  expect(preview.getAttribute('src')).toBe('data:image/png;base64,abc');
  expect(screen.getByLabelText('Capture region')).toBeTruthy();
});

test('CaptureScreen does not auto-preview an empty saved scene', async () => {
  client.getRecorderScene.mockResolvedValue({
    ...sceneFixture([]),
    canvasWidth: 1440,
  });

  render(<CaptureScreen />);

  expect(await screen.findByText('No monitor sources selected.')).toBeTruthy();
  expect(client.startRecorderPreview).not.toHaveBeenCalled();
});

test('CaptureScreen deleting the last monitor and saving does not restart preview', async () => {
  const savedScene = sceneFixture(['main']);
  const emptyScene: RecorderScene = {
    ...savedScene,
    sources: [],
  };
  client.getRecorderScene.mockResolvedValue(savedScene);

  render(<CaptureScreen />);

  await waitFor(() =>
    expect(client.startRecorderPreview).toHaveBeenCalledTimes(1),
  );

  fireEvent.click(screen.getByRole('button', { name: /delete main/i }));

  await waitFor(() => expect(client.stopRecorderPreview).toHaveBeenCalled());
  expect(screen.getByText('No monitor sources selected.')).toBeTruthy();

  client.saveRecorderScene.mockResolvedValue(emptyScene);
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(setupClient.saveCaptureSetup).toHaveBeenCalledWith(
      expect.objectContaining({
        scene: expect.objectContaining({
          sources: [],
        }),
      }),
    ),
  );
  expect(client.startRecorderPreview).toHaveBeenCalledTimes(1);
});

test('CaptureScreen auto-preview after stop starts one backend request while preview is in flight', async () => {
  const savedScene = sceneFixture(['main']);
  let statusHandler: ((status: RecorderStatus) => void) | undefined;
  const recordingStatus: RecorderStatus = {
    ...status,
    activeRecordingId: 'recording-1',
    activeSceneId: savedScene.id,
    activeSegmentId: 'segment-1',
    state: 'recording',
  };
  const preview =
    deferred<Awaited<ReturnType<typeof client.startRecorderPreview>>>();
  client.getRecorderScene.mockResolvedValue(savedScene);
  client.getRecorderStatus.mockResolvedValue(recordingStatus);
  client.listenToRecorderStatusChanged.mockImplementation((handler) => {
    statusHandler = handler;

    return Promise.resolve(vi.fn());
  });
  client.startRecorderPreview.mockReturnValue(preview.promise);

  render(<CaptureScreen />);

  await waitFor(() => expect(statusHandler).toBeDefined());
  expect(client.startRecorderPreview).not.toHaveBeenCalled();

  statusHandler?.({
    ...status,
    updatedAtMs: 10,
  });

  await waitFor(() =>
    expect(client.startRecorderPreview).toHaveBeenCalledTimes(1),
  );
  await waitFor(() =>
    expect(screen.getByText('Preview: starting')).toBeTruthy(),
  );
  expect(client.startRecorderPreview).toHaveBeenCalledTimes(1);

  preview.resolve({
    error: null,
    sceneId: savedScene.id,
    sourceIds: ['main'],
    state: 'live',
    updatedAtMs: 11,
  });

  await waitFor(() => expect(screen.getByText('Preview: live')).toBeTruthy());
  expect(client.startRecorderPreview).toHaveBeenCalledTimes(1);
});

test('CaptureScreen makes scene configuration read-only while recording', async () => {
  const savedScene = sceneFixture(['main']);
  savedScene.desktopAudioSourceId = 'desktop';
  savedScene.microphoneAudioSourceId = 'mic';
  const recordingStatus: RecorderStatus = {
    ...status,
    activeRecordingId: 'recording-1',
    activeSceneId: savedScene.id,
    activeSegmentId: 'segment-1',
    state: 'recording',
  };
  client.getRecorderScene.mockResolvedValue(savedScene);
  client.getRecorderStatus.mockResolvedValue(recordingStatus);
  client.getRecorderRecordingStats.mockResolvedValue({
    activeSegmentId: 'segment-1',
    activeSegmentIndex: 2,
    activeSegmentSizeBytes: 2_048,
    activeSegmentStartedAtMs: 1_000,
    recordingId: 'recording-1',
    recordingStartedAtMs: 500,
    segmentCount: 2,
    totalSizeBytes: 1_048_576,
    updatedAtMs: 66_000,
  });

  render(<CaptureScreen />);

  expect(await screen.findByText('Recording in progress')).toBeTruthy();
  expect(screen.getByText('Status: recording')).toBeTruthy();
  expect(screen.queryByTestId('scene-source-main')).toBeNull();
  expect(screen.queryByRole('button', { name: /move main/i })).toBeNull();
  expect(screen.queryAllByRole('button', { name: /resize/i })).toHaveLength(0);
  expect(screen.getByText('2 of 2')).toBeTruthy();
  expect(screen.getByText('1:05')).toBeTruthy();
  expect(screen.getByText('2.05 KB')).toBeTruthy();
  expect(screen.getByText('1.05 MB')).toBeTruthy();
  expect(
    screen.getByText(
      new Intl.DateTimeFormat(undefined, {
        dateStyle: 'medium',
        timeStyle: 'short',
      }).format(new Date(500)),
    ),
  ).toBeTruthy();
  expect(
    (screen.getByRole('button', { name: /^pause$/i }) as HTMLButtonElement)
      .disabled,
  ).toBe(false);
  expect(
    (screen.getByRole('button', { name: /^stop$/i }) as HTMLButtonElement)
      .disabled,
  ).toBe(false);
  expect(
    (screen.getByRole('button', { name: /^save$/i }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  expect(
    (
      screen.getByRole('button', {
        name: /add monitor sources/i,
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  expect(
    (
      screen.getByRole('button', {
        name: /add audio source/i,
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  expect(
    (screen.getByRole('button', { name: /delete main/i }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  expect(
    (
      screen.getByRole('button', {
        name: /delete desktop audio/i,
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  expect(
    (
      screen.getByRole('button', {
        name: /delete microphone audio/i,
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  expect(screen.queryByLabelText(/canvas preset/i)).toBeNull();
  expect(screen.queryByLabelText(/canvas w/i)).toBeNull();
  expect(
    screen.getByRole<HTMLButtonElement>('combobox', { name: 'Output scale' })
      .disabled,
  ).toBe(true);
  expect(
    screen.getByRole<HTMLButtonElement>('button', {
      name: 'Reset crop and scale',
    }).disabled,
  ).toBe(true);

  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));
  fireEvent.click(screen.getByRole('button', { name: /add monitor sources/i }));

  expect(setupClient.saveCaptureSetup).not.toHaveBeenCalled();
  expect(client.selectCaptureSources).not.toHaveBeenCalled();
});

test('CaptureScreen reconciles a protected-window pause on return without discarding the scene', async () => {
  const savedScene = sceneFixture(['main']);
  const recordingStatus = {
    ...status,
    activeRecordingId: 'recording-1',
    activeSceneId: savedScene.id,
    state: 'recording',
  } satisfies RecorderStatus;
  client.getRecorderScene.mockResolvedValue(savedScene);
  client.getRecorderStatus.mockResolvedValue(recordingStatus);
  const first = render(<CaptureScreen />);
  await waitFor(() =>
    expect(
      (screen.getByRole('button', { name: /^Pause$/i }) as HTMLButtonElement)
        .disabled,
    ).toBe(false),
  );
  first.unmount();
  client.getRecorderStatus.mockResolvedValue({
    ...recordingStatus,
    state: 'paused',
    updatedAtMs: 5,
  });
  render(<CaptureScreen />);
  await waitFor(() =>
    expect(
      (screen.getByRole('button', { name: /^Resume$/i }) as HTMLButtonElement)
        .disabled,
    ).toBe(false),
  );
  expect(screen.queryByRole('button', { name: /^Pause$/i })).toBeNull();
  expect(client.getRecorderScene).toHaveBeenCalledTimes(1);
  expect(screen.getByText('Status: paused')).toBeTruthy();
});

test('CaptureScreen deletes a draft source and saves the reduced scene', async () => {
  render(<CaptureScreen />);

  expect(await screen.findByText('Monitor 2')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /delete monitor 2/i }));

  expect(screen.queryByText('Monitor 2')).toBeNull();
  const reducedScene = useRecorderWorkspaceStore.getState().scene;
  client.saveRecorderScene.mockResolvedValue(reducedScene);

  fireEvent.click(screen.getByRole('button', { name: /save/i }));

  await waitFor(() =>
    expect(setupClient.saveCaptureSetup).toHaveBeenCalledWith(
      expect.objectContaining({
        scene: expect.objectContaining({
          sources: [
            expect.objectContaining({
              sourceId: 'main',
            }),
          ],
        }),
      }),
    ),
  );
});

test('CaptureScreen adds and deletes optional audio sources', async () => {
  render(<CaptureScreen />);

  await screen.findByRole('button', { name: /add audio source/i });

  expect(useRecorderWorkspaceStore.getState().desktopAudioSourceId).toBe(null);

  fireEvent.click(screen.getByRole('button', { name: /add audio source/i }));
  const dialog = await screen.findByRole('dialog', {
    name: /add audio source/i,
  });
  expect(within(dialog).getByLabelText(/audio source/i)).toBeTruthy();

  fireEvent.click(within(dialog).getByRole('button', { name: /^add$/i }));
  await waitFor(() =>
    expect(
      screen.queryByRole('dialog', { name: /add audio source/i }),
    ).toBeNull(),
  );

  expect(useRecorderWorkspaceStore.getState().desktopAudioSourceId).toBe(
    'desktop',
  );
  expect(screen.getByText('Desktop monitor')).toBeTruthy();

  fireEvent.click(
    screen.getByRole('button', { name: /delete desktop audio/i }),
  );

  expect(useRecorderWorkspaceStore.getState().desktopAudioSourceId).toBe(null);
});

test('CaptureScreen saves and reloads selected audio sources', async () => {
  render(<CaptureScreen />);

  await screen.findByRole('button', { name: /add audio source/i });

  await addFirstAvailableAudioSource();

  expect(useRecorderWorkspaceStore.getState().desktopAudioSourceId).toBe(
    'desktop',
  );
  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(true);

  const savedScene = {
    ...(useRecorderWorkspaceStore.getState().scene ?? sceneFixture(['main'])),
    desktopAudioSourceId: 'desktop',
    microphoneAudioSourceId: null,
  };
  client.saveRecorderScene.mockResolvedValue(savedScene);

  fireEvent.click(screen.getByRole('button', { name: /save/i }));

  await waitFor(() =>
    expect(setupClient.saveCaptureSetup).toHaveBeenCalledWith(
      expect.objectContaining({
        scene: expect.objectContaining({
          desktopAudioSourceId: 'desktop',
          microphoneAudioSourceId: null,
        }),
      }),
    ),
  );
  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(false);

  cleanup();
  useRecorderWorkspaceStore.setState(
    useRecorderWorkspaceStore.getInitialState(),
    true,
  );
  client.getRecorderStatus.mockResolvedValue(status);
  client.getCaptureSources.mockResolvedValue(captureSources);
  client.getAudioSources.mockResolvedValue(audioSources);
  client.getRecorderScene.mockResolvedValue(savedScene);

  render(<CaptureScreen />);

  expect(await screen.findByText('Desktop monitor')).toBeTruthy();
  expect(useRecorderWorkspaceStore.getState().desktopAudioSourceId).toBe(
    'desktop',
  );
  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(false);
});

test('CaptureScreen shows saving banner while stop finalization is finishing', async () => {
  const statusEvents: {
    handler?: (status: RecorderStatus) => void;
  } = {};
  const recordingStatus: RecorderStatus = {
    ...status,
    activeRecordingId: 'recording-1',
    activeSceneId: 'scene-1',
    activeSegmentId: 'segment-1',
    state: 'recording',
  };
  client.getRecorderStatus.mockResolvedValue(recordingStatus);
  client.getRecorderScene.mockResolvedValue(sceneFixture(['main']));
  client.listenToRecorderStatusChanged.mockImplementation((handler) => {
    statusEvents.handler = handler;
    return Promise.resolve(vi.fn());
  });
  client.stopRecording.mockResolvedValue({
    ...recordingStatus,
    activeRecordingId: null,
    activeSegmentId: 'segment-1',
    finalization: {
      action: 'stop',
      phase: 'stopping-capture',
      recordingId: 'recording-1',
      segmentId: 'segment-1',
      startedAtMs: 2,
    },
    state: 'idle',
    updatedAtMs: 2,
  });

  render(<CaptureScreen />);

  const stopButton = await screen.findByRole('button', { name: /stop/i });
  fireEvent.click(stopButton);

  await waitFor(() =>
    expect(client.stopRecording).toHaveBeenCalledWith({
      recordingId: 'recording-1',
    }),
  );
  expect(await screen.findByText('Saving recording')).toBeTruthy();
  expect(screen.queryByRole('button', { name: /stopping/i })).toBeNull();
  expect(
    (screen.getByRole('button', { name: /start/i }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  expect(screen.queryByRole('button', { name: /pause/i })).toBeNull();
  expect(
    (
      screen.getByRole('button', {
        name: /add monitor sources/i,
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);

  expect(statusEvents.handler).toBeDefined();
  statusEvents.handler?.({
    ...recordingStatus,
    activeRecordingId: null,
    activeSceneId: null,
    activeSegmentId: null,
    finalization: null,
    state: 'idle',
  });

  await waitFor(() =>
    expect(screen.getByRole('button', { name: /start/i })).toBeTruthy(),
  );
  expect(
    useRecorderWorkspaceStore.getState().pendingLifecycleAction,
  ).toBeNull();
});

test('CaptureScreen audio picker shows every unadded device', async () => {
  client.getAudioSources.mockResolvedValue({
    sources: [
      ...audioSources.sources,
      {
        displayName: 'Secondary desktop monitor',
        id: 'secondary-desktop',
        isDefault: false,
        kind: 'monitor',
        roles: ['desktop-audio'],
      },
    ],
  });

  render(<CaptureScreen />);

  await screen.findByRole('button', { name: /add audio source/i });

  fireEvent.click(screen.getByRole('button', { name: /add audio source/i }));
  fireEvent.click(
    within(
      await screen.findByRole('dialog', { name: /add audio source/i }),
    ).getByRole('button', { name: /^add$/i }),
  );
  await waitFor(() =>
    expect(
      screen.queryByRole('dialog', { name: /add audio source/i }),
    ).toBeNull(),
  );
  expect(useRecorderWorkspaceStore.getState().desktopAudioSourceId).toBe(
    'desktop',
  );

  fireEvent.click(screen.getByRole('button', { name: /add audio source/i }));
  const dialog = await screen.findByRole('dialog', {
    name: /add audio source/i,
  });
  const select = within(dialog).getByLabelText(/audio source/i);

  fireEvent.click(select);

  expect(
    await screen.findByRole('option', {
      name: 'Secondary desktop monitor',
    }),
  ).toBeTruthy();
  expect(
    screen.getByRole('option', {
      name: 'Microphone (default)',
    }),
  ).toBeTruthy();
  expect(
    screen.queryByRole('option', {
      name: 'Desktop monitor (default)',
    }),
  ).toBeNull();

  fireEvent.click(
    screen.getByRole('option', { name: 'Secondary desktop monitor' }),
  );
  fireEvent.click(within(dialog).getByRole('button', { name: /^add$/i }));
  await waitFor(() =>
    expect(
      screen.queryByRole('dialog', { name: /add audio source/i }),
    ).toBeNull(),
  );

  expect(useRecorderWorkspaceStore.getState().desktopAudioSourceId).toBe(
    'secondary-desktop',
  );
  expect(
    useRecorderWorkspaceStore.getState().microphoneAudioSourceId,
  ).toBeNull();
});

test('CaptureScreen disables audio add when no eligible devices remain', async () => {
  render(<CaptureScreen />);

  await screen.findByRole('button', { name: /add audio source/i });

  await addFirstAvailableAudioSource();

  await addFirstAvailableAudioSource();

  fireEvent.click(screen.getByRole('button', { name: /add audio source/i }));
  const dialog = await screen.findByRole('dialog', {
    name: /add audio source/i,
  });

  expect(within(dialog).getByText('No more devices available')).toBeTruthy();
  expect(
    (
      within(dialog).getByRole('button', {
        name: /^add$/i,
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
});

function sceneFixture(sourceIds: string[]): RecorderScene {
  return {
    independentVideo: true,
    canvasHeight: 900,
    canvasWidth: sourceIds.length * 1440,
    desktopAudioSourceId: null,
    id: 'saved-scene',
    layoutMode: 'horizontal',
    microphoneAudioSourceId: null,
    name: 'Saved scene',
    sources: sourceIds.map((sourceId, index) => ({
      crop: {
        bottom: 0,
        left: 0,
        right: 0,
        top: 0,
      },
      displayName: sourceId,
      id: `saved-scene-source-${sourceId}`,
      preserveAspectRatio: true,
      rect: {
        height: 900,
        width: 1440,
        x: index * 1440,
        y: 0,
      },
      sourceId,
      sourceHeight: 900,
      sourceKind: 'monitor',
      sourceWidth: 1440,
      zIndex: index,
    })),
  };
}

function sceneSource(scene: RecorderScene, index = 0) {
  const source = scene.sources[index];
  if (!source) {
    throw new Error(`Expected scene source at index ${index}.`);
  }

  return source;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });

  return { promise, reject, resolve };
}
