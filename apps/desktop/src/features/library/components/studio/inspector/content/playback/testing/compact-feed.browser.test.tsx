import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import { useStudioResultWindowStore as windows } from '../../../../results/studio-result-window-store';
import type {
  InspectorContentEntry,
  InspectorContentIndex,
} from '../../inspector-content';
import { PlaybackInspectorFeed } from '../../playback-inspector-feed';

afterEach(() => {
  windows.setState(windows.getInitialState(), true);
  cleanup();
  vi.restoreAllMocks();
  delete document.documentElement.dataset.theme;
});
const microphone: InspectorContentEntry = {
  id: 'mic',
  kind: 'speech',
  source: 'Microphone',
  trackId: 'mic',
  startMs: 1000,
  endMs: 3000,
  text: 'Hello, world!',
  words: [
    {
      id: 'hello',
      text: 'Hello,',
      startOffsetMs: 1000,
      endOffsetMs: 1500,
      probability: null,
    },
    {
      id: 'world',
      text: 'world!',
      startOffsetMs: 1600,
      endOffsetMs: 2400,
      probability: null,
    },
  ],
  fragments: [],
  boundary: false,
};
const annotation: InspectorContentEntry = {
  ...microphone,
  id: 'note',
  trackId: 'screen',
  kind: 'observation',
  observationKind: 'screen-observation',
  source: 'Monitor',
  startMs: 1555,
  endMs: 1556,
  text:
    'A synthetic document was updated. '.repeat(16) + 'Synthetic note ending.',
  words: [],
};
const index: InspectorContentIndex = {
  entries: [microphone, annotation],
  speechBySource: new Map([['mic', [microphone]]]),
  focus: [],
};

test('Review filters preserve source categories and page browsing returns to the live playback window', async () => {
  windows.setState(windows.getInitialState(), true);
  const activity: InspectorContentEntry = {
    ...microphone,
    id: 'activity',
    kind: 'activity',
    source: 'Mouse',
    trackId: 'mouse',
    text: 'Synthetic mouse activity',
    words: [],
  };
  render(
    <PlaybackInspectorFeed
      index={{ ...index, entries: [microphone, activity, annotation] }}
      time={1100}
      isPlaying={false}
      ownerId="review-pages"
      durationMs={1_200_000}
      readPlaybackTimeMs={() => 1100}
      onSeek={vi.fn()}
    />,
  );
  await userEvent.click(screen.getByRole('button', { name: 'Review filters' }));
  await userEvent.click(screen.getByRole('checkbox', { name: 'Speech' }));
  await waitFor(() => expect(screen.queryByText('Hello,')).toBeNull());
  expect(screen.getByText('Synthetic mouse activity')).toBeTruthy();
  await userEvent.click(screen.getByRole('checkbox', { name: 'Speech' }));
  await waitFor(() => expect(screen.getByText('Hello,')).toBeTruthy());
  await userEvent.click(screen.getByRole('checkbox', { name: 'Activity' }));
  await waitFor(() =>
    expect(screen.queryByText('Synthetic mouse activity')).toBeNull(),
  );
  await userEvent.click(screen.getByRole('checkbox', { name: 'Activity' }));
  await userEvent.click(
    screen.getByRole('checkbox', { name: 'Screen changes' }),
  );
  await waitFor(() =>
    expect(
      screen.queryByRole('button', {
        name: 'Expand AI observation from Monitor at 0:01',
      }),
    ).toBeNull(),
  );
  await userEvent.click(
    screen.getByRole('checkbox', { name: 'Screen changes' }),
  );
  await userEvent.keyboard('{Escape}');
  await userEvent.click(screen.getByRole('button', { name: 'Later' }));
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Earlier' })).toBeEnabled(),
  );
  expect(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Earlier' }));
  await userEvent.click(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  );
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Earlier' })).toBeDisabled(),
  );
  expect(screen.getByText('Hello,')).toBeTruthy();
});

test('live words continue during browsing, stop on pause, and invalidate on seeks and privacy changes', async () => {
  let clock = 1100;
  const readPlaybackTimeMs = vi.fn(() => clock);
  const seek = vi.fn();
  const view = render(
    <PlaybackInspectorFeed
      index={index}
      time={1100}
      isPlaying
      readPlaybackTimeMs={readPlaybackTimeMs}
      seekRevision={0}
      onSeek={seek}
    />,
  );
  clock = 1700;
  await waitFor(() =>
    expect(screen.getByText('world!').getAttribute('aria-current')).toBe(
      'true',
    ),
  );
  fireEvent.wheel(screen.getByText('world!'), { deltaY: 100 });
  expect(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeTruthy();
  expect(screen.getByText('world!')).toHaveAttribute('aria-current', 'true');
  clock = 1100;
  await waitFor(() =>
    expect(screen.getByText('Hello,')).toHaveAttribute('aria-current', 'true'),
  );
  expect(screen.getByText('world!')).not.toHaveAttribute('aria-current');
  expect(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeTruthy();
  expect(seek).not.toHaveBeenCalled();
  await userEvent.click(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  );
  await waitFor(() =>
    expect(screen.getByText('Hello,').getAttribute('aria-current')).toBe(
      'true',
    ),
  );
  view.rerender(
    <PlaybackInspectorFeed
      index={index}
      time={1100}
      isPlaying={false}
      readPlaybackTimeMs={readPlaybackTimeMs}
      seekRevision={0}
      onSeek={seek}
    />,
  );
  const reads = readPlaybackTimeMs.mock.calls.length;
  clock = 1700;
  await new Promise<void>((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
  );
  expect(readPlaybackTimeMs.mock.calls.length).toBe(reads);
  expect(screen.getByText('Hello,').getAttribute('aria-current')).toBe('true');
  view.rerender(
    <PlaybackInspectorFeed
      index={index}
      time={1700}
      isPlaying={false}
      readPlaybackTimeMs={readPlaybackTimeMs}
      seekRevision={0}
      onSeek={seek}
    />,
  );
  expect(screen.getByText('world!').getAttribute('aria-current')).toBe('true');
  fireEvent.wheel(screen.getByText('world!'), { deltaY: 100 });
  view.rerender(
    <PlaybackInspectorFeed
      index={index}
      time={1700}
      isPlaying={false}
      readPlaybackTimeMs={readPlaybackTimeMs}
      seekRevision={1}
      onSeek={seek}
    />,
  );
  expect(
    screen.queryByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeNull();
  expect(screen.getByText('world!').getAttribute('aria-current')).toBe('true');
  fireEvent.wheel(screen.getByText('world!'), { deltaY: 100 });
  view.rerender(
    <PlaybackInspectorFeed
      index={{ entries: [], speechBySource: new Map(), focus: [] }}
      time={1700}
      isPlaying={false}
      readPlaybackTimeMs={readPlaybackTimeMs}
      seekRevision={1}
      onSeek={seek}
    />,
  );
  expect(screen.queryByText('world!')).toBeNull();
  expect(
    screen.queryByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeNull();
});

test.each(
  ['light', 'dark'].flatMap((theme) =>
    [1440, 390].map((width) => ({ theme, width })),
  ),
)(
  'compact transcript and AI notes remain readable and navigable at $width px in $theme',
  async ({ theme, width }) => {
    await page.viewport(width, 650);
    document.documentElement.dataset.theme = theme;
    const desktop: InspectorContentEntry = {
      ...microphone,
      id: 'desktop',
      source: 'Desktop audio',
      trackId: 'desktop',
      text: 'Synthetic desktop speech.',
      words: [],
    };
    const later = {
      ...microphone,
      id: 'later',
      startMs: 3000,
      endMs: 5000,
      text: 'More synthetic speech in the next paragraph.',
      words: [],
    };
    const duplicate = {
      ...annotation,
      id: 'duplicate',
      startMs: 1800,
      endMs: 1801,
    };
    const corrected = {
      ...annotation,
      id: 'corrected',
      observationKind: 'corrected-ocr' as const,
      text: 'Synthetic OCR correction',
    };
    const raw = {
      ...annotation,
      id: 'raw',
      kind: 'screen-text' as const,
      text: 'Synthetic raw OCR',
    };
    const prepared = {
      ...index,
      entries: [
        microphone,
        desktop,
        annotation,
        corrected,
        raw,
        duplicate,
        later,
      ],
      speechBySource: new Map([
        ['mic', [microphone, later]],
        ['desktop', [desktop]],
      ]),
    };
    const seek = vi.fn();
    const view = render(
      <div className="bg-muted text-foreground rounded-panel max-w-[420px]">
        <div className="border-b px-6 py-4 text-sm">Teleprompter</div>
        <PlaybackInspectorFeed
          index={prepared}
          time={1700}
          isPlaying={false}
          onSeek={seek}
        />
      </div>,
    );
    expect(screen.queryByText('Synthetic raw OCR')).toBeNull();
    expect(screen.queryByText('Synthetic OCR correction')).toBeNull();
    const expand = await screen.findByRole('button', {
      name: 'Expand AI observation from Monitor at 0:01',
    });
    expect(
      screen.getAllByRole('button', {
        name: 'Jump to AI observation from Monitor at 0:01',
      }),
    ).toHaveLength(1);
    await page.screenshot({
      element: view.container,
      path: `__screenshots__/compact-feed-${theme}-${width}.png`,
    });
    expand.focus();
    await userEvent.keyboard('{Enter}');
    expect(
      screen
        .getByRole('button', {
          name: 'Collapse AI observation from Monitor at 0:01',
        })
        .getAttribute('aria-expanded'),
    ).toBe('true');
    await userEvent.click(
      screen.getByRole('button', {
        name: 'Jump to AI observation from Monitor at 0:01',
      }),
    );
    expect(seek).toHaveBeenCalledWith(1555);
    await userEvent.click(
      screen.getByRole('button', {
        name: 'Collapse AI observation from Monitor at 0:01',
      }),
    );
    expect(
      screen
        .getByRole('button', {
          name: 'Expand AI observation from Monitor at 0:01',
        })
        .getAttribute('aria-expanded'),
    ).toBe('false');
  },
);
