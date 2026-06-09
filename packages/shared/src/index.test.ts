import { expect, test } from 'vitest';
import type {
  DaydreamDiagnostics,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingSummary,
  LibraryWaveformTiles,
  LibraryWaveforms,
  RecorderMediaWriterCapabilities,
  RecorderError,
  RecorderPreviewFrame,
  RecorderPreviewStatus,
  RecorderScene,
  RecorderSegment,
  RecorderStatus,
  ProcessingJob,
  RecordingTranscript,
  RecordingVisualObservations,
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

test('ai contracts support local baseline results and guarded vlm states', () => {
  const migratedJob = {
    createdAtMs: 1,
    errorCode: null,
    id: 'job-1',
    kind: 'frame-extraction',
    progress: 0,
    recordingId: 'recording-1',
    status: 'queued',
    updatedAtMs: 1,
  } satisfies ProcessingJob;
  const transcript = {
    recordingId: 'recording-1',
    segments: [
      {
        chunkId: 'chunk-1',
        confidence: 0.9,
        endOffsetMs: 1_000,
        id: 'segment-1',
        jobId: 'job-1',
        model: 'small',
        provider: 'local-whisper-cpp',
        recordingId: 'recording-1',
        segmentId: 'media-segment-1',
        sourceRole: 'microphone',
        startOffsetMs: 0,
        text: 'hello',
        trackId: 'track-1',
        words: [
          {
            endOffsetMs: 500,
            id: 'word-1',
            probability: 0.95,
            startOffsetMs: 0,
            text: 'hello',
            transcriptSegmentId: 'segment-1',
          },
        ],
      },
    ],
  } satisfies RecordingTranscript;
  const visual = {
    ocrRegions: [
      {
        chunkId: 'chunk-2',
        confidence: 0.88,
        frameTimestampMs: 500,
        id: 'ocr-1',
        jobId: 'job-2',
        model: 'tesseract',
        provider: 'tesseract',
        recordingId: 'recording-1',
        region: {
          height: 20,
          width: 100,
          x: 10,
          y: 20,
        },
        sourceFrameRef: 'segment-1@500',
        text: 'Daydream',
      },
    ],
    recordingId: 'recording-1',
    vlmObservations: [],
  } satisfies RecordingVisualObservations;

  expect(migratedJob.kind).toBe('frame-extraction');
  expect(transcript.segments[0]!.provider).toBe('local-whisper-cpp');
  expect(visual.vlmObservations).toHaveLength(0);
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

test('library contracts support missing recordings and playable details', () => {
  const missing = {
    availability: 'missing-files',
    durationMs: 12_000,
    endedAtMs: 13_000,
    id: 'recording-1',
    missingReasons: ['Segment 1 video file is missing.'],
    playableSegmentCount: 1,
    segmentCount: 1,
    sizeBytes: 1_024,
    startedAtMs: 1_000,
    status: 'completed',
    storageDir: '/tmp/daydream/recordings/recording-1',
    thumbnailPath: null,
  } satisfies LibraryRecordingSummary;

  const detail = {
    ...missing,
    availability: 'available',
    missingReasons: [],
    segments: [
      {
        desktopAudioStreamIndex: 1,
        durationMs: 4_000,
        endedAtMs: 5_000,
        fileExists: true,
        filePath: '/tmp/daydream/recordings/recording-1/segments/0001.mkv',
        id: 'segment-1',
        microphoneAudioStreamIndex: 2,
        pauseAfterMs: 3_000,
        playbackOffsetMs: 0,
        segmentIndex: 1,
        startedAtMs: 1_000,
        state: 'finalized',
        videoStreamIndex: 0,
      },
    ],
    thumbnailPath: '/tmp/daydream/recordings/recording-1/segments/0001.mkv',
    tracks: [
      {
        codec: 'h264',
        description: 'Composited screen recording',
        id: 'video-0',
        kind: 'video',
        label: 'Video',
        streamIndex: 0,
      },
    ],
  } satisfies LibraryRecordingDetail;

  const locked = {
    ...missing,
    availability: 'locked',
    missingReasons: ['Recording is still in progress.'],
    status: 'recording',
  } satisfies LibraryRecordingSummary;

  expect(missing.availability).toBe('missing-files');
  expect(locked.availability).toBe('locked');
  expect(detail.segments[0]?.pauseAfterMs).toBe(3_000);
});

test('library waveform contracts support audio lanes', () => {
  const waveforms = {
    error: null,
    recordingId: 'recording-1',
    tracks: [
      {
        kind: 'desktop-audio',
        segments: [
          {
            generatedAtMs: 2_000,
            maxAmplitude: 1,
            samples: [0.2, 0.6, 0.4],
            segmentId: 'segment-1',
            streamIndex: 1,
          },
        ],
        trackId: 'desktop-audio',
      },
    ],
  } satisfies LibraryWaveforms;

  expect(waveforms.tracks[0]?.segments[0]?.samples).toHaveLength(3);
});

test('library waveform tile contracts support lazy peak and rms lanes', () => {
  const waveformTiles = {
    binMs: 10,
    error: null,
    recordingId: 'recording-1',
    tracks: [
      {
        kind: 'desktop-audio',
        scaleAmplitude: 0.9,
        tiles: [
          {
            binMs: 10,
            durationMs: 2_000,
            generatedAtMs: 2_000,
            peakSamples: [0.2, 0.8, 0.4],
            rmsSamples: [0.1, 0.5, 0.2],
            segmentId: 'segment-1',
            streamIndex: 1,
            timelineStartMs: 0,
          },
        ],
        trackId: 'desktop-audio',
      },
    ],
  } satisfies LibraryWaveformTiles;

  expect(waveformTiles.tracks[0]?.tiles[0]?.peakSamples).toHaveLength(3);
  expect(waveformTiles.tracks[0]?.tiles[0]?.rmsSamples).toHaveLength(3);
});

test('library playback contracts support tokenized streaming URLs', () => {
  const assets = {
    error: null,
    recordingId: 'recording-1',
    segments: [
      {
        audio: [
          {
            kind: 'desktop-audio',
            streamIndex: 1,
            trackId: 'desktop-audio-1',
            url: 'http://127.0.0.1:5000/playback/session/token/segments/segment-1/audio/desktop-audio-1',
          },
        ],
        segmentId: 'segment-1',
        thumbnails: [
          {
            offsetMs: 0,
            url: 'http://127.0.0.1:5000/playback/session/token/segments/segment-1/thumbnail/0.jpg',
          },
        ],
        videoUrl:
          'http://127.0.0.1:5000/playback/session/token/segments/segment-1/video',
      },
    ],
    sessionId: 'session',
  } satisfies LibraryPlaybackAssets;

  expect(assets.segments[0]?.audio[0]?.trackId).toBe('desktop-audio-1');
  expect(assets.segments[0]?.videoUrl).toContain('/playback/session/token/');
});
