import type {
  AudioSourceList,
  CaptureSourceList,
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
  pauseRecording: vi.fn(),
  resumeRecording: vi.fn(),
  saveRecorderScene: vi.fn(),
  selectCaptureSources: vi.fn(),
  startRecording: vi.fn(),
  stopRecording: vi.fn(),
}));

vi.mock('@/features/recorder/recorder-client', () => client);

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
  expect(screen.getAllByText('1440 x 900')).toHaveLength(2);

  fireEvent.click(screen.getByRole('checkbox', { name: /aspect/i }));
  expect(
    useRecorderWorkspaceStore.getState().scene?.sources[0]?.preserveAspectRatio,
  ).toBe(false);

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
    clientY: 40,
    pointerId: 2,
  });
  fireEvent.pointerUp(source.parentElement ?? source, {
    clientX: 120,
    clientY: 40,
    pointerId: 2,
  });
  expect(
    useRecorderWorkspaceStore.getState().scene?.sources[0]?.rect.x,
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

  fireEvent.click(screen.getByRole('button', { name: /add/i }));
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
        canvasHeight: 900,
        canvasWidth: 2880,
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

function sceneFixture(sourceIds: string[]): RecorderScene {
  return {
    canvasHeight: 900,
    canvasWidth: sourceIds.length * 1440,
    id: 'saved-scene',
    layoutMode: 'horizontal',
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
