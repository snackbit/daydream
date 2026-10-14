import type {
  AudioSourceList,
  CaptureSourceList,
  RecorderMediaWriterCapabilities,
  RecorderPreviewFrame,
  RecorderPreviewStatus,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import { stopPreviewForRecording } from './recorder-preview-store';
import { createRecorderWorkspaceStore } from './recorder-workspace-store';

let useRecorderWorkspaceStore = createRecorderWorkspaceStore();

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

vi.mock('./recorder-client', () => client);

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
  useRecorderWorkspaceStore = createRecorderWorkspaceStore();
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
  client.stopRecorderPreview.mockResolvedValue({
    error: null,
    sceneId: null,
    sourceIds: [],
    state: 'idle',
    updatedAtMs: 2,
  });
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

test('preference invalidation fences an old scene load and saves only the new workspace scene', async () => {
  const deletedScene = sceneWithSources(['deleted-scene-source']);
  let finish: ((scene: RecorderScene | null) => void) | undefined;
  client.getRecorderScene.mockReturnValueOnce(
    new Promise<RecorderScene | null>((resolve) => {
      finish = resolve;
    }),
  );
  const old = useRecorderWorkspaceStore.getState().load();
  useRecorderWorkspaceStore.getState().invalidatePreferences();
  await useRecorderWorkspaceStore.getState().reloadPreferences(null);
  const fresh = useRecorderWorkspaceStore.getState().scene;
  if (!fresh) throw new Error('Expected a fresh default scene');
  finish?.(deletedScene);
  await old;
  expect(useRecorderWorkspaceStore.getState().scene?.id).toBe(fresh.id);
  expect(fresh.id).not.toBe(deletedScene.id);
  expect(client.getRecorderScene).toHaveBeenCalledOnce();
  client.saveRecorderScene.mockResolvedValue(fresh);
  await useRecorderWorkspaceStore.getState().saveScene();
  expect(client.saveRecorderScene).toHaveBeenCalledExactlyOnceWith({
    scene: fresh,
  });
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

test('startPreview records live status and preview frame events by scene source id', async () => {
  const scene = sceneWithSources(['main']);
  let previewFrameHandler: ((frame: RecorderPreviewFrame) => void) | undefined;
  client.listenToRecorderPreviewFrames.mockImplementation((handler) => {
    previewFrameHandler = handler;
    return Promise.resolve(vi.fn());
  });
  client.startRecorderPreview.mockResolvedValue({
    error: null,
    sceneId: scene.id,
    sourceIds: ['main'],
    state: 'live',
    updatedAtMs: 2,
  });

  await useRecorderWorkspaceStore.getState().subscribeToPreviewFrames();
  await useRecorderWorkspaceStore.getState().startPreview(scene.id);
  previewFrameHandler?.({
    dataUrl: 'data:image/png;base64,abc',
    height: 180,
    sceneSourceId: scene.sources[0]?.id ?? '',
    sourceId: 'main',
    timestampMs: 3,
    width: 320,
  });

  expect(client.startRecorderPreview).toHaveBeenCalledWith({
    sceneId: scene.id,
  });
  expect(useRecorderWorkspaceStore.getState().previewStatus?.state).toBe(
    'live',
  );
  expect(
    useRecorderWorkspaceStore.getState().previewFramesBySceneSourceId[
      scene.sources[0]?.id ?? ''
    ]?.dataUrl,
  ).toBe('data:image/png;base64,abc');
});

test('startPreview does not invoke backend while recorder lifecycle is pending', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    pendingLifecycleAction: 'start',
    scene,
    status,
  });

  await useRecorderWorkspaceStore.getState().startPreview(scene.id);

  expect(client.startRecorderPreview).not.toHaveBeenCalled();
});

test('startPreview is single-flight for duplicate calls while backend request is pending', async () => {
  const scene = sceneWithSources(['main']);
  const preview = deferred<RecorderPreviewStatus>();
  client.startRecorderPreview.mockReturnValue(preview.promise);

  const first = useRecorderWorkspaceStore.getState().startPreview(scene.id);
  const second = useRecorderWorkspaceStore.getState().startPreview(scene.id);

  expect(client.startRecorderPreview).toHaveBeenCalledTimes(1);

  preview.resolve({
    error: null,
    sceneId: scene.id,
    sourceIds: ['main'],
    state: 'live',
    updatedAtMs: 2,
  });
  await Promise.all([first, second]);

  expect(useRecorderWorkspaceStore.getState().previewStatus?.state).toBe(
    'live',
  );
  expect(
    useRecorderWorkspaceStore.getState().previewStartInFlightSceneId,
  ).toBeNull();

  await useRecorderWorkspaceStore.getState().startPreview(scene.id);

  expect(client.startRecorderPreview).toHaveBeenCalledTimes(1);
});

test('startPreview blocks different scene requests while preview start is in flight', async () => {
  const scene = sceneWithSources(['main']);
  const otherScene = { ...sceneWithSources(['secondary']), id: 'scene-2' };
  const preview = deferred<RecorderPreviewStatus>();
  client.startRecorderPreview.mockReturnValue(preview.promise);

  const first = useRecorderWorkspaceStore.getState().startPreview(scene.id);
  const second = useRecorderWorkspaceStore
    .getState()
    .startPreview(otherScene.id);

  expect(client.startRecorderPreview).toHaveBeenCalledTimes(1);

  preview.resolve({
    error: null,
    sceneId: scene.id,
    sourceIds: ['main'],
    state: 'live',
    updatedAtMs: 2,
  });
  await Promise.all([first, second]);

  expect(useRecorderWorkspaceStore.getState().previewStatus?.sceneId).toBe(
    scene.id,
  );
});

test('start stops active preview before starting recording', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    previewStatus: {
      error: null,
      sceneId: scene.id,
      sourceIds: ['main'],
      state: 'live',
      updatedAtMs: 2,
    },
    scene,
    status,
  });
  client.startRecording.mockResolvedValue({
    ...status,
    activeRecordingId: 'recording-1',
    state: 'recording',
  });
  client.stopRecorderPreview.mockImplementation(() => {
    expect(useRecorderWorkspaceStore.getState().pendingLifecycleAction).toBe(
      'start',
    );

    return Promise.resolve({
      error: null,
      sceneId: null,
      sourceIds: [],
      state: 'idle',
      updatedAtMs: 2,
    });
  });

  await useRecorderWorkspaceStore.getState().start();

  expect(client.stopRecorderPreview.mock.invocationCallOrder[0]).toBeLessThan(
    client.startRecording.mock.invocationCallOrder[0] ??
      Number.MAX_SAFE_INTEGER,
  );
  expect(client.startRecording).toHaveBeenCalledWith(
    expect.objectContaining({ sceneId: scene.id }),
  );
  expect(useRecorderWorkspaceStore.getState().previewStatus?.state).toBe(
    'idle',
  );
});

test('start surfaces stale screen permission without opening source chooser', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    isDirty: false,
    scene,
    status,
  });
  client.startRecording.mockRejectedValue({
    category: 'permission',
    code: 'recorder_portal_restore_token_invalid',
    message: 'Screen permission expired. Reselect monitor sources.',
  });

  await useRecorderWorkspaceStore.getState().start();

  expect(client.startRecording).toHaveBeenCalledWith(
    expect.objectContaining({ sceneId: scene.id }),
  );
  expect(client.selectCaptureSources).not.toHaveBeenCalled();
  expect(useRecorderWorkspaceStore.getState().error).toBe(
    'Screen permission expired. Reselect monitor sources.',
  );
  expect(
    useRecorderWorkspaceStore.getState().pendingLifecycleAction,
  ).toBeNull();
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
    microphoneAudioSourceId: null,
    qualityPreset: 'balanced-ocr',
    sceneId: savedScene.id,
  });
});

test('load records client failures without creating a draft scene', async () => {
  client.getCaptureSources.mockRejectedValue(new Error('portal unavailable'));

  await expect(useRecorderWorkspaceStore.getState().load()).rejects.toThrow(
    'portal unavailable',
  );

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

  await expect(
    useRecorderWorkspaceStore.getState().updateCaptureSources(),
  ).rejects.toBe('chooser failed');

  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    error: 'chooser failed',
    loadStatus: 'error',
  });
});

test('updateCaptureSources ignores manual chooser cancellation', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    error: 'previous error',
    isDirty: false,
    loadStatus: 'ready',
    scene,
    selectedSceneSourceId: scene.sources[0]?.id ?? null,
  });
  client.selectCaptureSources.mockRejectedValue({
    category: 'permission',
    code: 'recorder_portal_permission_denied',
    message: 'ScreenCast source selection was denied or cancelled.',
  });

  await expect(
    useRecorderWorkspaceStore.getState().updateCaptureSources(),
  ).resolves.toBeUndefined();

  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    error: null,
    isDirty: false,
    loadStatus: 'ready',
    pendingDuplicateCaptureSources: [],
    scene,
    selectedSceneSourceId: scene.sources[0]?.id ?? null,
  });
});

test('updateCaptureSources stops active preview before opening chooser', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    previewFramesBySceneSourceId: {
      [scene.sources[0]?.id ?? 'main']: {
        dataUrl: 'data:image/png;base64,preview',
        height: 1,
        sceneSourceId: scene.sources[0]?.id ?? 'main',
        sourceId: 'main',
        timestampMs: 1,
        width: 1,
      },
    },
    previewStatus: {
      error: null,
      sceneId: scene.id,
      sourceIds: ['main'],
      state: 'live',
      updatedAtMs: 1,
    },
    scene,
  });

  await useRecorderWorkspaceStore.getState().updateCaptureSources();

  expect(client.stopRecorderPreview.mock.invocationCallOrder[0]).toBeLessThan(
    client.selectCaptureSources.mock.invocationCallOrder[0] ??
      Number.POSITIVE_INFINITY,
  );
  expect(
    useRecorderWorkspaceStore.getState().previewFramesBySceneSourceId,
  ).toEqual({});
});

test('source selection owns the wait for preview cleanup and prevents duplicate choosers', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    scene,
    status,
    loadStatus: 'ready',
    previewStatus: {
      error: null,
      sceneId: scene.id,
      sourceIds: ['main'],
      state: 'live',
      updatedAtMs: 1,
    },
  });
  let finishStop: ((value: RecorderPreviewStatus) => void) | undefined;
  client.stopRecorderPreview.mockReturnValue(
    new Promise<RecorderPreviewStatus>((resolve) => {
      finishStop = resolve;
    }),
  );
  const selecting = useRecorderWorkspaceStore.getState().updateCaptureSources();
  expect(useRecorderWorkspaceStore.getState().loadStatus).toBe('loading');
  await useRecorderWorkspaceStore.getState().updateCaptureSources();
  expect(client.stopRecorderPreview).toHaveBeenCalledOnce();
  expect(client.selectCaptureSources).not.toHaveBeenCalled();
  finishStop?.({
    error: null,
    sceneId: null,
    sourceIds: [],
    state: 'idle',
    updatedAtMs: 2,
  });
  await selecting;
  expect(client.selectCaptureSources).toHaveBeenCalledOnce();
});

test.each(['resolve', 'reject'] as const)(
  'old preview start %s cannot overwrite a new request for the same scene',
  async (settlement) => {
    const scene = sceneWithSources(['main']);
    useRecorderWorkspaceStore.setState({ scene, status });
    let finishOld: ((value: RecorderPreviewStatus) => void) | undefined;
    let failOld: ((error: Error) => void) | undefined;
    let finishNew: ((value: RecorderPreviewStatus) => void) | undefined;
    client.startRecorderPreview
      .mockReturnValueOnce(
        new Promise<RecorderPreviewStatus>((resolve, reject) => {
          finishOld = resolve;
          failOld = reject;
        }),
      )
      .mockReturnValueOnce(
        new Promise<RecorderPreviewStatus>((resolve) => {
          finishNew = resolve;
        }),
      );
    const old = useRecorderWorkspaceStore.getState().startPreview(scene.id);
    await useRecorderWorkspaceStore.getState().stopPreview();
    const current = useRecorderWorkspaceStore.getState().startPreview(scene.id);
    if (settlement === 'resolve')
      finishOld?.({
        error: null,
        sceneId: scene.id,
        sourceIds: ['obsolete'],
        state: 'live',
        updatedAtMs: 2,
      });
    else failOld?.(new Error('Synthetic old startup failure'));
    await old;
    expect(useRecorderWorkspaceStore.getState().previewStatus?.state).toBe(
      'starting',
    );
    expect(
      useRecorderWorkspaceStore.getState().previewStartInFlightSceneId,
    ).toBe(scene.id);
    finishNew?.({
      error: null,
      sceneId: scene.id,
      sourceIds: ['main'],
      state: 'live',
      updatedAtMs: 3,
    });
    await current;
    expect(
      useRecorderWorkspaceStore.getState().previewStatus?.sourceIds,
    ).toEqual(['main']);
  },
);

test('preview shutdown is shared and blocks new preview admission until cleanup replies', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    scene,
    status,
    previewStatus: {
      error: null,
      sceneId: scene.id,
      sourceIds: ['main'],
      state: 'live',
      updatedAtMs: 1,
    },
  });
  let finish: ((value: RecorderPreviewStatus) => void) | undefined;
  client.stopRecorderPreview.mockReturnValue(
    new Promise<RecorderPreviewStatus>((resolve) => {
      finish = resolve;
    }),
  );
  const first = useRecorderWorkspaceStore.getState().stopPreview();
  const second = useRecorderWorkspaceStore.getState().stopPreview();
  expect(second).toBe(first);
  expect(useRecorderWorkspaceStore.getState().previewStopInFlight).toBe(true);
  await useRecorderWorkspaceStore.getState().startPreview(scene.id);
  expect(client.startRecorderPreview).not.toHaveBeenCalled();
  finish?.({
    error: null,
    sceneId: null,
    sourceIds: [],
    state: 'idle',
    updatedAtMs: 2,
  });
  await first;
  expect(client.stopRecorderPreview).toHaveBeenCalledOnce();
  expect(useRecorderWorkspaceStore.getState().previewStopInFlight).toBe(false);
});

test.each(['resolve', 'reject'] as const)(
  'stale shutdown %s cannot overwrite a reset workspace preview',
  async (settlement) => {
    const scene = sceneWithSources(['main']);
    useRecorderWorkspaceStore.setState({
      scene,
      status,
      previewStatus: {
        error: null,
        sceneId: scene.id,
        sourceIds: ['main'],
        state: 'live',
        updatedAtMs: 1,
      },
    });
    let finish: ((value: RecorderPreviewStatus) => void) | undefined;
    let fail: ((error: Error) => void) | undefined;
    client.stopRecorderPreview.mockReturnValueOnce(
      new Promise<RecorderPreviewStatus>((resolve, reject) => {
        finish = resolve;
        fail = reject;
      }),
    );
    const old = useRecorderWorkspaceStore.getState().stopPreview();
    useRecorderWorkspaceStore.getState().invalidatePreferences();
    useRecorderWorkspaceStore.setState({ scene, status });
    client.startRecorderPreview.mockResolvedValue({
      error: null,
      sceneId: scene.id,
      sourceIds: ['fresh'],
      state: 'live',
      updatedAtMs: 3,
    });
    await useRecorderWorkspaceStore.getState().startPreview(scene.id);
    if (settlement === 'resolve')
      finish?.({
        error: null,
        sceneId: null,
        sourceIds: [],
        state: 'idle',
        updatedAtMs: 2,
      });
    else fail?.(new Error('Synthetic obsolete cleanup failure'));
    await old;
    expect(
      useRecorderWorkspaceStore.getState().previewStatus?.sourceIds,
    ).toEqual(['fresh']);
    expect(useRecorderWorkspaceStore.getState().error).toBeNull();
  },
);

test('failed preview cleanup keeps source configuration and prevents chooser admission', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    scene,
    status,
    loadStatus: 'ready',
    previewStatus: {
      error: null,
      sceneId: scene.id,
      sourceIds: ['main'],
      state: 'live',
      updatedAtMs: 1,
    },
  });
  client.stopRecorderPreview.mockRejectedValue(
    new Error('Synthetic cleanup failure'),
  );
  await expect(
    useRecorderWorkspaceStore.getState().updateCaptureSources(),
  ).rejects.toThrow();
  expect(client.selectCaptureSources).not.toHaveBeenCalled();
  expect(useRecorderWorkspaceStore.getState().scene).toBe(scene);
  expect(useRecorderWorkspaceStore.getState().previewStatus?.state).toBe(
    'error',
  );
  expect(useRecorderWorkspaceStore.getState().captureSelectionPending).toBe(
    false,
  );
});

test('source reselection recovers from an existing preview startup error', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    scene,
    status,
    previewStatus: {
      error: null,
      sceneId: scene.id,
      sourceIds: [],
      state: 'error',
      updatedAtMs: 1,
    },
  });
  await useRecorderWorkspaceStore.getState().updateCaptureSources();
  expect(client.selectCaptureSources).toHaveBeenCalledOnce();
  expect(client.stopRecorderPreview).not.toHaveBeenCalled();
  expect(useRecorderWorkspaceStore.getState().captureSelectionPending).toBe(
    false,
  );
});

test('current preview failure and mismatched scene replies remain recoverable errors', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({ scene, status });
  const error = {
    category: 'permission',
    code: 'recorder_portal_restore_token_invalid',
    message: 'Screen permission expired.',
  };
  client.startRecorderPreview.mockRejectedValueOnce(error);
  await useRecorderWorkspaceStore.getState().startPreview(scene.id);
  expect(useRecorderWorkspaceStore.getState().previewStatus?.error).toEqual(
    error,
  );
  expect(
    useRecorderWorkspaceStore.getState().previewStartInFlightSceneId,
  ).toBeNull();
  client.startRecorderPreview.mockResolvedValueOnce({
    error: null,
    sceneId: 'other-scene',
    sourceIds: [],
    state: 'live',
    updatedAtMs: 2,
  });
  await useRecorderWorkspaceStore.getState().startPreview(scene.id);
  expect(useRecorderWorkspaceStore.getState().previewStatus?.state).toBe(
    'error',
  );
  expect(
    useRecorderWorkspaceStore.getState().previewFramesBySceneSourceId,
  ).toEqual({});
});

test.each(['start', 'saveScene'] as const)(
  'failed preview shutdown blocks %s and preserves its failure',
  async (action) => {
    const scene = sceneWithSources(action === 'saveScene' ? [] : ['main']);
    useRecorderWorkspaceStore.setState({
      scene,
      status,
      previewStatus: {
        error: null,
        sceneId: scene.id,
        sourceIds: ['main'],
        state: 'live',
        updatedAtMs: 1,
      },
    });
    client.stopRecorderPreview.mockRejectedValueOnce(
      new Error('Synthetic cleanup failure'),
    );
    await useRecorderWorkspaceStore.getState()[action]();
    expect(client.startRecording).not.toHaveBeenCalled();
    expect(client.saveRecorderScene).not.toHaveBeenCalled();
    expect(useRecorderWorkspaceStore.getState().error).toBe(
      'Synthetic cleanup failure',
    );
    expect(
      useRecorderWorkspaceStore.getState().pendingLifecycleAction,
    ).toBeNull();
    if (action === 'saveScene')
      expect(useRecorderWorkspaceStore.getState().saveStatus).toBe('error');
  },
);

test('recording stop precondition exposes a recovery message when preview failure has no native details', async () => {
  useRecorderWorkspaceStore.setState({
    previewStatus: {
      error: null,
      sceneId: 'synthetic',
      sourceIds: [],
      state: 'error',
      updatedAtMs: 1,
    },
  });
  const allowed = await stopPreviewForRecording(
    useRecorderWorkspaceStore.setState,
    useRecorderWorkspaceStore.getState,
  );
  expect(allowed).toBe(false);
  expect(useRecorderWorkspaceStore.getState().error).toBeTruthy();
});

test('preference invalidation during cleanup prevents an obsolete chooser from opening', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    scene,
    status,
    previewStatus: {
      error: null,
      sceneId: scene.id,
      sourceIds: ['main'],
      state: 'live',
      updatedAtMs: 1,
    },
  });
  const stop = deferred<RecorderPreviewStatus>();
  client.stopRecorderPreview.mockReturnValueOnce(stop.promise);
  const selection = useRecorderWorkspaceStore.getState().updateCaptureSources();
  useRecorderWorkspaceStore.getState().invalidatePreferences();
  stop.resolve({
    error: null,
    sceneId: null,
    sourceIds: [],
    state: 'idle',
    updatedAtMs: 2,
  });
  await selection;
  expect(client.selectCaptureSources).not.toHaveBeenCalled();
  expect(useRecorderWorkspaceStore.getState().scene).toBeNull();
});

test('preview frame ordering ignores old generations and sequences while accepting new frames', async () => {
  const scene = sceneWithSources(['main']);
  let publish: ((frame: RecorderPreviewFrame) => void) | undefined;
  client.listenToRecorderPreviewFrames.mockImplementation(async (listener) => {
    publish = listener;
    return () => undefined;
  });
  useRecorderWorkspaceStore.setState({ scene });
  await useRecorderWorkspaceStore.getState().subscribeToPreviewFrames();
  const frame: RecorderPreviewFrame = {
    sceneSourceId: scene.sources[0]?.id ?? '',
    sourceId: 'main',
    dataUrl: 'synthetic-frame',
    width: 1,
    height: 1,
    timestampMs: 1,
    generation: 3,
    sequence: 2,
  };
  publish?.(frame);
  publish?.({ ...frame, generation: 2, sequence: 100 });
  publish?.({ ...frame, sequence: 1 });
  expect(
    useRecorderWorkspaceStore.getState().previewFramesBySceneSourceId[
      frame.sceneSourceId
    ],
  ).toEqual(frame);
  publish?.({ ...frame, sequence: 3 });
  expect(
    useRecorderWorkspaceStore.getState().previewFramesBySceneSourceId[
      frame.sceneSourceId
    ]?.sequence,
  ).toBe(3);
  publish?.({ ...frame, generation: 4, sequence: 1 });
  expect(
    useRecorderWorkspaceStore.getState().previewFramesBySceneSourceId[
      frame.sceneSourceId
    ]?.generation,
  ).toBe(4);
  publish?.({ ...frame, sceneSourceId: 'deleted' });
  expect(
    useRecorderWorkspaceStore.getState().previewFramesBySceneSourceId.deleted,
  ).toBeUndefined();
});

test('failed status subscription can reconnect without retaining its rejected listener', async () => {
  client.listenToRecorderStatusChanged.mockRejectedValueOnce(
    new Error('Synthetic subscription failure'),
  );
  await expect(
    useRecorderWorkspaceStore.getState().subscribeToStatusEvents(),
  ).rejects.toThrow('Synthetic subscription failure');
  const unsubscribe = await useRecorderWorkspaceStore
    .getState()
    .subscribeToStatusEvents();
  unsubscribe();
  expect(client.listenToRecorderStatusChanged).toHaveBeenCalledTimes(2);
});

test('failed recording stats refresh reports a recoverable error', async () => {
  useRecorderWorkspaceStore.setState({
    status: { ...status, state: 'recording', activeRecordingId: 'synthetic' },
  });
  client.getRecorderRecordingStats.mockRejectedValueOnce(
    new Error('Synthetic stats failure'),
  );
  await useRecorderWorkspaceStore.getState().refreshRecordingStats();
  expect(useRecorderWorkspaceStore.getState().recordingStatsError).toBe(
    'Synthetic stats failure',
  );
});

test('Capture status refresh failure remains visible until the next successful refresh', async () => {
  client.getRecorderStatus.mockRejectedValueOnce(
    new Error('Synthetic readiness failure'),
  );
  const unsubscribe = await useRecorderWorkspaceStore
    .getState()
    .subscribeToStatusEvents();
  await vi.waitFor(() =>
    expect(useRecorderWorkspaceStore.getState().error).toBe(
      'Synthetic readiness failure',
    ),
  );
  unsubscribe();
  const reconnected = await useRecorderWorkspaceStore
    .getState()
    .subscribeToStatusEvents();
  await vi.waitFor(() =>
    expect(useRecorderWorkspaceStore.getState().status).toEqual(status),
  );
  expect(useRecorderWorkspaceStore.getState().error).toBeNull();
  reconnected();
});

test('stats for a recording stopped during the read do not return to the workspace', async () => {
  const stats =
    deferred<Awaited<ReturnType<typeof client.getRecorderRecordingStats>>>();
  client.getRecorderRecordingStats.mockReturnValueOnce(stats.promise);
  useRecorderWorkspaceStore.setState({
    status: { ...status, state: 'recording', activeRecordingId: 'synthetic' },
  });
  const refresh = useRecorderWorkspaceStore.getState().refreshRecordingStats();
  useRecorderWorkspaceStore.setState({ status });
  stats.resolve(null);
  await refresh;
  expect(useRecorderWorkspaceStore.getState().recordingStats).toBeNull();
  expect(useRecorderWorkspaceStore.getState().recordingStatsError).toBeNull();
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

test('saveScene stops active preview before persisting an empty scene', async () => {
  const scene = sceneWithSources(['main']);
  const emptyScene: RecorderScene = {
    ...scene,
    sources: [],
  };
  useRecorderWorkspaceStore.setState({
    isDirty: true,
    previewFramesBySceneSourceId: {
      [scene.sources[0]?.id ?? 'main']: {
        dataUrl: 'data:image/png;base64,preview',
        height: 1,
        sceneSourceId: scene.sources[0]?.id ?? 'main',
        sourceId: 'main',
        timestampMs: 1,
        width: 1,
      },
    },
    previewStartInFlightSceneId: scene.id,
    previewStatus: {
      error: null,
      sceneId: scene.id,
      sourceIds: ['main'],
      state: 'live',
      updatedAtMs: 1,
    },
    scene: emptyScene,
  });
  client.saveRecorderScene.mockResolvedValue(emptyScene);

  await useRecorderWorkspaceStore.getState().saveScene();

  expect(client.stopRecorderPreview.mock.invocationCallOrder[0]).toBeLessThan(
    client.saveRecorderScene.mock.invocationCallOrder[0] ??
      Number.POSITIVE_INFINITY,
  );
  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    isDirty: false,
    previewFramesBySceneSourceId: {},
    previewStartInFlightSceneId: null,
    previewStatus: null,
  });
});

test('saveScene clears stale preview error for the saved scene', async () => {
  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({
    isDirty: true,
    previewFramesBySceneSourceId: {
      [scene.sources[0]?.id ?? 'main']: {
        dataUrl: 'data:image/png;base64,preview',
        height: 1,
        sceneSourceId: scene.sources[0]?.id ?? 'main',
        sourceId: 'main',
        timestampMs: 1,
        width: 1,
      },
    },
    previewStatus: {
      error: {
        category: 'video',
        code: 'recorder_capture_source_missing',
        message: 'Capture source is missing.',
      },
      sceneId: scene.id,
      sourceIds: [],
      state: 'error',
      updatedAtMs: 1,
    },
    scene,
  });
  client.saveRecorderScene.mockResolvedValue(scene);

  await useRecorderWorkspaceStore.getState().saveScene();

  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    isDirty: false,
    previewFramesBySceneSourceId: {},
    previewStatus: null,
  });
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

test('saveScene preserves edits made while the persisted scene is in flight', async () => {
  const submitted = sceneWithSources(['main']);
  const newer = { ...submitted, canvasWidth: submitted.canvasWidth + 100 };
  const request = deferred<RecorderScene>();
  client.saveRecorderScene.mockReturnValue(request.promise);
  useRecorderWorkspaceStore.setState({
    scene: submitted,
    isDirty: true,
    status,
  });
  const saving = useRecorderWorkspaceStore.getState().saveScene();
  useRecorderWorkspaceStore.getState().setDraftScene(newer);
  const duplicateSaving = useRecorderWorkspaceStore.getState().saveScene();
  request.resolve(submitted);
  await Promise.all([saving, duplicateSaving]);
  expect(client.saveRecorderScene).toHaveBeenCalledTimes(1);
  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    scene: newer,
    isDirty: true,
    saveStatus: 'ready',
  });
});

test('status subscription refreshes missed transitions without replacing a dirty scene', async () => {
  const dirtyScene = sceneWithSources(['main']);
  let handler: ((next: RecorderStatus) => void) | undefined;
  client.listenToRecorderStatusChanged.mockImplementation((nextHandler) => {
    handler = nextHandler;
    return Promise.resolve(vi.fn());
  });
  const snapshot = deferred<RecorderStatus>();
  client.getRecorderStatus.mockReturnValue(snapshot.promise);
  useRecorderWorkspaceStore.setState({
    scene: dirtyScene,
    isDirty: true,
    status: { ...status, activeRecordingId: 'recording-1', state: 'recording' },
    loadStatus: 'ready',
  });
  const subscribing = useRecorderWorkspaceStore
    .getState()
    .subscribeToStatusEvents();
  await Promise.resolve();
  expect(client.getRecorderStatus).toHaveBeenCalledTimes(1);
  handler?.({
    ...status,
    activeRecordingId: 'recording-1',
    state: 'paused',
    updatedAtMs: 3,
  });
  snapshot.resolve({ ...status, state: 'recording', updatedAtMs: 2 });
  await subscribing;
  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    scene: dirtyScene,
    isDirty: true,
    status: { state: 'paused' },
  });
});

test('saveScene reports missing scenes and save failures', async () => {
  await useRecorderWorkspaceStore.getState().saveScene();

  expect(useRecorderWorkspaceStore.getState().error).toBe(
    'No recorder scene is available to save.',
  );

  const scene = sceneWithSources(['main']);
  useRecorderWorkspaceStore.setState({ scene });
  client.saveRecorderScene.mockRejectedValue(new Error('disk full'));

  await expect(
    useRecorderWorkspaceStore.getState().saveScene(),
  ).rejects.toThrow('disk full');

  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    error: 'disk full',
    saveStatus: 'error',
  });
});

test('shell and Capture share one status subscription until both release it', async () => {
  const cleanup = vi.fn();
  client.listenToRecorderStatusChanged.mockResolvedValue(cleanup);
  const [shell, capture] = await Promise.all([
    useRecorderWorkspaceStore.getState().subscribeToStatusEvents(),
    useRecorderWorkspaceStore.getState().subscribeToStatusEvents(),
  ]);
  expect(client.listenToRecorderStatusChanged).toHaveBeenCalledTimes(1);
  shell();
  await Promise.resolve();
  expect(cleanup).not.toHaveBeenCalled();
  capture();
  await Promise.resolve();
  expect(cleanup).toHaveBeenCalledTimes(1);
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

test('configuration mutations are ignored while recording is active', async () => {
  const scene = sceneWithSources(['main', 'secondary']);
  const replacementScene = sceneWithSources(['replacement']);
  const source = sceneSource(scene);
  useRecorderWorkspaceStore.setState({
    desktopAudioSourceId: 'desktop',
    isDirty: false,
    pendingDuplicateCaptureSources: [
      {
        existingSource: source,
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
    selectedSceneSourceId: source.id,
    status: {
      ...status,
      activeRecordingId: 'recording-1',
      activeSceneId: scene.id,
      activeSegmentId: 'segment-1',
      state: 'recording',
    },
  });

  useRecorderWorkspaceStore.getState().setDraftScene(replacementScene);
  useRecorderWorkspaceStore.getState().deleteSceneSource(source.id);
  useRecorderWorkspaceStore.getState().selectAudioSource('desktop-audio', null);
  useRecorderWorkspaceStore
    .getState()
    .resolveDuplicateCaptureSource('refresh-existing');
  await useRecorderWorkspaceStore.getState().updateCaptureSources();
  await useRecorderWorkspaceStore.getState().saveScene();

  expect(client.selectCaptureSources).not.toHaveBeenCalled();
  expect(client.saveRecorderScene).not.toHaveBeenCalled();
  expect(useRecorderWorkspaceStore.getState()).toMatchObject({
    desktopAudioSourceId: 'desktop',
    isDirty: false,
    pendingDuplicateCaptureSources: [
      expect.objectContaining({
        existingSource: source,
      }),
    ],
    scene,
    selectedSceneSourceId: source.id,
  });
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

test('async pause shows paused while segment finalization is pending', async () => {
  const statusEvents: {
    handler?: (status: RecorderStatus) => void;
  } = {};
  const recordingStatus = {
    ...status,
    activeRecordingId: 'recording-1',
    activeSegmentId: 'segment-1',
    state: 'recording' as const,
  };
  client.getRecorderStatus.mockResolvedValue(recordingStatus);
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
    activeSegmentId: 'segment-1',
    finalization: {
      action: 'pause',
      phase: 'stopping-capture',
      recordingId: 'recording-1',
      segmentId: 'segment-1',
      startedAtMs: 2,
    },
    state: 'paused',
  });

  await useRecorderWorkspaceStore.getState().subscribeToStatusEvents();
  await useRecorderWorkspaceStore.getState().pause();

  expect(
    useRecorderWorkspaceStore.getState().pendingLifecycleAction,
  ).toBeNull();
  expect(useRecorderWorkspaceStore.getState().status?.state).toBe('paused');
  expect(
    useRecorderWorkspaceStore.getState().status?.finalization?.action,
  ).toBe('pause');
  await useRecorderWorkspaceStore.getState().resume();
  expect(client.resumeRecording).not.toHaveBeenCalled();

  expect(statusEvents.handler).toBeDefined();
  statusEvents.handler?.({
    ...recordingStatus,
    activeSegmentId: null,
    finalization: null,
    state: 'paused',
  });

  expect(useRecorderWorkspaceStore.getState().status?.finalization).toBeNull();
  expect(useRecorderWorkspaceStore.getState().status?.state).toBe('paused');
});

test('async stop settles through status polling when completion event is missed', async () => {
  vi.useFakeTimers();
  try {
    const recordingStatus = {
      ...status,
      activeRecordingId: 'recording-1',
      activeSegmentId: 'segment-1',
      state: 'recording' as const,
    };
    const finalizingStatus: RecorderStatus = {
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
    };
    const settledStatus: RecorderStatus = {
      ...finalizingStatus,
      finalization: null,
      updatedAtMs: 3,
    };
    useRecorderWorkspaceStore.setState({
      isDirty: false,
      scene: sceneWithSources(['main']),
      status: recordingStatus,
    });
    client.stopRecording.mockResolvedValue(finalizingStatus);
    client.getRecorderStatus.mockResolvedValue(settledStatus);

    await useRecorderWorkspaceStore.getState().stop();

    expect(useRecorderWorkspaceStore.getState().status?.state).toBe('idle');
    expect(
      useRecorderWorkspaceStore.getState().status?.finalization?.action,
    ).toBe('stop');
    expect(
      useRecorderWorkspaceStore.getState().pendingLifecycleAction,
    ).toBeNull();

    await vi.advanceTimersByTimeAsync(250);

    expect(client.getRecorderStatus).toHaveBeenCalled();
    expect(
      useRecorderWorkspaceStore.getState().status?.finalization,
    ).toBeNull();
  } finally {
    vi.useRealTimers();
  }
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

  await expect(
    useRecorderWorkspaceStore.getState().pause(),
  ).rejects.toMatchObject({
    message: 'Cannot pause now.',
  });

  expect(useRecorderWorkspaceStore.getState().error).toBe('Cannot pause now.');
});

test('refreshRecordingStats stores active recording stats and clears them after recording', async () => {
  const recordingStatus: RecorderStatus = {
    ...status,
    activeRecordingId: 'recording-1',
    activeSegmentId: 'segment-1',
    state: 'recording',
  };
  const recordingStats = {
    activeSegmentId: 'segment-1',
    activeSegmentIndex: 1,
    activeSegmentSizeBytes: 2_048,
    activeSegmentStartedAtMs: 1,
    recordingId: 'recording-1',
    recordingStartedAtMs: 1,
    segmentCount: 1,
    totalSizeBytes: 2_048,
    updatedAtMs: 2,
  };
  client.getRecorderRecordingStats.mockResolvedValue(recordingStats);
  useRecorderWorkspaceStore.setState({ status: recordingStatus });

  await useRecorderWorkspaceStore.getState().refreshRecordingStats();

  expect(client.getRecorderRecordingStats).toHaveBeenCalled();
  expect(useRecorderWorkspaceStore.getState().recordingStats).toEqual(
    recordingStats,
  );

  useRecorderWorkspaceStore.setState({ status });
  await useRecorderWorkspaceStore.getState().refreshRecordingStats();

  expect(useRecorderWorkspaceStore.getState().recordingStats).toBeNull();
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

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });

  return { promise, reject, resolve };
}
