import { cleanup, render, screen, fireEvent } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import type {
  InspectorContentEntry,
  InspectorContentIndex,
} from './inspector-content';
import { PlaybackInspectorFeed } from './playback-inspector-feed';

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});
function speech(
  id: string,
  source: string,
  startMs: number,
  endMs: number,
): InspectorContentEntry {
  return {
    id,
    kind: 'speech',
    source,
    trackId: source,
    startMs,
    endMs,
    text: `Synthetic ${id}`,
    words: [],
    fragments: [],
    boundary: false,
  };
}
test('playback feed keeps simultaneous sources, seeks from entries and allows browsing before returning to playback', async () => {
  const microphone = speech('microphone', 'Microphone', 1000, 4000),
    desktop = speech('desktop', 'Desktop audio', 2000, 3000),
    upcoming = speech('next', 'Microphone', 5000, 6000);
  const focus: InspectorContentEntry = {
    ...speech('focus', 'Window focus', 0, 1),
    kind: 'focus',
    text: 'Synthetic editor · Notes',
  };
  const index: InspectorContentIndex = {
    entries: [focus, microphone, desktop, upcoming],
    speechBySource: new Map([
      ['Microphone', [microphone, upcoming]],
      ['Desktop audio', [desktop]],
    ]),
    focus: [focus],
  };
  const seek = vi.fn();
  const view = render(
    <PlaybackInspectorFeed index={index} time={2500} isPlaying onSeek={seek} />,
  );
  expect(screen.getByText('Synthetic microphone')).toBeTruthy();
  expect(screen.getByText('Synthetic desktop')).toBeTruthy();
  view.container.style.maxWidth = '420px';
  document.documentElement.dataset.theme = 'light';
  await page.screenshot({
    element: view.container,
    path: '__screenshots__/playback-feed-light.png',
  });
  document.documentElement.dataset.theme = 'dark';
  await page.screenshot({
    element: view.container,
    path: '__screenshots__/playback-feed-dark.png',
  });
  delete document.documentElement.dataset.theme;
  await userEvent.click(
    screen.getByRole('button', { name: /Jump to Microphone at 0:05/ }),
  );
  expect(seek).toHaveBeenCalledWith(5000);
  fireEvent.wheel(screen.getByText('Synthetic desktop'), { deltaY: 100 });
  expect(
    screen.getByRole('button', { name: 'Return to playback' }),
  ).toBeTruthy();
  view.rerender(
    <PlaybackInspectorFeed index={index} time={2800} isPlaying onSeek={seek} />,
  );
  expect(
    screen.getByRole('button', { name: 'Return to playback' }),
  ).toBeTruthy();
  await userEvent.click(
    screen.getByRole('button', { name: 'Return to playback' }),
  );
  expect(
    screen.queryByRole('button', { name: 'Return to playback' }),
  ).toBeNull();
  fireEvent.wheel(screen.getByText('Synthetic desktop'), { deltaY: 100 });
  view.rerender(
    <PlaybackInspectorFeed
      index={{ entries: [], speechBySource: new Map(), focus: [] }}
      time={2800}
      isPlaying
      onSeek={seek}
    />,
  );
  expect(screen.queryByText('Synthetic desktop')).toBeNull();
  expect(
    screen.queryByRole('button', { name: 'Return to playback' }),
  ).toBeNull();
});
