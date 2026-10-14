import { act, cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { useRedactionAutoSkip } from './use-redaction-auto-skip';

function Countdown({
  recordingId = 'recording-1',
  isPlaying,
  targetMs = 14_000,
  seekRevision = 0,
  onSkip,
}: {
  recordingId?: string;
  isPlaying: boolean;
  targetMs?: number | null;
  seekRevision?: number;
  onSkip: (target: number) => void;
}) {
  const seconds = useRedactionAutoSkip({
    recordingId,
    isPlaying,
    timelineMs: 6_000,
    targetMs,
    seekRevision,
    onSkip,
  });
  return <output>{seconds ?? 'Paused inspection'}</output>;
}
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

test('paused inspection cancels a countdown and resumed playback starts a new countdown', async () => {
  vi.useFakeTimers();
  const skip = vi.fn();
  const { rerender } = render(<Countdown isPlaying onSkip={skip} />);
  expect(screen.getByText('3')).toBeTruthy();
  await act(() => vi.advanceTimersByTimeAsync(1_000));
  expect(screen.getByText('2')).toBeTruthy();
  rerender(<Countdown isPlaying={false} onSkip={skip} />);
  await act(() => vi.advanceTimersByTimeAsync(5_000));
  expect(skip).not.toHaveBeenCalled();
  expect(screen.getByText('Paused inspection')).toBeTruthy();
  rerender(<Countdown isPlaying onSkip={skip} />);
  expect(screen.getByText('3')).toBeTruthy();
  await act(() => vi.advanceTimersByTimeAsync(3_000));
  expect(skip).toHaveBeenCalledExactlyOnceWith(14_000);
});

test('manual seeking and unmount cancel pending automatic skips', async () => {
  vi.useFakeTimers();
  const skip = vi.fn();
  const { rerender, unmount } = render(<Countdown isPlaying onSkip={skip} />);
  await act(() => vi.advanceTimersByTimeAsync(2_000));
  rerender(
    <Countdown isPlaying targetMs={null} seekRevision={1} onSkip={skip} />,
  );
  await act(() => vi.advanceTimersByTimeAsync(3_000));
  expect(skip).not.toHaveBeenCalled();
  rerender(
    <Countdown isPlaying targetMs={30_000} seekRevision={2} onSkip={skip} />,
  );
  unmount();
  await act(() => vi.advanceTimersByTimeAsync(5_000));
  expect(skip).not.toHaveBeenCalled();
});

test('fully redacted recordings keep the recording end as a valid skip destination', async () => {
  vi.useFakeTimers();
  const skip = vi.fn();
  render(<Countdown isPlaying targetMs={30_000} onSkip={skip} />);
  await act(() => vi.advanceTimersByTimeAsync(3_000));
  expect(skip).toHaveBeenCalledExactlyOnceWith(30_000);
});

test('changing the transport callback does not restart the active redaction deadline', async () => {
  vi.useFakeTimers();
  const previous = vi.fn();
  const current = vi.fn();
  const { rerender } = render(<Countdown isPlaying onSkip={previous} />);
  await act(() => vi.advanceTimersByTimeAsync(2_000));
  rerender(<Countdown isPlaying onSkip={current} />);
  expect(screen.getByText('1')).toBeTruthy();
  await act(() => vi.advanceTimersByTimeAsync(1_000));
  expect(previous).not.toHaveBeenCalled();
  expect(current).toHaveBeenCalledExactlyOnceWith(14_000);
});

test('changing recordings cancels the old deadline even when their redacted spans match', async () => {
  vi.useFakeTimers();
  const skip = vi.fn();
  const { rerender } = render(<Countdown isPlaying onSkip={skip} />);
  await act(() => vi.advanceTimersByTimeAsync(2_000));
  rerender(<Countdown isPlaying recordingId="recording-2" onSkip={skip} />);
  await act(() => vi.advanceTimersByTimeAsync(1_000));
  expect(skip).not.toHaveBeenCalled();
  expect(screen.getByText('2')).toBeTruthy();
  await act(() => vi.advanceTimersByTimeAsync(2_000));
  expect(skip).toHaveBeenCalledExactlyOnceWith(14_000);
});
