import type {
  CaptureSetup,
  RecorderPreviewStatus,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { cleanup } from '@testing-library/react';
import { vi } from 'vitest';
import { page } from 'vitest/browser';
import * as recorderClient from '@/features/recorder/recorder-client';
import { useRecorderWorkspaceStore } from '@/features/recorder/recorder-workspace-store';
import * as sourceClient from '@/features/recorder/source-capture-client';

export const client = vi.mocked(recorderClient);
export const setupClient = vi.mocked(sourceClient);
export const savedScene: RecorderScene = {
  independentVideo: true,
  canvasHeight: 900,
  canvasWidth: 1440,
  desktopAudioSourceId: null,
  microphoneAudioSourceId: null,
  id: 'synthetic-scene',
  name: 'Synthetic source setup',
  layoutMode: 'custom',
  sources: [
    {
      id: 'synthetic-source',
      sourceId: 'synthetic-monitor',
      displayName: 'Synthetic monitor',
      sourceKind: 'monitor',
      sourceWidth: 1440,
      sourceHeight: 900,
      preserveAspectRatio: true,
      zIndex: 0,
      crop: { left: 0, top: 0, right: 0, bottom: 0 },
      rect: { x: 0, y: 0, width: 1440, height: 900 },
    },
  ],
};
export const readyStatus: RecorderStatus = {
  activeRecordingId: null,
  activeSceneId: null,
  activeSegmentId: null,
  finalization: null,
  lastError: null,
  state: 'idle',
  updatedAtMs: 1,
  backendHealth: {
    desktopAudio: { available: true, defaultSourceId: null, error: null },
    microphone: { available: true, defaultSourceId: null, error: null },
    pipewire: { available: true, error: null },
    portal: {
      available: true,
      error: null,
      cursorModes: ['embedded'],
      screenCastVersion: 5,
      sourceTypes: ['monitor'],
    },
    mediaWriter: {
      available: true,
      error: null,
      version: null,
      writer: null,
      capabilities: {
        appsrc: true,
        audioConvert: true,
        audioResample: true,
        fileSink: true,
        h264Parse: true,
        matroskaMuxer: true,
        opusEncoder: true,
        pulseSource: true,
        queue: true,
        videoConvert: true,
        x264Encoder: true,
      },
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
  },
};

export async function resetCaptureFixture() {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  useRecorderWorkspaceStore.setState(
    useRecorderWorkspaceStore.getInitialState(),
    true,
  );
  client.getRecorderStatus.mockResolvedValue(readyStatus);
  client.getRecorderScene.mockResolvedValue(structuredClone(savedScene));
  client.getCaptureSources.mockResolvedValue({
    sources: [
      {
        id: 'synthetic-monitor',
        displayName: 'Synthetic monitor',
        kind: 'monitor',
        width: 1440,
        height: 900,
        logicalWidth: 1440,
        logicalHeight: 900,
        scaleFactor: 1,
      },
    ],
  });
  client.getAudioSources.mockResolvedValue({ sources: [] });
  client.getRecorderRecordingStats.mockResolvedValue(null);
  const previewIdle: RecorderPreviewStatus = {
    state: 'idle',
    error: null,
    sceneId: null,
    sourceIds: [],
    updatedAtMs: 1,
  };
  client.getRecorderPreviewStatus.mockResolvedValue(previewIdle);
  client.stopRecorderPreview.mockResolvedValue(previewIdle);
  client.startRecorderPreview.mockImplementation(async ({ sceneId }) => ({
    state: 'live',
    error: null,
    sceneId,
    sourceIds: ['synthetic-monitor'],
    updatedAtMs: 2,
  }));
  client.listenToRecorderPreviewFrames.mockResolvedValue(vi.fn());
  client.listenToRecorderStatusChanged.mockResolvedValue(vi.fn());
  setupClient.getCaptureSetup.mockResolvedValue({
    scene: structuredClone(savedScene),
    fps: 0.2,
    videoBitrateKbps: 2000,
  });
  setupClient.saveCaptureSetup.mockImplementation(async (input) => input);
}

export function cleanupCaptureFixture() {
  cleanup();
  useRecorderWorkspaceStore.setState(
    useRecorderWorkspaceStore.getInitialState(),
    true,
  );
}

export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

export function savedSetup(): CaptureSetup {
  const call = setupClient.saveCaptureSetup.mock.calls.at(-1)?.[0];
  if (!call) throw new Error('Expected a submitted source setup');
  return structuredClone(call);
}
