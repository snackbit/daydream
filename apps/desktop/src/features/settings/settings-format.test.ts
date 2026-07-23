import type {
  AppSettings,
  RecorderScene,
  RecordWindowProtectionSettings,
} from '@daydream/shared';
import { expect, test } from 'vitest';
import {
  estimateOneHourRecordingBytes,
  formatFps,
  formatSceneDimensions,
  formatStorageSize,
  gibToBytes,
} from './settings-format';

const defaultWindowProtection: RecordWindowProtectionSettings = {
  defaultAction: 'pause',
  dwellMs: 2_000,
  enabled: false,
  mergeGapMs: 5_000,
  rules: [],
};

const settings = {
  ai: {
    activity: {
      frameDeltas: {
        enabled: true,
      },
      keyboard: {
        enabled: false,
        windowMs: 30_000,
      },
      mouse: {
        enabled: false,
        windowMs: 5_000,
      },
      windows: {
        enabled: true,
      },
    },
    ocr: {
      enabled: true,
      engine: 'tesseract',
    },
    processing: {
      autoStartAfterRecording: true,
      frameRecognitionFps: 0.2,
      frameRecognitionSyncWithRecording: true,
      frameSimilarityEnabled: true,
      maxFullFrameCheckpointsPerMinute: 2,
      maxParallelChunksPerJob: 1,
      maxParallelJobs: 1,
      maxParallelOcrJobs: 1,
      maxParallelVlmJobs: 1,
      periodicFullFrameIntervalMs: 60_000,
      visualDeltaTileSizePx: 64,
    },
    transcription: {
      language: 'auto',
      localBackend: 'cpu',
      localModelId: 'small',
      maxInitialTimestampSeconds: 3,
      mergeGapMs: 700,
      minSpeechMs: 250,
      provider: 'local-whisper-cpp',
      remoteApiKeyRef: null,
      remoteBaseUrl: null,
      remoteModel: null,
      silenceGateEnabled: true,
      silenceThresholdDb: -45,
      wordTimestampsEnabled: true,
    },
    vision: {
      apiKeyRef: null,
      baseUrl: null,
      model: null,
      provider: 'disabled',
    },
  },
  debug: {
    enabled: false,
  },
  expert: {
    enabled: false,
  },
  record: {
    fps: 0.2,
    videoBitrateKbps: 1_500,
    windowProtection: defaultWindowProtection,
  },
  storage: {
    retentionDays: 7,
    sizeLimitBytes: 50 * 1024 * 1024 * 1024,
  },
} satisfies AppSettings;

const scene = {
  canvasHeight: 1440,
  canvasWidth: 2560,
  desktopAudioSourceId: 'desktop',
  id: 'scene-1',
  layoutMode: 'horizontal',
  microphoneAudioSourceId: 'mic',
  name: 'Scene',
  sources: [],
} satisfies RecorderScene;

test('settings formatters describe cadence and scene dimensions', () => {
  expect(formatFps(0.2)).toBe('1 frame every 5 seconds');
  expect(formatFps(5)).toBe('5 frames per second');
  expect(formatSceneDimensions(scene)).toBe('2560 x 1440');
  expect(formatSceneDimensions(null)).toBe('No saved scene');
});

test('one hour estimate includes selected audio streams', () => {
  expect(formatStorageSize(estimateOneHourRecordingBytes(settings, null))).toBe(
    '643.73 MiB',
  );
  expect(
    formatStorageSize(estimateOneHourRecordingBytes(settings, scene)),
  ).toBe('712.39 MiB');
  expect(gibToBytes(50)).toBe(50 * 1024 * 1024 * 1024);
});
