import { useCallback, useState } from 'react';
import type {
  LibraryRecordingDetail,
  OcrTimelineFrame,
} from '@daydream/shared';
import { cleanup, render } from '@testing-library/react';
import { afterAll, afterEach, beforeAll, expect, test } from 'vitest';
import { page } from 'vitest/browser';
import {
  buildStudioVisualFrames,
  type StudioVisualFrame,
} from './studio-visual-observations';
import { VisualFrameSegments } from './track-lane-visual-markers';

const durationMs = 38_000;
const detail: LibraryRecordingDetail = {
  availability: 'available',
  durationMs,
  endedAtMs: durationMs,
  id: 'synthetic-recording',
  missingReasons: [],
  playableSegmentCount: 1,
  redactions: [],
  sizeBytes: 0,
  startedAtMs: 0,
  status: 'completed',
  storageDir: '/tmp/synthetic',
  thumbnailPath: null,
  title: null,
  tracks: [
    {
      codec: 'h264',
      description: 'Composited video',
      id: 'video-0',
      kind: 'video',
      label: 'Video',
      streamIndex: 0,
    },
  ],
  segments: [
    {
      desktopAudioStreamIndex: null,
      durationMs,
      endedAtMs: durationMs,
      fileExists: true,
      filePath: '/tmp/synthetic.mkv',
      id: 'segment-1',
      microphoneAudioStreamIndex: null,
      pauseAfterMs: 0,
      playbackOffsetMs: 0,
      segmentIndex: 0,
      startedAtMs: 0,
      state: 'finalized',
      videoStreamIndex: 0,
    },
  ],
};
const samples: OcrTimelineFrame[] = [0, 5_000, 10_000].map(
  (frameTimestampMs) => ({
    documentId: `sample-${frameTimestampMs}`,
    errorCode: null,
    frameTimestampMs,
    id: `sample-${frameTimestampMs}`,
    ocrOutputCount: 1,
    recordingId: detail.id,
    sampleEndMs: frameTimestampMs + 1,
    segmentId: 'segment-1',
    sourceFrameRef: `opaque-${frameTimestampMs}`,
    sourceTrackId: 'video-0',
    status: 'succeeded',
    textEmpty: false,
  }),
);
const frames = buildStudioVisualFrames(null, detail, samples);
let stylesheet: HTMLLinkElement;
const originalTheme = document.documentElement.classList.contains('dark');
const originalViewport = {
  width: window.innerWidth,
  height: window.innerHeight,
};

beforeAll(async () => {
  stylesheet = document.createElement('link');
  stylesheet.rel = 'stylesheet';
  stylesheet.href = '/src/index.css';
  const loaded = new Promise<void>((resolve, reject) => {
    stylesheet.addEventListener('load', () => resolve(), { once: true });
    stylesheet.addEventListener(
      'error',
      () => reject(new Error('Could not load observation styles.')),
      { once: true },
    );
  });
  document.head.append(stylesheet);
  await loaded;
});
afterEach(() => {
  cleanup();
  document.documentElement.classList.toggle('dark', originalTheme);
});
afterAll(async () => {
  stylesheet.remove();
  await page.viewport(originalViewport.width, originalViewport.height);
});

function ObservationRow({ width }: { width: number }) {
  const [selected, setSelected] = useState<StudioVisualFrame | null>(null);
  const handleSelect = useCallback(
    (frame: StudioVisualFrame) => setSelected(frame),
    [],
  );
  return (
    <>
      <div
        role="group"
        aria-label="Visual observations"
        style={{ position: 'relative', height: 40, width }}
      >
        <VisualFrameSegments
          onSelectVisualFrame={handleSelect}
          selectedVisualFrameIds={new Set(selected ? [selected.id] : [])}
          timelineWidthPx={width}
          totalDurationMs={durationMs}
          visualFrameStates={[]}
          visualFrames={frames}
        />
      </div>
      <output aria-label="Selected observation">
        {selected?.frameTimestampMs ?? 'None'}
      </output>
    </>
  );
}

test.each([
  { width: 1440, dark: false },
  { width: 1440, dark: true },
  { width: 620, dark: false },
  { width: 620, dark: true },
])(
  'observation cells remain clickable between samples at $width px, dark=$dark',
  async ({ width, dark }) => {
    await page.viewport(width, 960);
    document.documentElement.classList.toggle('dark', dark);
    const timelineWidth = width - 32;
    render(<ObservationRow width={timelineWidth} />);
    const row = page.getByRole('group', { name: 'Visual observations' });
    // Real browser clicks inside each navigation interval catch the tiny-dot regression.
    // The last sample keeps the remaining segment width without altering its evidence.
    const clicks: ReadonlyArray<readonly [number, number]> = [
      [2_500, 0],
      [7_500, 5_000],
      [25_000, 10_000],
    ];
    for (const [clickTime, observationTime] of clicks) {
      await row.click({
        position: { x: (clickTime / durationMs) * timelineWidth, y: 25 },
      });
      await expect
        .element(page.getByLabelText('Selected observation'))
        .toHaveTextContent(String(observationTime));
    }
    expect(frames.map((frame) => frame.ocrSampleEndMs)).toEqual([
      1, 5_001, 10_001,
    ]);
  },
);
