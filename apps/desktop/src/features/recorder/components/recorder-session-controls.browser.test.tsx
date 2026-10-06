import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { RecorderSessionControls } from './recorder-session-controls';

afterEach(cleanup);
test('shell controls pause, resume and stop without navigating and retain recorded time while paused', async () => {
  const pause = vi.fn().mockResolvedValue(undefined),
    resume = vi.fn().mockResolvedValue(undefined),
    stop = vi.fn().mockResolvedValue(undefined);
  const props = {
    stats: { recordedDurationMs: 65_000, totalSizeBytes: 1024 },
    locked: false,
    pause,
    resume,
    stop,
  };
  const view = render(
    <RecorderSessionControls
      {...props}
      status={{ state: 'recording', finalization: null }}
    />,
  );
  expect(screen.getByLabelText('Recorded duration').textContent).toBe(
    '00:01:05',
  );
  expect(screen.getByLabelText('Recording size').textContent).not.toBe(
    'Size unavailable',
  );
  await fireEvent.click(
    screen.getByRole('button', { name: 'Pause recording' }),
  );
  expect(pause).toHaveBeenCalledTimes(1);
  view.rerender(
    <RecorderSessionControls
      {...props}
      status={{ state: 'paused', finalization: null }}
    />,
  );
  expect(screen.getByLabelText('Recorded duration').textContent).toBe(
    '00:01:05',
  );
  await fireEvent.click(
    screen.getByRole('button', { name: 'Resume recording' }),
  );
  expect(resume).toHaveBeenCalledTimes(1);
  await fireEvent.click(screen.getByRole('button', { name: 'Stop recording' }));
  expect(stop).toHaveBeenCalledTimes(1);
  view.rerender(
    <RecorderSessionControls
      {...props}
      locked
      status={{ state: 'paused', finalization: null }}
    />,
  );
  expect(
    screen.getByRole<HTMLButtonElement>('button', { name: 'Stop recording' })
      .disabled,
  ).toBe(true);
});
