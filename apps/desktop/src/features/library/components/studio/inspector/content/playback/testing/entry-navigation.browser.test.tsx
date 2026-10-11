import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import type { InspectorContentEntry } from '../../inspector-content';
import { PlaybackInspectorFeed } from '../../playback-inspector-feed';

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});
const speech: InspectorContentEntry = {
  id: 'speech',
  kind: 'speech',
  source: 'Microphone',
  trackId: 'mic',
  startMs: 1234,
  endMs: 2345,
  text: 'A synthetic phrase.',
  words: [],
  fragments: [],
  boundary: false,
};
const note: InspectorContentEntry = {
  ...speech,
  id: 'note',
  kind: 'observation',
  observationKind: 'screen-observation',
  source: 'Monitor 2',
  trackId: 'video-2',
  startMs: 3456,
  endMs: 3457,
  text: 'A synthetic annotation. '.repeat(30),
};

test('transcript text and heading seek precisely with pointer and keyboard', async () => {
  const seek = vi.fn();
  render(
    <div style={{ height: 400, width: 390 }}>
      <PlaybackInspectorFeed
        index={{ entries: [speech], speechBySource: new Map(), focus: [] }}
        time={0}
        isPlaying={false}
        onSeek={seek}
      />
    </div>,
  );
  await userEvent.click(screen.getByText(speech.text));
  expect(seek).toHaveBeenLastCalledWith(1234);
  await userEvent.click(screen.getByText('Microphone'));
  expect(seek).toHaveBeenCalledTimes(2);
  const button = screen.getByRole('button', {
    name: 'Jump to transcript from Microphone at 0:01',
  });
  screen.getByRole('list').focus();
  await userEvent.keyboard('{Tab}');
  expect(
    screen.queryByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeNull();
  button.focus();
  await userEvent.keyboard('{Enter}');
  expect(seek).toHaveBeenCalledTimes(3);
  await userEvent.keyboard(' ');
  expect(seek).toHaveBeenCalledTimes(4);
  expect(
    screen.queryByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeNull();
  const text = screen.getByText(speech.text);
  const range = document.createRange();
  range.selectNodeContents(text);
  const selection = window.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
  fireEvent.click(text, { detail: 1 });
  expect(seek).toHaveBeenCalledTimes(4);
  selection?.removeAllRanges();
});

test.each(['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 'Home', 'End', ' '])(
  'scrolling with %s on the list pauses following without seeking',
  async (key) => {
    const seek = vi.fn();
    render(
      <div style={{ height: 400, width: 390 }}>
        <PlaybackInspectorFeed
          index={{ entries: [speech], speechBySource: new Map(), focus: [] }}
          time={0}
          isPlaying={false}
          onSeek={seek}
        />
      </div>,
    );
    screen.getByRole('list').focus();
    await userEvent.keyboard(key === ' ' ? ' ' : `{${key}}`);
    expect(
      screen.getByRole('button', { name: 'Resume auto-scroll' }),
    ).toBeTruthy();
    expect(seek).not.toHaveBeenCalled();
    await userEvent.click(
      screen.getByRole('button', { name: 'Resume auto-scroll' }),
    );
    expect(
      screen.queryByRole('button', { name: 'Resume auto-scroll' }),
    ).toBeNull();
  },
);

test('zoom, zero-motion wheels and handled or modified keys do not pause following; repeated scroll gestures do', async () => {
  const seek = vi.fn();
  render(
    <div style={{ height: 400, width: 390 }}>
      <PlaybackInspectorFeed
        index={{ entries: [speech], speechBySource: new Map(), focus: [] }}
        time={1500}
        isPlaying={false}
        onSeek={seek}
      />
    </div>,
  );
  const list = screen.getByRole('list');
  fireEvent.wheel(list, { deltaX: 20, deltaY: 0 });
  fireEvent.wheel(list, { deltaY: 20, ctrlKey: true });
  const wheel = new WheelEvent('wheel', {
    deltaY: 20,
    bubbles: true,
    cancelable: true,
  });
  wheel.preventDefault();
  fireEvent(list, wheel);
  for (const modifier of ['altKey', 'ctrlKey', 'metaKey'])
    fireEvent.keyDown(list, { key: 'ArrowDown', [modifier]: true });
  const handled = new KeyboardEvent('keydown', {
    key: 'ArrowDown',
    bubbles: true,
    cancelable: true,
  });
  handled.preventDefault();
  fireEvent(list, handled);
  fireEvent.pointerDown(list);
  expect(
    screen.queryByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeNull();
  fireEvent.wheel(list, { deltaY: 20 });
  fireEvent.wheel(list, { deltaY: 20 });
  expect(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeTruthy();
  await userEvent.click(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  );
  fireEvent.touchMove(list);
  expect(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeTruthy();
  expect(seek).not.toHaveBeenCalled();
});

test.each([
  { theme: 'light', width: 390 },
  { theme: 'dark', width: 390 },
  { theme: 'light', width: 900 },
  { theme: 'dark', width: 900 },
])(
  'entry icons and click targets are consistent in $theme at $width pixels',
  async ({ theme, width }) => {
    await page.viewport(width, 650);
    document.documentElement.dataset.theme = theme;
    const mouse: InspectorContentEntry = {
      ...speech,
      id: 'mouse',
      kind: 'activity',
      source: 'Mouse',
      text: 'Mouse activity',
      startMs: 3000,
    };
    const keyboard: InspectorContentEntry = {
      ...mouse,
      id: 'keyboard',
      source: 'Keyboard',
      text: 'Keyboard activity',
      startMs: 4000,
    };
    const focus: InspectorContentEntry = {
      ...mouse,
      id: 'focus',
      kind: 'focus',
      source: 'Window focus',
      text: 'Synthetic window',
      startMs: 5000,
    };
    const annotation = { ...note, text: 'A synthetic note.', startMs: 6000 };
    const seek = vi.fn();
    const view = render(
      <div className="bg-muted text-foreground" style={{ height: 500, width }}>
        <PlaybackInspectorFeed
          index={{
            entries: [speech, mouse, keyboard, focus, annotation],
            speechBySource: new Map(),
            focus: [],
          }}
          time={0}
          isPlaying={false}
          onSeek={seek}
        />
      </div>,
    );
    for (const entry of [speech, mouse, keyboard, focus, annotation]) {
      await userEvent.click(screen.getByText(entry.text));
      expect(seek).toHaveBeenLastCalledWith(entry.startMs);
    }
    const icons = view.container.querySelectorAll('[data-inspector-entry] svg');
    expect(icons.length).toBe(5);
    for (const icon of icons) {
      expect(icon.getBoundingClientRect().width).toBe(12);
      expect(icon.getBoundingClientRect().height).toBe(12);
    }
    await page.screenshot({
      element: view.container,
      path: `__screenshots__/entry-navigation-${theme}-${width}.png`,
    });
  },
);

test('annotation text seeks while expansion is independent', async () => {
  const seek = vi.fn();
  render(
    <div style={{ height: 400, width: 390 }}>
      <PlaybackInspectorFeed
        index={{ entries: [note], speechBySource: new Map(), focus: [] }}
        time={0}
        isPlaying={false}
        onSeek={seek}
      />
    </div>,
  );
  await userEvent.click(
    screen.getByRole('button', {
      name: 'Expand AI observation from Monitor 2 at 0:03',
    }),
  );
  expect(seek).not.toHaveBeenCalled();
  await userEvent.click(screen.getByText(note.text.trim()));
  expect(seek).toHaveBeenCalledWith(3456);
});
