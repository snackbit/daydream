import type {
  AudioSourceList,
  CaptureSourceList,
  RecorderFfmpegCapabilities,
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
  getRecorderScene: vi.fn(),
  getRecorderStatus: vi.fn(),
  listenToRecorderStatusChanged: vi.fn(),
  pauseRecording: vi.fn(),
  resumeRecording: vi.fn(),
  saveRecorderScene: vi.fn(),
  selectCaptureSources: vi.fn(),
  startRecording: vi.fn(),
  stopRecording: vi.fn(),
}));

vi.mock('@/features/recorder/recorder-client', () => client);

const unavailableFfmpegCapabilities = {
  libopusEncoder: false,
  libx264Encoder: false,
  matroskaMuxer: false,
  pulseInput: false,
  rawvideoDemuxer: false,
} satisfies RecorderFfmpegCapabilities;

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
    ffmpeg: {
      available: false,
      capabilities: unavailableFfmpegCapabilities,
      error: 'FFmpeg is not wired yet.',
      path: null,
      versionLine: null,
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
  client.getCaptureSources.mockResolvedValue(captureSources);
  client.getRecorderScene.mockResolvedValue(null);
  client.getAudioSources.mockResolvedValue(audioSources);
  client.listenToRecorderStatusChanged.mockResolvedValue(vi.fn());
  client.selectCaptureSources.mockResolvedValue(captureSources);
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

  fireEvent.change(screen.getByLabelText(/canvas preset/i), {
    target: { value: '2560x1440' },
  });
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
    activeSegmentId: null,
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
  ).toBe(false);

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

  expect(
    within(select).getByRole('option', {
      name: 'Secondary desktop monitor',
    }),
  ).toBeTruthy();
  expect(
    within(select).getByRole('option', {
      name: 'Microphone (default)',
    }),
  ).toBeTruthy();
  expect(
    within(select).queryByRole('option', {
      name: 'Desktop monitor (default)',
    }),
  ).toBeNull();

  fireEvent.change(select, {
    target: { value: 'secondary-desktop' },
  });
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
