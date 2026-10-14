import { useCallback } from 'react';
import type { LibraryRecordingDetail } from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { userEvent } from 'vitest/browser';
import {
  workflowAssets,
  workflowDetail,
} from '../testing/studio-workflow-fixture';
import { useRecordingPlaybackController } from './use-recording-playback-controller';

afterEach(cleanup);
const muted = new Set<string>();
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
