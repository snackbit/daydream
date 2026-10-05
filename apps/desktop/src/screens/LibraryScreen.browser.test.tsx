import type { ReactNode } from 'react';
import type { LibraryRecordingSummary } from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { formatTimestamp } from '@/features/library/library-format';
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

vi.mock('@tanstack/react-router', () => ({
  Link: ({
    children,
    params,
    to,
  }: {
    children: ReactNode;
    params?: { recordingId: string };
    to: string;
  }) => <a href={params ? `/library/${params.recordingId}` : to}>{children}</a>,
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

beforeEach(() => {
  vi.resetAllMocks();
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
});

test('LibraryScreen defaults to dense list view and disables play for missing files', async () => {
  render(<LibraryScreen />);

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

  render(<LibraryScreen />);

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

  render(<LibraryScreen />);

  expect(await screen.findByRole('alert')).toBeTruthy();
  expect(screen.getByText('Local library is unavailable.')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Try again' }));

  expect(await screen.findByText('Ready')).toBeTruthy();
  await waitFor(() =>
    expect(client.listLibraryRecordings).toHaveBeenCalledTimes(2),
  );
});

test('LibraryScreen preserves multi-selection across views and deletes selected recordings', async () => {
  render(<LibraryScreen />);

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
  render(<LibraryScreen />);

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

  render(<LibraryScreen />);

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
  render(<LibraryScreen />);

  await screen.findByText('Ready');
  fireEvent.doubleClick(screen.getByText('Ready'));

  expect(router.navigate).toHaveBeenCalledWith({
    params: { recordingId: 'recording-ready' },
    to: '/library/$recordingId',
  });
});

test('LibraryScreen deletes a recording from its visible row action after confirmation', async () => {
  render(<LibraryScreen />);

  await screen.findByText('Ready');
  const readyDeleteLabel = `Delete Recording, ${formatTimestamp(readyRecording.startedAtMs)}`;
  fireEvent.click(screen.getByRole('button', { name: readyDeleteLabel }));
  expect(client.deleteLibraryRecording).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(client.deleteLibraryRecording).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole('button', { name: readyDeleteLabel }));
  fireEvent.click(screen.getByRole('button', { name: /^delete$/i }));

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

  render(<LibraryScreen />);

  await screen.findByText('Initial capture name');
  fireEvent.click(
    screen.getByRole('button', { name: /rename initial capture name/i }),
  );
  fireEvent.change(screen.getByLabelText('Title'), {
    target: { value: 'Updated capture name' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(client.renameLibraryRecording).toHaveBeenCalledWith({
      recordingId: 'recording-ready',
      title: 'Updated capture name',
    }),
  );
  expect(await screen.findByText('Updated capture name')).toBeTruthy();

  fireEvent.click(
    screen.getByRole('button', { name: /rename updated capture name/i }),
  );
  fireEvent.change(screen.getByLabelText('Title'), {
    target: { value: '   ' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(client.renameLibraryRecording).toHaveBeenLastCalledWith({
      recordingId: 'recording-ready',
      title: null,
    }),
  );
  expect(screen.queryByText('Updated capture name')).toBeNull();
});
