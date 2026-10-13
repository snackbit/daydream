import type { RecordingAiProcessingState } from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import {
  chunk,
  detail,
  monitor,
  state,
} from '../library/components/studio/timeline/processing/processing-test-fixtures';
import { TimelineProcessingOverlay } from '../library/components/studio/timeline/processing/timeline-processing-overlay';
import { timelineProcessingRanges } from '../library/components/studio/timeline/processing/timeline-processing-ranges';
import { useRecordingAiProcessingState } from './use-recording-ai-processing-state';

const client = vi.hoisted(() => ({
  getRecordingAiProcessingState: vi.fn(),
  listenToAiProcessingChanged: vi.fn(),
}));
vi.mock('./ai-client', () => client);
beforeEach(() => {
  vi.resetAllMocks();
  client.listenToAiProcessingChanged.mockResolvedValue(() => undefined);
});
afterEach(cleanup);
function Probe({ recordingId = detail.id }: { recordingId?: string }) {
  const processing = useRecordingAiProcessingState(recordingId);
  return (
    <div>
      <p role="status">{processing.state?.job?.status ?? processing.status}</p>
      {processing.error ? <p role="alert">{processing.error}</p> : null}
      <TimelineProcessingOverlay
        ranges={timelineProcessingRanges({
          detail: { ...detail, id: recordingId },
          track: monitor,
          state: processing.state,
        })}
        trackLabel="Monitor"
        video
        timelineWidthPx={1000}
        totalDurationMs={12000}
        visibleRange={{ startTimelineMs: 0, endTimelineMs: 12000 }}
      />
    </div>
  );
}
function work(status: 'queued' | 'succeeded'): RecordingAiProcessingState {
  const value = state([
    chunk({ sourceKind: 'ocr', trackId: 'screen-a', status }),
    chunk({ id: 'vlm', sourceKind: 'vlm', trackId: 'screen-a', status }),
  ]);
  return { ...value, job: value.job ? { ...value.job, status } : null };
}
test('pending screen indicators clear through periodic reconciliation when completion events are lost', async () => {
  client.getRecordingAiProcessingState
    .mockResolvedValueOnce(work('queued'))
    .mockResolvedValue(work('succeeded'));
  render(<Probe />);
  await screen.findByRole('progressbar');
  await waitFor(() => expect(screen.queryByRole('progressbar')).toBeNull(), {
    timeout: 6500,
  });
  expect(screen.getByRole('status').textContent).toBe('succeeded');
  expect(client.getRecordingAiProcessingState).toHaveBeenCalledTimes(2);
}, 8000);
test('focus reconciles missed work and stale responses from a previous recording cannot replace current state', async () => {
  let finish: ((value: RecordingAiProcessingState) => void) | undefined;
  client.getRecordingAiProcessingState
    .mockReturnValueOnce(
      new Promise<RecordingAiProcessingState>((resolve) => {
        finish = resolve;
      }),
    )
    .mockResolvedValue({ ...work('queued'), recordingId: 'next' });
  const view = render(<Probe />);
  view.rerender(<Probe recordingId="next" />);
  await screen.findByRole('progressbar');
  finish?.(work('succeeded'));
  await waitFor(() =>
    expect(screen.getByRole('status').textContent).toBe('queued'),
  );
  client.getRecordingAiProcessingState.mockResolvedValue({
    ...work('succeeded'),
    recordingId: 'next',
  });
  window.dispatchEvent(new Event('focus'));
  await waitFor(() => expect(screen.queryByRole('progressbar')).toBeNull());
});
test('subscription failure retains loaded work and polling stays available', async () => {
  client.getRecordingAiProcessingState.mockResolvedValue(work('queued'));
  client.listenToAiProcessingChanged.mockRejectedValue({
    code: 'events_unavailable',
  });
  render(<Probe />);
  await screen.findByRole('progressbar');
  expect(screen.queryByRole('alert')).toBeNull();
  client.getRecordingAiProcessingState.mockResolvedValue(work('succeeded'));
  window.dispatchEvent(new Event('focus'));
  await waitFor(() => expect(screen.queryByRole('progressbar')).toBeNull());
});
