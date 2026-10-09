import type {
  AppSettingsState,
  InputActivityStatus,
  RecorderScene,
  RecordWindowProtectionSettings,
} from '@daydream/shared';
import { recallSettingsFixture } from '@/features/recall/recall-test-fixtures';

export const defaultAiSettings = {
  recall: recallSettingsFixture(),
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
    observationsEnabled: false,
    provider: 'disabled',
  },
} as const;

export const defaultWindowProtection: RecordWindowProtectionSettings = {
  defaultAction: 'pause',
  dwellMs: 2_000,
  enabled: false,
  mergeGapMs: 5_000,
  rules: [],
};

export const defaultSettingsState = {
  defaults: {
    ai: defaultAiSettings,
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
  },
  limits: {
    ai: {
      activity: {
        keyboardWindowMsMax: 300_000,
        keyboardWindowMsMin: 1_000,
        mouseWindowMsMax: 300_000,
        mouseWindowMsMin: 1_000,
      },
      processing: {
        frameRecognitionFpsMax: 5,
        frameRecognitionFpsMin: 1 / 60,
        maxFullFrameCheckpointsPerMinuteMax: 60,
        maxFullFrameCheckpointsPerMinuteMin: 1,
        maxParallelChunksPerJobMax: 8,
        maxParallelChunksPerJobMin: 1,
        maxParallelJobsMax: 4,
        maxParallelJobsMin: 1,
        maxParallelOcrJobsMax: 8,
        maxParallelOcrJobsMin: 1,
        maxParallelVlmJobsMax: 8,
        maxParallelVlmJobsMin: 1,
        periodicFullFrameIntervalMsMax: 600_000,
        periodicFullFrameIntervalMsMin: 5_000,
        visualDeltaTileSizePxMax: 256,
        visualDeltaTileSizePxMin: 32,
      },
      transcription: {
        maxInitialTimestampSecondsMax: 10,
        maxInitialTimestampSecondsMin: 1,
        mergeGapMsMax: 5_000,
        mergeGapMsMin: 0,
        minSpeechMsMax: 2_000,
        minSpeechMsMin: 50,
        silenceThresholdDbMax: -20,
        silenceThresholdDbMin: -80,
      },
    },
    record: {
      fpsMax: 5,
      fpsMin: 1 / 60,
      videoBitrateKbpsMax: 10_000,
      videoBitrateKbpsMin: 250,
      windowProtectionDwellMsMax: 30_000,
      windowProtectionDwellMsMin: 250,
      windowProtectionMergeGapMsMax: 120_000,
      windowProtectionMergeGapMsMin: 0,
      windowProtectionRuleCountMax: 100,
      windowProtectionTextMaxChars: 160,
    },
    storage: {
      retentionDaysMax: 365,
      retentionDaysMin: 1,
      sizeLimitBytesMax: null,
      sizeLimitBytesMin: 1024 * 1024 * 1024,
    },
  },
  settings: {
    ai: defaultAiSettings,
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
  },
} satisfies AppSettingsState;

export const scene = {
  canvasHeight: 1440,
  canvasWidth: 2560,
  desktopAudioSourceId: 'desktop',
  id: 'scene-1',
  layoutMode: 'horizontal',
  microphoneAudioSourceId: 'mic',
  name: 'Scene',
  sources: [],
} satisfies RecorderScene;

export const aiModels = [
  {
    checksum: '55356645c2b361a969dfd0ef2c5a50d530afd8d5',
    checksumAlgorithm: 'sha1',
    createdAtMs: 0,
    downloadedBytes: 0,
    downloadStatus: 'missing',
    etag: null,
    errorCode: null,
    id: 'small',
    label: 'Whisper small',
    localPath: null,
    provider: 'local-whisper-cpp',
    selfTestStatus: 'not-run',
    sizeBytes: null,
    sourceUrl:
      'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin',
    totalBytes: null,
    updatedAtMs: 0,
    verificationStatus: 'unknown',
  },
] as const;

export const availableInputActivityStatus = {
  available: true,
  candidateDeviceCount: 2,
  currentUser: 'felixf',
  message: 'Keyboard and mouse activity capture can read Linux input devices.',
  permissionDeniedDeviceCount: 0,
  platform: 'linux',
  readableDeviceCount: 2,
  reason: 'available',
  repairAvailable: false,
  userInInputGroup: true,
} satisfies InputActivityStatus;
