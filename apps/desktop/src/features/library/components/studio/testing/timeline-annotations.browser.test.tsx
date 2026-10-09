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
import { TrackLane } from '../track-lane';
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
