import { useRef } from 'react';
import type {
  LibraryRecordingTrack,
  ReprocessingTarget,
} from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import { TRACK_LABEL_WIDTH_PX } from '../timeline/interactions/track-timeline-math';
import { useTrackTimelineSelection } from '../timeline/interactions/use-track-timeline-selection';
import { TrackLaneHeader } from '../timeline/lanes/track-lane-header';
import { useStudioTimelineStore } from '../timeline/state/studio-timeline-store';
import { TimelineReprocessingContextMenu } from './timeline-reprocessing-context-menu';

afterEach(() => {
  cleanup();
  useStudioTimelineStore.getState().clearSelection();
  delete document.documentElement.dataset.theme;
});
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
  seek = vi.fn(),
}: {
  reprocess: (targets: ReprocessingTarget[]) => void;
  seek?: (timeMs: number) => void;
}) {
  const contentRef = useRef<HTMLDivElement>(null);
  const noop = vi.fn();
  const selection = useStudioTimelineStore((s) => s.selection);
  const handlers = useTrackTimelineSelection({
    activeLaneIds: [track.id],
    activeLaneLayouts: [{ id: track.id, label: track.label, heightPx: 112 }],
    actions: {
      handleTimelineInput: seek,
      commitTimelineInput: seek,
      cancelTimelineInput: noop,
    },
    contentRef,
    clearSelection: useStudioTimelineStore.getState().clearSelection,
    selection,
    setFollowPlayhead: noop,
    setSelection: useStudioTimelineStore.getState().setSelection,
    timelineWidthPx: 1000,
    totalDurationMs: 10000,
  });
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
      <div
        ref={contentRef}
        style={{
          display: 'grid',
          gridTemplateColumns: `${TRACK_LABEL_WIDTH_PX}px 1000px`,
          width: TRACK_LABEL_WIDTH_PX + 1000,
        }}
        onPointerDown={handlers.handleTimelinePointerDown}
        onPointerMove={handlers.handleTimelinePointerMove}
        onPointerUp={handlers.handleTimelinePointerEnd}
        onClickCapture={handlers.handleTimelineClickCapture}
      >
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
            style={{ position: 'absolute', left: 200, top: 50 }}
          >
            Inside range
          </button>
          <button
            type="button"
            style={{ position: 'absolute', left: 600, top: 50 }}
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

test.each([
  { theme: 'light', width: 1440 },
  { theme: 'dark', width: 390 },
])(
  'header menu clicks do not seek or dismiss selection ($theme, $width)',
  async ({ theme, width }) => {
    await page.viewport(width, 800);
    document.documentElement.dataset.theme = theme;
    useStudioTimelineStore.getState().setTimeRangeSelection({
      laneIds: [track.id],
      startMs: 1000,
      endMs: 4000,
    });
    const selected = useStudioTimelineStore.getState().selection;
    const reprocess = vi.fn();
    const seek = vi.fn();
    render(<ContextWorkflow reprocess={reprocess} seek={seek} />);
    await userEvent.click(
      screen.getByRole('button', { name: 'Microphone actions' }),
    );
    const item = screen.getByRole('menuitem', {
      name: /Reprocess microphone/i,
    });
    await userEvent.click(item, {
      position: { x: item.getBoundingClientRect().width - 8, y: 10 },
    });
    expect(reprocess).toHaveBeenCalledExactlyOnceWith([
      { trackId: track.id, ranges: null },
    ]);
    expect(seek).not.toHaveBeenCalled();
    expect(useStudioTimelineStore.getState().selection).toEqual(selected);
    await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
    await userEvent.click(
      screen.getByRole('button', { name: 'Select Microphone' }),
      { button: 'right' },
    );
    await userEvent.click(
      screen.getByRole('menuitem', { name: /Reprocess microphone/i }),
    );
    expect(reprocess).toHaveBeenCalledTimes(2);
    expect(seek).not.toHaveBeenCalled();
    expect(useStudioTimelineStore.getState().selection).toEqual(selected);
    await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
    await userEvent.click(screen.getByRole('button', { name: 'Inside range' }));
    expect(seek).toHaveBeenCalled();
    expect(useStudioTimelineStore.getState().selection).toEqual(selected);
  },
);
