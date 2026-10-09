import type {
  LibraryRecordingSegment,
  LibraryRecordingTrack,
  LibraryWaveformTileTrack,
} from '@daydream/shared';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterAll, afterEach, beforeAll, expect, test } from 'vitest';
import { Button } from '@/components/ui/button';
import { TrackClipMedia } from '../track-lane-media';

let stylesheet: HTMLLinkElement;
beforeAll(async () => {
  stylesheet = document.createElement('link');
  stylesheet.rel = 'stylesheet';
  stylesheet.href = '/src/index.css';
  const loaded = new Promise<void>((resolve, reject) => {
    stylesheet.onload = () => resolve();
    stylesheet.onerror = () => reject(new Error('Stylesheet failed'));
  });
  document.head.append(stylesheet);
  await loaded;
});
afterEach(cleanup);
afterAll(() => stylesheet.remove());

const segment: LibraryRecordingSegment = {
  id: 'synthetic-segment',
  durationMs: 30_000,
  playbackOffsetMs: 0,
  startedAtMs: 0,
  endedAtMs: 30_000,
  segmentIndex: 1,
  pauseAfterMs: 0,
  state: 'finalized',
  fileExists: true,
  filePath: '/synthetic.mkv',
  videoStreamIndex: 0,
  desktopAudioStreamIndex: 1,
  microphoneAudioStreamIndex: 2,
};
const track: LibraryRecordingTrack = {
  id: 'microphone-2',
  kind: 'microphone',
  label: 'Microphone',
  codec: 'opus',
  description: 'Synthetic microphone',
  streamIndex: 2,
};
const waveformTrack: LibraryWaveformTileTrack = {
  trackId: track.id,
  kind: 'microphone',
  scaleAmplitude: 1,
  tiles: [
    {
      segmentId: segment.id,
      streamIndex: 2,
      binMs: 5,
      durationMs: 30_000,
      timelineStartMs: 0,
      generatedAtMs: 0,
      peakSamples: [0.2, 0.7, 0.3, 0.9, 0.4],
      rmsSamples: [0.1, 0.3, 0.1, 0.4, 0.2],
    },
  ],
};

test('an image-load failure shows a readable unavailable preview instead of a broken image', async () => {
  render(
    <TrackClipMedia
      segment={segment}
      track={{ ...track, kind: 'video' }}
      waveformTrack={undefined}
      thumbnailStatus="ready"
      thumbnails={[{ offsetMs: 6000, url: '/synthetic-missing-thumbnail.jpg' }]}
    />,
  );
  fireEvent.error(screen.getByLabelText('Video timeline thumbnail'));
  expect(
    await screen.findByRole('img', { name: 'Thumbnail unavailable' }),
  ).toBeTruthy();
  expect(screen.queryByLabelText('Video timeline thumbnail')).toBeNull();
});

test('waveform fills the audio media area inside selectable clip controls', async () => {
  render(
    <Button
      aria-label="Synthetic clip"
      style={{ position: 'relative', width: 640, height: 64 }}
    >
      <TrackClipMedia
        segment={segment}
        track={track}
        waveformTrack={waveformTrack}
        thumbnailStatus="idle"
        thumbnails={[]}
      />
    </Button>,
  );
  const clip = screen.getByRole('button', { name: 'Synthetic clip' });
  const waveform = screen.getByLabelText('Audio waveform');
  // Intentional browser visual regression: the reported bug shrinks a media
  // plot to icon dimensions. Check the visible plot, not implementation classes.
  expect(waveform.getBoundingClientRect().width).toBeGreaterThan(
    clip.getBoundingClientRect().width * 0.9,
  );
  expect(waveform.getBoundingClientRect().height).toBeGreaterThan(40);
});

test('a short audio tile with one time bin paints a visible waveform', () => {
  render(
    <div style={{ width: 640, height: 64 }}>
      <TrackClipMedia
        segment={{ ...segment, durationMs: 10 }}
        track={track}
        waveformTrack={{
          ...waveformTrack,
          tiles: waveformTrack.tiles.map((tile) => ({
            ...tile,
            durationMs: 10,
            binMs: 250,
            peakSamples: [0.5],
            rmsSamples: [0.3],
          })),
        }}
        thumbnailStatus="idle"
        thumbnails={[]}
      />
    </div>,
  );
  // Intentional browser visual coverage: a single time bin must paint an area,
  // rather than a zero-width polygon that hides short recorded audio.
  const polygon = screen
    .getByLabelText('Audio waveform')
    .querySelector('polygon')!;
  expect(polygon.getBBox().width).toBeGreaterThan(90);
});
