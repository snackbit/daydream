import type { AppSettings, RecorderScene } from '@daydream/shared';
import { expect, test } from 'vitest';
import {
  estimateOneHourRecordingBytes,
  formatFps,
  formatSceneDimensions,
  formatStorageSize,
  gibToBytes,
} from './settings-format';

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
    },
    ocr: {
      enabled: true,
      engine: 'tesseract',
    },
    processing: {
      autoStartAfterRecording: true,
      frameSimilarityEnabled: true,
      maxParallelChunksPerJob: 1,
      maxParallelJobs: 1,
      periodicFullFrameIntervalMs: 300_000,
    },
    transcription: {
      localBackend: 'cpu',
      localModelId: 'small',
      provider: 'local-whisper-cpp',
      remoteApiKeyRef: null,
      remoteBaseUrl: null,
      remoteModel: null,
      wordTimestampsEnabled: true,
    },
    vision: {
      apiKeyRef: null,
      baseUrl: null,
      model: null,
      provider: 'disabled',
    },
  },
  record: {
    fps: 0.2,
    videoBitrateKbps: 1_500,
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
