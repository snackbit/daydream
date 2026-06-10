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
import { useRecorderWorkspaceStore } from '@/features/recorder/recorder-workspace-store';
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

beforeEach(() => {
  vi.resetAllMocks();
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
});

test('CaptureScreen renders selected sources and explicitly saves the scene', async () => {
  render(<CaptureScreen />);

  expect(await screen.findAllByText('Monitor 1')).toHaveLength(3);
  expect(
    document.querySelector('[data-slot="resizable-panel-group"]'),
  ).toBeTruthy();
  expect(screen.getByText('Status: idle')).toBeTruthy();
  expect(screen.getByText('Record defaults: Settings')).toBeTruthy();
  expect(screen.getAllByText('1440 x 900')).toHaveLength(3);
  expect(screen.getByText('Original')).toBeTruthy();
  expect(screen.getByText('100%')).toBeTruthy();
  expect(screen.getByRole('button', { name: /^fit$/i })).toBeTruthy();
  expect(screen.getByRole('button', { name: /^fill$/i })).toBeTruthy();
  expect(screen.getByRole('button', { name: /^center$/i })).toBeTruthy();
  expect(screen.getByRole('button', { name: /^fill empty$/i })).toBeTruthy();

  fireEvent.click(screen.getByRole('checkbox', { name: /aspect/i }));
  expect(
    useRecorderWorkspaceStore.getState().scene?.sources[0]?.preserveAspectRatio,
  ).toBe(false);
  expect(screen.getByText('100% x 100%')).toBeTruthy();

  const source = screen.getByTestId('scene-source-main');
  const sourceBodyRectBeforeClick =
    useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect;
  fireEvent.pointerDown(source, { clientX: 20, clientY: 40, pointerId: 1 });
  fireEvent.pointerMove(source.parentElement ?? source, {
    clientX: 120,
    clientY: 40,
    pointerId: 1,
  });
  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect).toEqual(
    sourceBodyRectBeforeClick,
  );

  const moveHandle = screen.getByRole('button', { name: /move monitor 1/i });
  fireEvent.pointerDown(moveHandle, { clientX: 20, clientY: 40, pointerId: 2 });
  fireEvent.pointerMove(source.parentElement ?? source, {
    clientX: 120,
    clientY: 140,
    pointerId: 2,
  });
  fireEvent.pointerUp(source.parentElement ?? source, {
    clientX: 120,
    clientY: 140,
    pointerId: 2,
  });
  expect(
    useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect.y,
  ).toBeGreaterThan(0);

  const resizeHandle = within(source).getByRole('button', {
    name: /resize se/i,
  });
  expect(resizeHandle.className).toContain('cursor-nwse-resize');
  fireEvent.pointerDown(resizeHandle, {
    clientX: 120,
    clientY: 80,
    pointerId: 3,
  });
  fireEvent.pointerMove(source.parentElement ?? source, {
    clientX: 10000,
    clientY: 10000,
    pointerId: 3,
  });
  fireEvent.pointerUp(source.parentElement ?? source, {
    clientX: 10000,
    clientY: 10000,
    pointerId: 3,
  });
  const maxClampedRect =
    useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect;
  fireEvent.pointerMove(source.parentElement ?? source, {
    clientX: 200,
    clientY: 200,
    pointerId: 3,
  });
  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect).toEqual(
    maxClampedRect,
  );

  fireEvent.click(screen.getByRole('button', { name: /add monitor sources/i }));
  await waitFor(() =>
    expect(client.selectCaptureSources).toHaveBeenCalledWith({
      forceChooser: true,
    }),
  );

  const savedScene = useRecorderWorkspaceStore.getState().scene;
  if (!savedScene) {
    throw new Error('Expected the workspace to build a scene.');
  }
  client.saveRecorderScene.mockResolvedValue(savedScene);

  fireEvent.click(screen.getByRole('button', { name: /save/i }));

  await waitFor(() =>
    expect(client.saveRecorderScene).toHaveBeenCalledWith({
      scene: expect.objectContaining({
        canvasHeight: 1800,
        canvasWidth: 1440,
        sources: expect.arrayContaining([
          expect.objectContaining({
            sourceId: 'main',
          }),
        ]),
      }),
    }),
  );
  expect(screen.getByText('Scene saved')).toBeTruthy();
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

test('CaptureScreen updates canvas presets and layout actions', async () => {
  render(<CaptureScreen />);

  await screen.findAllByText('Monitor 1');

  fireEvent.click(screen.getByLabelText(/canvas preset/i));
  fireEvent.click(await screen.findByRole('option', { name: '2560 x 1440' }));
  expect(useRecorderWorkspaceStore.getState().scene).toMatchObject({
    canvasHeight: 1440,
    canvasWidth: 2560,
    layoutMode: 'auto-16-9',
  });

  fireEvent.click(screen.getByRole('button', { name: /vertical layout/i }));
  expect(useRecorderWorkspaceStore.getState().scene).toMatchObject({
    canvasHeight: 1440,
    canvasWidth: 2560,
    layoutMode: 'vertical',
  });
  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect).toEqual({
    height: 720,
    width: 1152,
    x: 704,
    y: 0,
  });
  expect(useRecorderWorkspaceStore.getState().scene?.sources[1]?.rect).toEqual({
    height: 720,
    width: 1152,
    x: 704,
    y: 720,
  });

  fireEvent.click(screen.getByRole('button', { name: /auto layout/i }));
  expect(useRecorderWorkspaceStore.getState().scene).toMatchObject({
    canvasHeight: 1440,
    canvasWidth: 2560,
    layoutMode: 'auto-16-9',
  });
  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect).toEqual({
    height: 800,
    width: 1280,
    x: 0,
    y: 320,
  });
  expect(useRecorderWorkspaceStore.getState().scene?.sources[1]?.rect).toEqual({
    height: 800,
    width: 1280,
    x: 1280,
    y: 320,
  });
});

test('CaptureScreen switches scale labels and relocks distorted sources', async () => {
  const savedScene = sceneFixture(['saved-main']);
  savedScene.sources[0] = {
    ...sceneSource(savedScene),
    preserveAspectRatio: false,
    rect: {
      height: 900,
      width: 720,
      x: 0,
      y: 0,
    },
  };
  client.getRecorderScene.mockResolvedValue(savedScene);

  render(<CaptureScreen />);

  expect(await screen.findByText('50% x 100%')).toBeTruthy();

  fireEvent.click(screen.getByRole('checkbox', { name: /aspect/i }));

  expect(screen.getByText('100%')).toBeTruthy();
  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect).toEqual({
    height: 900,
    width: 1440,
    x: 0,
    y: 0,
  });
  expect(
    useRecorderWorkspaceStore.getState().scene?.sources[0]?.preserveAspectRatio,
  ).toBe(true);
});

test('CaptureScreen fill respects the aspect checkbox', async () => {
  const savedScene = sceneFixture(['ultrawide']);
  savedScene.canvasHeight = 1080;
  savedScene.canvasWidth = 1920;
  savedScene.layoutMode = 'custom';
  savedScene.sources[0] = {
    ...sceneSource(savedScene),
    preserveAspectRatio: true,
    rect: {
      height: 360,
      width: 1280,
      x: 0,
      y: 0,
    },
    sourceHeight: 1440,
    sourceWidth: 5120,
  };
  client.getRecorderScene.mockResolvedValue(savedScene);

  render(<CaptureScreen />);

  expect(await screen.findByText('25%')).toBeTruthy();

  fireEvent.change(screen.getByDisplayValue('1280'), {
    target: { value: '960' },
  });
  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect).toEqual({
    height: 270,
    width: 960,
    x: 0,
    y: 0,
  });
  expect(screen.getByText('19%')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /^fill$/i }));

  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect).toEqual({
    height: 540,
    width: 1920,
    x: 0,
    y: 270,
  });
  expect(screen.getByText('38%')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /^fit$/i }));

  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect).toEqual({
    height: 540,
    width: 1920,
    x: 0,
    y: 270,
  });
  expect(screen.getByText('38%')).toBeTruthy();

  fireEvent.click(screen.getByRole('checkbox', { name: /aspect/i }));
  fireEvent.click(screen.getByRole('button', { name: /^fill$/i }));

  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect).toEqual({
    height: 1080,
    width: 1920,
    x: 0,
    y: 0,
  });
  expect(screen.getByText('38% x 75%')).toBeTruthy();
});

test('CaptureScreen shows resize handles only on the temporarily fronted selection', async () => {
  const savedScene = sceneFixture(['back-monitor', 'front-monitor']);
  savedScene.sources = savedScene.sources.map((source, index) => ({
    ...source,
    rect: {
      height: 720,
      width: 1152,
      x: index === 0 ? 0 : 240,
      y: index === 0 ? 0 : 120,
    },
  }));
  client.getRecorderScene.mockResolvedValue(savedScene);

  render(<CaptureScreen />);

  const backSource = await screen.findByTestId('scene-source-back-monitor');
  const frontSource = screen.getByTestId('scene-source-front-monitor');
  const backSourceListButton = screen.getByRole('button', {
    name: /focus back-monitor/i,
  });
  const frontSourceListButton = screen.getByRole('button', {
    name: /focus front-monitor/i,
  });

  expect(screen.getAllByRole('button', { name: /resize/i })).toHaveLength(8);
  expect(
    within(backSource).getByRole('button', { name: /resize se/i }),
  ).toBeTruthy();
  expect(
    within(frontSource).queryByRole('button', { name: /resize se/i }),
  ).toBeNull();
  expect(backSource.style.zIndex).toBe('2');
  expect(frontSource.style.zIndex).toBe('1');
  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.zIndex).toBe(
    0,
  );
  expect(useRecorderWorkspaceStore.getState().scene?.sources[1]?.zIndex).toBe(
    1,
  );
  expect(backSourceListButton.getAttribute('aria-pressed')).toBe('true');
  expect(frontSourceListButton.getAttribute('aria-pressed')).toBe('false');
  expect(within(backSourceListButton).getByText('Focused')).toBeTruthy();
  expect(
    within(frontSourceListButton).getByText('Click to focus'),
  ).toBeTruthy();

  fireEvent.click(frontSourceListButton);

  expect(screen.getAllByRole('button', { name: /resize/i })).toHaveLength(8);
  expect(
    within(backSource).queryByRole('button', { name: /resize se/i }),
  ).toBeNull();
  expect(
    within(frontSource).getByRole('button', { name: /resize se/i }),
  ).toBeTruthy();
  expect(backSource.style.zIndex).toBe('0');
  expect(frontSource.style.zIndex).toBe('2');
  expect(useRecorderWorkspaceStore.getState().scene?.sources[0]?.zIndex).toBe(
    0,
  );
  expect(useRecorderWorkspaceStore.getState().scene?.sources[1]?.zIndex).toBe(
    1,
  );
  expect(backSourceListButton.getAttribute('aria-pressed')).toBe('false');
  expect(frontSourceListButton.getAttribute('aria-pressed')).toBe('true');
  expect(within(backSourceListButton).getByText('Click to focus')).toBeTruthy();
  expect(within(frontSourceListButton).getByText('Focused')).toBeTruthy();
});

test('CaptureScreen renders a saved scene on startup without adding sources', async () => {
  const savedScene = sceneFixture(['saved-main']);
  const savedSource = sceneSource(savedScene);
  savedSource.displayName = 'Saved monitor';
  savedSource.rect.x = 24;
  client.getRecorderScene.mockResolvedValue(savedScene);

  render(<CaptureScreen />);

  expect(await screen.findAllByText('Saved monitor')).toHaveLength(3);
  expect(client.selectCaptureSources).not.toHaveBeenCalled();
  expect(screen.getByText('Scene saved')).toBeTruthy();
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
    name: /main live preview/i,
  });
  expect(preview.getAttribute('src')).toBe('data:image/png;base64,abc');
  expect(preview.className).toContain('object-fill');
  expect(preview.className).not.toContain('object-cover');
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
    expect(client.saveRecorderScene).toHaveBeenCalledWith({
      scene: expect.objectContaining({
        sources: [],
      }),
    }),
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
    (screen.getByRole('checkbox', { name: /aspect/i }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  expect(
    (screen.getByRole('button', { name: /^fit$/i }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);

  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));
  fireEvent.click(screen.getByRole('button', { name: /add monitor sources/i }));

  expect(client.saveRecorderScene).not.toHaveBeenCalled();
  expect(client.selectCaptureSources).not.toHaveBeenCalled();
});

test('CaptureScreen deletes a draft source and saves the reduced scene', async () => {
  render(<CaptureScreen />);

  expect(await screen.findAllByText('Monitor 2')).toHaveLength(2);

  fireEvent.click(screen.getByRole('button', { name: /delete monitor 2/i }));

  expect(screen.queryByText('Monitor 2')).toBeNull();
  const reducedScene = useRecorderWorkspaceStore.getState().scene;
  client.saveRecorderScene.mockResolvedValue(reducedScene);

  fireEvent.click(screen.getByRole('button', { name: /save/i }));

  await waitFor(() =>
    expect(client.saveRecorderScene).toHaveBeenCalledWith({
      scene: expect.objectContaining({
        sources: [
          expect.objectContaining({
            sourceId: 'main',
          }),
        ],
      }),
    }),
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

  fireEvent.click(screen.getByRole('button', { name: /add audio source/i }));
  fireEvent.click(
    within(
      await screen.findByRole('dialog', { name: /add audio source/i }),
    ).getByRole('button', { name: /^add$/i }),
  );

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
    expect(client.saveRecorderScene).toHaveBeenCalledWith({
      scene: expect.objectContaining({
        desktopAudioSourceId: 'desktop',
        microphoneAudioSourceId: null,
      }),
    }),
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
  expect(
    (screen.getByRole('button', { name: /pause/i }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
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

  fireEvent.click(screen.getByRole('button', { name: /add audio source/i }));
  fireEvent.click(
    within(
      await screen.findByRole('dialog', { name: /add audio source/i }),
    ).getByRole('button', { name: /^add$/i }),
  );

  fireEvent.click(screen.getByRole('button', { name: /add audio source/i }));
  fireEvent.click(
    within(
      await screen.findByRole('dialog', { name: /add audio source/i }),
    ).getByRole('button', { name: /^add$/i }),
  );

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
