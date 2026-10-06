import type {
  LibraryRecordingSegment,
  LibraryRecordingTrack,
  LibraryWaveformTileTrack,
} from '@daydream/shared';
import { cleanup, render, screen } from '@testing-library/react';
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
