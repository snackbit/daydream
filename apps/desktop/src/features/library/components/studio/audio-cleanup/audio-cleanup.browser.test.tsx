import { useCallback } from 'react';
import type * as TauriCore from '@tauri-apps/api/core';
import type {
  AudioCleanupState,
  ApplyAudioCleanupInput,
  LibraryRecordingDetail,
} from '@daydream/shared';
import { cleanup, render, fireEvent } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import { studioStoryRecording } from '../../../recording-studio-route/stories/studio-route-fixture';
import { ReprocessingDialog } from '../reprocessing/reprocessing-dialog';
import { useStudioReprocessing } from '../reprocessing/use-studio-reprocessing';
import { TrackLaneHeader } from '../track-lane-header';
import { AudioCleanupDialog } from './audio-cleanup-dialog';
import { useAudioCleanup } from './use-audio-cleanup';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
let saved: AudioCleanupState;
let failApply = false;
let completeCleanup = true;
const pause = vi.fn(),
  refresh = vi.fn(),
  restore = vi.fn();
function Harness({
  detail = studioStoryRecording,
  unavailableReason = null,
}: {
  detail?: LibraryRecordingDetail;
  unavailableReason?: string | null;
}) {
  const range = useCallback(
    () => ({ startOffsetMs: 1000, endOffsetMs: 6000 }),
    [],
  );
  const controller = useAudioCleanup({
    detail,
    readRange: range,
    pausePlayback: pause,
    refreshPlayback: refresh,
    restorePlayback: restore,
  });
  const reprocessing = useStudioReprocessing(studioStoryRecording.id);
  function regenerate() {
    reprocessing.open([{ trackId: 'microphone-2', ranges: null }]);
    reprocessing.operations(['transcription']);
  }
  function reprocess() {
    regenerate();
  }
  function ignored() {}
  return (
    <>
      <TrackLaneHeader
        audioTrack
        lane={{
          label: 'Microphone',
          description: 'Synthetic source',
          trackId: 'microphone-2',
          kind: 'microphone',
          colorId: 'green',
        }}
        dragHandleProps={{}}
        muted={false}
        selectedLane={false}
        trackId="microphone-2"
        onSelectLane={ignored}
        onToggleTrackMuted={ignored}
        onReprocess={reprocess}
        onAudioCleanup={controller.open}
      />
      <AudioCleanupDialog
        controller={controller}
        onRegenerate={regenerate}
        unavailableReason={unavailableReason}
      />
      <ReprocessingDialog controller={reprocessing} />
    </>
  );
}
beforeEach(() => {
  saved = {
    recordingId: studioStoryRecording.id,
    config: { mode: 'off', matchThreshold: 0.65, amount: 1 },
    pendingConfig: null,
    revision: null,
    phase: 'idle',
    progress: 0,
    matchedMs: 0,
    referenceAvailable: true,
    transcriptsOutdated: false,
    error: null,
  };
  failApply = false;
  completeCleanup = true;
  pause.mockClear();
  refresh.mockClear();
  restore.mockClear();
  native.invoke.mockReset();
  native.invoke.mockImplementation(
    async (
      command: string,
      args?: { input?: ApplyAudioCleanupInput; recordingId?: string },
    ) => {
      if (command === 'getAudioCleanup') {
        if (saved.phase === 'running' && completeCleanup)
          saved = {
            ...saved,
            config: saved.pendingConfig ?? saved.config,
            pendingConfig: null,
            phase: 'ready',
            progress: 1,
            revision: 'cleaned-1',
            matchedMs: 4000,
            transcriptsOutdated: true,
          };
        return saved;
      }
      if (command === 'applyAudioCleanup') {
        if (failApply) {
          failApply = false;
          throw new Error('Synthetic failure');
        }
        if (!args?.input) throw new Error('Missing cleanup input');
        if (args.input.config.mode === 'off')
          saved = {
            ...saved,
            config: args.input.config,
            pendingConfig: null,
            revision: null,
            phase: 'idle',
          };
        else
          saved = {
            ...saved,
            pendingConfig: args.input.config,
            phase: 'running',
            progress: 0.2,
          };
        return saved;
      }
      if (command === 'previewAudioCleanup')
        return {
          recordingId: saved.recordingId,
          sessionId: 'comparison-1',
          originalUrl:
            'http://127.0.0.1:54321/playback/session/token/segments/original/audio/microphone-0',
          cleanedUrl:
            'http://127.0.0.1:54321/playback/session/token/segments/cleaned/audio/microphone-0',
          matchedMs: 4000,
          durationMs: 5000,
        };
      if (command === 'closeLibraryPlaybackSession') return null;
      if (command === 'startRecordingReprocessing')
        return {
          id: 'synthetic-job',
          recordingId: saved.recordingId,
          kind: 'transcription',
          status: 'queued',
          createdAtMs: 1,
          updatedAtMs: 1,
          errorCode: null,
          progress: 0,
        };
      if (command === 'cancelAudioCleanup') {
        saved = { ...saved, phase: 'cancelled' };
        return saved;
      }
      if (command === 'previewRecordingReprocessing')
        return {
          recordingId: saved.recordingId,
          planFingerprint: 'synthetic-plan',
          blockedReason: null,
          operations: [
            {
              operation: 'transcription',
              enabled: true,
              remote: false,
              providerLabel: 'Local Whisper',
              blockedReason: null,
            },
          ],
          scopes: [
            {
              id: 'microphone-scope',
              trackId: 'microphone-2',
              sourceLabel: 'Microphone',
              operation: 'transcription',
              startOffsetMs: 0,
              endOffsetMs: 10_000,
            },
          ],
        };
      throw new Error(`Unexpected synthetic IPC: ${command}`);
    },
  );
});
afterEach(() => {
  cleanup();
  document.documentElement.removeAttribute('data-theme');
});
async function open() {
  await userEvent.click(
    page.getByRole('button', { name: 'Microphone actions' }),
  );
  await userEvent.click(
    page.getByRole('menuitem', { name: 'Remove speaker bleed…' }),
  );
  await expect
    .element(page.getByRole('combobox', { name: 'Cancellation' }))
    .toBeEnabled();
}
async function auto() {
  await userEvent.click(page.getByRole('combobox', { name: 'Cancellation' }));
  await userEvent.click(
    page.getByRole('option', { name: 'Auto', exact: true }),
  );
}
test('microphone menu previews tuning, applies cleanup, persists, and offers explicit transcription', async () => {
  render(<Harness />);
  await open();
  await auto();
  await userEvent.click(page.getByRole('button', { name: 'Advanced tuning' }));
  await userEvent.click(
    page.getByRole('slider', { name: 'Cancellation amount' }),
  );
  await userEvent.keyboard(
    '{Home}{ArrowRight}{ArrowRight}{ArrowRight}{ArrowRight}{ArrowRight}{ArrowRight}{ArrowRight}{ArrowRight}',
  );
  await userEvent.click(
    page.getByRole('button', { name: 'Preview selected range' }),
  );
  await expect
    .element(page.getByRole('tab', { name: 'Cleaned', exact: true }))
    .toBeVisible();
  await userEvent.click(
    page.getByRole('tab', { name: 'Original', exact: true }),
  );
  await expect
    .element(page.getByRole('tab', { name: 'Original', exact: true }))
    .toHaveAttribute('aria-selected', 'true');
  expect(pause).toHaveBeenCalled();
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === 'applyAudioCleanup',
    ),
  ).toBe(false);
  await userEvent.click(
    page.getByRole('button', { name: 'Apply', exact: true }),
  );
  await expect
    .element(page.getByText('Cleaned microphone audio is ready for playback.'))
    .toBeVisible();
  expect(refresh).toHaveBeenCalled();
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === 'startRecordingReprocessing',
    ),
  ).toBe(false);
  await userEvent.click(
    page.getByRole('button', { name: 'Regenerate microphone transcripts' }),
  );
  await expect
    .element(page.getByRole('button', { name: 'Reprocess', exact: true }))
    .toBeEnabled();
  await expect
    .element(page.getByText('Local Whisper', { exact: false }))
    .toBeVisible();
  await userEvent.click(
    page.getByRole('button', { name: 'Reprocess', exact: true }),
  );
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === 'startRecordingReprocessing',
    ),
  ).toBe(true);
  cleanup();
  render(<Harness />);
  await open();
  await expect
    .element(page.getByRole('combobox', { name: 'Cancellation' }))
    .toHaveTextContent('Auto');
  await userEvent.click(page.getByRole('combobox', { name: 'Cancellation' }));
  await userEvent.click(page.getByRole('option', { name: 'Off', exact: true }));
  await userEvent.click(
    page.getByRole('button', { name: 'Apply', exact: true }),
  );
  expect(saved.revision).toBeNull();
});
test('failed apply keeps tuning and retries in a narrow dark dialog', async () => {
  await page.viewport(390, 740);
  document.documentElement.setAttribute('data-theme', 'dark');
  render(<Harness />);
  await open();
  await auto();
  failApply = true;
  await userEvent.click(
    page.getByRole('button', { name: 'Apply', exact: true }),
  );
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Cleanup could not start');
  await expect
    .element(page.getByRole('combobox', { name: 'Cancellation' }))
    .toHaveTextContent('Auto');
  await userEvent.click(
    page.getByRole('button', { name: 'Apply', exact: true }),
  );
  await expect
    .element(page.getByText('Cleaned microphone audio is ready for playback.'))
    .toBeVisible();
  expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(390);
  await page.screenshot({ path: '__screenshots__/audio-cleanup-dark-390.png' });
  await page.viewport(1440, 960);
});

test('sensitivity and threshold stay linked, cancellation can stop and retry', async () => {
  completeCleanup = false;
  render(<Harness />);
  await open();
  await auto();
  await userEvent.click(
    page.getByRole('slider', { name: 'Speaker bleed sensitivity' }),
  );
  await userEvent.keyboard('{Home}{ArrowRight}{ArrowRight}{ArrowRight}');
  await userEvent.click(page.getByRole('button', { name: 'Advanced tuning' }));
  await userEvent.click(page.getByRole('slider', { name: 'Match threshold' }));
  await userEvent.keyboard('{Home}{ArrowRight}{ArrowRight}');
  await userEvent.click(
    page.getByRole('button', { name: 'Apply', exact: true }),
  );
  await expect
    .element(page.getByRole('button', { name: 'Cancel cleanup' }))
    .toBeEnabled();
  const base = native.invoke.getMockImplementation();
  let failCancel = true;
  native.invoke.mockImplementation((command, args) => {
    if (command === 'cancelAudioCleanup' && failCancel) {
      failCancel = false;
      return Promise.reject(new Error('Synthetic cancellation failure'));
    }
    return base?.(command, args);
  });
  await userEvent.click(page.getByRole('button', { name: 'Cancel cleanup' }));
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Cleanup is still stopping');
  native.invoke.mockImplementation((command, args) => base?.(command, args));
  await userEvent.click(page.getByRole('button', { name: 'Cancel cleanup' }));
  await expect
    .element(page.getByRole('button', { name: 'Retry cleanup' }))
    .toBeEnabled();
  completeCleanup = true;
  await userEvent.click(page.getByRole('button', { name: 'Retry cleanup' }));
  await expect
    .element(page.getByText('Cleaned microphone audio is ready for playback.'))
    .toBeVisible();
  await userEvent.keyboard('{Escape}');
  await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
});

test.each(['cancelled', 'failed', 'invalidated'] as const)(
  'saved %s cleanup explains recovery and keeps the original choice',
  async (phase) => {
    saved = {
      ...saved,
      phase,
      config:
        phase === 'invalidated'
          ? { ...saved.config, mode: 'auto' }
          : saved.config,
      pendingConfig:
        phase === 'invalidated' ? null : { ...saved.config, mode: 'auto' },
      revision: phase === 'invalidated' ? 'previous-cleanup' : null,
      transcriptsOutdated: phase === 'invalidated',
    };
    render(<Harness />);
    await open();
    await expect.element(page.getByRole('dialog')).toBeVisible();
    if (phase === 'invalidated')
      await expect
        .element(
          page.getByRole('button', {
            name: 'Regenerate microphone transcripts',
          }),
        )
        .toBeDisabled();
    else
      await expect
        .element(page.getByRole('button', { name: 'Retry cleanup' }))
        .toBeEnabled();
    await userEvent.keyboard('{Escape}');
    await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
  },
);

test('missing desktop reference blocks preview and Auto apply with guidance', async () => {
  saved = { ...saved, referenceAvailable: false };
  render(<Harness />);
  await open();
  await auto();
  await expect
    .element(
      page.getByText(
        'Both desktop and microphone audio are needed for cancellation.',
      ),
    )
    .toBeVisible();
  await expect
    .element(page.getByRole('button', { name: 'Preview selected range' }))
    .toBeDisabled();
  await expect
    .element(page.getByRole('button', { name: 'Apply', exact: true }))
    .toBeDisabled();
});

test('preview failure is recoverable and stale results close after dismissal', async () => {
  render(<Harness />);
  await open();
  await auto();
  const base = native.invoke.getMockImplementation();
  native.invoke.mockImplementationOnce(async () => {
    throw new Error('Synthetic preview failure');
  });
  await userEvent.click(
    page.getByRole('button', { name: 'Preview selected range' }),
  );
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('This range could not be previewed');
  let resolve: ((value: unknown) => void) | undefined;
  native.invoke.mockImplementation((command, args) =>
    command === 'previewAudioCleanup'
      ? new Promise((done) => {
          resolve = done;
        })
      : base?.(command, args),
  );
  await userEvent.click(
    page.getByRole('button', { name: 'Preview selected range' }),
  );
  await expect.element(page.getByText('Preparing comparison…')).toBeVisible();
  await userEvent.keyboard('{Escape}');
  resolve?.({
    recordingId: saved.recordingId,
    sessionId: 'stale-preview',
    originalUrl:
      'http://127.0.0.1:54321/playback/session/token/segments/original/audio/microphone-0',
    cleanedUrl:
      'http://127.0.0.1:54321/playback/session/token/segments/cleaned/audio/microphone-0',
    matchedMs: 0,
    durationMs: 5000,
  });
  await expect
    .poll(() =>
      native.invoke.mock.calls.some(
        ([command, args]) =>
          command === 'closeLibraryPlaybackSession' &&
          args?.input?.sessionId === 'stale-preview',
      ),
    )
    .toBe(true);
});

test('loading failures can reconnect and late settings cannot reopen a dismissed dialog', async () => {
  const base = native.invoke.getMockImplementation();
  let failLoad = true;
  native.invoke.mockImplementation((command, args) => {
    if (command === 'getAudioCleanup' && failLoad) {
      failLoad = false;
      return Promise.reject(new Error('Synthetic settings failure'));
    }
    return base?.(command, args);
  });
  render(<Harness />);
  await userEvent.click(
    page.getByRole('button', { name: 'Microphone actions' }),
  );
  await userEvent.click(
    page.getByRole('menuitem', { name: 'Remove speaker bleed…' }),
  );
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Cleanup settings could not be loaded');
  await userEvent.keyboard('{Escape}');
  await open();
  await userEvent.keyboard('{Escape}');
  let resolve: ((value: unknown) => void) | undefined;
  native.invoke.mockImplementation((command, args) =>
    command === 'getAudioCleanup'
      ? new Promise((done) => {
          resolve = done;
        })
      : base?.(command, args),
  );
  await userEvent.click(
    page.getByRole('button', { name: 'Microphone actions' }),
  );
  await userEvent.click(
    page.getByRole('menuitem', { name: 'Remove speaker bleed…' }),
  );
  await expect
    .element(page.getByRole('combobox', { name: 'Cancellation' }))
    .toBeDisabled();
  await userEvent.keyboard('{Escape}');
  resolve?.(saved);
  await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
  expect(restore).toHaveBeenCalled();
});

test('live recordings explain disabled cleanup', async () => {
  render(
    <Harness
      detail={{
        ...studioStoryRecording,
        audioCleanup: saved,
        status: 'recording',
      }}
      unavailableReason="Finish recording before removing speaker bleed."
    />,
  );
  await open();
  await auto();
  await expect
    .element(page.getByText('Finish recording before removing speaker bleed.'))
    .toBeVisible();
  await expect
    .element(page.getByRole('button', { name: 'Apply', exact: true }))
    .toBeDisabled();
});

test('completed saved cleanup with no matching audio explains unchanged microphone', async () => {
  saved = {
    ...saved,
    phase: 'ready',
    revision: 'previous',
    progress: 1,
    config: { ...saved.config, mode: 'auto' },
  };
  render(<Harness detail={{ ...studioStoryRecording, audioCleanup: saved }} />);
  await open();
  await expect
    .element(
      page.getByText(
        'No speaker bleed detected. Microphone audio is unchanged.',
      ),
    )
    .toBeVisible();
});

test('applying cannot dismiss midway and late admission cannot change a different recording', async () => {
  const view = render(<Harness />);
  await open();
  await auto();
  const base = native.invoke.getMockImplementation();
  let resolve: ((value: unknown) => void) | undefined;
  native.invoke.mockImplementation((command, args) =>
    command === 'applyAudioCleanup'
      ? new Promise((done) => {
          resolve = done;
        })
      : base?.(command, args),
  );
  await userEvent.click(
    page.getByRole('button', { name: 'Apply', exact: true }),
  );
  await expect
    .element(page.getByRole('button', { name: 'Apply', exact: true }))
    .toBeDisabled();
  await userEvent.keyboard('{Escape}');
  await expect.element(page.getByRole('dialog')).toBeVisible();
  view.rerender(
    <Harness detail={{ ...studioStoryRecording, id: 'different-recording' }} />,
  );
  resolve?.({
    ...saved,
    phase: 'running',
    pendingConfig: { ...saved.config, mode: 'auto' },
  });
  await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
  expect(refresh).not.toHaveBeenCalled();
});

test('late cancellation completion cannot reopen a departed recording', async () => {
  completeCleanup = false;
  const view = render(<Harness />);
  await open();
  await auto();
  await userEvent.click(
    page.getByRole('button', { name: 'Apply', exact: true }),
  );
  const base = native.invoke.getMockImplementation();
  let resolve: ((value: unknown) => void) | undefined;
  native.invoke.mockImplementation((command, args) =>
    command === 'cancelAudioCleanup'
      ? new Promise((done) => {
          resolve = done;
        })
      : base?.(command, args),
  );
  await userEvent.click(page.getByRole('button', { name: 'Cancel cleanup' }));
  await expect
    .element(page.getByRole('button', { name: 'Cancel cleanup' }))
    .toBeDisabled();
  await userEvent.keyboard('{Escape}');
  await expect.element(page.getByRole('dialog')).toBeVisible();
  view.unmount();
  resolve?.({ ...saved, phase: 'cancelled' });
  await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
});

test.each([
  'getAudioCleanup',
  'previewAudioCleanup',
  'applyAudioCleanup',
  'cancelAudioCleanup',
] as const)('late %s failure cannot affect navigation', async (command) => {
  completeCleanup = false;
  const view = render(<Harness />);
  if (command !== 'getAudioCleanup') {
    await open();
    await auto();
  }
  if (command === 'cancelAudioCleanup')
    await userEvent.click(
      page.getByRole('button', { name: 'Apply', exact: true }),
    );
  const base = native.invoke.getMockImplementation();
  let reject: ((error: Error) => void) | undefined;
  native.invoke.mockImplementation((name, args) =>
    name === command
      ? new Promise((_resolve, failed) => {
          reject = failed;
        })
      : base?.(name, args),
  );
  if (command === 'getAudioCleanup') {
    await userEvent.click(
      page.getByRole('button', { name: 'Microphone actions' }),
    );
    await userEvent.click(
      page.getByRole('menuitem', { name: 'Remove speaker bleed…' }),
    );
  } else
    await userEvent.click(
      page.getByRole('button', {
        name:
          command === 'previewAudioCleanup'
            ? 'Preview selected range'
            : command === 'applyAudioCleanup'
              ? 'Apply'
              : 'Cancel cleanup',
        exact: true,
      }),
    );
  await expect.poll(() => Boolean(reject)).toBe(true);
  view.unmount();
  reject?.(new Error('Synthetic stale failure'));
  await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
  expect(refresh).not.toHaveBeenCalled();
});

test('progress read failures remain beside cleanup controls', async () => {
  completeCleanup = false;
  render(<Harness />);
  await open();
  await auto();
  await userEvent.click(
    page.getByRole('button', { name: 'Apply', exact: true }),
  );
  const base = native.invoke.getMockImplementation();
  native.invoke.mockImplementation((command, args) =>
    command === 'getAudioCleanup'
      ? Promise.reject(new Error('Synthetic progress failure'))
      : base?.(command, args),
  );
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Cleanup progress could not be read');
});

test('preview tabs restore the media clock and failed preview disposal stays visible', async () => {
  render(<Harness />);
  await open();
  await auto();
  await userEvent.click(
    page.getByRole('button', { name: 'Preview selected range' }),
  );
  await expect
    .element(page.getByRole('tab', { name: 'Cleaned', exact: true }))
    .toBeVisible();
  const media = document.querySelector('audio');
  if (!media) throw new Error('Synthetic media element missing');
  // The media API is the external boundary here; real codec playback is proven natively.
  Object.defineProperty(media, 'duration', { configurable: true, value: 5 });
  media.currentTime = 2.5;
  await userEvent.click(
    page.getByRole('tab', { name: 'Original', exact: true }),
  );
  media.currentTime = 0;
  fireEvent.loadedMetadata(media);
  expect(media.currentTime).toBe(2.5);
  const base = native.invoke.getMockImplementation();
  native.invoke.mockImplementation((command, args) =>
    command === 'closeLibraryPlaybackSession'
      ? Promise.reject(new Error('Synthetic disposal failure'))
      : base?.(command, args),
  );
  await userEvent.click(
    page.getByRole('slider', { name: 'Speaker bleed sensitivity' }),
  );
  await userEvent.keyboard('{ArrowRight}');
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('The comparison could not close');
});
