import type { ReactNode } from 'react';
import type * as TauriCore from '@tauri-apps/api/core';
import {
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { useSettingsStore } from '@/features/settings/settings-store';
import { defaultSettingsState } from '@/features/settings/testing/settings-browser-fixtures';
import '@/index.css';
import { RecordingStudioScreen } from '@/screens/RecordingStudioScreen';
import { useStudioTimelineStore } from '../../components/studio/timeline/state/studio-timeline-store';
import { deferredStory, studioStoryRecording } from './studio-route-fixture';

const native = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
const router = vi.hoisted(() => ({ navigate: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: native.listen }));
vi.mock('@tanstack/react-router', () => ({
  Link: ({ children, to }: { children: ReactNode; to: string }) => (
    <a href={to}>{children}</a>
  ),
  useNavigate: () => router.navigate,
  useParams: () => ({ recordingId: studioStoryRecording.id }),
  useSearch: () => ({ evidenceId: null, source: null, seek: null }),
}));

let renameError: Error | null;
let deleteError: Error | null;
let deletePending: Promise<void> | null;
let redactionPending: Promise<void> | null;
let debugError: Error | null;
let savedRecording = studioStoryRecording;

beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  renameError = null;
  deleteError = null;
  deletePending = null;
  redactionPending = null;
  debugError = null;
  savedRecording = structuredClone(studioStoryRecording);
  useSettingsStore.setState({
    loadStatus: 'ready',
    draft: null,
    settingsState: null,
  });
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
  native.listen.mockResolvedValue(() => undefined);
  router.navigate.mockResolvedValue(undefined);
  native.invoke.mockImplementation(
    async (
      command: string,
      args?: { input?: { title?: string | null; redactionId?: string } },
    ) => {
      switch (command) {
        case 'getLibraryRecording':
          return savedRecording;
        case 'getLibraryRecordingDebug':
          if (debugError) throw debugError;
          return {
            recordingId: savedRecording.id,
            detail: savedRecording,
            processingState: {
              recordingId: savedRecording.id,
              job: null,
              chunks: [],
              transcript: { recordingId: savedRecording.id, tracks: [] },
            },
            visualObservations: {
              recordingId: savedRecording.id,
              ocrRegions: [],
              vlmObservations: [],
              vlmState: {
                status: 'disabled',
                providerStatus: 'disabled',
                errorCode: null,
              },
            },
            raw: { synthetic: true },
          };
        case 'getRecordingAiProcessingState':
          return {
            recordingId: savedRecording.id,
            job: null,
            chunks: [],
            transcript: { recordingId: savedRecording.id, tracks: [] },
          };
        case 'getRecordingActivity':
          return {
            recordingId: savedRecording.id,
            captureStates: [],
            events: [],
            intervals: [],
            windowSnapshots: [],
          };
        case 'getRecordingVisualObservations':
          return {
            recordingId: savedRecording.id,
            ocrRegions: [],
            vlmObservations: [],
            vlmState: {
              status: 'disabled',
              providerStatus: 'disabled',
              errorCode: null,
            },
          };
        case 'getOcrTimelineFrames':
          return {
            recordingId: savedRecording.id,
            frames: [],
            nextCursor: null,
          };
        case 'getRecordingWaveformTiles':
          return {
            recordingId: savedRecording.id,
            binMs: 10,
            tracks: [],
            error: null,
          };
        case 'renameLibraryRecording':
          if (renameError) throw renameError;
          savedRecording = {
            ...savedRecording,
            title: args?.input?.title ?? null,
          };
          return savedRecording;
        case 'deleteLibraryRecording':
          if (deleteError) throw deleteError;
          return deletePending;
        case 'deleteLibraryRedaction':
          if (redactionPending) await redactionPending;
          savedRecording = {
            ...savedRecording,
            redactions: savedRecording.redactions.filter(
              (mask) => mask.id !== args?.input?.redactionId,
            ),
          };
          return savedRecording;
        default:
          throw new Error(`Unexpected synthetic native command: ${command}`);
      }
    },
  );
});
afterEach(() => {
  cleanup();
  useSettingsStore.setState(useSettingsStore.getInitialState(), true);
  vi.restoreAllMocks();
});
function calls(command: string) {
  return native.invoke.mock.calls.filter(([name]) => name === command);
}

test('STU-01 rename cancels without writing, validates the limit, keeps a failed draft and retries the exact title', async () => {
  render(<RecordingStudioScreen />);
  await userEvent.click(await screen.findByRole('button', { name: 'Rename' }));
  let dialog = screen.getByRole('dialog');
  await userEvent.fill(
    within(dialog).getByRole('textbox', { name: 'Title' }),
    'Discarded draft',
  );
  await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  expect(calls('renameLibraryRecording')).toHaveLength(0);
  await userEvent.click(screen.getByRole('button', { name: 'Rename' }));
  dialog = screen.getByRole('dialog');
  const title = within(dialog).getByRole('textbox', { name: 'Title' });
  expect(title).toHaveValue('Synthetic review');
  await userEvent.fill(title, 'x'.repeat(120));
  expect(within(dialog).getByRole('button', { name: 'Save' })).toBeEnabled();
  await userEvent.fill(title, 'x'.repeat(121));
  expect(within(dialog).getByRole('button', { name: 'Save' })).toBeDisabled();
  await userEvent.fill(title, '  Retry review  ');
  renameError = new Error('Synthetic rename denied');
  await userEvent.click(within(dialog).getByRole('button', { name: 'Save' }));
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    'Synthetic rename denied',
  );
  expect(title).toHaveValue('  Retry review  ');
  expect(
    screen.getByRole('heading', { level: 1, hidden: true }),
  ).toHaveTextContent('Synthetic review');
  renameError = null;
  await userEvent.click(within(dialog).getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(
    screen.getByRole('heading', { level: 1, hidden: true }),
  ).toHaveTextContent('Retry review');
  expect(calls('renameLibraryRecording').map(([, input]) => input)).toEqual([
    { input: { recordingId: savedRecording.id, title: 'Retry review' } },
    { input: { recordingId: savedRecording.id, title: 'Retry review' } },
  ]);
  await userEvent.click(screen.getByRole('button', { name: 'Rename' }));
  await userEvent.fill(screen.getByRole('textbox', { name: 'Title' }), '   ');
  await userEvent.click(
    within(screen.getByRole('dialog')).getByRole('button', { name: 'Save' }),
  );
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(calls('renameLibraryRecording').at(-1)?.[1]).toEqual({
    input: { recordingId: savedRecording.id, title: null },
  });
});

test('STU-02 deletion cancels safely, preserves the recording on failure and prevents repeat writes while retrying', async () => {
  render(<RecordingStudioScreen />);
  await userEvent.click(await screen.findByRole('button', { name: 'Delete' }));
  await userEvent.click(
    within(screen.getByRole('dialog')).getByRole('button', { name: 'Cancel' }),
  );
  await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
  expect(calls('deleteLibraryRecording')).toHaveLength(0);
  await userEvent.click(screen.getByRole('button', { name: 'Delete' }));
  await expect.element(page.getByRole('dialog')).toBeVisible();
  let dialog = screen.getByRole('dialog');
  deleteError = new Error('Synthetic storage busy');
  await userEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    'Synthetic storage busy',
  );
  expect(router.navigate).not.toHaveBeenCalled();
  expect(
    screen.getByRole('heading', { level: 1, hidden: true }),
  ).toHaveTextContent('Synthetic review');
  deleteError = null;
  const pending = deferredStory<void>();
  deletePending = pending.promise;
  await userEvent.dblClick(
    within(dialog).getByRole('button', { name: 'Delete' }),
  );
  dialog = screen.getByRole('dialog');
  expect(
    within(dialog).getByRole('button', { name: /Deleting/ }),
  ).toBeDisabled();
  expect(calls('deleteLibraryRecording')).toHaveLength(2);
  pending.resolve();
  await waitFor(() =>
    expect(router.navigate).toHaveBeenCalledWith({ to: '/library' }),
  );
});

test('STU-04 Debug stays opt-in, recovers from a failed read on reopen and retries a failed clipboard copy', async () => {
  render(<RecordingStudioScreen />);
  await screen.findByRole('heading', { name: 'Synthetic review' });
  expect(screen.queryByRole('button', { name: 'Debug' })).toBeNull();
  expect(calls('getLibraryRecordingDebug')).toHaveLength(0);
  useSettingsStore.setState({
    draft: { ...defaultSettingsState.settings, debug: { enabled: true } },
  });
  debugError = new Error('Synthetic debug read unavailable');
  await userEvent.click(await screen.findByRole('button', { name: 'Debug' }));
  let dialog = screen.getByRole('dialog');
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    'Synthetic debug read unavailable',
  );
  expect(
    within(dialog).getByRole('button', { name: 'Copy JSON' }),
  ).toBeDisabled();
  await userEvent.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  debugError = null;
  await userEvent.click(screen.getByRole('button', { name: 'Debug' }));
  dialog = screen.getByRole('dialog');
  await waitFor(() =>
    expect(
      within(dialog).getByRole('button', { name: 'Copy JSON' }),
    ).toBeEnabled(),
  );
  const copy = vi
    .spyOn(navigator.clipboard, 'writeText')
    .mockRejectedValueOnce(new Error('Synthetic clipboard denied'))
    .mockResolvedValue(undefined);
  await userEvent.click(
    within(dialog).getByRole('button', { name: 'Copy JSON' }),
  );
  expect(
    await within(dialog).findByText('Unable to copy debug JSON.'),
  ).toBeTruthy();
  await userEvent.click(
    within(dialog).getByRole('button', { name: 'Copy JSON' }),
  );
  expect(await within(dialog).findByText('Debug JSON copied.')).toBeTruthy();
  expect(copy).toHaveBeenCalledTimes(2);
  expect(JSON.parse(copy.mock.calls[1]?.[0] ?? '{}')).toMatchObject({
    recordingId: savedRecording.id,
    raw: { synthetic: true },
  });
  await userEvent.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});

test('STU-11 unhiding an older mask cannot clear a newer selected mask after native completion', async () => {
  savedRecording = {
    ...savedRecording,
    tracks: [
      {
        id: 'video-0',
        kind: 'video',
        label: 'Screen',
        codec: 'h264',
        description: '',
        streamIndex: 0,
      },
    ],
    segments: [
      {
        id: 'segment',
        durationMs: 10000,
        endedAtMs: 11000,
        startedAtMs: 1000,
        fileExists: false,
        filePath: '/tmp/synthetic-story-missing.mkv',
        pauseAfterMs: 0,
        playbackOffsetMs: 0,
        segmentIndex: 0,
        state: 'finalized',
        videoStreamIndex: 0,
        desktopAudioStreamIndex: null,
        microphoneAudioStreamIndex: null,
      },
    ],
    redactions: [
      {
        id: 'older-mask',
        recordingId: savedRecording.id,
        trackIds: ['video-0'],
        startOffsetMs: 1000,
        endOffsetMs: 2000,
        createdAtMs: 1,
        reason: null,
      },
      {
        id: 'newer-mask',
        recordingId: savedRecording.id,
        trackIds: ['video-0'],
        startOffsetMs: 4000,
        endOffsetMs: 5000,
        createdAtMs: 2,
        reason: null,
      },
    ],
  };
  const pending = deferredStory<void>();
  redactionPending = pending.promise;
  render(<RecordingStudioScreen />);
  await screen.findByRole('button', { name: 'Select Screen' });
  const masks = screen.getAllByRole('button', {
    name: 'Screen redacted range',
  });
  if (!masks[0] || !masks[1])
    throw new Error('Expected two independent synthetic masks');
  await userEvent.click(masks[0]);
  await userEvent.click(screen.getByRole('button', { name: 'Unredact' }));
  expect(screen.getByRole('button', { name: 'Unredact' })).toBeDisabled();
  await userEvent.click(masks[1]);
  expect(await screen.findByText('0:04 - 0:05')).toBeTruthy();
  pending.resolve();
  await waitFor(() =>
    expect(
      screen.getAllByRole('button', { name: 'Screen redacted range' }),
    ).toHaveLength(1),
  );
  expect(screen.getByText('0:04 - 0:05')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Unredact' })).toBeEnabled();
  expect(calls('deleteLibraryRedaction')).toEqual([
    [
      'deleteLibraryRedaction',
      { input: { recordingId: savedRecording.id, redactionId: 'older-mask' } },
    ],
  ]);
});
