import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import { useStudioSelectionDismissal } from '../interactions/use-studio-selection-dismissal';
import { useStudioTimelineStore } from '../studio-timeline-store';
import { ReprocessingDialog } from './reprocessing-dialog';
import { ReprocessingDropdown } from './reprocessing-menu';
import { useStudioReprocessing } from './use-studio-reprocessing';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
  useStudioTimelineStore.getState().clearSelection();
  delete document.documentElement.dataset.theme;
});
const targets = [
  {
    trackId: 'microphone-2',
    ranges: [{ startOffsetMs: 1500, endOffsetMs: 2000 }],
  },
];
const preview = {
  recordingId: 'synthetic',
  planFingerprint: 'settings-and-scope',
  blockedReason: null,
  operations: [
    {
      operation: 'transcription',
      enabled: true,
      remote: false,
      providerLabel: 'Local transcription',
      blockedReason: null,
    },
  ],
  scopes: [
    {
      id: 'effective',
      trackId: 'microphone-2',
      sourceLabel: 'Microphone',
      operation: 'transcription',
      startOffsetMs: 1000,
      endOffsetMs: 3000,
    },
  ],
};
function Workflow() {
  const controller = useStudioReprocessing('synthetic');
  const clear = useCallback(() => {}, []);
  useStudioSelectionDismissal(clear);
  const selected = useStudioTimelineStore((s) => s.selection.ranges.length > 0);
  return (
    <>
      <button type="button">Recording header</button>
      <div data-studio-root>
        <div data-studio-preserve-selection>
          <ReprocessingDropdown
            label="Selection"
            targets={targets}
            onReprocess={controller.open}
          />
          <span>{selected ? 'Range selected' : 'No selection'}</span>
        </div>
        <button type="button">Preview area</button>
        <ReprocessingDialog controller={controller} />
        {controller.queued ? <p role="status">Replacement queued</p> : null}
      </div>
    </>
  );
}
test.each([
  { theme: 'light', width: 1440 },
  { theme: 'dark', width: 390 },
])(
  'selection reprocessing reviews expanded scope, retains scope on error, and outside click clears it ($theme, $width)',
  async ({ theme, width }) => {
    await page.viewport(width, 800);
    document.documentElement.dataset.theme = theme;
    useStudioTimelineStore.getState().setSelection({
      clipIds: [],
      laneIds: [],
      redactionIds: [],
      visualFrameIds: [],
      ranges: [
        {
          id: 'range',
          laneIds: ['microphone-2'],
          startMs: 1500,
          endMs: 2000,
        },
      ],
    });
    vi.mocked(invoke)
      .mockResolvedValueOnce(preview)
      .mockRejectedValueOnce(new Error('synthetic boundary failure'));
    render(<Workflow />);
    await userEvent.click(
      screen.getByRole('button', { name: 'Selection actions' }),
    );
    expect(screen.getByText('Range selected')).toBeTruthy();
    await userEvent.click(
      screen.getByRole('menuitem', { name: /Reprocess selection/ }),
    );
    await waitFor(() => expect(screen.getByText(/0:01.*0:03/)).toBeTruthy());
    expect(screen.getByText('Range selected')).toBeTruthy();
    await page.screenshot({
      element: screen.getByRole('dialog'),
      path: `__screenshots__/reprocessing-${theme}-${width}.png`,
    });
    expect(screen.getByText('Range selected')).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: 'Reprocess' }));
    await waitFor(() => expect(screen.getByRole('alert')).toBeTruthy());
    expect(invoke).toHaveBeenLastCalledWith('startRecordingReprocessing', {
      input: {
        recordingId: 'synthetic',
        targets,
        planFingerprint: 'settings-and-scope',
      },
    });
    expect(screen.getByText('Range selected')).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await userEvent.click(
      await screen.findByRole('button', { name: 'Recording header' }),
    );
    expect(screen.getByText('No selection')).toBeTruthy();
  },
);
