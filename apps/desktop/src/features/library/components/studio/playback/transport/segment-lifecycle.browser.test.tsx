import { useCallback, useRef, useState } from 'react';
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { workflowSegment } from '../../testing/studio-workflow-fixture';
import { usePlaybackResumeTimer } from './recording-playback-resume-timer';
import type { PlaybackSegmentLifecycleInput } from './use-playback-segment-lifecycle';
import { usePlaybackSegmentLifecycle } from './use-playback-segment-lifecycle';

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});
function LifecycleWorkflow({
  segment = workflowSegment(),
  count = 2,
  shortGaps = false,
  playing = true,
  elapsed = 0,
}: {
  segment?: PlaybackSegmentLifecycleInput['currentSegment'];
  count?: number;
  shortGaps?: boolean;
  playing?: boolean;
  elapsed?: number;
}) {
  const [index, setIndex] = useState(0),
    [revision, setRevision] = useState(0);
  const [resumeProgress, setResumeProgress] = useState<number | null>(null);
  const [position, setPosition] = useState(0),
    [finished, setFinished] = useState(false);
  const intent = useRef(playing),
    video = useRef<HTMLVideoElement>(null);
  const timer = usePlaybackResumeTimer({ setResumeProgress });
  const advance = useCallback((next: number) => {
    setIndex(next);
    setRevision((r) => r + 1);
    setResumeProgress(null);
  }, []);
  const retire = useCallback((wanted: boolean) => {
    intent.current = wanted;
  }, []);
  const current = useCallback(
    (element: HTMLMediaElement) => element === video.current,
    [],
  );
  const clockTime = useCallback(() => elapsed, [elapsed]);
  const lifecycle = usePlaybackSegmentLifecycle({
    ...timer,
    currentSegment: segment,
    currentIndex: index,
    segmentCount: count,
    streamRevision: revision,
    streamStartMs: 0,
    playbackRate: 1,
    isPlaying: playing,
    resumeProgress,
    shortGaps,
    playbackIntentRef: intent,
    advanceToSegment: advance,
    replaceStream: retire,
    isCurrentMedia: current,
    setCurrentTime: setPosition,
    setPlaybackEndedAtEnd: setFinished,
    clockTime,
  });
  return (
    <>
      <video
        aria-label="Current source"
        ref={video}
        onEnded={lifecycle.handleEnded}
      />
      <video aria-label="Retired source" onEnded={lifecycle.handleEnded} />
      <button onClick={lifecycle.playNowAfterPause}>Skip gap</button>
      <output>{finished ? 'Finished' : `Segment ${index + 1}`}</output>
      <p>Media revision {revision}</p>
      <p>Position {position}</p>
      <p>{resumeProgress === null ? 'No gap' : 'Gap pending'}</p>
    </>
  );
}
test('duplicate completion advances once and retired media cannot advance the replacement', () => {
  render(<LifecycleWorkflow />);
  const video = screen.getByLabelText('Current source');
  act(() => {
    fireEvent.ended(video);
    fireEvent.ended(video);
  });
  expect(screen.getByText('Segment 2')).toBeTruthy();
  expect(screen.getByText('Media revision 1')).toBeTruthy();
  fireEvent.ended(screen.getByLabelText('Retired source'));
  expect(screen.getByText('Segment 2')).toBeTruthy();
  fireEvent.ended(video);
  expect(screen.getByText('Finished')).toBeTruthy();
  expect(screen.getByText('Position 1')).toBeTruthy();
});
test.each([true, false])(
  'a pause gap waits for its configured duration, short gaps=%s',
  async (shortGaps) => {
    vi.useFakeTimers();
    render(
      <LifecycleWorkflow
        shortGaps={shortGaps}
        segment={workflowSegment({ pauseAfterMs: 4000 })}
      />,
    );
    fireEvent.ended(screen.getByLabelText('Current source'));
    expect(screen.getByText('Gap pending')).toBeTruthy();
    // A second native completion during the pending gap cannot publish another advance.
    fireEvent.ended(screen.getByLabelText('Current source'));
    await act(async () => {
      vi.advanceTimersByTime(shortGaps ? 2950 : 3950);
    });
    expect(screen.getByText('Segment 1')).toBeTruthy();
    await act(async () => {
      vi.advanceTimersByTime(50);
    });
    expect(screen.getByText('Segment 2')).toBeTruthy();
    expect(screen.getByText('No gap')).toBeTruthy();
  },
);
test('paused gap progress cannot advance until the user skips it, and the last source cannot be skipped', async () => {
  vi.useFakeTimers();
  render(
    <LifecycleWorkflow
      playing={false}
      segment={workflowSegment({ pauseAfterMs: 4000 })}
    />,
  );
  fireEvent.ended(screen.getByLabelText('Current source'));
  await act(async () => {
    vi.advanceTimersByTime(5000);
  });
  expect(screen.getByText('Segment 1')).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Skip gap' }));
  expect(screen.getByText('Segment 2')).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Skip gap' }));
  expect(screen.getByText('Segment 2')).toBeTruthy();
});
test('calendar slices finish at the source-local boundary and release their timeout on unmount', async () => {
  vi.useFakeTimers();
  const view = render(
    <LifecycleWorkflow
      segment={workflowSegment({ mediaStartMs: 10000 })}
      count={1}
      elapsed={0.95}
    />,
  );
  expect(screen.getByText('Segment 1')).toBeTruthy();
  view.rerender(
    <LifecycleWorkflow
      segment={workflowSegment({ mediaStartMs: 10000 })}
      count={1}
      elapsed={1}
    />,
  );
  expect(screen.getByText('Finished')).toBeTruthy();
  view.unmount();
  expect(vi.getTimerCount()).toBe(0);
});
test('late completion with no source is ignored, and unknown source duration ends at zero', () => {
  const view = render(<LifecycleWorkflow segment={null} />);
  fireEvent.ended(screen.getByLabelText('Current source'));
  expect(screen.getByText('Segment 1')).toBeTruthy();
  view.rerender(
    <LifecycleWorkflow
      count={1}
      segment={workflowSegment({ durationMs: null })}
    />,
  );
  fireEvent.ended(screen.getByLabelText('Current source'));
  expect(screen.getByText('Finished')).toBeTruthy();
  expect(screen.getByText('Position 0')).toBeTruthy();
});
