import type {
  DaydreamDiagnostics,
  RecorderMediaWriterCapabilities,
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
import { useDiagnosticsStore } from '@/features/diagnostics/diagnostics-store';
import { DiagnosticsScreen } from './DiagnosticsScreen';

const client = vi.hoisted(() => ({
  getDebugLogBundle: vi.fn(),
  getDiagnostics: vi.fn(),
}));

vi.mock('@/features/diagnostics/diagnostics-client', () => client);
vi.mock('@/features/settings/use-debug-enabled', () => ({
  useDebugEnabled: () => true,
}));

const mediaWriterCapabilities = {
  appsrc: true,
  audioConvert: true,
  audioResample: true,
  fileSink: true,
  h264Parse: true,
  matroskaMuxer: true,
  opusEncoder: false,
  pulseSource: false,
  queue: true,
  videoConvert: true,
  x264Encoder: true,
} satisfies RecorderMediaWriterCapabilities;

const diagnostics = {
  app: {
    identifier: 'com.snackbit.daydream',
    name: 'Daydream',
    version: '0.1.0',
  },
  aiProcessing: {
    chunkCount: 0,
    chunkStatusCounts: {},
    chunks: [],
    error: null,
    latestJob: null,
    recordingId: null,
    settings: {
      autoStartAfterRecording: true,
      maxParallelChunksPerJob: 1,
      maxParallelJobs: 1,
      maxParallelOcrJobs: 1,
      maxParallelVlmJobs: 1,
      transcriptionBackend: 'cpu',
      transcriptionLanguage: 'auto',
      transcriptionMaxInitialTimestampSeconds: 3,
      transcriptionMergeGapMs: 700,
      transcriptionMinSpeechMs: 250,
      transcriptionModel: 'large-v3-turbo',
      transcriptionProvider: 'local-whisper-cpp',
      transcriptionSilenceGateEnabled: true,
      transcriptionSilenceThresholdDb: -45,
      wordTimestampsEnabled: true,
    },
    transcriptTracks: [],
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
    latestFileBytes: 256,
    retentionDays: 7,
    stdout: true,
  },
  mediaWriter: {
    available: false,
    capabilities: mediaWriterCapabilities,
    error: 'GStreamer is missing required elements: pulsesrc, opusenc.',
    version: '1.28.3',
    writer: 'gstreamer',
  },
  ocr: {
    dataDownloadStatus: 'missing',
    dataPathPresent: false,
    dataSource: 'missing',
    enabled: true,
    engine: 'tesseract',
    languages: [],
    runtimeDownloadStatus: 'missing',
    runtimePath: null,
    runtimePathPresent: false,
    runtimeSource: 'missing',
    runtimeVersion: null,
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
    createdAtMs: 1000,
    error: null,
    expectedId: 'linux-screencast-monitor',
    lastUsedAtMs: 2000,
    portal: 'linux-screencast',
    present: true,
    tokenLength: 36,
    updatedAtMs: 2000,
  },
  recorder: {
    desktopAudio: {
      available: true,
      defaultSourceId: 'desktop',
      error: null,
    },
    mediaWriter: {
      available: false,
      capabilities: mediaWriterCapabilities,
      error: 'GStreamer is missing required elements: pulsesrc, opusenc.',
      version: '1.28.3',
      writer: 'gstreamer',
    },
    gpuCapture: {
      available: true,
      cleanupError: null,
      dmabufImportAvailable: true,
      eglAvailable: true,
      eglClientApis: ['OpenGL_ES', 'OpenGL'],
      eglPlatform: 'surfaceless',
      eglVendor: 'NVIDIA',
      eglVersion: '1.5',
      error: null,
      modifierImportAvailable: true,
      openglAvailable: true,
      renderer: 'NVIDIA GeForce RTX 4090',
      vendor: 'NVIDIA Corporation',
      version: '4.6.0 NVIDIA',
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
  storage: {
    appDataDir: '/tmp/daydream',
  },
  system: {
    arch: 'x86_64',
    family: 'unix',
    platform: 'linux',
  },
} satisfies DaydreamDiagnostics;

beforeEach(() => {
  vi.resetAllMocks();
  useDiagnosticsStore.setState(useDiagnosticsStore.getInitialState(), true);
  client.getDiagnostics.mockResolvedValue(diagnostics);
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText: vi.fn().mockResolvedValue(undefined),
    },
  });
});

afterEach(() => {
  cleanup();
});

test('DiagnosticsScreen shows required media writer capability support', async () => {
  render(<DiagnosticsScreen />);

  expect(await screen.findByText('Required capabilities')).toBeTruthy();

  const mediaWriterCard = screen.getByRole('heading', {
    name: 'Media writer',
  }).parentElement;

  if (!mediaWriterCard) {
    throw new Error('Expected media writer diagnostics card to render.');
  }

  expect(within(mediaWriterCard).getByText('appsrc')).toBeTruthy();
  expect(within(mediaWriterCard).getByText('Matroska muxer')).toBeTruthy();
  expect(within(mediaWriterCard).getByText('x264enc')).toBeTruthy();
  expect(within(mediaWriterCard).getByText('pulsesrc')).toBeTruthy();
  expect(within(mediaWriterCard).getByText('opusenc')).toBeTruthy();
  expect(within(mediaWriterCard).getAllByText('Supported')).toHaveLength(9);
  expect(within(mediaWriterCard).getAllByText('Missing')).toHaveLength(2);
});

test('DiagnosticsScreen shows GPU capture probe details', async () => {
  render(<DiagnosticsScreen />);

  expect(await screen.findByText('GPU capture')).toBeTruthy();

  const recorderCard = screen.getByRole('heading', {
    name: 'Recorder backend',
  }).parentElement;

  if (!recorderCard) {
    throw new Error('Expected recorder backend diagnostics card to render.');
  }

  expect(within(recorderCard).getByText('GPU capture')).toBeTruthy();
  expect(within(recorderCard).getByText('DMA-BUF')).toBeTruthy();
  expect(within(recorderCard).getByText('surfaceless')).toBeTruthy();
  expect(
    within(recorderCard).getByText('NVIDIA GeForce RTX 4090'),
  ).toBeTruthy();
});

test('DiagnosticsScreen copies raw diagnostics JSON to the clipboard', async () => {
  render(<DiagnosticsScreen />);

  const copyButton = await screen.findByRole('button', { name: /copy json/i });
  fireEvent.click(copyButton);

  const writeText = vi.mocked(navigator.clipboard.writeText);
  expect(writeText).toHaveBeenCalledWith(JSON.stringify(diagnostics, null, 2));
  expect(await screen.findByText('Diagnostics JSON copied.')).toBeTruthy();
});

test('DiagnosticsScreen copies debug log bundle to the clipboard', async () => {
  client.getDebugLogBundle.mockResolvedValue({
    diagnostics,
    logTailLines: ['{"event":"recorder_start_requested"}'],
  });

  render(<DiagnosticsScreen />);

  const copyButton = await screen.findByRole('button', {
    name: 'Copy bundle',
  });
  fireEvent.click(copyButton);

  const writeText = vi.mocked(navigator.clipboard.writeText);
  expect(await screen.findByText('Debug bundle copied.')).toBeTruthy();
  expect(client.getDebugLogBundle).toHaveBeenCalledOnce();
  expect(writeText).toHaveBeenCalledWith(
    expect.stringContaining('recorder_start_requested'),
  );
});

test('DiagnosticsScreen keeps its shell and retries after the initial device probe fails', async () => {
  client.getDiagnostics
    .mockRejectedValueOnce(new Error('Device probe unavailable.'))
    .mockResolvedValueOnce(diagnostics);

  render(<DiagnosticsScreen />);

  expect(await screen.findByText('Could not refresh diagnostics')).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Diagnostics' })).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Try again' }));

  expect(
    await screen.findByRole('heading', { name: 'Runtime overview' }),
  ).toBeTruthy();
  await waitFor(() => expect(client.getDiagnostics).toHaveBeenCalledTimes(2));
});

test('DiagnosticsScreen preserves loaded facts and exposes retry after refresh fails', async () => {
  useDiagnosticsStore.setState({
    diagnostics,
    error: null,
    status: 'ready',
  });
  client.getDiagnostics.mockRejectedValueOnce(
    new Error('Device probe temporarily unavailable.'),
  );

  render(<DiagnosticsScreen />);

  fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));

  expect(
    await screen.findByText('Device probe temporarily unavailable.'),
  ).toBeTruthy();
  expect(
    screen.getByRole('heading', { name: 'Runtime overview' }),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
  expect(
    await screen.findByRole('heading', { name: 'Runtime overview' }),
  ).toBeTruthy();
  await waitFor(() => expect(client.getDiagnostics).toHaveBeenCalledTimes(2));
});
