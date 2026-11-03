import { useCallback, useMemo } from 'react';
import type { LibraryRecordingDetail } from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import {
  workflowAssets,
  workflowDetail,
} from '../testing/studio-workflow-fixture';
import { StudioPlayer } from './controls/studio-player';
import { useRecordingPlaybackController } from './use-recording-playback-controller';

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});
const muted = new Set<string>();
function PreparedPlayback() {
  const assets = useMemo(
    () => ({
      ...workflowAssets(workflowDetail),
      segments: workflowAssets(workflowDetail).segments.map((segment) => ({
        ...segment,
        canSeekInStream: true,
      })),
    }),
    [],
  );
  const playback = useRecordingPlaybackController({
    detail: workflowDetail,
    playbackAssets: assets,
    mutedTrackIds: muted,
  });
  const seek = useCallback(
    () => playback.actions.seekTimeline(500),
    [playback.actions],
  );
  const refresh = useCallback(() => undefined, []);
  return (
    <>
      <button onClick={seek}>Seek prepared media</button>
      <StudioPlayer
        detail={workflowDetail}
        playbackAssets={assets}
        playback={playback}
        onRefreshPlayback={refresh}
      />
    </>
  );
}

test('prepared media seeks in the existing decoded element and preserves its URL', async () => {
  const view = render(<PreparedPlayback />);
  await waitFor(() =>
    expect(
      view.container.querySelector('video')?.readyState,
    ).toBeGreaterThanOrEqual(2),
  );
  const video = view.container.querySelector('video');
  if (!video) throw new Error('Prepared video missing');
  const url = video.currentSrc;
  expect(url).not.toContain('startMs=');
  await userEvent.click(
    screen.getByRole('button', { name: 'Seek prepared media' }),
  );
  await waitFor(() => expect(video.currentTime).toBeCloseTo(0.5, 1));
  expect(view.container.querySelector('video')).toBe(video);
  expect(video.currentSrc).toBe(url);
  await userEvent.click(screen.getByRole('button', { name: /^Play$/ }));
  await waitFor(() => expect(video.currentTime).toBeGreaterThan(0.5));
});

test.each(['buffered', 'empty', 'behind', 'seeking'])(
  'a sparse video waiting event respects %s data before pausing playback',
  async (mode) => {
    const view = render(<PreparedPlayback />);
    await userEvent.click(screen.getByRole('button', { name: /^Play$/ }));
    const video = view.container.querySelector('video');
    if (!video) throw new Error('Prepared video missing');
    await waitFor(() => expect(video.currentTime).toBeGreaterThan(0.05));
    const pause = vi.spyOn(video, 'pause');
    let waiting = true;
    Object.defineProperties(video, {
      readyState: { configurable: true, get: () => (waiting ? 2 : 4) },
      seeking: { configurable: true, get: () => waiting && mode === 'seeking' },
      buffered: {
        configurable: true,
        get: () => ({
          length: mode === 'empty' ? 0 : 1,
          start: () => 0,
          end: () => (mode === 'behind' ? 0.01 : 10),
        }),
      },
    });
    fireEvent.waiting(video);
    if (mode === 'buffered') {
      expect(pause).not.toHaveBeenCalled();
      const time = video.currentTime;
      await waitFor(() =>
        expect(video.currentTime).toBeGreaterThan(time + 0.05),
      );
    } else {
      expect(pause).toHaveBeenCalled();
      expect(video.paused).toBe(true);
      waiting = false;
      fireEvent.canPlay(video);
      await waitFor(() => expect(video.paused).toBe(false));
    }
  },
);
function UnattachedPlayback({ detail }: { detail: LibraryRecordingDetail }) {
  const playback = useRecordingPlaybackController({
    detail,
    playbackAssets: workflowAssets(detail),
    mutedTrackIds: muted,
  });
  const start = useCallback(() => {
    void playback.actions.togglePlayback();
  }, [playback.actions]);
  const seekEnd = useCallback(
    () => playback.actions.seekTimeline(10000),
    [playback.actions],
  );
  const seekBefore = useCallback(
    () => playback.actions.seekTimeline(-1),
    [playback.actions],
  );
  return (
    <>
      <button onClick={start}>Play before attachment</button>
      <button onClick={seekEnd}>Seek end</button>
      <button onClick={seekBefore}>Seek before start</button>
      <output>
        {playback.state.playRequested ? 'Requested' : 'Paused'} ·{' '}
        {playback.state.displayedTimelineMs}
      </output>
    </>
  );
}
test('early Play cannot claim playback before a media element is registered', async () => {
  render(<UnattachedPlayback detail={workflowDetail} />);
  await userEvent.click(
    screen.getByRole('button', { name: 'Play before attachment' }),
  );
  expect(screen.getByText('Paused · 0')).toBeTruthy();
});
test('empty recording seeks remain inert while both ends of the public timeline API are checked', async () => {
  render(<UnattachedPlayback detail={{ ...workflowDetail, segments: [] }} />);
  await userEvent.click(screen.getByRole('button', { name: 'Seek end' }));
  await userEvent.click(
    screen.getByRole('button', { name: 'Seek before start' }),
  );
  await waitFor(() => expect(screen.getByText('Paused · 0')).toBeTruthy());
});

test('replay resets the completed source and queues playback until its replacement media attaches', async () => {
  render(<UnattachedPlayback detail={workflowDetail} />);
  await userEvent.click(screen.getByRole('button', { name: 'Seek end' }));
  expect(screen.getByText('Paused · 1000')).toBeTruthy();
  await userEvent.click(
    screen.getByRole('button', { name: 'Play before attachment' }),
  );
  await screen.findByText('Requested · 0');
});
