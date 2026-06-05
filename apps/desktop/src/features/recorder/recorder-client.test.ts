import type {
  PauseRecordingInput,
  RecorderFfmpegCapabilities,
  RecorderError,
  RecorderScene,
  ResumeRecordingInput,
  SaveRecorderSceneInput,
  StartRecordingInput,
  StopRecordingInput,
} from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import {
  getAudioSources,
  getCaptureSources,
  getRecorderScene,
  getRecorderStatus,
  listenToRecorderStatusChanged,
  pauseRecording,
  recorderStatusChangedEvent,
  resumeRecording,
  saveRecorderScene,
  selectCaptureSources,
  startRecording,
  stopRecording,
} from './recorder-client';

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

const scene: RecorderScene = {
  canvasHeight: 1440,
  canvasWidth: 2560,
  desktopAudioSourceId: 'desktop-monitor',
  id: 'scene-1',
  layoutMode: 'horizontal',
  microphoneAudioSourceId: 'mic',
  name: 'Workday',
  sources: [],
};

const unavailableFfmpegCapabilities = {
  libopusEncoder: false,
  libx264Encoder: false,
  matroskaMuxer: false,
  pulseInput: false,
  rawvideoDemuxer: false,
} satisfies RecorderFfmpegCapabilities;

beforeEach(() => {
  invokeMock.mockReset();
  listenMock.mockReset();
});

test('recorder read helpers use locked camelCase command names', async () => {
  invokeMock.mockResolvedValue({ sources: [] });

  await getCaptureSources();
  await getRecorderScene();
  await selectCaptureSources();
  await selectCaptureSources({ forceChooser: true });
  await getAudioSources();

  expect(invokeMock).toHaveBeenNthCalledWith(1, 'getCaptureSources');
  expect(invokeMock).toHaveBeenNthCalledWith(2, 'getRecorderScene');
  expect(invokeMock).toHaveBeenNthCalledWith(3, 'selectCaptureSources');
  expect(invokeMock).toHaveBeenNthCalledWith(4, 'selectCaptureSources', {
    input: { forceChooser: true },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(5, 'getAudioSources');
});

test('getRecorderStatus uses locked camelCase command name', async () => {
  invokeMock.mockResolvedValue({
    activeRecordingId: null,
    activeSceneId: null,
    activeSegmentId: null,
    backendHealth: {
      desktopAudio: {
        available: false,
        defaultSourceId: null,
        error: null,
      },
      ffmpeg: {
        available: false,
        capabilities: unavailableFfmpegCapabilities,
        error: null,
        path: null,
        versionLine: null,
      },
      microphone: {
        available: false,
        defaultSourceId: null,
        error: null,
      },
      pipewire: {
        available: false,
        error: null,
      },
      portal: {
        available: false,
        cursorModes: [],
        error: null,
        screenCastVersion: null,
        sourceTypes: [],
      },
    },
    lastError: null,
    state: 'unavailable',
    updatedAtMs: 1,
  });

  await getRecorderStatus();

  expect(invokeMock).toHaveBeenCalledWith('getRecorderStatus');
});

test('recorder write helpers pass input payloads unchanged', async () => {
  invokeMock.mockResolvedValue(scene);

  const saveInput: SaveRecorderSceneInput = {
    scene,
  };
  const startInput: StartRecordingInput = {
    desktopAudioSourceId: 'desktop-monitor',
    fps: 1,
    microphoneAudioSourceId: 'mic',
    qualityPreset: 'balanced-ocr',
    sceneId: scene.id,
  };
  const pauseInput: PauseRecordingInput = {
    recordingId: 'recording-1',
  };
  const resumeInput: ResumeRecordingInput = {
    recordingId: 'recording-1',
  };
  const stopInput: StopRecordingInput = {
    recordingId: 'recording-1',
  };

  await saveRecorderScene(saveInput);
  await startRecording(startInput);
  await pauseRecording(pauseInput);
  await resumeRecording(resumeInput);
  await stopRecording(stopInput);

  expect(invokeMock).toHaveBeenNthCalledWith(1, 'saveRecorderScene', {
    input: saveInput,
  });
  expect(invokeMock).toHaveBeenNthCalledWith(2, 'startRecording', {
    input: startInput,
  });
  expect(invokeMock).toHaveBeenNthCalledWith(3, 'pauseRecording', {
    input: pauseInput,
  });
  expect(invokeMock).toHaveBeenNthCalledWith(4, 'resumeRecording', {
    input: resumeInput,
  });
  expect(invokeMock).toHaveBeenNthCalledWith(5, 'stopRecording', {
    input: stopInput,
  });
});

test('recorder helpers propagate typed recorder errors', async () => {
  const error: RecorderError = {
    category: 'unavailable',
    code: 'recorder_backend_not_implemented',
    message: 'Recorder backend is not implemented yet.',
  };

  invokeMock.mockRejectedValue(error);

  await expect(getRecorderStatus()).rejects.toEqual(error);
});

test('recorder status event helper listens with locked event name', async () => {
  const unlisten = vi.fn();
  const handler = vi.fn();
  const status = {
    activeRecordingId: null,
    activeSceneId: null,
    activeSegmentId: null,
    backendHealth: {
      desktopAudio: {
        available: false,
        defaultSourceId: null,
        error: null,
      },
      ffmpeg: {
        available: false,
        capabilities: unavailableFfmpegCapabilities,
        error: null,
        path: null,
        versionLine: null,
      },
      microphone: {
        available: false,
        defaultSourceId: null,
        error: null,
      },
      pipewire: {
        available: false,
        error: null,
      },
      portal: {
        available: false,
        cursorModes: [],
        error: null,
        screenCastVersion: null,
        sourceTypes: [],
      },
    },
    lastError: null,
    state: 'idle',
    updatedAtMs: 1,
  };
  listenMock.mockImplementation((_eventName, listener) => {
    listener({ payload: status });
    return Promise.resolve(unlisten);
  });

  await expect(listenToRecorderStatusChanged(handler)).resolves.toBe(unlisten);

  expect(listenMock).toHaveBeenCalledWith(
    recorderStatusChangedEvent,
    expect.any(Function),
  );
  expect(handler).toHaveBeenCalledWith(status);
});
