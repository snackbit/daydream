import { useRef } from 'react';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { useStudioTimelineStore } from '../studio-timeline-store';
import { useTrackTimelineSelection } from '../use-track-timeline-selection';

afterEach(() => {
  cleanup();
  useStudioTimelineStore.getState().clearSelection();
});
function Timeline({ selectClip }: { selectClip: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const selection = useStudioTimelineStore((s) => s.selection);
  const handlers = useTrackTimelineSelection({
    activeLaneIds: ['microphone-2'],
    activeLaneLayouts: [
      { id: 'microphone-2', label: 'Microphone', heightPx: 112 },
    ],
    actions: {
      handleTimelineInput: vi.fn(),
      commitTimelineInput: vi.fn(),
      cancelTimelineInput: vi.fn(),
    },
    contentRef: ref,
    clearSelection: useStudioTimelineStore.getState().clearSelection,
    selection,
    setFollowPlayhead: vi.fn(),
    setSelection: useStudioTimelineStore.getState().setSelection,
    timelineWidthPx: 600,
    totalDurationMs: 6000,
  });
  return (
    <>
      <div
        ref={ref}
        aria-label="Track timeline"
        style={{ width: 816, height: 144, position: 'relative' }}
        onPointerDown={handlers.handleTimelinePointerDown}
        onPointerMove={handlers.handleTimelinePointerMove}
        onPointerUp={handlers.handleTimelinePointerEnd}
        onClickCapture={handlers.handleTimelineClickCapture}
      >
        <div data-time-ruler style={{ height: 32 }} />
        <div
          data-lane-content
          data-track-id="microphone-2"
          style={{ height: 112 }}
        >
          <button
            style={{ position: 'absolute', left: 400, top: 50 }}
            type="button"
            onClick={selectClip}
          >
            Inside
          </button>
          <button
            style={{ position: 'absolute', left: 700, top: 50 }}
            type="button"
            onClick={selectClip}
          >
            Outside
          </button>
        </div>
      </div>
      <p>{selection.ranges.length ? 'Range selected' : 'Selection cleared'}</p>
    </>
  );
}
test('clicking inside a selected lane range retains it; clicking outside clears it without selecting the underlying clip', async () => {
  useStudioTimelineStore.getState().setTimeRangeSelection({
    laneIds: ['microphone-2'],
    startMs: 1000,
    endMs: 3000,
  });
  const select = vi.fn();
  render(<Timeline selectClip={select} />);
  await userEvent.click(screen.getByRole('button', { name: 'Inside' }));
  expect(screen.getByText('Range selected')).toBeTruthy();
  expect(select).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole('button', { name: 'Outside' }));
  await waitFor(() =>
    expect(screen.getByText('Selection cleared')).toBeTruthy(),
  );
  expect(select).not.toHaveBeenCalled();
});
