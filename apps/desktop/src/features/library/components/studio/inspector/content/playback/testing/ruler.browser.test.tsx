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
import type {
  InspectorContentEntry,
  InspectorContentIndex,
} from '../../inspector-content';
import { PlaybackInspectorFeed } from '../../playback-inspector-feed';

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});
const speech: InspectorContentEntry = {
  id: 'mic',
  kind: 'speech',
  source: 'Microphone',
  trackId: 'mic',
  startMs: 1001,
  endMs: 3900,
  text: 'Hello, world!',
  fragments: [],
  boundary: false,
  words: [
    {
      id: 'hello',
      text: 'Hello,',
      startOffsetMs: 1001,
      endOffsetMs: 1500,
      probability: null,
    },
    {
      id: 'world',
      text: 'world!',
      startOffsetMs: 2100,
      endOffsetMs: 3100,
      probability: null,
    },
  ],
};
const index: InspectorContentIndex = {
  entries: [speech],
  speechBySource: new Map([['mic', [speech]]]),
  focus: [],
};

test('the ruler and word highlights sample live playback while browsing and timestamps seek precisely', async () => {
  let clock = 1100;
  const readPlaybackTimeMs = () => clock;
  const seek = vi.fn();
  render(
    <div style={{ width: 390, height: 400 }}>
      <PlaybackInspectorFeed
        index={index}
        time={1100}
        isPlaying
        readPlaybackTimeMs={readPlaybackTimeMs}
        onSeek={seek}
      />
    </div>,
  );
  expect(
    screen.getByRole('timer', { name: 'Playback position' }).textContent,
  ).toBe('0:01');
  expect(
    await screen.findByLabelText('Current playback position'),
  ).toBeTruthy();
  fireEvent.wheel(screen.getByRole('list'), { deltaY: 100 });
  clock = 2200;
  await waitFor(() =>
    expect(screen.getByRole('timer').textContent).toBe('0:02'),
  );
  expect(screen.getByText('world!')).toHaveAttribute('aria-current', 'true');
  expect(
    screen
      .getByLabelText('Current playback position')
      .getAttribute('data-word-id'),
  ).toBe('world');
  expect(seek).not.toHaveBeenCalled();
  await userEvent.click(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  );
  await waitFor(() =>
    expect(screen.getByText('world!').getAttribute('aria-current')).toBe(
      'true',
    ),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Jump to Microphone at 0:01' }),
  );
  expect(seek).toHaveBeenCalledWith(1001);
});

test.each([
  {
    time: 0,
    key: '{End}',
    direction: 'above',
    position: '0:00',
    paragraph: 0,
    destination: 299,
  },
  {
    time: 25000,
    key: '{End}',
    direction: 'above',
    position: '0:25',
    paragraph: 2,
    destination: 299,
  },
  {
    time: 2405000,
    key: '{Home}',
    direction: 'below',
    position: '40:05',
    paragraph: 240,
    destination: 0,
  },
  {
    time: 3000000,
    key: '{Home}',
    direction: 'below',
    position: '50:00',
    paragraph: 299,
    destination: 0,
  },
] as const)(
  'an offscreen playhead $direction returns to playback without seeking or mounting all entries',
  async ({ time, key, direction, position, paragraph, destination }) => {
    const entries = Array.from({ length: 300 }, (_, i) => ({
      ...speech,
      id: `speech-${i}`,
      startMs: i * 10000 + 1001,
      endMs: i * 10000 + 9000,
      words: [],
      text: `Synthetic paragraph ${i}.`,
    }));
    const longIndex = {
      entries,
      speechBySource: new Map([['mic', entries]]),
      focus: [],
    };
    const seek = vi.fn();
    render(
      <div style={{ width: 390, height: 400 }}>
        <PlaybackInspectorFeed
          index={longIndex}
          time={time}
          isPlaying={false}
          onSeek={seek}
        />
      </div>,
    );
    screen.getByRole('list').focus();
    await userEvent.keyboard(key);
    await screen.findByText(`Synthetic paragraph ${destination}.`);
    await userEvent.click(
      await screen.findByRole('button', {
        name: `Playback is ${direction} at ${position}; resume auto-scroll`,
      }),
    );
    await screen.findByText(`Synthetic paragraph ${paragraph}.`);
    expect(seek).not.toHaveBeenCalled();
    expect(screen.getAllByRole('listitem').length).toBeLessThan(40);
  },
);

test.each(['light', 'dark'] as const)(
  'ruler, wrapped text and annotations fit a narrow panel in %s mode',
  async (theme) => {
    await page.viewport(390, 650);
    document.documentElement.dataset.theme = theme;
    const note: InspectorContentEntry = {
      ...speech,
      id: 'note',
      kind: 'observation',
      observationKind: 'screen-observation',
      source: 'Monitor',
      trackId: 'monitor',
      startMs: 900,
      endMs: 901,
      text: 'A synthetic document was updated. '.repeat(14),
      words: [],
    };
    const hour = {
      ...speech,
      id: 'hour',
      startMs: 3600123,
      endMs: 3605123,
      words: [],
      text: 'An hour later, the synthetic recording continues.',
    };
    const view = render(
      <div
        className="bg-muted text-foreground"
        style={{ width: 390, height: 550 }}
      >
        <PlaybackInspectorFeed
          index={{
            ...index,
            entries: [
              note,
              {
                ...speech,
                text: 'Hello, this is a synthetic wrapped transcript with canonical text.\nworld!',
              },
              hour,
            ],
          }}
          time={2200}
          isPlaying={false}
          onSeek={vi.fn()}
        />
      </div>,
    );
    await screen.findByLabelText('Current playback position');
    await page.screenshot({
      element: view.container,
      path: `__screenshots__/ruler-${theme}-390.png`,
    });
    await userEvent.click(
      screen.getByRole('button', {
        name: 'Expand AI observation from Monitor at 0:00',
      }),
    );
    await page.screenshot({
      element: view.container,
      path: `__screenshots__/ruler-expanded-${theme}-390.png`,
    });
  },
);

test('the playhead anchors to microphone words, holds the previous word in silence, and discards replaced content', async () => {
  const desktop = {
    ...speech,
    id: 'desktop',
    source: 'Desktop audio',
    trackId: 'desktop',
    text: 'Desktop speech',
    words: [],
  };
  const overlapping = {
    ...index,
    entries: [desktop, speech],
    speechBySource: new Map([
      ['mic', [speech]],
      ['desktop', [desktop]],
    ]),
  };
  const seek = vi.fn();
  const view = render(
    <div style={{ width: 390, height: 400 }}>
      <PlaybackInspectorFeed
        index={overlapping}
        time={2200}
        isPlaying={false}
        onSeek={seek}
      />
    </div>,
  );
  expect(
    (await screen.findByLabelText('Current playback position')).getAttribute(
      'data-entry-id',
    ),
  ).toBe('mic');
  expect(
    screen
      .getByLabelText('Current playback position')
      .getAttribute('data-word-id'),
  ).toBe('world');
  view.rerender(
    <div style={{ width: 390, height: 400 }}>
      <PlaybackInspectorFeed
        index={overlapping}
        time={1700}
        isPlaying={false}
        seekRevision={1}
        onSeek={seek}
      />
    </div>,
  );
  expect(
    screen
      .getByLabelText('Current playback position')
      .getAttribute('data-word-id'),
  ).toBe('hello');
  expect(screen.getByText('Hello,').hasAttribute('aria-current')).toBe(false);
  expect(screen.getByText('world!').hasAttribute('aria-current')).toBe(false);
  fireEvent.wheel(screen.getByRole('list'), { deltaY: 100 });
  view.rerender(
    <div style={{ width: 390, height: 400 }}>
      <PlaybackInspectorFeed
        index={{
          ...overlapping,
          entries: [desktop],
          speechBySource: new Map([['desktop', [desktop]]]),
        }}
        time={2200}
        isPlaying={false}
        seekRevision={1}
        onSeek={seek}
      />
    </div>,
  );
  expect(screen.queryByText('world!')).toBeNull();
  expect(
    screen
      .getByLabelText('Current playback position')
      .getAttribute('data-entry-id'),
  ).toBe('desktop');
  expect(
    screen.queryByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeTruthy();
});
