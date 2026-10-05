import type { AnchorHTMLAttributes, ReactNode } from 'react';
import type { LibraryRecordingSummary } from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { HomeScreen } from './HomeScreen';

const libraryClient = vi.hoisted(() => ({
  listLibraryRecordings: vi.fn(),
}));

vi.mock('@/features/library/library-client', () => libraryClient);
vi.mock('@tanstack/react-router', () => ({
  Link: ({
    children,
    params,
    to,
    ...anchorProps
  }: AnchorHTMLAttributes<HTMLAnchorElement> & {
    children: ReactNode;
    params?: { recordingId: string };
    to: string;
  }) => (
    <a href={params ? `/library/${params.recordingId}` : to} {...anchorProps}>
      {children}
    </a>
  ),
}));

beforeEach(() => {
  vi.resetAllMocks();
  libraryClient.listLibraryRecordings.mockResolvedValue([]);
});

afterEach(() => {
  cleanup();
});

test('Home shows the newest local sessions and gives playable sessions a timeline link', async () => {
  const oldest = recording('oldest', 'Older session', 1_000);
  const recent = recording('recent', 'Recent session', 3_000);
  const newest = recording('newest', 'Newest session', 5_000);
  libraryClient.listLibraryRecordings.mockResolvedValue([
    recent,
    oldest,
    newest,
  ]);

  render(<HomeScreen />);

  const newestLink = await screen.findByRole('link', {
    name: /open newest session/i,
  });
  const recordingLinks = screen
    .getAllByRole('link')
    .filter((link) => link.getAttribute('href')?.startsWith('/library/'));
  expect(newestLink.getAttribute('href')).toBe('/library/newest');
  expect(recordingLinks.map((link) => link.getAttribute('href'))).toEqual([
    '/library/newest',
    '/library/recent',
    '/library/oldest',
  ]);
  expect(screen.getByText('3 · 3.00 MB')).toBeTruthy();
});

test('Home shows an empty state without hiding the capture action', async () => {
  render(<HomeScreen />);

  expect(await screen.findByText('No recordings yet')).toBeTruthy();
  const emptyState = screen.getByRole('status');
  expect(emptyState.textContent).toContain('No recordings yet');
  expect(
    screen.getByRole('link', { name: /prepare capture/i }).getAttribute('href'),
  ).toBe('/capture');
});

test('Home exposes loading state while the local recording list is pending', () => {
  libraryClient.listLibraryRecordings.mockReturnValue(new Promise(() => {}));

  render(<HomeScreen />);

  expect(screen.getByRole('status').getAttribute('aria-busy')).toBe('true');
});

test('Home retries a failed recent-recordings load and recovers visibly', async () => {
  libraryClient.listLibraryRecordings
    .mockRejectedValueOnce(new Error('Local library is unavailable.'))
    .mockResolvedValueOnce([recording('retry', 'Recovered session', 4_000)]);

  render(<HomeScreen />);

  expect(await screen.findByRole('alert')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Try again' }));

  expect(
    await screen.findByRole('link', { name: /open recovered session/i }),
  ).toBeTruthy();
  await waitFor(() =>
    expect(libraryClient.listLibraryRecordings).toHaveBeenCalledTimes(2),
  );
});

function recording(
  id: string,
  title: string,
  startedAtMs: number,
): LibraryRecordingSummary {
  return {
    availability: 'available',
    durationMs: 120_000,
    endedAtMs: startedAtMs + 120_000,
    id,
    missingReasons: [],
    playableSegmentCount: 1,
    segmentCount: 1,
    sizeBytes: 1_000_000,
    startedAtMs,
    status: 'completed',
    storageDir: `/tmp/daydream/recordings/${id}`,
    thumbnailPath: null,
    title,
  };
}
