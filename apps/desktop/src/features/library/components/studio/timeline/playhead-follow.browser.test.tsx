import { useCallback } from 'react';
import {
  cleanup,
  render,
  screen,
  fireEvent,
  waitFor,
} from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { userEvent } from 'vitest/browser';
import {
  useTrackTimelineViewport,
  useTrackTimelinePlayheadFollow,
} from '../use-track-timeline-viewport';

afterEach(cleanup);
function Timeline({
  head,
  seek = 0,
  suspended = false,
}: {
  head: number;
  seek?: number;
  suspended?: boolean;
}) {
  const {
    viewportRef,
    followPlayhead,
    keepPlayheadVisible,
    setFollowPlayhead,
  } = useTrackTimelineViewport({
    detailId: 'synthetic',
    isPlaying: true,
    totalDurationMs: 60000,
  });
  useTrackTimelinePlayheadFollow({
    followPlayhead,
    isPlaying: true,
    keepPlayheadVisible,
    playheadLeftPx: head,
    seekRevision: seek,
    setFollowPlayhead,
    suspended,
  });
  const follow = useCallback(
    () => setFollowPlayhead(true),
    [setFollowPlayhead],
  );
  return (
    <>
      <div
        ref={viewportRef}
        aria-label="Timeline viewport"
        style={{ width: 600, height: 120, overflow: 'auto' }}
      >
        <div style={{ width: 2400, height: 200 }}>
          <span style={{ position: 'relative', left: head }}>Playhead</span>
        </div>
      </div>
      {followPlayhead ? (
        <p>Following playback</p>
      ) : (
        <button type="button" onClick={follow}>
          Follow playback
        </button>
      )}
    </>
  );
}
test('playback follows continuously, manual horizontal browsing detaches, and explicit follow or seek resumes', async () => {
  const view = render(<Timeline head={300} />);
  const viewport = screen.getByLabelText('Timeline viewport');
  view.rerender(<Timeline head={1200} />);
  await waitFor(() => expect(viewport.scrollLeft).toBeGreaterThan(600));
  const first = viewport.scrollLeft;
  view.rerender(<Timeline head={1300} />);
  await waitFor(() => expect(viewport.scrollLeft).toBeGreaterThan(first));
  // Scroll position is the owned visible contract of playback following.
  viewport.scrollLeft = 100;
  fireEvent.scroll(viewport);
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Follow playback' }),
    ).toBeTruthy(),
  );
  view.rerender(<Timeline head={1400} />);
  expect(viewport.scrollLeft).toBe(100);
  await userEvent.click(
    screen.getByRole('button', { name: 'Follow playback' }),
  );
  await waitFor(() => expect(viewport.scrollLeft).toBeGreaterThan(800));
  viewport.scrollLeft = 200;
  fireEvent.scroll(viewport);
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Follow playback' }),
    ).toBeTruthy(),
  );
  view.rerender(<Timeline head={400} seek={1} />);
  await waitFor(() =>
    expect(screen.getByText('Following playback')).toBeTruthy(),
  );
  const before = viewport.scrollLeft;
  view.rerender(<Timeline head={1600} seek={1} suspended />);
  expect(viewport.scrollLeft).toBe(before);
});
