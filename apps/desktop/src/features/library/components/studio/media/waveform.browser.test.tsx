import type {
  LibraryRecordingSegment,
  LibraryRecordingTrack,
  LibraryWaveformTile,
  LibraryWaveformTileTrack,
} from '@daydream/shared';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterAll, afterEach, beforeAll, expect, test } from 'vitest';
import { Button } from '@/components/ui/button';
import { TrackClipMedia } from '../track-lane-media';

let stylesheet: HTMLLinkElement;
const initialDark = document.documentElement.classList.contains('dark');
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
afterEach(() => {
  cleanup();
  document.documentElement.classList.toggle('dark', initialDark);
});
afterAll(() => stylesheet.remove());

function waveformGeometry() {
  const svg = screen.getByLabelText('Audio waveform');
  if (!(svg instanceof SVGSVGElement)) throw new Error('Waveform SVG missing');
  const polygon = svg.querySelector('polygon');
  const matrix = svg.getScreenCTM();
  if (!polygon || !matrix) throw new Error('Waveform geometry missing');
  return { polygon, transform: matrix.inverse() };
}

test.each([
  { kind: 'desktop-audio' as const, start: 0, width: 960, dark: false },
  { kind: 'microphone' as const, start: 0, width: 396, dark: true },
  { kind: 'desktop-audio' as const, start: 2000, width: 396, dark: false },
  { kind: 'microphone' as const, start: 2000, width: 960, dark: true },
])(
  'clipping keeps a beep at its real time: $kind, start $start, width $width, dark $dark',
  ({ kind, start, width, dark }) => {
    document.documentElement.classList.toggle('dark', dark);
    const samples = Array.from({ length: 150 }, (_, index) =>
      index === 40 || index === 41 ? 1 : 0,
    );
    render(
      <div
        data-testid="waveform-clip"
        style={{ position: 'relative', width, height: 64 }}
      >
        <TrackClipMedia
          segment={{ ...segment, durationMs: 12_603, playbackOffsetMs: start }}
          track={{ ...track, kind }}
          waveformTrack={{
            ...waveformTrack,
            tiles: [
              {
                ...waveformTile,
                durationMs: 15_000,
                binMs: 100,
                peakSamples: samples,
                rmsSamples: samples,
              },
            ],
          }}
          thumbnailStatus="idle"
          thumbnails={[]}
        />
      </div>,
    );
    const bounds = screen.getByTestId('waveform-clip').getBoundingClientRect();
    const { polygon, transform } = waveformGeometry();
    // Intentional visual timing regression: sample the painted waveform at the
    // real beep's timeline position, rather than asserting styles or SVG strings.
    function paintedAt(timeMs: number) {
      const point = new DOMPoint(
        bounds.left + ((timeMs - start) / 12_603) * bounds.width,
        bounds.top + 16,
      ).matrixTransform(transform);
      return polygon.isPointInFill(point);
    }
    expect(paintedAt(4100)).toBe(true);
    expect(paintedAt(3360)).toBe(false);
    expect(paintedAt(5300)).toBe(false);
  },
);

test('a partial final waveform bin occupies only its remaining time', () => {
  render(
    <div
      data-testid="waveform-clip"
      style={{ position: 'relative', width: 630, height: 64 }}
    >
      <TrackClipMedia
        segment={{ ...segment, durationMs: 1050 }}
        track={track}
        waveformTrack={{
          ...waveformTrack,
          tiles: [
            {
              ...waveformTile,
              durationMs: 1050,
              binMs: 500,
              peakSamples: [0, 0, 1],
              rmsSamples: [0, 0, 1],
            },
          ],
        }}
        thumbnailStatus="idle"
        thumbnails={[]}
      />
    </div>,
  );
  const bounds = screen.getByTestId('waveform-clip').getBoundingClientRect();
  const { polygon, transform } = waveformGeometry();
  expect(
    polygon.isPointInFill(
      new DOMPoint(
        bounds.left + (bounds.width * 800) / 1050,
        bounds.top + 32,
      ).matrixTransform(transform),
    ),
  ).toBe(false);
  expect(
    polygon.isPointInFill(
      new DOMPoint(
        bounds.left + (bounds.width * 1025) / 1050,
        bounds.top + 32,
      ).matrixTransform(transform),
    ),
  ).toBe(true);
});

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
const waveformTile: LibraryWaveformTile = {
  segmentId: segment.id,
  streamIndex: 2,
  binMs: 5000,
  durationMs: 30_000,
  timelineStartMs: 0,
  generatedAtMs: 0,
  peakSamples: [0.2, 0.7, 0.3, 0.9, 0.4, 0.5],
  rmsSamples: [0.1, 0.3, 0.1, 0.4, 0.2, 0.3],
};
const waveformTrack: LibraryWaveformTileTrack = {
  trackId: track.id,
  kind: 'microphone',
  scaleAmplitude: 1,
  tiles: [waveformTile],
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
