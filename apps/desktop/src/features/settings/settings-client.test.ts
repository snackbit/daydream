import type { SaveAppSettingsInput } from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import {
  getAppSettings,
  previewAppSettingsStorageCleanup,
  resetAppSettings,
  saveAppSettings,
} from './settings-client';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockReset();
});

test('settings client uses typed camelCase command names', async () => {
  const input: SaveAppSettingsInput = {
    settings: {
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
          frameRecognitionFps: 0.2,
          frameRecognitionSyncWithRecording: true,
          frameSimilarityEnabled: true,
          maxFullFrameCheckpointsPerMinute: 2,
          maxParallelChunksPerJob: 1,
          maxParallelJobs: 1,
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
      },
      storage: {
        retentionDays: 7,
        sizeLimitBytes: 50 * 1024 * 1024 * 1024,
      },
    },
  };
  invokeMock.mockResolvedValue({ settings: input.settings });

  await getAppSettings();
  await previewAppSettingsStorageCleanup(input);
  await saveAppSettings(input);
  await resetAppSettings();

  expect(invokeMock).toHaveBeenNthCalledWith(1, 'getAppSettings');
  expect(invokeMock).toHaveBeenNthCalledWith(
    2,
    'previewAppSettingsStorageCleanup',
    {
      input,
    },
  );
  expect(invokeMock).toHaveBeenNthCalledWith(3, 'saveAppSettings', {
    input,
  });
  expect(invokeMock).toHaveBeenNthCalledWith(4, 'resetAppSettings');
});
