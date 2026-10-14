import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { TooltipProvider } from '@/components/ui/tooltip';
import '@/index.css';
import { SourceOverviewStrip } from './source-overview-strip';

afterEach(cleanup);

test('counts remain clickable during a panned detail read, failure, and recovery', async () => {
  const zoom = vi.fn();
  const props = {
    buckets: [
      {
        trackId: 'source',
        kind: 'speech' as const,
        startOffsetMs: 0,
        endOffsetMs: 50_000,
        count: 4,
      },
    ],
    trackId: 'source',
    width: 1000,
    duration: 100_000,
    onZoom: zoom,
    range: { startTimelineMs: 20_000, endTimelineMs: 40_000 },
  };
  const view = render(
    <TooltipProvider>
      <div style={{ position: 'relative', width: 1000, height: 100 }}>
        <SourceOverviewStrip {...props} pending />
      </div>
    </TooltipProvider>,
  );
  expect(screen.getByRole('status')).toHaveTextContent('Loading details');
  await userEvent.click(
    screen.getByRole('button', { name: 'Zoom into 4 transcript entries' }),
  );
  expect(zoom).toHaveBeenCalledExactlyOnceWith(0, 50_000);
  view.rerender(
    <TooltipProvider>
      <div style={{ position: 'relative', width: 1000, height: 100 }}>
        <SourceOverviewStrip {...props} error />
      </div>
    </TooltipProvider>,
  );
  expect(screen.getByRole('status')).toHaveTextContent('Refresh to retry');
  expect(
    screen.getByRole('button', { name: 'Zoom into 4 transcript entries' }),
  ).toBeVisible();
  view.rerender(
    <TooltipProvider>
      <div style={{ position: 'relative', width: 1000, height: 100 }}>
        <SourceOverviewStrip {...props} />
      </div>
    </TooltipProvider>,
  );
  expect(screen.queryByRole('status')).toBeNull();
});
test.each([
  ['speech', 'transcript entries'],
  ['observation', 'screen changes'],
  ['activity', 'activity events'],
] as const)(
  'overview %s counts remain clickable and zoom to their supporting range',
  async (kind, label) => {
    const zoom = vi.fn();
    render(
      <TooltipProvider>
        <div style={{ position: 'relative', width: 1000, height: 100 }}>
          <SourceOverviewStrip
            buckets={[
              {
                trackId: 'source',
                kind,
                startOffsetMs: 0,
                endOffsetMs: 1000,
                count: 20,
              },
              {
                trackId: 'source',
                kind,
                startOffsetMs: 2000,
                endOffsetMs: 3000,
                count: 30,
              },
              {
                trackId: 'other',
                kind,
                startOffsetMs: 0,
                endOffsetMs: 1000,
                count: 99,
              },
            ]}
            trackId="source"
            width={1000}
            duration={8 * 60 * 60_000}
            onZoom={zoom}
          />
        </div>
      </TooltipProvider>,
    );
    const group = screen.getByRole('button', { name: `Zoom into 50 ${label}` });
    expect(screen.getAllByRole('button')).toHaveLength(1);
    await userEvent.click(group);
    expect(zoom).toHaveBeenCalledExactlyOnceWith(0, 3000);
  },
);
