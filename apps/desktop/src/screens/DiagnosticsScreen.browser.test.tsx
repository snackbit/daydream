import type {
  DaydreamDiagnostics,
  RecorderFfmpegCapabilities,
} from '@daydream/shared';
import { cleanup, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { useDiagnosticsStore } from '@/features/diagnostics/diagnostics-store';
import { DiagnosticsScreen } from './DiagnosticsScreen';

const client = vi.hoisted(() => ({
  getDiagnostics: vi.fn(),
}));

vi.mock('@/features/diagnostics/diagnostics-client', () => client);

const ffmpegCapabilities = {
  libopusEncoder: false,
  libx264Encoder: true,
  matroskaMuxer: true,
  pulseInput: false,
  rawvideoDemuxer: true,
} satisfies RecorderFfmpegCapabilities;

const diagnostics = {
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
  ffmpeg: {
    available: false,
    capabilities: ffmpegCapabilities,
    error:
      'Selected FFmpeg is missing required capabilities: Pulse input, libopus encoder.',
    path: '/usr/bin/ffmpeg',
    source: 'path',
    versionLine: 'ffmpeg version 6.1.1',
  },
  latestRecording: {
    activeCaptureTiming: null,
    activeRecordingId: null,
    activeSegmentId: null,
    error: null,
    latestSegment: null,
    recordingId: null,
    segmentCount: 0,
  },
  recorder: {
    desktopAudio: {
      available: true,
      defaultSourceId: 'desktop',
      error: null,
    },
    ffmpeg: {
      available: false,
      capabilities: ffmpegCapabilities,
      error:
        'Selected FFmpeg is missing required capabilities: Pulse input, libopus encoder.',
      path: '/usr/bin/ffmpeg',
      versionLine: 'ffmpeg version 6.1.1',
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
});

afterEach(() => {
  cleanup();
});

test('DiagnosticsScreen shows required FFmpeg capability support', async () => {
  render(<DiagnosticsScreen />);

  expect(await screen.findByText('Required capabilities')).toBeTruthy();

  const ffmpegCard = screen.getByRole('heading', {
    name: 'FFmpeg',
  }).parentElement;

  if (!ffmpegCard) {
    throw new Error('Expected FFmpeg diagnostics card to render.');
  }

  expect(within(ffmpegCard).getByText('rawvideo demuxer')).toBeTruthy();
  expect(within(ffmpegCard).getByText('Matroska muxer')).toBeTruthy();
  expect(within(ffmpegCard).getByText('libx264 encoder')).toBeTruthy();
  expect(within(ffmpegCard).getByText('Pulse input')).toBeTruthy();
  expect(within(ffmpegCard).getByText('libopus encoder')).toBeTruthy();
  expect(within(ffmpegCard).getAllByText('Supported')).toHaveLength(3);
  expect(within(ffmpegCard).getAllByText('Missing')).toHaveLength(2);
});

test('DiagnosticsScreen shows GPU capture probe details', async () => {
  render(<DiagnosticsScreen />);

  expect(await screen.findByText('GPU capture')).toBeTruthy();

  const recorderCard = screen.getByRole('heading', {
    name: 'Recorder Backend',
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
