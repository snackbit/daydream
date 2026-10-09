import { useState } from 'react';
import type * as TauriCore from '@tauri-apps/api/core';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import { RecordingStudio } from '../../components/studio/recording-studio';
import { useStudioTimelineStore } from '../../components/studio/studio-timeline-store';
import { createLibraryPlaybackSession } from '../../library-client';
import syntheticMedia from './fixtures/synthetic-player.webm?url';
import { studioStoryRecording } from './studio-route-fixture';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
const detail: LibraryRecordingDetail = {
  ...studioStoryRecording,
  availability: 'available',
  missingReasons: [],
  playableSegmentCount: 1,
  tracks: [
    {
      id: 'video-0',
      kind: 'video',
      label: 'Screen',
      codec: 'h264',
      description: '',
      streamIndex: 0,
    },
    {
      id: 'desktop-audio-1',
      kind: 'desktop-audio',
      label: 'Desktop audio',
      codec: 'opus',
      description: '',
      streamIndex: 1,
    },
    {
      id: 'microphone-2',
      kind: 'microphone',
      label: 'Microphone',
      codec: 'opus',
      description: '',
      streamIndex: 2,
    },
  ],
  segments: [
    {
      id: 'segment',
      durationMs: 10_000,
      endedAtMs: 11_000,
      startedAtMs: 1_000,
      fileExists: true,
      filePath: '/tmp/synthetic-story.mkv',
      pauseAfterMs: 0,
      playbackOffsetMs: 0,
      segmentIndex: 0,
      state: 'finalized',
      videoStreamIndex: 0,
      desktopAudioStreamIndex: 1,
      microphoneAudioStreamIndex: 2,
    },
  ],
};
const assets: LibraryPlaybackAssets = {
  recordingId: detail.id,
  sessionId: 'synthetic-session',
  error: null,
  segments: [
    {
      segmentId: 'segment',
      videoUrl: new URL(syntheticMedia, window.location.origin).href,
      thumbnails: [],
      audio: [
        {
          kind: 'desktop-audio',
          trackId: 'desktop-audio-1',
          streamIndex: 1,
          url: `${new URL(syntheticMedia, window.location.origin).href}?source=desktop`,
        },
        {
          kind: 'microphone',
          trackId: 'microphone-2',
          streamIndex: 2,
          url: `${new URL(syntheticMedia, window.location.origin).href}?source=microphone`,
        },
      ],
    },
  ],
};
const noAction = () => undefined;
const noMutation = async () => undefined;
beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'createLibraryPlaybackSession') return assets;
    if (command === 'getRecordingWaveformTiles')
      return { recordingId: detail.id, binMs: 10, tracks: [], error: null };
    throw new Error(`Unexpected native command: ${command}`);
  });
  vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined);
  vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(noAction);
});

function PreparationRetryStudio() {
  const [playbackAssets, setPlaybackAssets] = useState<LibraryPlaybackAssets>({
    ...assets,
    error: 'Synthetic preparation failed',
    segments: [],
  });
  async function retry() {
    setPlaybackAssets(
      await createLibraryPlaybackSession({ recordingId: detail.id }),
    );
  }
  function refresh() {
    void retry();
  }
  return (
    <RecordingStudio
      detail={detail}
      playbackAssets={playbackAssets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={refresh}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />
  );
}

test('STU-14 Retry loading rebuilds failed playback assets through the actual typed client and restores playable controls', async () => {
  render(<PreparationRetryStudio />);
  expect(await screen.findByText('Synthetic preparation failed')).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Retry loading' }));
  expect(await screen.findByRole('button', { name: 'Play' })).toBeTruthy();
  expect(screen.queryByText('Synthetic preparation failed')).toBeNull();
  expect(native.invoke).toHaveBeenCalledWith('createLibraryPlaybackSession', {
    input: { recordingId: detail.id },
  });
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

test('STU-03 adjusting volume and global mute preserves independent source mute while editing the Inspector', async () => {
  const view = render(
    <RecordingStudio
      detail={detail}
      playbackAssets={assets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  await screen.findByRole('button', { name: 'Mute Desktop audio' });
  const desktop = view.container.querySelector<HTMLAudioElement>(
    'audio[src*="desktop"]',
  );
  const microphone = view.container.querySelector<HTMLAudioElement>(
    'audio[src*="microphone"]',
  );
  if (!desktop || !microphone)
    throw new Error('Synthetic source audio elements missing');
  expect(desktop.muted).toBe(false);
  expect(microphone.muted).toBe(false);
  await userEvent.click(
    screen.getByRole('button', { name: 'Mute Desktop audio' }),
  );
  await waitFor(() => expect(desktop.muted).toBe(true));
  expect(microphone.muted).toBe(false);
  await userEvent.click(screen.getByRole('button', { name: 'Mute' }));
  await waitFor(() => expect(microphone.muted).toBe(true));
  await userEvent.click(screen.getByRole('button', { name: 'Unmute' }));
  await waitFor(() => expect(microphone.muted).toBe(false));
  expect(desktop.muted).toBe(true);
  const volume = screen.getByRole('slider', { name: 'Volume' });
  volume.focus();
  await userEvent.keyboard('{Home}{ArrowRight}{ArrowRight}{ArrowRight}');
  await waitFor(() => expect(microphone.volume).toBeCloseTo(0.15));
  expect(desktop.volume).toBeCloseTo(0.15);
  await userEvent.click(
    screen.getByRole('button', { name: 'Select Microphone' }),
  );
  const trackName = await screen.findByRole('textbox', { name: 'Track name' });
  await userEvent.fill(trackName, 'Interview source');
  await userEvent.click(
    screen.getByRole('button', { name: 'Mute Interview source' }),
  );
  await waitFor(() => expect(microphone.muted).toBe(true));
  await userEvent.click(
    screen.getByRole('button', { name: 'Unmute Desktop audio' }),
  );
  await waitFor(() => expect(desktop.muted).toBe(false));
  expect(microphone.muted).toBe(true);
  expect(trackName).toHaveValue('Interview source');
  expect(Number(volume.getAttribute('aria-valuenow'))).toBeCloseTo(0.15);
});

test('STU-05 skip buttons clamp both ends while paused and do not autoplay', async () => {
  render(
    <RecordingStudio
      detail={detail}
      playbackAssets={assets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  const position = await screen.findByRole('slider', {
    name: 'Playback position',
  });
  await userEvent.click(screen.getByRole('button', { name: 'Skip back' }));
  expect(position).toHaveAttribute('aria-valuenow', '0');
  await userEvent.click(screen.getByRole('button', { name: 'Skip forward' }));
  expect(position).toHaveAttribute('aria-valuenow', '10000');
  await userEvent.click(screen.getByRole('button', { name: 'Skip forward' }));
  expect(position).toHaveAttribute('aria-valuenow', '10000');
  await userEvent.click(screen.getByRole('button', { name: 'Skip back' }));
  expect(position).toHaveAttribute('aria-valuenow', '0');
  expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy();
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
});

test('STU-06 keyboard reorders a track and cancelling a later drag preserves its committed order', async () => {
  render(
    <RecordingStudio
      detail={detail}
      playbackAssets={assets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  const reorder = await screen.findByRole('button', {
    name: 'Reorder Microphone',
  });
  function labels() {
    return screen
      .getAllByRole('button', { name: /^Select / })
      .map((element) => element.getAttribute('aria-label'));
  }
  const before = labels();
  await userEvent.click(reorder);
  await userEvent.keyboard('{Enter}');
  await screen.findByText(
    /^(Picked up draggable item microphone-2\.|Draggable item microphone-2 was moved over)/,
  );
  await userEvent.keyboard('{ArrowUp}');
  await screen.findByText(
    'Draggable item microphone-2 was moved over droppable area desktop-audio-1.',
  );
  await userEvent.keyboard('{Enter}');
  await waitFor(() => expect(labels()).not.toEqual(before));
  const committed = labels();
  expect(committed.indexOf('Select Microphone')).toBe(
    before.indexOf('Select Microphone') - 1,
  );
  await userEvent.click(reorder);
  await userEvent.keyboard('{Enter}');
  await screen.findByText(
    /^(Picked up draggable item microphone-2\.|Draggable item microphone-2 was moved over)/,
  );
  await userEvent.keyboard('{ArrowUp}{Escape}');
  expect(labels()).toEqual(committed);
  expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy();
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
});

test('STU-12 Skip redacted part reaches its exact eligible boundary without playing paused media', async () => {
  const redacted = {
    ...detail,
    redactions: [
      {
        id: 'video-cut',
        recordingId: detail.id,
        trackIds: ['video-0'],
        startOffsetMs: 0,
        endOffsetMs: 3000,
        createdAtMs: 1,
        reason: null,
      },
    ],
  };
  render(
    <RecordingStudio
      detail={redacted}
      playbackAssets={assets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  await userEvent.click(
    await screen.findByRole('button', { name: 'Skip redacted part' }),
  );
  expect(
    screen.getByRole('slider', { name: 'Playback position' }),
  ).toHaveAttribute('aria-valuenow', '3000');
  expect(screen.queryByLabelText('Redacted video')).toBeNull();
  expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy();
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
});

test('STU-13 Retry playback clears the rejected play and resumes the same source after canplay', async () => {
  vi.mocked(HTMLMediaElement.prototype.play).mockRejectedValueOnce(
    new Error('Synthetic playback blocked'),
  );
  const view = render(
    <RecordingStudio
      detail={detail}
      playbackAssets={assets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  expect(await screen.findByText(/Synthetic playback blocked/)).toBeTruthy();
  const video = view.container.querySelector('video');
  if (!video) throw new Error('Synthetic video element missing');
  await userEvent.click(screen.getByRole('button', { name: 'Retry playback' }));
  expect(screen.queryByText(/Synthetic playback blocked/)).toBeNull();
  fireEvent.canPlay(video);
  fireEvent.play(video);
  expect(await screen.findByRole('button', { name: 'Pause' })).toBeTruthy();
  expect(
    screen.getByRole('slider', { name: 'Playback position' }),
  ).toHaveAttribute('aria-valuenow', '0');
  expect(video.getAttribute('src')).toContain('synthetic-player.webm');
});
