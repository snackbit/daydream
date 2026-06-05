import type {
  AudioSourceList,
  CaptureSourceList,
  RecorderFfmpegCapabilities,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import { useRecorderWorkspaceStore } from './recorder-workspace-store';

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

vi.mock('./recorder-client', () => client);

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
  lastError: null,
  state: 'idle',
  updatedAtMs: 1,
};

const captureSources: CaptureSourceList = {
  sources: [
    {
      displayName: 'Main',
      height: null,
      id: 'main',
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
      displayName: 'Desktop',
      id: 'desktop',
      isDefault: true,
      kind: 'monitor',
      roles: ['desktop-audio'],
    },
    {
      displayName: 'Mic',
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

test('load initializes status, audio defaults, and logical-dimension fallback scene', async () => {
  await useRecorderWorkspaceStore.getState().load();

  const state = useRecorderWorkspaceStore.getState();
  expect(state.status?.state).toBe('idle');
  expect(state.desktopAudioSourceId).toBe(null);
  expect(state.microphoneAudioSourceId).toBe(null);
  expect(state.scene?.canvasWidth).toBe(1440);
  expect(state.scene?.canvasHeight).toBe(900);
  expect(state.isDirty).toBe(true);
});

test('load uses the saved scene and marks it clean', async () => {
  const savedScene = sceneWithSources(['saved-main']);
  savedScene.desktopAudioSourceId = 'desktop';
  savedScene.microphoneAudioSourceId = 'mic';
  const savedSource = sceneSource(savedScene);
  savedSource.rect.x = 42;
  client.getRecorderScene.mockResolvedValue(savedScene);

  await useRecorderWorkspaceStore.getState().load();

  const state = useRecorderWorkspaceStore.getState();
  expect(state.scene).toEqual(savedScene);
  expect(state.desktopAudioSourceId).toBe('desktop');
  expect(state.microphoneAudioSourceId).toBe('mic');
  expect(state.selectedSceneSourceId).toBe(savedSource.id);
  expect(state.isDirty).toBe(false);
});

test('load drops unavailable saved audio source ids before exposing the scene', async () => {
  const savedScene = sceneWithSources(['saved-main']);
  savedScene.desktopAudioSourceId = 'missing-desktop';
  savedScene.microphoneAudioSourceId = 'desktop';
  client.getRecorderScene.mockResolvedValue(savedScene);
  client.startRecording.mockResolvedValue(status);

  await useRecorderWorkspaceStore.getState().load();

  const state = useRecorderWorkspaceStore.getState();
  expect(state.desktopAudioSourceId).toBe(null);
  expect(state.microphoneAudioSourceId).toBe(null);
  expect(state.scene).toMatchObject({
    desktopAudioSourceId: null,
    microphoneAudioSourceId: null,
  });
  expect(state.isDirty).toBe(true);

  useRecorderWorkspaceStore.setState({ isDirty: false });
  await useRecorderWorkspaceStore.getState().start();

  expect(client.startRecording).toHaveBeenCalledWith({
    desktopAudioSourceId: null,
    fps: 1,
    microphoneAudioSourceId: null,
    qualityPreset: 'balanced-ocr',
    sceneId: savedScene.id,
  });
});

test('load records client failures without creating a draft scene', async () => {
  client.getCaptureSources.mockRejectedValue(new Error('portal unavailable'));

  await useRecorderWorkspaceStore.getState().load();

  const state = useRecorderWorkspaceStore.getState();
  expect(state.loadStatus).toBe('error');
  expect(state.error).toBe('portal unavailable');
  expect(state.scene).toBeNull();
});

test('updateCaptureSources forces the chooser and merges sources into the draft scene', async () => {
  const scene = sceneWithSources(['main']);
  scene.layoutMode = 'custom';
  const mainSource = sceneSource(scene);
  mainSource.rect.x = 120;
  useRecorderWorkspaceStore.setState({
    scene,
    selectedSceneSourceId: mainSource.id,
  });
  client.selectCaptureSources.mockResolvedValue({
    sources: [
      captureSource(captureSources),
      {
        displayName: 'Secondary',
        height: null,
        id: 'secondary',
        kind: 'monitor',
        logicalHeight: 720,
        logicalWidth: 1280,
        scaleFactor: null,
        width: null,
      },
    ],
  });

  await useRecorderWorkspaceStore.getState().updateCaptureSources();

  const nextScene = useRecorderWorkspaceStore.getState().scene;
  expect(client.selectCaptureSources).toHaveBeenCalledWith({
    forceChooser: true,
  });
  expect(nextScene?.sources).toHaveLength(2);
  expect(nextScene?.sources[0]).toEqual(mainSource);
  expect(nextScene?.sources[1]).toMatchObject({
    rect: {
      height: 720,
      width: 1280,
      x: 1560,
      y: 0,
    },
    sourceId: 'secondary',
  });
  expect(nextScene?.canvasWidth).toBe(2840);
  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(true);
});

test('updateCaptureSources starts from an empty scene when none is loaded', async () => {
  client.selectCaptureSources.mockResolvedValue({
    sources: [
      {
        displayName: 'Fresh',
        height: null,
        id: 'fresh',
        kind: 'monitor',
        logicalHeight: 720,
        logicalWidth: 1280,
        scaleFactor: null,
        width: null,
      },
    ],
  });

  await useRecorderWorkspaceStore.getState().updateCaptureSources();

  const state = useRecorderWorkspaceStore.getState();
  expect(state.scene?.sources.map((source) => source.sourceId)).toEqual([
    'fresh',
  ]);
  expect(state.selectedSceneSourceId).toBe(state.scene?.sources[0]?.id);
  expect(state.isDirty).toBe(true);
});

test('updateCaptureSources ignores duplicate source ids', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({ scene });

  await useRecorderWorkspaceStore.getState().updateCaptureSources();

  expect(useRecorderWorkspaceStore.getState().scene?.sources).toHaveLength(1);
});

test('updateCaptureSources preserves dirty state when no sources are merged', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    scene,
    selectedSceneSourceId: 'missing-selection',
  });

  await useRecorderWorkspaceStore.getState().updateCaptureSources();

  const state = useRecorderWorkspaceStore.getState();
  expect(state.scene).toBe(scene);
  expect(state.selectedSceneSourceId).toBe(scene.sources[0]?.id);
  expect(state.isDirty).toBe(false);
});

test('updateCaptureSources records chooser errors', async () => {
  client.selectCaptureSources.mockRejectedValue('chooser failed');

  await useRecorderWorkspaceStore.getState().updateCaptureSources();

  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    error: 'chooser failed',
    loadStatus: 'error',
  });
});

test('updateCaptureSources queues likely duplicate monitor matches for resolution', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({ scene });
  client.selectCaptureSources.mockResolvedValue({
    sources: [
      {
        displayName: 'main',
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

  await useRecorderWorkspaceStore.getState().updateCaptureSources();

  const state = useRecorderWorkspaceStore.getState();
  expect(state.scene?.sources).toHaveLength(1);
  expect(state.pendingDuplicateCaptureSources).toHaveLength(1);
  expect(state.pendingDuplicateCaptureSources[0]?.source.id).toBe(
    'main-new-portal-id',
  );
});

test('resolveDuplicateCaptureSource refreshes an existing source id', () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    pendingDuplicateCaptureSources: [
      {
        existingSource: sceneSource(scene),
        source: {
          displayName: 'main',
          height: null,
          id: 'main-new-portal-id',
          kind: 'monitor',
          logicalHeight: 900,
          logicalWidth: 1440,
          scaleFactor: null,
          width: null,
        },
      },
    ],
    scene,
  });

  useRecorderWorkspaceStore
    .getState()
    .resolveDuplicateCaptureSource('refresh-existing');

  const state = useRecorderWorkspaceStore.getState();
  expect(state.scene?.sources).toHaveLength(1);
  expect(state.scene?.sources[0]?.sourceId).toBe('main-new-portal-id');
  expect(state.scene?.sources[0]?.rect).toEqual(scene.sources[0]?.rect);
  expect(state.pendingDuplicateCaptureSources).toHaveLength(0);
  expect(state.isDirty).toBe(true);
});

test('resolveDuplicateCaptureSource can skip or no-op without dirtying the scene', () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    pendingDuplicateCaptureSources: [
      {
        existingSource: sceneSource(scene),
        source: {
          displayName: 'main',
          height: null,
          id: 'main-new-portal-id',
          kind: 'monitor',
          logicalHeight: 900,
          logicalWidth: 1440,
          scaleFactor: null,
          width: null,
        },
      },
    ],
    scene,
  });

  useRecorderWorkspaceStore.getState().resolveDuplicateCaptureSource('skip');

  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    isDirty: false,
    pendingDuplicateCaptureSources: [],
    scene,
  });

  useRecorderWorkspaceStore
    .getState()
    .resolveDuplicateCaptureSource('refresh-existing');

  expect(useRecorderWorkspaceStore.getState().scene).toBe(scene);
});

test('deleteSceneSource removes a source and marks the scene dirty', () => {
  const scene = sceneWithSources(['left', 'right']);
  const leftSource = sceneSource(scene);
  const rightSource = sceneSource(scene, 1);
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    scene,
    selectedSceneSourceId: leftSource.id,
  });

  useRecorderWorkspaceStore.getState().deleteSceneSource(leftSource.id);

  const state = useRecorderWorkspaceStore.getState();
  expect(state.scene?.sources.map((source) => source.sourceId)).toEqual([
    'right',
  ]);
  expect(state.selectedSceneSourceId).toBe(rightSource.id);
  expect(state.isDirty).toBe(true);
});

test('deleteSceneSource ignores missing scenes and missing source ids', () => {
  useRecorderWorkspaceStore.getState().deleteSceneSource('missing');
  expect(useRecorderWorkspaceStore.getState().scene).toBeNull();

  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    scene,
    selectedSceneSourceId: sceneSource(scene).id,
  });

  useRecorderWorkspaceStore.getState().deleteSceneSource('missing');

  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    isDirty: false,
    scene,
    selectedSceneSourceId: sceneSource(scene).id,
  });
});

test('saveScene after delete persists the reduced source list', async () => {
  const scene = sceneWithSources(['left', 'right']);
  const rightSource = sceneSource(scene, 1);
  const reducedScene: RecorderScene = {
    ...scene,
    layoutMode: 'custom',
    sources: [rightSource],
  };
  useRecorderWorkspaceStore.setState({
    isDirty: true,
    scene: reducedScene,
    selectedSceneSourceId: rightSource.id,
  });
  client.saveRecorderScene.mockResolvedValue(reducedScene);

  await useRecorderWorkspaceStore.getState().saveScene();

  expect(client.saveRecorderScene).toHaveBeenCalledWith({
    scene: expect.objectContaining({
      sources: [
        expect.objectContaining({
          sourceId: 'right',
        }),
      ],
    }),
  });
  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(false);
});

test('saveScene persists the current scene and clears dirty state', async () => {
  await useRecorderWorkspaceStore.getState().load();
  const scene = useRecorderWorkspaceStore.getState().scene;
  useRecorderWorkspaceStore
    .getState()
    .selectAudioSource('desktop-audio', 'desktop');
  useRecorderWorkspaceStore.getState().selectAudioSource('microphone', 'mic');
  const sceneWithAudio = {
    ...(scene ?? sceneWithSources(['main'])),
    desktopAudioSourceId: 'desktop',
    microphoneAudioSourceId: 'mic',
  };
  client.saveRecorderScene.mockResolvedValue(sceneWithAudio);

  await useRecorderWorkspaceStore.getState().saveScene();

  expect(client.saveRecorderScene).toHaveBeenCalledWith({
    scene: sceneWithAudio,
  });
  expect(useRecorderWorkspaceStore.getState().desktopAudioSourceId).toBe(
    'desktop',
  );
  expect(useRecorderWorkspaceStore.getState().microphoneAudioSourceId).toBe(
    'mic',
  );
  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(false);
});

test('saveScene reports missing scenes and save failures', async () => {
  await useRecorderWorkspaceStore.getState().saveScene();

  expect(useRecorderWorkspaceStore.getState().error).toBe(
    'No recorder scene is available to save.',
  );

  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({ scene });
  client.saveRecorderScene.mockRejectedValue(new Error('disk full'));

  await useRecorderWorkspaceStore.getState().saveScene();

  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    error: 'disk full',
    saveStatus: 'error',
  });
});

test('setDraftScene marks dirty and falls back to an existing selection', () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({ selectedSceneSourceId: 'gone' });

  useRecorderWorkspaceStore.getState().setDraftScene(scene);

  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    isDirty: true,
    scene,
    selectedSceneSourceId: sceneSource(scene).id,
  });
});

test('selectAudioSource explicitly adds and deletes optional audio roles', () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    scene,
  });

  useRecorderWorkspaceStore
    .getState()
    .selectAudioSource('desktop-audio', 'desktop');
  useRecorderWorkspaceStore.getState().selectAudioSource('microphone', 'mic');

  expect(useRecorderWorkspaceStore.getState().desktopAudioSourceId).toBe(
    'desktop',
  );
  expect(useRecorderWorkspaceStore.getState().microphoneAudioSourceId).toBe(
    'mic',
  );
  expect(useRecorderWorkspaceStore.getState().scene).toMatchObject({
    desktopAudioSourceId: 'desktop',
    microphoneAudioSourceId: 'mic',
  });
  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(true);

  useRecorderWorkspaceStore.setState({ isDirty: false });
  useRecorderWorkspaceStore
    .getState()
    .selectAudioSource('desktop-audio', 'desktop');

  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(false);

  useRecorderWorkspaceStore.getState().selectAudioSource('desktop-audio', null);

  expect(useRecorderWorkspaceStore.getState().desktopAudioSourceId).toBe(null);
  expect(useRecorderWorkspaceStore.getState().microphoneAudioSourceId).toBe(
    'mic',
  );
  expect(useRecorderWorkspaceStore.getState().scene).toMatchObject({
    desktopAudioSourceId: null,
    microphoneAudioSourceId: 'mic',
  });
  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(true);
});

test('start sends null for omitted optional audio roles', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    scene,
    status,
  });
  client.startRecording.mockResolvedValue(status);

  await useRecorderWorkspaceStore.getState().start();

  expect(client.startRecording).toHaveBeenCalledWith({
    desktopAudioSourceId: null,
    fps: 1,
    microphoneAudioSourceId: null,
    qualityPreset: 'balanced-ocr',
    sceneId: scene.id,
  });
});

test('start sends restored saved audio roles', async () => {
  const scene = {
    ...sceneWithSources(['main']),
    desktopAudioSourceId: 'desktop',
    microphoneAudioSourceId: 'mic',
  };
  client.getRecorderScene.mockResolvedValue(scene);
  client.startRecording.mockResolvedValue(status);

  await useRecorderWorkspaceStore.getState().load();
  await useRecorderWorkspaceStore.getState().start();

  expect(client.startRecording).toHaveBeenCalledWith({
    desktopAudioSourceId: 'desktop',
    fps: 1,
    microphoneAudioSourceId: 'mic',
    qualityPreset: 'balanced-ocr',
    sceneId: scene.id,
  });
});

test('pause resume and stop no-op without active recordings', async () => {
  await useRecorderWorkspaceStore.getState().pause();
  await useRecorderWorkspaceStore.getState().resume();
  await useRecorderWorkspaceStore.getState().stop();

  expect(client.pauseRecording).not.toHaveBeenCalled();
  expect(client.resumeRecording).not.toHaveBeenCalled();
  expect(client.stopRecording).not.toHaveBeenCalled();
});

test('pause resume stop and start update status through recorder commands', async () => {
  const scene = sceneWithSources(['main']);
  const recordingStatus = {
    ...status,
    activeRecordingId: 'recording-1',
  };
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    scene,
    status: recordingStatus,
  });
  client.pauseRecording.mockResolvedValue({
    ...recordingStatus,
    state: 'paused',
  });
  client.resumeRecording.mockResolvedValue({
    ...recordingStatus,
    state: 'recording',
  });
  client.stopRecording.mockResolvedValue({
    ...recordingStatus,
    activeRecordingId: null,
    state: 'idle',
  });
  client.startRecording.mockResolvedValue({
    ...recordingStatus,
    state: 'recording',
  });

  await useRecorderWorkspaceStore.getState().pause();
  expect(client.pauseRecording).toHaveBeenCalledWith({
    recordingId: 'recording-1',
  });
  expect(useRecorderWorkspaceStore.getState().status?.state).toBe('paused');

  await useRecorderWorkspaceStore.getState().resume();
  expect(client.resumeRecording).toHaveBeenCalledWith({
    recordingId: 'recording-1',
  });
  expect(useRecorderWorkspaceStore.getState().status?.state).toBe('recording');

  await useRecorderWorkspaceStore.getState().stop();
  expect(client.stopRecording).toHaveBeenCalledWith({
    recordingId: 'recording-1',
  });
  expect(useRecorderWorkspaceStore.getState().status?.state).toBe('idle');

  await useRecorderWorkspaceStore.getState().start();
  expect(client.startRecording).toHaveBeenCalledWith(
    expect.objectContaining({ sceneId: scene.id }),
  );
  expect(useRecorderWorkspaceStore.getState().status?.state).toBe('recording');
});

test('async pause keeps lifecycle action pending until status event settles', async () => {
  const statusEvents: {
    handler?: (status: RecorderStatus) => void;
  } = {};
  const recordingStatus = {
    ...status,
    activeRecordingId: 'recording-1',
    state: 'recording' as const,
  };
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    scene: sceneWithSources(['main']),
    status: recordingStatus,
  });
  client.listenToRecorderStatusChanged.mockImplementation((handler) => {
    statusEvents.handler = handler;
    return Promise.resolve(vi.fn());
  });
  client.pauseRecording.mockResolvedValue({
    ...recordingStatus,
    state: 'pausing',
  });

  await useRecorderWorkspaceStore.getState().subscribeToStatusEvents();
  await useRecorderWorkspaceStore.getState().pause();

  expect(useRecorderWorkspaceStore.getState().pendingLifecycleAction).toBe(
    'pause',
  );
  expect(useRecorderWorkspaceStore.getState().status?.state).toBe('pausing');

  expect(statusEvents.handler).toBeDefined();
  statusEvents.handler?.({
    ...recordingStatus,
    activeSegmentId: null,
    state: 'paused',
  });

  expect(
    useRecorderWorkspaceStore.getState().pendingLifecycleAction,
  ).toBeNull();
  expect(useRecorderWorkspaceStore.getState().status?.state).toBe('paused');
});

test('locked lifecycle transition rejects concurrent recorder commands', async () => {
  useRecorderWorkspaceStore.setState({
    pendingLifecycleAction: 'stop',
    status: {
      ...status,
      activeRecordingId: 'recording-1',
      state: 'stopping',
    },
  });

  await useRecorderWorkspaceStore.getState().pause();
  await useRecorderWorkspaceStore.getState().resume();
  await useRecorderWorkspaceStore.getState().stop();

  expect(client.pauseRecording).not.toHaveBeenCalled();
  expect(client.resumeRecording).not.toHaveBeenCalled();
  expect(client.stopRecording).not.toHaveBeenCalled();
});

test('recorder command failures preserve typed recorder error messages', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    scene,
    status: {
      ...status,
      activeRecordingId: 'recording-1',
    },
  });
  client.pauseRecording.mockRejectedValue({
    category: 'lifecycle',
    code: 'recorder_invalid_transition',
    message: 'Cannot pause now.',
  });

  await useRecorderWorkspaceStore.getState().pause();

  expect(useRecorderWorkspaceStore.getState().error).toBe('Cannot pause now.');
});

test('start requires a saved scene before invoking the recorder command', async () => {
  await useRecorderWorkspaceStore.getState().load();

  await useRecorderWorkspaceStore.getState().start();

  expect(client.startRecording).not.toHaveBeenCalled();
  expect(useRecorderWorkspaceStore.getState().error).toBe(
    'Save the recorder scene before starting.',
  );
});

function sceneWithSources(sourceIds: string[]): RecorderScene {
  return {
    canvasHeight: 900,
    canvasWidth: sourceIds.length * 1440,
    desktopAudioSourceId: null,
    id: 'scene-1',
    layoutMode: 'horizontal',
    microphoneAudioSourceId: null,
    name: 'Recorder scene',
    sources: sourceIds.map((sourceId, index) => ({
      crop: {
        bottom: 0,
        left: 0,
        right: 0,
        top: 0,
      },
      displayName: sourceId,
      id: `scene-source-${sourceId}`,
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

function captureSource(sourceList: CaptureSourceList, index = 0) {
  const source = sourceList.sources[index];
  if (!source) {
    throw new Error(`Expected capture source at index ${index}.`);
  }

  return source;
}
