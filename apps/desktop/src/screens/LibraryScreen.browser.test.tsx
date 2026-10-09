import type { ComponentProps } from 'react';
import type {
  LibraryRecordingSummary,
  RecallDayCard,
  LibraryRecordingThumbnail,
} from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  act,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { formatTimestamp } from '@/features/library/library-format';
import { LibraryDayCard } from '@/features/recall/library/library-day-card';
import {
  recallDayFixture,
  recallStatusFixture,
} from '@/features/recall/testing/recall-browser-fixtures';
import '../index.css';
import { LibraryScreen } from './LibraryScreen';

const client = vi.hoisted(() => ({
  deleteLibraryRecording: vi.fn(),
  deleteLibraryRecordings: vi.fn(),
  listLibraryRecordings: vi.fn(),
  localAssetUrl: vi.fn((path: string) => `asset://${path}`),
  prepareLibraryRecordingThumbnail: vi.fn(),
  renameLibraryRecording: vi.fn(),
}));
const router = vi.hoisted(() => ({
  navigate: vi.fn(),
}));

vi.mock('@/features/library/library-client', () => client);
const recall = vi.hoisted(() => ({
  listRecallDays: vi.fn<() => Promise<RecallDayCard[]>>(async () => []),
  getRecallStatus: vi.fn(),
  listenToRecallStateChanged: vi.fn(async () => () => undefined),
}));
vi.mock('@/features/recall/recall-client', () => recall);

async function renderRecordings() {
  render(<LibraryScreen />);
  await userEvent.click(screen.getByRole('tab', { name: 'Recordings' }));
}

vi.mock('@tanstack/react-router', () => ({
  Link: ({
    children,
    params,
    to,
    ...anchorProps
  }: {
    params?: { recordingId?: string; date?: string };
    to: string;
  } & ComponentProps<'a'>) => (
    <a
      {...anchorProps}
      href={
        params?.recordingId
          ? `/library/${params.recordingId}`
          : params?.date
            ? `/day/${params.date}`
            : to
      }
    >
      {children}
    </a>
  ),
  useNavigate: () => router.navigate,
}));

const readyRecording = {
  availability: 'available',
  durationMs: 90_000,
  endedAtMs: 91_000,
  id: 'recording-ready',
  missingReasons: [],
  playableSegmentCount: 1,
  segmentCount: 1,
  sizeBytes: 1_000_000,
  startedAtMs: 1_000,
  status: 'completed',
  storageDir: '/tmp/daydream/recordings/recording-ready',
  thumbnailPath: '/tmp/daydream/recordings/recording-ready/segments/0001.mkv',
  title: null,
} satisfies LibraryRecordingSummary;

const missingRecording = {
  availability: 'missing-files',
  durationMs: 30_000,
  endedAtMs: 31_000,
  id: 'recording-missing',
  missingReasons: ['Segment 1 video file is missing.'],
  playableSegmentCount: 1,
  segmentCount: 1,
  sizeBytes: 500_000,
  startedAtMs: 2_000,
  status: 'completed',
  storageDir: '/tmp/daydream/recordings/recording-missing',
  thumbnailPath: null,
  title: null,
} satisfies LibraryRecordingSummary;

const lockedRecording = {
  availability: 'locked',
  durationMs: null,
  endedAtMs: null,
  id: 'recording-locked',
  missingReasons: ['Recording is still in progress.'],
  playableSegmentCount: 0,
  segmentCount: 1,
  sizeBytes: 2_000_000,
  startedAtMs: 1_500,
  status: 'recording',
  storageDir: '/tmp/daydream/recordings/recording-locked',
  thumbnailPath: null,
  title: null,
} satisfies LibraryRecordingSummary;

beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  vi.setSystemTime(new Date('2026-10-06T12:00:00'));
  recall.getRecallStatus.mockResolvedValue(recallStatusFixture);
  client.listLibraryRecordings.mockResolvedValue([
    readyRecording,
    missingRecording,
  ]);
  client.deleteLibraryRecording.mockResolvedValue(undefined);
  client.deleteLibraryRecordings.mockResolvedValue(undefined);
  client.renameLibraryRecording.mockImplementation(
    ({ recordingId, title }: { recordingId: string; title: string | null }) =>
      Promise.resolve({
        ...readyRecording,
        id: recordingId,
        segments: [],
        title,
        tracks: [],
      }),
  );
  client.prepareLibraryRecordingThumbnail.mockResolvedValue({
    error: null,
    path: '/tmp/daydream/recordings/recording-ready/playback/segment-1/overview.jpg',
    recordingId: 'recording-ready',
  });
  router.navigate.mockResolvedValue(undefined);
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  delete document.documentElement.dataset.theme;
});

test('Library defaults to newest-first Days regardless of review status', async () => {
  recall.listRecallDays.mockResolvedValue([
    { ...recallDayFixture.day, dayId: '2026-10-06', reviewed: true },
    recallDayFixture.day,
  ]);
  render(<LibraryScreen />);
  expect(
    screen.getByRole('tab', { name: 'Days' }).getAttribute('aria-selected'),
  ).toBe('true');
  await screen.findByRole('link', { name: /open day.*october 6/i });
  expect(
    screen.getAllByRole('link', { name: /open day/i })[0],
  ).toHaveAccessibleName(/october 6/i);
  expect(screen.queryByText('Most recent unreviewed day')).toBeNull();
  expect(screen.queryByText(recallDayFixture.day.summary ?? '')).toBeNull();
  expect(screen.getByRole('link', { name: /today/i })).toBeTruthy();
  await userEvent.click(screen.getByRole('tab', { name: 'Recordings' }));
  await screen.findByRole('status', { name: 'Library summary' });
});

test('Library Days preserves primary links in both themes at desktop and narrow widths', async () => {
  recall.listRecallDays.mockResolvedValue([recallDayFixture.day]);
  for (const dark of [false, true]) {
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
    for (const width of [1440, 1024, 768, 390, 320]) {
      await page.viewport(width, 960);
      const view = render(<LibraryScreen />);
      await screen.findByRole('link', { name: /open day/i });
      await expect
        .element(page.getByRole('link', { name: /today/i }))
        .toBeVisible();
      await page.screenshot({ element: view.container });
      cleanup();
    }
  }
});

test('Day thumbnails use the native privacy adapter and reject old callbacks after eligibility changes', async () => {
  let finishOld: ((thumbnail: LibraryRecordingThumbnail) => void) | undefined;
  let finishCurrent:
    ((thumbnail: LibraryRecordingThumbnail) => void) | undefined;
  client.prepareLibraryRecordingThumbnail.mockImplementation(
    ({ recordingId }: { recordingId: string }) =>
      new Promise<LibraryRecordingThumbnail>((resolve) => {
        if (recordingId === 'recording-old') finishOld = resolve;
        else finishCurrent = resolve;
      }),
  );
  const view = render(
    <LibraryDayCard
      day={{ ...recallDayFixture.day, thumbnailRecordingId: 'recording-old' }}
    />,
  );
  await waitFor(() =>
    expect(client.prepareLibraryRecordingThumbnail).toHaveBeenCalledWith({
      recordingId: 'recording-old',
    }),
  );
  view.rerender(
    <LibraryDayCard
      day={{
        ...recallDayFixture.day,
        privacyRevision: 2,
        thumbnailRecordingId: 'recording-current',
      }}
    />,
  );
  await act(async () => {
    finishOld?.({
      recordingId: 'recording-old',
      path: '/tmp/synthetic-old-preview.jpg',
      error: null,
    });
  });
  expect(screen.queryByRole('img')).toBeNull();
  await waitFor(() =>
    expect(client.prepareLibraryRecordingThumbnail).toHaveBeenCalledWith({
      recordingId: 'recording-current',
    }),
  );
  await act(async () => {
    finishCurrent?.({
      recordingId: 'recording-current',
      path: '/tmp/synthetic-current-preview.jpg',
      error: null,
    });
  });
  expect(
    await screen.findByRole('img', { name: 'Privacy-aware recording preview' }),
  ).toHaveProperty('src', 'asset:///tmp/synthetic-current-preview.jpg');
  view.rerender(
    <LibraryDayCard
      day={{
        ...recallDayFixture.day,
        privacyRevision: 3,
        thumbnailRecordingId: null,
      }}
    />,
  );
  expect(screen.queryByRole('img')).toBeNull();
});

test('Today follows the local calendar after midnight when the window resumes', async () => {
  render(<LibraryScreen />);
  const today = await screen.findByRole('link', { name: /today/i });
  expect(today.getAttribute('href')).toBe('/day/2026-10-06');
  act(() => {
    vi.setSystemTime(new Date('2026-10-07T12:00:00'));
    window.dispatchEvent(new Event('focus'));
  });
  await waitFor(() =>
    expect(
      screen.getByRole('link', { name: /today/i }).getAttribute('href'),
    ).toBe('/day/2026-10-07'),
  );
});

test('LibraryScreen defaults to dense list view and disables play for missing files', async () => {
  await renderRecordings();

  expect(await screen.findByText('Missing files')).toBeTruthy();
  expect(
    screen
      .getByRole('button', { name: /list view/i })
      .getAttribute('data-variant'),
  ).toBe('secondary');
  expect(screen.getByText('Segment 1 video file is missing.')).toBeTruthy();
  const summary = screen.getByRole('status', { name: 'Library summary' });
  expect(summary.textContent).toContain('2 recordings');
  expect(summary.textContent).toContain('1.50 MB stored locally');
  expect(
    (
      screen.getByRole('button', {
        name: /play .*video file is missing/i,
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  expect(client.prepareLibraryRecordingThumbnail).toHaveBeenCalledWith({
    recordingId: 'recording-ready',
  });
  await waitFor(() =>
    expect(client.localAssetUrl).toHaveBeenCalledWith(
      '/tmp/daydream/recordings/recording-ready/playback/segment-1/overview.jpg',
    ),
  );
  expect(client.localAssetUrl).not.toHaveBeenCalledWith(
    readyRecording.thumbnailPath,
  );
});

test('LibraryScreen explains an empty local library and links to capture setup', async () => {
  client.listLibraryRecordings.mockResolvedValue([]);

  await renderRecordings();

  expect(await screen.findByText('Your Library is empty')).toBeTruthy();
  expect(
    screen
      .getByRole('link', { name: /set up a recording/i })
      .getAttribute('href'),
  ).toBe('/capture');
  expect(screen.getByRole('button', { name: /refresh/i })).toBeTruthy();
});

test('LibraryScreen retries a failed local listing and shows recovered sessions', async () => {
  client.listLibraryRecordings
    .mockRejectedValueOnce(new Error('Local library is unavailable.'))
    .mockResolvedValueOnce([readyRecording]);

  await renderRecordings();

  expect(await screen.findByRole('alert')).toBeTruthy();
  expect(screen.getByText('Local library is unavailable.')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Try again' }));

  expect(await screen.findByText('Ready')).toBeTruthy();
  await waitFor(() =>
    expect(client.listLibraryRecordings).toHaveBeenCalledTimes(2),
  );
});

test('LibraryScreen preserves multi-selection across views and deletes selected recordings', async () => {
  await renderRecordings();

  await screen.findByText('Ready');
  fireEvent.click(screen.getByText('Ready'));
  fireEvent.click(screen.getByText('Missing files'), { ctrlKey: true });
  const summary = screen.getByRole('status', { name: 'Library summary' });
  expect(summary.textContent).toContain('2 selected');
  expect(summary.textContent).toContain('1.50 MB selected');

  fireEvent.click(screen.getByRole('button', { name: /grid view/i }));
  expect(summary.textContent).toContain('2 selected');
  fireEvent.click(screen.getByRole('button', { name: /list view/i }));
  expect(summary.textContent).toContain('2 selected');

  fireEvent.click(screen.getByRole('button', { name: /delete selected/i }));
  fireEvent.click(screen.getAllByRole('button', { name: /^delete$/i }).at(-1)!);

  await waitFor(() =>
    expect(client.deleteLibraryRecordings).toHaveBeenCalledWith({
      recordingIds: ['recording-ready', 'recording-missing'],
    }),
  );
});

test('LibraryScreen exposes labeled row selection controls for bulk actions', async () => {
  await renderRecordings();

  const rowSelections = await screen.findAllByRole('checkbox', {
    name: /select recording/i,
  });
  const rowSelection = rowSelections[0];
  if (!rowSelection) {
    throw new Error('Expected at least one recording selection control.');
  }
  fireEvent.click(rowSelection);

  expect(rowSelection.getAttribute('aria-checked')).toBe('true');
  const summary = screen.getByRole('status', { name: 'Library summary' });
  expect(summary.textContent).toContain('1 selected');
  expect(summary.textContent).toMatch(/\d+(?:\.\d+)? (?:KB|MB) selected/);
});

test('LibraryScreen keeps locked recordings out of selection and delete actions', async () => {
  client.listLibraryRecordings.mockResolvedValue([
    readyRecording,
    lockedRecording,
    missingRecording,
  ]);

  await renderRecordings();

  await screen.findByText('In progress');
  fireEvent.click(screen.getByText('In progress'));
  expect(screen.queryByRole('button', { name: /^delete$/i })).toBeNull();
  const summary = screen.getByRole('status', { name: 'Library summary' });
  expect(summary.textContent).toContain('3 recordings');
  expect(summary.textContent).toContain('3.50 MB stored locally');

  fireEvent.click(screen.getByText('Ready'));
  fireEvent.click(screen.getByText('In progress'), { ctrlKey: true });
  expect(summary.textContent).toContain('1 selected');
  expect(summary.textContent).toContain('1.00 MB selected');

  fireEvent.click(screen.getByText('Missing files'), { shiftKey: true });
  expect(summary.textContent).toContain('2 selected');
  expect(summary.textContent).toContain('1.50 MB selected');

  fireEvent.doubleClick(screen.getByText('In progress'));
  expect(router.navigate).not.toHaveBeenCalled();
});

test('LibraryScreen opens playable recordings on double click', async () => {
  await renderRecordings();

  await screen.findByText('Ready');
  fireEvent.doubleClick(screen.getByText('Ready'));

  expect(router.navigate).toHaveBeenCalledWith({
    params: { recordingId: 'recording-ready' },
    to: '/library/$recordingId',
  });
});

test('LibraryScreen deletes a recording from its visible row action after confirmation', async () => {
  await renderRecordings();

  await screen.findByText('Ready');
  const readyDeleteLabel = `Delete Recording, ${formatTimestamp(readyRecording.startedAtMs)}`;
  await userEvent.click(screen.getByRole('button', { name: readyDeleteLabel }));
  expect(client.deleteLibraryRecording).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(client.deleteLibraryRecording).not.toHaveBeenCalled();

  await userEvent.click(
    await screen.findByRole('button', { name: readyDeleteLabel }),
  );
  await userEvent.click(screen.getByRole('button', { name: /^delete$/i }));

  await waitFor(() =>
    expect(client.deleteLibraryRecording).toHaveBeenCalledWith({
      recordingId: 'recording-ready',
    }),
  );
  await waitFor(() => expect(screen.queryByText('Ready')).toBeNull());
});

test('LibraryScreen renames and clears a recording from the context menu', async () => {
  const titledRecording = {
    ...readyRecording,
    title: 'Initial capture name',
  } satisfies LibraryRecordingSummary;
  client.listLibraryRecordings.mockResolvedValue([titledRecording]);

  await renderRecordings();

  await screen.findByText('Initial capture name');
  await userEvent.click(
    screen.getByRole('button', { name: /rename initial capture name/i }),
  );
  await userEvent.fill(screen.getByLabelText('Title'), 'Updated capture name');
  await userEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(client.renameLibraryRecording).toHaveBeenCalledWith({
      recordingId: 'recording-ready',
      title: 'Updated capture name',
    }),
  );
  expect(await screen.findByText('Updated capture name')).toBeTruthy();

  await userEvent.click(
    await screen.findByRole('button', { name: /rename updated capture name/i }),
  );
  await userEvent.fill(screen.getByLabelText('Title'), '   ');
  await userEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(client.renameLibraryRecording).toHaveBeenLastCalledWith({
      recordingId: 'recording-ready',
      title: null,
    }),
  );
  expect(screen.queryByText('Updated capture name')).toBeNull();
});
