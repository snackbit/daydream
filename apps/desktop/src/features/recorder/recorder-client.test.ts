import type {
  PauseRecordingInput,
  RecorderMediaWriterCapabilities,
  RecorderError,
  RecorderPreviewFrame,
  RecorderScene,
  ResumeRecordingInput,
  SaveRecorderSceneInput,
  StartRecorderPreviewInput,
  StartRecordingInput,
  StopRecordingInput,
} from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import {
  getAudioSources,
  getCaptureSources,
  getRecorderPerformanceStats,
  getRecorderPreviewStatus,
  getRecorderScene,
  getRecorderStatus,
  listenToRecorderPreviewFrames,
  listenToRecorderStatusChanged,
  pauseRecording,
  recorderPreviewFrameEvent,
  recorderStatusChangedEvent,
  resumeRecording,
  saveRecorderScene,
  selectCaptureSources,
  startRecorderPreview,
  startRecording,
  stopRecorderPreview,
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

beforeEach(() => {
  invokeMock.mockReset();
  listenMock.mockReset();
});

test('recorder read helpers use locked camelCase command names', async () => {
  invokeMock.mockResolvedValue({ sources: [] });

  await getCaptureSources();
  await getRecorderScene();
  await getRecorderPreviewStatus();
  await selectCaptureSources();
  await selectCaptureSources({ forceChooser: true });
  await getAudioSources();

  expect(invokeMock).toHaveBeenNthCalledWith(1, 'getCaptureSources');
  expect(invokeMock).toHaveBeenNthCalledWith(2, 'getRecorderScene');
  expect(invokeMock).toHaveBeenNthCalledWith(3, 'getRecorderPreviewStatus');
  expect(invokeMock).toHaveBeenNthCalledWith(4, 'selectCaptureSources');
  expect(invokeMock).toHaveBeenNthCalledWith(5, 'selectCaptureSources', {
    input: { forceChooser: true },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(6, 'getAudioSources');
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
      mediaWriter: {
        available: false,
        capabilities: unavailableMediaWriterCapabilities,
        error: null,
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
    finalization: null,
    lastError: null,
    state: 'unavailable',
    updatedAtMs: 1,
  });

  await getRecorderStatus();

  expect(invokeMock).toHaveBeenCalledWith('getRecorderStatus');
});

test('getRecorderPerformanceStats uses locked camelCase command name', async () => {
  invokeMock.mockResolvedValue({
    desiredFps: null,
    segmentStartedAtMs: null,
    state: 'idle',
    timing: null,
    updatedAtMs: 1,
  });

  await getRecorderPerformanceStats();

  expect(invokeMock).toHaveBeenCalledWith('getRecorderPerformanceStats');
});

test('recorder write helpers pass input payloads unchanged', async () => {
  invokeMock.mockResolvedValue(scene);

  const saveInput: SaveRecorderSceneInput = {
    scene,
  };
  const previewInput: StartRecorderPreviewInput = {
    sceneId: scene.id,
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
  await startRecorderPreview(previewInput);
  await stopRecorderPreview();
  await startRecording(startInput);
  await pauseRecording(pauseInput);
  await resumeRecording(resumeInput);
  await stopRecording(stopInput);

  expect(invokeMock).toHaveBeenNthCalledWith(1, 'saveRecorderScene', {
    input: saveInput,
  });
  expect(invokeMock).toHaveBeenNthCalledWith(2, 'startRecorderPreview', {
    input: previewInput,
  });
  expect(invokeMock).toHaveBeenNthCalledWith(3, 'stopRecorderPreview');
  expect(invokeMock).toHaveBeenNthCalledWith(4, 'startRecording', {
    input: startInput,
  });
  expect(invokeMock).toHaveBeenNthCalledWith(5, 'pauseRecording', {
    input: pauseInput,
  });
  expect(invokeMock).toHaveBeenNthCalledWith(6, 'resumeRecording', {
    input: resumeInput,
  });
  expect(invokeMock).toHaveBeenNthCalledWith(7, 'stopRecording', {
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
      mediaWriter: {
        available: false,
        capabilities: unavailableMediaWriterCapabilities,
        error: null,
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
    finalization: null,
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

test('recorder preview frame event helper listens with locked event name', async () => {
  const unlisten = vi.fn();
  const handler = vi.fn();
  const frame: RecorderPreviewFrame = {
    dataUrl: 'data:image/png;base64,abc',
    height: 180,
    sceneSourceId: 'scene-source-1',
    sourceId: 'monitor-1',
    timestampMs: 1,
    width: 320,
  };
  listenMock.mockImplementation((_eventName, listener) => {
    listener({ payload: frame });
    return Promise.resolve(unlisten);
  });

  await expect(listenToRecorderPreviewFrames(handler)).resolves.toBe(unlisten);

  expect(listenMock).toHaveBeenCalledWith(
    recorderPreviewFrameEvent,
    expect.any(Function),
  );
  expect(handler).toHaveBeenCalledWith(frame);
});
