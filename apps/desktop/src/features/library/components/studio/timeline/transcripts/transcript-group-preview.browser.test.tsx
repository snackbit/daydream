import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { TooltipProvider } from '@/components/ui/tooltip';
import { getRecordingTranscriptPreview } from '@/features/ai/transcript/preview-client';
import '@/index.css';
import { SourceOverviewStrip } from '../overview/source-overview-strip';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => native);
beforeEach(() => vi.resetAllMocks());
afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});

function overview(revision = 'initial') {
  return (
    <TooltipProvider>
      <div style={{ position: 'relative', width: 500, height: 100 }}>
        <SourceOverviewStrip
          key={revision}
          previewRevision={revision}
          buckets={[
            {
              trackId: 'microphone-2',
              kind: 'speech',
              startOffsetMs: 1000,
              endOffsetMs: 2000,
              count: 4,
            },
          ]}
          trackId="microphone-2"
          recordingId="synthetic"
          sourceRole="microphone"
          trackLabel="Microphone"
          width={500}
          duration={5000}
          onZoom={zoom}
          readPreview={getRecordingTranscriptPreview}
        />
      </div>
    </TooltipProvider>
  );
}
const zoom = vi.fn();
const response = {
  recordingId: 'synthetic',
  sourceRole: 'microphone',
  entries: [
    {
      id: 'utterance',
      segmentId: 'segment',
      startOffsetMs: 1000,
      endOffsetMs: 1500,
      text: 'Synthetic excerpt to preview',
      truncated: true,
    },
  ],
};

test.each([
  ['light', 1440],
  ['dark', 600],
] as const)(
  'group preview uses real tooltip and decoded IPC in %s at %i pixels',
  async (theme, width) => {
    await page.viewport(width, 600);
    document.documentElement.dataset.theme = theme;
    let finish: (value: unknown) => void = () => {
      throw new Error('Preview request missing');
    };
    native.invoke.mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    render(overview());
    expect(native.invoke).not.toHaveBeenCalled();
    const group = screen.getByRole('button', {
      name: 'Zoom into 4 transcript entries',
    });
    await userEvent.hover(group);
    await screen.findByRole('tooltip');
    await waitFor(() =>
      expect(native.invoke).toHaveBeenCalledWith(
        'getRecordingTranscriptPreview',
        {
          input: {
            recordingId: 'synthetic',
            sourceRole: 'microphone',
            startOffsetMs: 1000,
            endOffsetMs: 2000,
          },
        },
      ),
    );
    expect(screen.getByRole('tooltip')).toHaveTextContent('Loading preview');
    finish(response);
    await waitFor(() =>
      expect(screen.getByRole('tooltip')).toHaveTextContent(
        'Synthetic excerpt to preview…',
      ),
    );
    expect(screen.getByRole('tooltip')).toHaveTextContent('0:01');
    await page.screenshot({
      path: `__screenshots__/transcript-preview-${theme}.png`,
    });
    await userEvent.click(group);
    expect(zoom).toHaveBeenLastCalledWith(1000, 2000);
    await waitFor(() => expect(screen.queryByRole('tooltip')).toBeNull());
  },
);

test('preview failures retry on reopening and privacy revision discards pending text', async () => {
  native.invoke
    .mockRejectedValueOnce(new Error('Synthetic failure'))
    .mockResolvedValue(response);
  const view = render(overview());
  const group = screen.getByRole('button', {
    name: 'Zoom into 4 transcript entries',
  });
  group.focus();
  await waitFor(() =>
    expect(screen.getByRole('tooltip')).toHaveTextContent(
      'Preview could not load',
    ),
  );
  await userEvent.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('tooltip')).toBeNull());
  group.blur();
  group.focus();
  await waitFor(() =>
    expect(screen.getByRole('tooltip')).toHaveTextContent(
      response.entries[0]!.text,
    ),
  );
  await userEvent.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('tooltip')).toBeNull());
  group.blur();
  let finish: (value: unknown) => void = () => {
    throw new Error('Preview request missing');
  };
  native.invoke.mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  group.focus();
  await waitFor(() =>
    expect(screen.getByRole('tooltip')).toHaveTextContent('Loading preview'),
  );
  view.rerender(overview('redacted'));
  finish(response);
  await waitFor(() => expect(screen.queryByRole('tooltip')).toBeNull());
  expect(screen.queryByText(response.entries[0]!.text)).toBeNull();
  native.invoke.mockResolvedValue({ ...response, entries: [] });
  screen
    .getByRole('button', { name: 'Zoom into 4 transcript entries' })
    .focus();
  await waitFor(() =>
    expect(screen.getByRole('tooltip')).toHaveTextContent(
      'No transcript preview',
    ),
  );
});
