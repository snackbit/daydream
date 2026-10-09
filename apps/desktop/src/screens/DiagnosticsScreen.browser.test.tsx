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
import { page, userEvent } from 'vitest/browser';
import { useDiagnosticsStore } from '@/features/diagnostics/diagnostics-store';
import { useSettingsStore } from '@/features/settings/settings-store';
import { defaultSettingsState } from '@/features/settings/testing/settings-browser-fixtures';
import '@/index.css';
import { DiagnosticsScreen } from './DiagnosticsScreen';

const client = vi.hoisted(() => ({
  getDebugLogBundle: vi.fn(),
  getDiagnostics: vi.fn(),
}));

vi.mock('@/features/diagnostics/diagnostics-client', () => client);

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
  useSettingsStore.setState(
    {
      ...useSettingsStore.getInitialState(),
      draft: { ...defaultSettingsState.settings, debug: { enabled: true } },
      loadStatus: 'ready',
    },
    true,
  );
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

  await userEvent.click(
    within(mediaWriterCard).getByRole('button', { name: /^Video output/ }),
  );
  await userEvent.click(
    within(mediaWriterCard).getByRole('button', {
      name: /^Audio input and encoding/,
    }),
  );

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

test('DIAG-01 clipboard rejection is visible and copying can recover with loaded facts intact', async () => {
  const writeText = vi.mocked(navigator.clipboard.writeText);
  writeText.mockRejectedValueOnce(new Error('Clipboard access denied.'));
  render(<DiagnosticsScreen />);
  await screen.findByRole('heading', { name: 'Runtime overview' });
  await page.getByRole('button', { name: 'Copy JSON', exact: true }).click();
  expect(
    await screen.findByText('Unable to copy diagnostics JSON.'),
  ).toBeTruthy();
  expect(
    screen.getByRole('heading', { name: 'Runtime overview' }),
  ).toBeTruthy();
  await page.getByRole('button', { name: 'Copy JSON', exact: true }).click();
  expect(await screen.findByText('Diagnostics JSON copied.')).toBeTruthy();
  expect(writeText).toHaveBeenLastCalledWith(
    JSON.stringify(diagnostics, null, 2),
  );
});

test('DIAG-02 a failed bundle request does not copy partial data and can be retried', async () => {
  client.getDebugLogBundle
    .mockRejectedValueOnce(new Error('Log file is busy.'))
    .mockResolvedValueOnce({
      diagnostics,
      logTailLines: ['{"event":"synthetic_probe"}'],
    });
  render(<DiagnosticsScreen />);
  await screen.findByRole('heading', { name: 'Runtime overview' });
  await page.getByRole('button', { name: 'Copy bundle', exact: true }).click();
  expect(await screen.findByText('Unable to copy debug bundle.')).toBeTruthy();
  expect(navigator.clipboard.writeText).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Copy bundle', exact: true }).click();
  expect(await screen.findByText('Debug bundle copied.')).toBeTruthy();
  expect(navigator.clipboard.writeText).toHaveBeenCalledOnce();
  expect(client.getDebugLogBundle).toHaveBeenCalledTimes(2);
});

test('DIAG-03 raw diagnostics disclosure can be opened and closed without a new device probe', async () => {
  render(<DiagnosticsScreen />);
  await screen.findByText('Raw diagnostics JSON');
  const trigger = screen.getByRole('button', { name: 'Raw diagnostics JSON' });
  expect(trigger.getAttribute('aria-expanded')).toBe('false');
  await userEvent.click(trigger);
  expect(trigger.getAttribute('aria-expanded')).toBe('true');
  expect(
    await screen.findByText(/"identifier": "com.snackbit.daydream"/),
  ).toBeTruthy();
  await userEvent.click(trigger);
  expect(trigger.getAttribute('aria-expanded')).toBe('false');
  await waitFor(() =>
    expect(
      screen.queryByText(/"identifier": "com.snackbit.daydream"/),
    ).toBeNull(),
  );
  expect(client.getDiagnostics).toHaveBeenCalledOnce();
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

test.each(['light', 'dark'])(
  'diagnostic actions fit %s across desktop and narrow widths',
  async (theme) => {
    document.documentElement.dataset.theme = theme;
    render(<DiagnosticsScreen />);
    await screen.findByRole('heading', { name: 'Runtime overview' });
    for (const width of [1440, 1024, 768, 390, 320]) {
      await page.viewport(width, 900);
      for (const name of ['Copy bundle', 'Copy JSON', 'Refresh']) {
        const bounds = screen
          .getByRole('button', { name })
          .getBoundingClientRect();
        expect(bounds.left).toBeGreaterThanOrEqual(0);
        expect(bounds.right).toBeLessThanOrEqual(width);
      }
      expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(width);
      await page.screenshot();
    }
    delete document.documentElement.dataset.theme;
    await page.viewport(1440, 960);
  },
);
