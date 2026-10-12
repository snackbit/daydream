import { useCallback } from 'react';
import type * as TauriCore from '@tauri-apps/api/core';
import type { PreviewRecordingReprocessingInput } from '@daydream/shared';
import {
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import '@/index.css';
import { ReprocessingDialog } from '../components/studio/reprocessing/reprocessing-dialog';
import { buildDayTimeline } from './day-timeline';
import { dayTimelineFixture } from './day-timeline-fixtures';
import { useDayReprocessing } from './use-day-reprocessing';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
const model = buildDayTimeline(dayTimelineFixture());
const admitted: string[] = [];
beforeEach(() => {
  vi.resetAllMocks();
  admitted.length = 0;
  native.invoke.mockImplementation(
    async (
      command: string,
      args: { input: PreviewRecordingReprocessingInput },
    ) => {
      const input = args.input;
      if (command === 'previewRecordingReprocessing')
        return {
          recordingId: input.recordingId,
          planFingerprint: `scope-${input.recordingId}`,
          blockedReason: null,
          operations: [
            {
              operation: 'ocr',
              enabled: true,
              remote: false,
              providerLabel: 'Local OCR',
              blockedReason: null,
            },
          ],
          scopes: input.targets.flatMap((t) =>
            (t.ranges ?? []).map((r, i) => ({
              ...r,
              id: `scope-${i}`,
              trackId: t.trackId,
              sourceLabel: `Monitor ${input.recordingId}`,
              operation: 'ocr',
            })),
          ),
        };
      if (command === 'startRecordingReprocessing') {
        admitted.push(input.recordingId);
        return {
          id: `job-${input.recordingId}`,
          recordingId: input.recordingId,
          kind: 'ai-processing',
          status: 'queued',
          progress: 0,
          errorCode: null,
          createdAtMs: 1,
          updatedAtMs: 1,
        };
      }
      if (command === 'log_frontend_failure') return;
      throw new Error('Unexpected native command');
    },
  );
});
afterEach(cleanup);
function Workflow({ whole = false }: { whole?: boolean }) {
  const controller = useDayReprocessing(model);
  const open = useCallback(
    () =>
      controller.open([
        {
          trackId: 'day-video-source-monitor-0',
          ranges: whole ? null : [{ startOffsetMs: 4000, endOffsetMs: 12000 }],
        },
      ]),
    [controller, whole],
  );
  return (
    <>
      <button onClick={open}>Reprocess day sources</button>
      <ReprocessingDialog controller={controller} />
    </>
  );
}
test.each([false, true])(
  'each recording receives its own reviewed source-local scope, whole track=%s',
  async (whole) => {
    render(<Workflow whole={whole} />);
    await userEvent.click(
      screen.getByRole('button', { name: 'Reprocess day sources' }),
    );
    await screen.findByText(/Monitor a/);
    await userEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Reprocess',
      }),
    );
    await screen.findByText(/Monitor b/);
    expect(admitted).toEqual(['a']);
    await userEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Reprocess',
      }),
    );
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(admitted).toEqual(['a', 'b']);
    const inputs = native.invoke.mock.calls
      .filter(([command]) => command === 'previewRecordingReprocessing')
      .map(([, args]) => args.input);
    expect(inputs[0]).toMatchObject({
      recordingId: 'a',
      targets: [
        {
          trackId: 'a-screen-0',
          ranges: [
            { startOffsetMs: whole ? 21000 : 25000, endOffsetMs: 26000 },
          ],
        },
      ],
    });
    expect(
      inputs.some(
        (input) =>
          input.recordingId === 'b' &&
          input.targets[0].trackId === 'b-screen-0' &&
          input.targets[0].ranges[0].startOffsetMs === 0,
      ),
    ).toBe(true);
  },
);
test('canceling the second recording never starts unreviewed work', async () => {
  render(<Workflow />);
  await userEvent.click(
    screen.getByRole('button', { name: 'Reprocess day sources' }),
  );
  await screen.findByText(/Monitor a/);
  await userEvent.click(
    within(screen.getByRole('dialog')).getByRole('button', {
      name: 'Reprocess',
    }),
  );
  await screen.findByText(/Monitor b/);
  await userEvent.click(
    within(screen.getByRole('dialog')).getByRole('button', { name: 'Cancel' }),
  );
  expect(admitted).toEqual(['a']);
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});
