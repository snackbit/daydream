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
  }: {
    children: ReactNode;
    params: { recordingId: string };
  }) => <a href={`/library/${params.recordingId}`}>{children}</a>,
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

test('LibraryScreen defaults to grid view and disables play for missing files', async () => {
  render(<LibraryScreen />);

  expect(await screen.findByText('Missing files')).toBeTruthy();
  expect(
    screen
      .getByRole('button', { name: /grid view/i })
      .getAttribute('data-variant'),
  ).toBe('secondary');
  expect(screen.getByText('Segment 1 video file is missing.')).toBeTruthy();
  expect(screen.getByText(/2 recordings · 1.50 MB library size/i)).toBeTruthy();
  expect(
    (screen.getByRole('button', { name: /^play$/i }) as HTMLButtonElement)
      .disabled,
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

test('LibraryScreen preserves multi-selection across views and deletes selected recordings', async () => {
  render(<LibraryScreen />);

  await screen.findByText('Ready');
  fireEvent.click(screen.getByText('Ready'));
  fireEvent.click(screen.getByText('Missing files'), { ctrlKey: true });
  expect(screen.getByText(/2 selected · 1.50 MB/i)).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /list view/i }));
  expect(screen.getByText(/2 selected · 1.50 MB/i)).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /^delete$/i }));
  fireEvent.click(screen.getAllByRole('button', { name: /^delete$/i }).at(-1)!);

  await waitFor(() =>
    expect(client.deleteLibraryRecordings).toHaveBeenCalledWith({
      recordingIds: ['recording-ready', 'recording-missing'],
    }),
  );
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
  expect(screen.getByText(/3 recordings · 3.50 MB library size/i)).toBeTruthy();

  fireEvent.click(screen.getByText('Ready'));
  fireEvent.click(screen.getByText('In progress'), { ctrlKey: true });
  expect(screen.getByText(/1 selected · 1.00 MB/i)).toBeTruthy();

  fireEvent.click(screen.getByText('Missing files'), { shiftKey: true });
  expect(screen.getByText(/2 selected · 1.50 MB/i)).toBeTruthy();

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

test('LibraryScreen deletes a recording from the right-click context menu', async () => {
  render(<LibraryScreen />);

  await screen.findByText('Ready');
  fireEvent.contextMenu(screen.getByText('Ready'));
  fireEvent.click(await screen.findByText('Delete recording'));
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
  fireEvent.contextMenu(screen.getByText('Initial capture name'));
  fireEvent.click(await screen.findByText('Rename recording'));
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

  fireEvent.contextMenu(screen.getByText('Updated capture name'));
  fireEvent.click(await screen.findByText('Rename recording'));
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
