import type {
  LibraryRecordingTrack,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { settleVisualTransitions } from '@/app/design-accessibility-audit';
import { TooltipProvider } from '@/components/ui/tooltip';
import '@/index.css';
import { studioStoryRecording } from '../../../recording-studio-route/stories/studio-route-fixture';
import { ActivityTrackLane } from '../activity-track-lane';
import { SegmentTranscriptionLayer } from '../derived-ai-lane-strip';
import { TrackLane } from '../track-lane';
import { VisualFrameSegments } from '../track-lane-visual-markers';
import { trackLaneHeightPxForKind } from '../track-timeline-math';

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});
const tracks: LibraryRecordingTrack[] = [
  {
    id: 'video-0',
    kind: 'video',
    label: 'Monitor',
    description: 'Synthetic screen',
    codec: 'h264',
    streamIndex: 0,
  },
  {
    id: 'microphone-2',
    kind: 'microphone',
    label: 'Microphone',
    description: 'Synthetic microphone',
    codec: 'opus',
    streamIndex: 2,
  },
];
const segment = {
  id: 'synthetic-segment',
  durationMs: 10_000,
  playbackOffsetMs: 0,
  startedAtMs: 0,
  endedAtMs: 10_000,
  segmentIndex: 1,
  pauseAfterMs: 0,
  state: 'finalized' as const,
  fileExists: true,
  filePath: '/synthetic.mkv',
  videoStreamIndex: 0,
  desktopAudioStreamIndex: 1,
  microphoneAudioStreamIndex: 2,
};
const transcript = {
  id: 'synthetic-transcript',
  segmentId: segment.id,
  startOffsetMs: 2_000,
  endOffsetMs: 8_000,
  text: 'Synthetic transcript wraps onto a second readable line in this annotation strip. Additional synthetic words keep the example long enough to exercise two lines at every required viewport width.',
  words: [],
  confidence: null,
  provider: 'synthetic',
  model: null,
};
const processing: RecordingAiProcessingState = {
  recordingId: studioStoryRecording.id,
  chunks: [],
  job: null,
  transcript: { recordingId: studioStoryRecording.id, tracks: [] },
  transcriptionTracks: [
    {
      trackId: tracks[1]!.id,
      trackKey: tracks[1]!.id,
      textSegmentCount: 1,
      libraryTrackId: tracks[1]!.id,
      sourceRole: 'microphone',
      label: 'Microphone',
      description: 'Synthetic transcript',
      status: 'complete',
      statusLabel: 'Complete',
      chunks: [
        {
          id: 'synthetic-chunk',
          segmentId: segment.id,
          startOffsetMs: 0,
          endOffsetMs: 10_000,
          attemptCount: 1,
          status: 'succeeded',
          result: 'text',
          resultCode: null,
          errorCode: null,
          progress: 1,
          text: transcript.text,
          transcriptSegments: [transcript],
        },
      ],
    },
  ],
};

test('retained overview spans do not suppress loaded transcripts on the same source lane', async () => {
  const track = tracks[1]!;
  const inspect = vi.fn();
  render(
    <TooltipProvider>
      <div className="grid" style={{ gridTemplateColumns: '216px 1000px' }}>
        <TrackLane
          track={track}
          lane={{
            trackId: track.id,
            kind: track.kind,
            label: track.label,
            description: '',
            colorId: 'rose',
          }}
          detail={{ ...studioStoryRecording, tracks, segments: [segment] }}
          visibleRange={{ startTimelineMs: 0, endTimelineMs: 10_000 }}
          aiProcessingState={{
            ...processing,
            transcriptionTracks: [
              {
                ...processing.transcriptionTracks![0]!,
                trackKey: 'earlier-recording',
                chunks: [],
                textSegmentCount: 0,
              },
              ...processing.transcriptionTracks!,
            ],
            timelineOverview: [
              {
                trackId: track.id,
                kind: 'speech',
                startOffsetMs: 9000,
                endOffsetMs: 9500,
                count: 4,
              },
            ],
          }}
          clipDrafts={{}}
          dragHandleProps={{}}
          mutedTrackIds={new Set()}
          onSelectClip={vi.fn()}
          onInspectTranscript={inspect}
          onSelectLane={vi.fn()}
          onSelectRedaction={vi.fn()}
          onToggleTrackMuted={vi.fn()}
          playbackAssets={null}
          selectedClipIds={new Set()}
          selectedLane={false}
          selectedRedactionIds={new Set()}
          selectedVisualFrameIds={new Set()}
          thumbnailStatus="idle"
          timelineWidthPx={1000}
          timeRanges={[]}
          totalDurationMs={10_000}
          visualFrames={[]}
          waveformTiles={null}
          onSelectVisualFrame={vi.fn()}
          onZoomRange={vi.fn()}
        />
      </div>
    </TooltipProvider>,
  );
  await userEvent.click(
    screen.getByRole('button', { name: /Inspect Microphone transcript/ }),
  );
  expect(inspect).toHaveBeenCalledWith(
    expect.objectContaining({ text: transcript.text }),
  );
  expect(
    screen.getByRole('button', { name: 'Zoom into 4 transcript entries' }),
  ).toBeVisible();
});

test('zoomed-out speech and screen entries expose readable count actions with their original support range', async () => {
  const zoom = vi.fn();
  const chunk = processing.transcriptionTracks![0]!.chunks[0]!;
  const entries = [
    { ...transcript, startOffsetMs: 2000, endOffsetMs: 2100 },
    { ...transcript, id: 'second', startOffsetMs: 2200, endOffsetMs: 2300 },
  ];
  render(
    <TooltipProvider>
      <div style={{ position: 'relative', width: 1000, height: 100 }}>
        <SegmentTranscriptionLayer
          color="var(--foreground)"
          track={tracks[1]!}
          segment={{ ...segment, durationMs: 28_800_000 }}
          lane={{
            description: '',
            label: 'Speech',
            status: 'complete',
            statusLabel: 'Complete',
            trackState: null,
            chunks: [{ ...chunk, transcriptSegments: entries }],
          }}
          visibleRange={{ startTimelineMs: 0, endTimelineMs: 500_000 }}
          timelineWidthPx={1000}
          onInspectTranscript={vi.fn()}
          onZoomRange={zoom}
        />
      </div>
      <div style={{ position: 'relative', width: 1000, height: 100 }}>
        <VisualFrameSegments
          visualFrames={entries.map((entry) => ({
            id: entry.id,
            frameTimestampMs: entry.startOffsetMs,
            endTimestampMs: entry.endOffsetMs,
            sourceFrameRef: 'synthetic',
            sourceFrameRefs: [],
            ocrRegions: [],
            vlmObservations: [],
          }))}
          visualFrameStates={[]}
          selectedVisualFrameIds={new Set()}
          timelineWidthPx={1000}
          totalDurationMs={28_800_000}
          onSelectVisualFrame={vi.fn()}
          onZoomRange={zoom}
        />
      </div>
    </TooltipProvider>,
  );
  const speechGroup = screen.getByRole('button', {
    name: 'Zoom into 2 transcript entries',
  });
  await userEvent.hover(speechGroup);
  await screen.findByRole('tooltip');
  expect(screen.getByRole('tooltip')).toHaveTextContent(
    transcript.text.slice(0, 240),
  );
  await userEvent.click(speechGroup);
  expect(zoom).toHaveBeenLastCalledWith(2000, 2300);
  await userEvent.click(
    screen.getByRole('button', { name: 'Zoom into 2 screen entries' }),
  );
  expect(zoom).toHaveBeenLastCalledWith(2000, 2300);
});

test('scoped transcript actions remain reachable above retained historical chunk status', async () => {
  const chunk = processing.transcriptionTracks?.[0]?.chunks[0];
  const track = tracks[1];
  if (!chunk || !track)
    throw new Error('Synthetic transcript fixture is missing');
  const inspect = vi.fn();
  render(
    <TooltipProvider>
      <div style={{ position: 'relative', width: 800, height: 100 }}>
        <SegmentTranscriptionLayer
          color="var(--foreground)"
          track={track}
          segment={segment}
          onInspectTranscript={inspect}
          lane={{
            description: 'Synthetic transcript',
            label: 'Microphone',
            status: 'complete',
            statusLabel: 'Complete',
            trackState: null,
            chunks: [
              {
                ...chunk,
                id: 'current-scope',
                startOffsetMs: 2000,
                endOffsetMs: 8000,
              },
              {
                ...chunk,
                id: 'historical-chunk',
                result: 'no-speech',
                text: null,
                transcriptSegments: [],
              },
            ],
          }}
        />
      </div>
    </TooltipProvider>,
  );
  await userEvent.click(
    screen.getByRole('button', { name: /Inspect Microphone transcript/ }),
    { timeout: 1000 },
  );
  expect(inspect).toHaveBeenCalledWith(
    expect.objectContaining({
      text: transcript.text,
      kind: 'transcript-segment',
    }),
  );
});

test.each(
  ['light', 'dark'].flatMap((theme) =>
    [1440, 390].map((width) => ({ theme, width })),
  ),
)(
  'visual timeline annotations fit and remain behind sticky headers in $theme at $width pixels',
  async ({ theme, width }) => {
    await page.viewport(width, 800);
    document.documentElement.dataset.theme = theme;
    const inspect = vi.fn();
    const detail = { ...studioStoryRecording, tracks, segments: [segment] };
    const view = render(
      <TooltipProvider>
        <div
          style={{ width: '100%', overflow: 'auto' }}
          aria-label="Synthetic timeline viewport"
        >
          {tracks.map((track) => (
            <div
              className="group/lane grid"
              style={{ gridTemplateColumns: '216px 1000px', width: 1216 }}
              key={track.id}
            >
              <TrackLane
                track={track}
                lane={{
                  trackId: track.id,
                  kind: track.kind,
                  label: track.label,
                  description: track.description,
                  colorId: 'rose',
                }}
                detail={detail}
                visibleRange={{ startTimelineMs: 0, endTimelineMs: 10_000 }}
                aiProcessingState={processing}
                clipDrafts={{}}
                dragHandleProps={{}}
                mutedTrackIds={new Set()}
                onSelectClip={vi.fn()}
                onInspectTranscript={inspect}
                onSelectLane={vi.fn()}
                onSelectRedaction={vi.fn()}
                onToggleTrackMuted={vi.fn()}
                playbackAssets={null}
                selectedClipIds={new Set()}
                selectedLane={theme === 'dark'}
                selectedRedactionIds={new Set()}
                selectedVisualFrameIds={new Set()}
                thumbnailStatus="idle"
                timelineWidthPx={1000}
                timeRanges={[]}
                totalDurationMs={10_000}
                visualFrames={[
                  {
                    id: 'synthetic-frame',
                    frameTimestampMs: 0,
                    endTimestampMs: 5_000,
                    ocrRegions: [],
                    vlmObservations: [],
                    sourceFrameRef: 'synthetic-frame',
                    sourceFrameRefs: [],
                    ocrTextPreview: 'Synthetic OCR annotation',
                  },
                ]}
                waveformTiles={null}
                onSelectVisualFrame={vi.fn()}
              />
            </div>
          ))}
          <div
            className="group/lane grid"
            style={{ gridTemplateColumns: '216px 1000px', width: 1216 }}
          >
            <ActivityTrackLane
              activity={{
                recordingId: detail.id,
                captureStates: [],
                intervals: [],
                windowSnapshots: [],
                events: [
                  {
                    id: 'synthetic-focus',
                    recordingId: detail.id,
                    applicationName: 'Synthetic editor',
                    title: 'Synthetic document',
                    kind: 'focused-window',
                    occurredAtMs: 1450,
                    offsetMs: 1450,
                  },
                ],
              }}
              dragHandleProps={{}}
              lane={{
                trackId: 'activity-focus',
                kind: 'activity-focus',
                label: 'Activity',
                description: 'Synthetic focus',
                colorId: 'rose',
              }}
              onInspectActivity={inspect}
              onSelectLane={vi.fn()}
              onSelectRedaction={vi.fn()}
              redactions={[]}
              selectedLane={theme === 'dark'}
              selectedRedactionIds={new Set()}
              timelineWidthPx={1000}
              timeRanges={[]}
              totalDurationMs={10_000}
              segments={[segment]}
            />
          </div>
        </div>
      </TooltipProvider>,
    );
    const viewport = screen.getByLabelText('Synthetic timeline viewport');
    viewport.scrollLeft = 150;
    await settleVisualTransitions();
    // Intentional visual contract: annotations cannot paint or receive clicks over
    // the sticky label column; the two-line text must fit inside its own strip.
    for (const label of ['Monitor', 'Activity']) {
      const header = screen
        .getByRole('button', { name: `Select ${label}` })
        .closest('[data-track-id]')!;
      await userEvent.hover(header);
      const rect = header.getBoundingClientRect();
      for (const y of [rect.top + 12, rect.bottom - 12]) {
        expect(
          header.contains(document.elementFromPoint(rect.right - 5, y)),
        ).toBe(true);
      }
    }
    const annotation = screen.getByRole('button', {
      name: /Inspect Microphone transcript/,
    });
    const strip = annotation.parentElement!;
    const annotationRect = annotation.getBoundingClientRect(),
      stripRect = strip.getBoundingClientRect();
    expect(annotationRect.bottom).toBeLessThanOrEqual(stripRect.bottom);
    expect(
      annotation.querySelector('span')!.getBoundingClientRect().height,
    ).toBeGreaterThanOrEqual(32);
    expect(trackLaneHeightPxForKind('activity-mouse')).toBe(24);
    await page.screenshot({
      element: view.container,
      path: `__screenshots__/timeline-annotations-${theme}-${width}.png`,
    });
    await userEvent.click(annotation);
    expect(inspect).toHaveBeenCalledWith(
      expect.objectContaining({
        text: transcript.text,
        timelineOffsetMs: 2_000,
        sourceRole: 'microphone',
      }),
    );
  },
);
