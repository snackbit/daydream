import { DndContext } from '@dnd-kit/core';
import { SortableContext } from '@dnd-kit/sortable';
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { TooltipProvider } from '@/components/ui/tooltip';
import '@/index.css';
import { SortableTrackLane } from '../lanes/sortable-track-lane';
import {
  detail,
  microphone,
  state,
} from '../processing/processing-test-fixtures';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: native.invoke }));
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});
const noop = () => undefined;
test('sortable lanes retain the day-scoped transcript reader and revision when showing group previews', async () => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'log_frontend_failure') return;
    throw new Error(
      `Unexpected recording IPC for a day-scoped source: ${command}`,
    );
  });
  const read = vi.fn().mockResolvedValue({
    recordingId: detail.id,
    sourceRole: 'microphone',
    entries: [
      {
        id: 'source-utterance',
        segmentId: 'source-segment',
        startOffsetMs: 1000,
        endOffsetMs: 1500,
        text: 'Synthetic day excerpt',
        truncated: false,
      },
    ],
  });
  render(
    <TooltipProvider>
      <DndContext>
        <SortableContext items={[microphone.id]}>
          <SortableTrackLane
            track={microphone}
            lane={{
              trackId: microphone.id,
              kind: 'microphone',
              label: 'Microphone',
              description: 'Synthetic source',
              colorId: 'sage',
            }}
            detail={detail}
            aiProcessingState={{
              ...state([]),
              timelineOverview: [
                {
                  trackId: microphone.id,
                  kind: 'speech',
                  startOffsetMs: 1000,
                  endOffsetMs: 2000,
                  count: 4,
                },
              ],
            }}
            readTranscriptPreview={read}
            transcriptPreviewRevision="day-revision"
            clipDrafts={{}}
            mutedTrackIds={new Set()}
            onSelectClip={noop}
            onInspectTranscript={noop}
            onSelectLane={noop}
            onSelectRedaction={noop}
            onToggleTrackMuted={noop}
            playbackAssets={null}
            selectedClipIds={new Set()}
            selectedLane={false}
            selectedRedactionIds={new Set()}
            selectedVisualFrameIds={new Set()}
            thumbnailStatus="idle"
            timelineWidthPx={1000}
            timeRanges={[]}
            totalDurationMs={12000}
            visibleRange={{ startTimelineMs: 0, endTimelineMs: 12000 }}
            visualFrames={[]}
            waveformTiles={null}
            onSelectVisualFrame={noop}
            onZoomRange={noop}
          />
        </SortableContext>
      </DndContext>
    </TooltipProvider>,
  );
  await userEvent.hover(
    screen.getByRole('button', { name: 'Zoom into 4 transcript entries' }),
  );
  await screen.findByRole('tooltip');
  expect(read).toHaveBeenCalledExactlyOnceWith({
    recordingId: detail.id,
    sourceRole: 'microphone',
    startOffsetMs: 1000,
    endOffsetMs: 2000,
  });
  expect(screen.getByRole('tooltip')).toHaveTextContent(
    'Synthetic day excerpt',
  );
});
