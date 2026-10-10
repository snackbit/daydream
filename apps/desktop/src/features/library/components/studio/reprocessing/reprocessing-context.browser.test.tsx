import { useRef } from 'react';
import type {
  LibraryRecordingTrack,
  ReprocessingTarget,
} from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import { TrackLaneHeader } from '../track-lane-header';
import { TRACK_LABEL_WIDTH_PX } from '../track-timeline-math';
import { TimelineReprocessingContextMenu } from './timeline-reprocessing-context-menu';

afterEach(cleanup);
const track: LibraryRecordingTrack = {
  id: 'microphone-2',
  kind: 'microphone',
  label: 'Microphone',
  description: 'Synthetic source',
  codec: null,
  streamIndex: 2,
};
const ranges = [
  { id: 'selected', laneIds: [track.id], startMs: 1000, endMs: 4000 },
];
function ContextWorkflow({
  reprocess,
}: {
  reprocess: (targets: ReprocessingTarget[]) => void;
}) {
  const contentRef = useRef<HTMLDivElement>(null);
  const noop = vi.fn();
  return (
    <TimelineReprocessingContextMenu
      contentRef={contentRef}
      ranges={ranges}
      tracks={[track]}
      timelineWidthPx={1000}
      totalDurationMs={10000}
      onReprocess={reprocess}
      onOpenChange={noop}
    >
      <div ref={contentRef} style={{ width: TRACK_LABEL_WIDTH_PX + 1000 }}>
        <TrackLaneHeader
          trackId={track.id}
          lane={{
            trackId: track.id,
            kind: 'microphone',
            label: 'Microphone',
            description: 'Synthetic source',
            colorId: 'sage',
          }}
          audioTrack
          muted={false}
          selectedLane={false}
          dragHandleProps={{}}
          onSelectLane={noop}
          onToggleTrackMuted={noop}
          onReprocess={reprocess}
        />
        <div
          data-track-id={track.id}
          style={{ position: 'relative', height: 100 }}
        >
          <button
            type="button"
            style={{ position: 'absolute', left: TRACK_LABEL_WIDTH_PX + 200 }}
          >
            Inside range
          </button>
          <button
            type="button"
            style={{ position: 'absolute', left: TRACK_LABEL_WIDTH_PX + 600 }}
          >
            Outside range
          </button>
        </div>
      </div>
    </TimelineReprocessingContextMenu>
  );
}
test('right-click reprocesses the selected range while track headers and other media regions target the whole source', async () => {
  await page.viewport(1440, 800);
  const reprocess = vi.fn();
  render(<ContextWorkflow reprocess={reprocess} />);
  await userEvent.click(screen.getByRole('button', { name: 'Inside range' }), {
    button: 'right',
  });
  await userEvent.click(
    screen.getByRole('menuitem', { name: /Reprocess selection/ }),
  );
  expect(reprocess).toHaveBeenLastCalledWith([
    { trackId: track.id, ranges: [{ startOffsetMs: 1000, endOffsetMs: 4000 }] },
  ]);
  await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
  await userEvent.click(screen.getByRole('button', { name: 'Outside range' }), {
    button: 'right',
  });
  await userEvent.click(
    screen.getByRole('menuitem', { name: /Reprocess Microphone/ }),
  );
  expect(reprocess).toHaveBeenLastCalledWith([
    { trackId: track.id, ranges: null },
  ]);
  await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
  await userEvent.click(
    screen.getByRole('button', { name: 'Select Microphone' }),
    { button: 'right' },
  );
  expect(screen.getAllByRole('menu')).toHaveLength(1);
  await userEvent.click(
    screen.getByRole('menuitem', { name: /Reprocess microphone/i }),
  );
  expect(reprocess).toHaveBeenLastCalledWith([
    { trackId: track.id, ranges: null },
  ]);
});
