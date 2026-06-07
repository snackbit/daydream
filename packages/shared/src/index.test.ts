import { expect, test } from 'vitest';
import type {
  DaydreamDiagnostics,
  RecorderMediaWriterCapabilities,
  RecorderError,
  RecorderPreviewFrame,
  RecorderPreviewStatus,
  RecorderScene,
  RecorderSegment,
  RecorderStatus,
} from './index.js';

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

test('diagnostics shape supports unavailable media writer state', () => {
  const diagnostics: DaydreamDiagnostics = {
    app: {
      identifier: 'com.snackbit.daydream',
      name: 'Daydream',
      version: '0.1.0',
    },
    database: {
      available: true,
      error: null,
      path: '/tmp/daydream/daydream.sqlite3',
      schemaVersion: 1,
      tableCounts: {
        recordings: 0,
      },
    },
    debugLogging: {
      enabled: true,
      filePath: '/tmp/daydream/logs/daydream.log',
      filter: 'daydream_desktop_lib=info,warn',
      format: 'stdout=compact,file=jsonl',
      latestFileBytes: 128,
      retentionDays: 7,
      stdout: true,
    },
    mediaWriter: {
      available: false,
      capabilities: unavailableMediaWriterCapabilities,
      error: 'GStreamer is unavailable',
      version: null,
      writer: null,
    },
    latestRecording: {
      activeCaptureTiming: null,
      activeRecordingId: null,
      activeSegmentId: null,
      error: null,
      latestSegment: null,
      previewStatus: null,
      recordingId: null,
      segmentCount: 0,
    },
    portalRestoreToken: {
      createdAtMs: null,
      error: null,
      expectedId: 'linux-screencast-monitor',
      lastUsedAtMs: null,
      portal: null,
      present: false,
      tokenLength: null,
      updatedAtMs: null,
    },
    recorder: {
      desktopAudio: {
        available: false,
        defaultSourceId: null,
        error: 'not implemented',
      },
      mediaWriter: {
        available: false,
        capabilities: unavailableMediaWriterCapabilities,
        error: 'not implemented',
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
        error: 'not implemented',
        modifierImportAvailable: null,
        openglAvailable: false,
        renderer: null,
        vendor: null,
        version: null,
      },
      microphone: {
        available: false,
        defaultSourceId: null,
        error: 'not implemented',
      },
      pipewire: {
        available: false,
        error: 'not implemented',
      },
      portal: {
        available: false,
        cursorModes: [],
        error: 'not implemented',
        screenCastVersion: null,
        sourceTypes: [],
      },
    },
    storage: {
      appDataDir: null,
    },
    system: {
      arch: 'x86_64',
      family: 'unix',
      platform: 'linux',
    },
  };

  expect(diagnostics.mediaWriter.available).toBe(false);
  expect(diagnostics.debugLogging.enabled).toBe(true);
  expect(diagnostics.portalRestoreToken.present).toBe(false);
});

test('recorder contracts support gate-a unavailable state', () => {
  const error = {
    category: 'unavailable',
    code: 'recorder_backend_not_implemented',
    message: 'Recorder backend is not implemented yet.',
  } satisfies RecorderError;

  const status = {
    activeRecordingId: null,
    activeSceneId: null,
    activeSegmentId: null,
    backendHealth: {
      desktopAudio: {
        available: false,
        defaultSourceId: null,
        error: 'not implemented',
      },
      mediaWriter: {
        available: false,
        capabilities: unavailableMediaWriterCapabilities,
        error: 'not implemented',
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
        error: 'not implemented',
        modifierImportAvailable: null,
        openglAvailable: false,
        renderer: null,
        vendor: null,
        version: null,
      },
      microphone: {
        available: false,
        defaultSourceId: null,
        error: 'not implemented',
      },
      pipewire: {
        available: false,
        error: 'not implemented',
      },
      portal: {
        available: false,
        cursorModes: [],
        error: 'not implemented',
        screenCastVersion: null,
        sourceTypes: [],
      },
    },
    finalization: null,
    lastError: error,
    state: 'unavailable',
    updatedAtMs: 1,
  } satisfies RecorderStatus;

  expect(status.lastError?.code).toBe('recorder_backend_not_implemented');
});

test('recorder contracts support scene and segment metadata', () => {
  const scene = {
    canvasHeight: 1440,
    canvasWidth: 5120,
    desktopAudioSourceId: 'desktop-monitor',
    id: 'scene-1',
    layoutMode: 'horizontal',
    microphoneAudioSourceId: 'mic',
    name: 'Workday',
    sources: [
      {
        crop: {
          bottom: 0,
          left: 0,
          right: 0,
          top: 0,
        },
        displayName: 'Display 1',
        id: 'source-1',
        preserveAspectRatio: true,
        rect: {
          height: 1440,
          width: 2560,
          x: 0,
          y: 0,
        },
        sourceId: 'monitor-1',
        sourceHeight: 1440,
        sourceKind: 'monitor',
        sourceWidth: 2560,
        zIndex: 0,
      },
    ],
  } satisfies RecorderScene;

  const segment = {
    desktopAudioStreamIndex: 1,
    durationMs: null,
    endedAtMs: null,
    errorCode: null,
    writerExitCode: null,
    filePath: '/tmp/daydream/recordings/r1/segments/0001.mkv',
    id: 'segment-1',
    microphoneAudioStreamIndex: 2,
    recordingId: 'recording-1',
    segmentIndex: 1,
    startedAtMs: 1,
    state: 'recording',
    videoStreamIndex: 0,
  } satisfies RecorderSegment;

  expect(scene.sources[0]?.sourceKind).toBe('monitor');
  expect(segment.videoStreamIndex).toBe(0);
});

test('recorder contracts support transient live preview metadata', () => {
  const status = {
    error: null,
    sceneId: 'scene-1',
    sourceIds: ['monitor-1'],
    state: 'live',
    updatedAtMs: 1,
  } satisfies RecorderPreviewStatus;
  const frame = {
    dataUrl: 'data:image/png;base64,abc',
    height: 180,
    sceneSourceId: 'source-1',
    sourceId: 'monitor-1',
    timestampMs: 2,
    width: 320,
  } satisfies RecorderPreviewFrame;

  expect(status.state).toBe('live');
  expect(frame.dataUrl).toMatch(/^data:image\/png;base64,/);
});
