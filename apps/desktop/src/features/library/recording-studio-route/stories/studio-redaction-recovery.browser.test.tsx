import type * as TauriCore from '@tauri-apps/api/core';
import type {
  CreateLibraryRedactionRangeInput,
  LibraryRecordingDetail,
} from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { RecordingStudio } from '../../components/studio/recording-studio';
import { StudioSidebarComponentSelection } from '../../components/studio/studio-sidebar-inspector-sections';
import { StudioSidebarRedactionSection } from '../../components/studio/studio-sidebar-redaction-section';
import { useStudioTimelineStore } from '../../components/studio/studio-timeline-store';
import {
  createLibraryRedactions,
  deleteLibraryRedaction,
  unredactLibraryRedactions,
} from '../../library-client';
import { deferredStory, studioStoryRecording } from './studio-route-fixture';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
const mask = {
  recordingId: studioStoryRecording.id,
  id: 'synthetic-mask',
  trackIds: ['video-0'],
  startOffsetMs: 1000,
  endOffsetMs: 2000,
  createdAtMs: 1,
  reason: null,
};
const detail: LibraryRecordingDetail = {
  ...studioStoryRecording,
  redactions: [mask],
};
let mutationError: Error | null;
let mutationPending: Promise<void> | null;
beforeEach(() => {
  vi.resetAllMocks();
  mutationError = new Error('Synthetic privacy write denied');
  mutationPending = null;
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
  native.invoke.mockImplementation(async () => {
    if (mutationError) throw mutationError;
    if (mutationPending) await mutationPending;
    return detail;
  });
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});
function selectRange() {
  useStudioTimelineStore.getState().setSelection({
    laneIds: [],
    clipIds: [],
    redactionIds: [],
    visualFrameIds: [],
    ranges: [
      {
        id: 'synthetic-range',
        laneIds: ['video-0'],
        startMs: 1000,
        endMs: 2000,
      },
    ],
  });
}
async function create(ranges: CreateLibraryRedactionRangeInput[]) {
  await createLibraryRedactions({ recordingId: detail.id, ranges });
}
async function unredact(ranges: CreateLibraryRedactionRangeInput[]) {
  await unredactLibraryRedactions({ recordingId: detail.id, ranges });
}
async function remove(redactionId: string) {
  await deleteLibraryRedaction({ recordingId: detail.id, redactionId });
}
function RangeInspector() {
  const selection = useStudioTimelineStore((state) => state.selection);
  const clear = useStudioTimelineStore((state) => state.clearSelection);
  return (
    <>
      <button onClick={selectRange}>Select source range</button>
      {selection.ranges.length ? (
        <StudioSidebarComponentSelection
          aiProcessingState={null}
          clearSelection={clear}
          detail={detail}
          onCreateRedactions={create}
          onUnredactRanges={unredact}
          selections={selection.ranges}
        />
      ) : (
        <p role="status">Selection cleared</p>
      )}
    </>
  );
}

const noAction = () => undefined;
function selectLaterRange() {
  useStudioTimelineStore.getState().setSelection({
    laneIds: [],
    clipIds: [],
    redactionIds: [],
    visualFrameIds: [],
    ranges: [
      { id: 'later-range', laneIds: ['video-0'], startMs: 5000, endMs: 7000 },
    ],
  });
}
function ToolbarPrivacyStudio() {
  return (
    <>
      {/* These fixture controls replace timeline selections atomically. */}
      <div data-studio-preserve-selection>
        <button onClick={selectRange}>Select source range</button>
        <button onClick={selectLaterRange}>Select later range</button>
      </div>
      <RecordingStudio
        detail={detail}
        playbackAssets={null}
        thumbnailStatus="idle"
        activity={null}
        aiProcessingState={null}
        visualObservations={null}
        onRefreshPlayback={noAction}
        onCreateRedactions={create}
        onDeleteRedaction={remove}
        onUnredactRanges={unredact}
      />
    </>
  );
}

test.each([
  ['Redact', 'createLibraryRedactions'],
  ['Unredact', 'unredactLibraryRedactions'],
])(
  'STU-07 %s keeps the exact source selection after a failed privacy write and clears it only after retry succeeds',
  async (name, command) => {
    render(<RangeInspector />);
    await userEvent.click(
      screen.getByRole('button', { name: 'Select source range' }),
    );
    await userEvent.click(screen.getByRole('button', { name }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Synthetic privacy write denied',
    );
    expect(screen.queryByRole('status')).toBeNull();
    mutationError = null;
    await userEvent.click(screen.getByRole('button', { name }));
    expect(await screen.findByRole('status')).toHaveTextContent(
      'Selection cleared',
    );
    expect(native.invoke.mock.calls).toEqual([
      [
        command,
        {
          input: {
            recordingId: detail.id,
            ranges: [
              { startOffsetMs: 1000, endOffsetMs: 2000, trackIds: ['video-0'] },
            ],
          },
        },
      ],
      [
        command,
        {
          input: {
            recordingId: detail.id,
            ranges: [
              { startOffsetMs: 1000, endOffsetMs: 2000, trackIds: ['video-0'] },
            ],
          },
        },
      ],
    ]);
  },
);

test('STU-10 pending timeline privacy writes prevent duplicate clicks and retain a newer selection on late success', async () => {
  mutationError = null;
  const pending = deferredStory<void>();
  mutationPending = pending.promise;
  render(<ToolbarPrivacyStudio />);
  await userEvent.click(
    screen.getByRole('button', { name: 'Select source range' }),
  );
  const toolbarButton = screen
    .getAllByRole('button', { name: 'Redact' })
    .find((button) => button.closest('aside') === null);
  if (!toolbarButton) throw new Error('Missing timeline editing control');
  await userEvent.dblClick(toolbarButton);
  expect(toolbarButton).toBeDisabled();
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'createLibraryRedactions',
    ),
  ).toHaveLength(1);
  await userEvent.click(
    screen.getByRole('button', { name: 'Select later range' }),
  );
  expect(screen.getAllByText('0:05 - 0:07').length).toBeGreaterThan(0);
  pending.resolve();
  await waitFor(() => expect(toolbarButton).toBeEnabled());
  expect(screen.getAllByText('0:05 - 0:07').length).toBeGreaterThan(0);
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'createLibraryRedactions',
    )[0]?.[1],
  ).toEqual({
    input: {
      recordingId: detail.id,
      ranges: [
        { startOffsetMs: 1000, endOffsetMs: 2000, trackIds: ['video-0'] },
      ],
    },
  });
});

test('STU-08 selected-mask Unredact exposes failed native writes and retries the original mask', async () => {
  render(
    <StudioSidebarRedactionSection
      detail={detail}
      redaction={mask}
      onDeleteRedaction={remove}
    />,
  );
  await userEvent.click(screen.getByRole('button', { name: 'Unredact' }));
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Synthetic privacy write denied',
  );
  mutationError = null;
  await userEvent.click(screen.getByRole('button', { name: 'Unredact' }));
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
  expect(native.invoke.mock.calls).toEqual([
    [
      'deleteLibraryRedaction',
      { input: { recordingId: detail.id, redactionId: mask.id } },
    ],
    [
      'deleteLibraryRedaction',
      { input: { recordingId: detail.id, redactionId: mask.id } },
    ],
  ]);
});

test.each([
  ['Redact', 'createLibraryRedactions'],
  ['Unredact', 'unredactLibraryRedactions'],
])(
  'STU-09 timeline toolbar %s preserves failed edits and exposes recovery before clearing a successful retry',
  async (name, command) => {
    render(<ToolbarPrivacyStudio />);
    await userEvent.click(
      screen.getByRole('button', { name: 'Select source range' }),
    );
    const toolbarButton = screen
      .getAllByRole('button', { name })
      .find((button) => button.closest('aside') === null);
    if (!toolbarButton) throw new Error('Missing timeline editing control');
    await userEvent.click(toolbarButton);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Synthetic privacy write denied',
    );
    expect(screen.getAllByRole('button', { name }).length).toBeGreaterThan(0);
    mutationError = null;
    await userEvent.click(toolbarButton);
    await waitFor(() =>
      expect(screen.queryByRole('button', { name })).toBeNull(),
    );
    expect(
      native.invoke.mock.calls.filter(
        ([nativeCommand]) => nativeCommand === command,
      ),
    ).toHaveLength(2);
  },
);
