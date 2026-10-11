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

test('spoken words follow their source timing without changing transcript text or rounding seek targets', async () => {
  const microphone = {
    ...speech('mic', 'Microphone', 1000, 3000),
    text: 'Hallo, Welt!',
    words: [
      {
        id: 'hello',
        text: 'Hallo,',
        startOffsetMs: 1000,
        endOffsetMs: 1500,
        probability: null,
      },
      {
        id: 'world',
        text: 'Welt!',
        startOffsetMs: 1600,
        endOffsetMs: 2300,
        probability: null,
      },
    ],
  };
  const desktop = {
    ...speech('desktop', 'Desktop audio', 1200, 2400),
    text: '同时播放',
    words: [
      {
        id: 'desktop-word',
        text: '同时播放',
        startOffsetMs: 1200,
        endOffsetMs: 2000,
        probability: null,
      },
    ],
  };
  const index: InspectorContentIndex = {
    entries: [microphone, desktop],
    speechBySource: new Map([
      ['mic', [microphone]],
      ['desktop', [desktop]],
    ]),
    focus: [],
  };
  const seek = vi.fn();
  const view = render(
    <PlaybackInspectorFeed index={index} time={1250} isPlaying onSeek={seek} />,
  );
  expect(screen.getByText('Hallo,').getAttribute('aria-current')).toBe('true');
  expect(screen.getByText('同时播放').getAttribute('aria-current')).toBe(
    'true',
  );
  view.rerender(
    <PlaybackInspectorFeed
      index={index}
      time={1500}
      isPlaying={false}
      onSeek={seek}
    />,
  );
  expect(screen.getByText('Hallo,').hasAttribute('aria-current')).toBe(false);
  view.rerender(
    <PlaybackInspectorFeed
      index={index}
      time={1700}
      isPlaying={false}
      onSeek={seek}
    />,
  );
  expect(screen.getByText('Welt!').getAttribute('aria-current')).toBe('true');
  const button = screen.getByRole('button', {
    name: 'Jump to Desktop audio at 0:01',
  });
  await userEvent.click(button);
  expect(seek).toHaveBeenCalledWith(1200);
});

test('raw screen text stays out of playback findings', () => {
  const ocr: InspectorContentEntry = {
    ...speech('ocr', 'Monitor', 1250, 2000),
    kind: 'screen-text',
    text: 'Synthetic raw OCR',
  };
  render(
    <PlaybackInspectorFeed
      index={{ entries: [ocr], speechBySource: new Map(), focus: [] }}
      time={1300}
      isPlaying={false}
      onSeek={vi.fn()}
    />,
  );
  expect(screen.queryByText('Synthetic raw OCR')).toBeNull();
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
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeTruthy();
  view.rerender(
    <PlaybackInspectorFeed index={index} time={2800} isPlaying onSeek={seek} />,
  );
  expect(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeTruthy();
  await userEvent.click(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  );
  expect(
    screen.queryByRole('button', { name: 'Resume auto-scroll' }),
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
    screen.queryByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeNull();
});
