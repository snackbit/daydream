import type {
  AudioSourceList,
  CaptureSourceList,
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
  pauseRecording: vi.fn(),
  resumeRecording: vi.fn(),
  saveRecorderScene: vi.fn(),
  selectCaptureSources: vi.fn(),
  startRecording: vi.fn(),
  stopRecording: vi.fn(),
}));

vi.mock('./recorder-client', () => client);

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
      error: 'FFmpeg is not wired yet.',
      path: null,
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
  client.selectCaptureSources.mockResolvedValue(captureSources);
});

test('load initializes status, audio defaults, and logical-dimension fallback scene', async () => {
  await useRecorderWorkspaceStore.getState().load();

  const state = useRecorderWorkspaceStore.getState();
  expect(state.status?.state).toBe('idle');
  expect(state.desktopAudioSourceId).toBe('desktop');
  expect(state.microphoneAudioSourceId).toBe('mic');
  expect(state.scene?.canvasWidth).toBe(1440);
  expect(state.scene?.canvasHeight).toBe(900);
  expect(state.isDirty).toBe(true);
});

test('load uses the saved scene and marks it clean', async () => {
  const savedScene = sceneWithSources(['saved-main']);
  const savedSource = sceneSource(savedScene);
  savedSource.rect.x = 42;
  client.getRecorderScene.mockResolvedValue(savedScene);

  await useRecorderWorkspaceStore.getState().load();

  const state = useRecorderWorkspaceStore.getState();
  expect(state.scene).toEqual(savedScene);
  expect(state.selectedSceneSourceId).toBe(savedSource.id);
  expect(state.isDirty).toBe(false);
});

test('updateCaptureSources forces the chooser and merges sources into the draft scene', async () => {
  const scene = sceneWithSources(['main']);
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

test('updateCaptureSources ignores duplicate source ids', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({ scene });

  await useRecorderWorkspaceStore.getState().updateCaptureSources();

  expect(useRecorderWorkspaceStore.getState().scene?.sources).toHaveLength(1);
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
  client.saveRecorderScene.mockResolvedValue(scene);

  await useRecorderWorkspaceStore.getState().saveScene();

  expect(client.saveRecorderScene).toHaveBeenCalledWith({ scene });
  expect(useRecorderWorkspaceStore.getState().isDirty).toBe(false);
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
    id: 'scene-1',
    layoutMode: 'horizontal',
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
      sourceKind: 'monitor',
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
