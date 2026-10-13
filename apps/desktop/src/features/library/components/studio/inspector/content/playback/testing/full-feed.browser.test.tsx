import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import type {
  InspectorContentEntry,
  InspectorContentIndex,
} from '../../inspector-content';
import { PlaybackInspectorFeed } from '../../playback-inspector-feed';

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});

test.each(['light', 'dark'] as const)(
  'a long virtualized transcript keeps highlights live beyond a minute without moving a browsed viewport in %s',
  async (theme) => {
    await page.viewport(390, 600);
    document.documentElement.dataset.theme = theme;
    const entries: InspectorContentEntry[] = Array.from(
      { length: 700 },
      (_, i) => ({
        id: `long-speech-${i}`,
        kind: 'speech',
        source: 'Microphone',
        trackId: 'mic',
        startMs: i * 2000,
        endMs: i * 2000 + 2000,
        text: `Synthetic phrase ${i}.`,
        fragments: [],
        boundary: false,
        words: [
          {
            id: `word-${i}`,
            text: `Synthetic phrase ${i}.`,
            startOffsetMs: i * 2000,
            endOffsetMs: i * 2000 + 2000,
            probability: null,
          },
        ],
      }),
    );
    const index: InspectorContentIndex = {
      entries,
      speechBySource: new Map([['mic', entries]]),
      focus: [],
    };
    let clock = 55000;
    const seek = vi.fn();
    const readPlaybackTimeMs = () => clock;
    const view = render(
      <div
        className="bg-muted text-foreground rounded-panel"
        style={{ height: 400, width: 390 }}
      >
        <PlaybackInspectorFeed
          index={index}
          time={55000}
          isPlaying
          readPlaybackTimeMs={readPlaybackTimeMs}
          onSeek={seek}
        />
      </div>,
    );
    const current = await screen.findByText('Synthetic phrase 27.');
    expect(current).toHaveAttribute('aria-current', 'true');
    const viewport = view.container.querySelector(
      '[data-slot="scroll-area-viewport"]',
    );
    if (!(viewport instanceof HTMLElement))
      throw new Error('Missing Teleprompter viewport');
    fireEvent.wheel(screen.getByRole('list'), { deltaY: 1 });
    const reconciliation = vi.spyOn(viewport, 'scrollTo');
    for (let burst = 0; burst < 20; burst++)
      fireEvent.wheel(screen.getByRole('list'), { deltaY: 100 });
    expect(reconciliation).not.toHaveBeenCalled();
    const top = viewport.scrollTop;
    clock = 61000;
    const next = await screen.findByText('Synthetic phrase 30.');
    await waitFor(() => expect(next).toHaveAttribute('aria-current', 'true'));
    expect(current).not.toHaveAttribute('aria-current');
    expect(next.closest('[data-phase]')).toHaveAttribute(
      'data-phase',
      'current',
    );
    expect(current.closest('[data-phase]')).toHaveAttribute(
      'data-phase',
      'past',
    );
    expect(viewport.scrollTop).toBe(top);
    await page.screenshot({
      element: view.container,
      path: `__screenshots__/live-browsing-after-minute-${theme}.png`,
    });
    clock = 90000;
    await waitFor(() =>
      expect(screen.getByRole('timer').textContent).toBe('1:30'),
    );
    expect(next).not.toHaveAttribute('aria-current');
    expect(viewport.scrollTop).toBe(top);
    expect(screen.queryByText('Synthetic phrase 45.')).toBeNull();
    await page.screenshot({
      element: view.container,
      path: `__screenshots__/live-browsing-offscreen-${theme}.png`,
    });
    await userEvent.click(
      screen.getByRole('button', { name: 'Resume auto-scroll' }),
    );
    const resumed = await screen.findByText('Synthetic phrase 45.');
    await waitFor(() =>
      expect(resumed).toHaveAttribute('aria-current', 'true'),
    );
    expect(seek).not.toHaveBeenCalled();
    expect(screen.getAllByRole('listitem').length).toBeLessThan(40);
  },
);

test('the complete recording is browsable without changing playback, then resumes at the current paragraph', async () => {
  const entries: InspectorContentEntry[] = Array.from(
    { length: 300 },
    (_, i) => ({
      id: `speech-${i}`,
      kind: 'speech',
      source: 'Microphone',
      trackId: 'mic',
      startMs: i * 10000,
      endMs: i * 10000 + 9000,
      text: `Synthetic paragraph ${i}.`,
      words: [],
      fragments: [],
      boundary: false,
    }),
  );
  const index: InspectorContentIndex = {
    entries,
    speechBySource: new Map([['mic', entries]]),
    focus: [],
  };
  const seek = vi.fn();
  const view = render(
    <div style={{ height: 400, width: 390 }}>
      <PlaybackInspectorFeed index={index} time={0} isPlaying onSeek={seek} />
    </div>,
  );
  const list = screen.getByRole('list');
  expect(screen.getAllByText('Microphone')).toHaveLength(1);
  list.focus();
  await userEvent.keyboard('{End}');
  await screen.findByText('Synthetic paragraph 299.');
  expect(screen.queryByText('Microphone')).toBeNull();
  expect(seek).not.toHaveBeenCalled();
  expect(screen.getAllByRole('listitem').length).toBeLessThan(40);
  view.rerender(
    <div style={{ height: 400, width: 390 }}>
      <PlaybackInspectorFeed
        index={index}
        time={25000}
        isPlaying
        onSeek={seek}
      />
    </div>,
  );
  expect(screen.getByText('Synthetic paragraph 299.')).toBeTruthy();
  await userEvent.click(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  );
  await screen.findByText('Synthetic paragraph 2.');
  await waitFor(() =>
    expect(screen.queryByText('Synthetic paragraph 299.')).toBeNull(),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Jump to Microphone at 0:20' }),
  );
  expect(seek).toHaveBeenCalledWith(20000);
  view.rerender(
    <div style={{ height: 400, width: 390 }}>
      <PlaybackInspectorFeed
        index={index}
        time={2400000}
        isPlaying={false}
        seekRevision={1}
        onSeek={seek}
      />
    </div>,
  );
  await screen.findByText('Synthetic paragraph 240.');
  view.rerender(
    <div style={{ height: 400, width: 390 }}>
      <PlaybackInspectorFeed
        index={{ entries: [], speechBySource: new Map(), focus: [] }}
        time={2400000}
        isPlaying={false}
        seekRevision={1}
        onSeek={seek}
      />
    </div>,
  );
  expect(screen.queryByText('Synthetic paragraph 240.')).toBeNull();
  expect(
    screen.queryByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeNull();
});

test('consecutive speech entries share a source heading while each timestamp remains seekable', async () => {
  await page.viewport(390, 600);
  const microphone: InspectorContentEntry = {
    id: 'mic-1',
    kind: 'speech',
    source: 'Microphone',
    trackId: 'mic',
    startMs: 1000,
    endMs: 2000,
    text: 'First synthetic phrase.',
    words: [],
    fragments: [],
    boundary: false,
  };
  const second = {
    ...microphone,
    id: 'mic-2',
    startMs: 2000,
    endMs: 3000,
    text: 'Second synthetic phrase.',
  };
  const desktop = {
    ...microphone,
    id: 'desktop',
    source: 'Desktop audio',
    trackId: 'desktop',
    startMs: 3000,
    endMs: 4000,
    text: 'Another source.',
  };
  const returned = {
    ...microphone,
    id: 'mic-3',
    startMs: 4000,
    endMs: 5000,
    text: 'Microphone again.',
  };
  const otherMicrophone = {
    ...microphone,
    id: 'other-mic',
    trackId: 'mic-2',
    startMs: 5000,
    endMs: 6000,
    text: 'A different microphone track.',
  };
  const seek = vi.fn();
  const view = render(
    <div
      className="bg-muted text-foreground"
      style={{ height: 400, width: 390 }}
    >
      <PlaybackInspectorFeed
        index={{
          entries: [microphone, second, desktop, returned, otherMicrophone],
          speechBySource: new Map(),
          focus: [],
        }}
        time={1500}
        isPlaying={false}
        onSeek={seek}
      />
    </div>,
  );
  const continuation = screen.getByText(second.text).closest('li');
  if (!continuation) throw new Error('Missing continuation row');
  expect(within(continuation).queryByText('Microphone')).toBeNull();
  expect(screen.getAllByText('Microphone')).toHaveLength(3);
  expect(screen.getByText('Desktop audio')).toBeTruthy();
  await userEvent.click(
    screen.getByRole('button', { name: 'Jump to Microphone at 0:02' }),
  );
  expect(seek).toHaveBeenCalledWith(2000);
  await page.screenshot({
    element: view.container,
    path: '__screenshots__/shared-speech-headings-390.png',
  });
});

test.each(['light', 'dark'] as const)(
  'long window titles keep timestamps and text inside the panel in %s mode',
  async (theme) => {
    await page.viewport(390, 600);
    document.documentElement.dataset.theme = theme;
    const focus: InspectorContentEntry = {
      id: 'focus',
      kind: 'focus',
      source: 'Window focus',
      trackId: 'window',
      startMs: 0,
      endMs: 1,
      text: `Synthetic-browser-${'long-title-'.repeat(14)} · Synthetic window`,
      words: [],
      fragments: [],
      boundary: false,
    };
    const mouse: InspectorContentEntry = {
      ...focus,
      id: 'mouse',
      kind: 'activity',
      source: 'Mouse',
      text: 'Mouse activity',
      startMs: 1000,
      endMs: 2000,
    };
    const index = {
      entries: [focus, mouse],
      speechBySource: new Map(),
      focus: [focus],
    };
    const view = render(
      <div
        className="bg-muted text-foreground rounded-panel"
        style={{ height: 400, width: 390 }}
      >
        <PlaybackInspectorFeed
          index={index}
          time={1000}
          isPlaying={false}
          onSeek={vi.fn()}
        />
      </div>,
    );
    expect(screen.getByText('Mouse activity')).toBeTruthy();
    await page.screenshot({
      element: view.container,
      path: `__screenshots__/full-feed-${theme}-390.png`,
    });
  },
);
