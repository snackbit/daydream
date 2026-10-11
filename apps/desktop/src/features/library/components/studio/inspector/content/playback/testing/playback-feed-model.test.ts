import { expect, test } from 'vitest';
import type {
  InspectorContentEntry,
  InspectorContentIndex,
} from '../../inspector-content';
import {
  buildPlaybackFeedIndex,
  playbackFeedFrame,
  playbackTimestamp,
  playbackWordSpans,
} from '../playback-feed-model';

function speech(
  changes: Partial<InspectorContentEntry> = {},
): InspectorContentEntry {
  return {
    id: 'mic',
    kind: 'speech',
    source: 'Microphone',
    trackId: 'mic',
    startMs: 1000,
    endMs: 4000,
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
    ...changes,
  };
}
function index(entries: InspectorContentEntry[]): InspectorContentIndex {
  const speechBySource = new Map<string, InspectorContentEntry[]>();
  for (const entry of entries)
    if (entry.kind === 'speech') {
      const source = speechBySource.get(entry.trackId) ?? [];
      source.push(entry);
      speechBySource.set(entry.trackId, source);
    }
  return { entries, speechBySource, focus: [] };
}
test('canonical punctuation, repeated words, whitespace and multilingual text remain intact', () => {
  for (const text of [
    'Hello,  world!',
    'Hello,\nworld!',
    'Hello,世界 world!',
  ]) {
    const spans = playbackWordSpans(speech({ text }));
    expect(spans.map((span) => span.text).join('')).toBe(text);
    expect(
      spans.filter((span) => span.word).map((span) => span.word?.id),
    ).toEqual(['hello', 'world']);
  }
  const repeated = speech({
    text: 'Hello, Hello,',
    words: speech().words.map((word) => ({ ...word, text: 'Hello,' })),
  });
  expect(
    playbackWordSpans(repeated)
      .map((span) => span.text)
      .join(''),
  ).toBe(repeated.text);
  expect(
    playbackWordSpans(
      speech({ words: [{ ...speech().words[0]!, text: 'Unmatched' }] }),
    ),
  ).toEqual([{ text: 'Hello, world!', word: null }]);
  expect(playbackWordSpans(speech({ words: [] }))).toEqual([
    { text: 'Hello, world!', word: null },
  ]);
});
test('intervals are start inclusive, end exclusive, with no active word in silence and one word per source', () => {
  const mic = speech(),
    desktop = speech({
      id: 'desktop',
      trackId: 'desktop',
      source: 'Desktop audio',
    });
  const prepared = buildPlaybackFeedIndex(index([mic, desktop]));
  expect(playbackFeedFrame(prepared, 1000).activeWords).toEqual(
    new Map([
      ['mic', 'hello'],
      ['desktop', 'hello'],
    ]),
  );
  expect([...playbackFeedFrame(prepared, 1500).activeWords]).toEqual([]);
  expect(playbackFeedFrame(prepared, 1600).activeWords).toEqual(
    new Map([
      ['mic', 'world'],
      ['desktop', 'world'],
    ]),
  );
  expect([...playbackFeedFrame(prepared, 2400).activeWords]).toEqual([]);
  expect(playbackFeedFrame(prepared, 1700).currentId).toBe('mic');
  const overlap = speech({
    words: [
      mic.words[0]!,
      { ...mic.words[1]!, startOffsetMs: 1400 },
      { ...mic.words[0]!, id: 'zero', endOffsetMs: 1000 },
    ],
  });
  expect([
    ...playbackFeedFrame(buildPlaybackFeedIndex(index([overlap])), 1450)
      .activeWords,
  ]).toEqual([['mic', 'world']]);
});
test('the ruler holds the previous word through timing gaps without treating it as spoken, including backward seeks', () => {
  const prepared = buildPlaybackFeedIndex(index([speech()]));
  for (const [time, word] of [
    [1000, 'hello'],
    [1499, 'hello'],
    [1500, 'hello'],
    [1599, 'hello'],
    [1600, 'world'],
    [2400, 'world'],
    [3999, 'world'],
    [1500, 'hello'],
  ] as const)
    expect(playbackFeedFrame(prepared, time).rulerWordId).toBe(word);
  expect(playbackFeedFrame(prepared, 1500).activeWords.size).toBe(0);
  expect(playbackFeedFrame(prepared, 2400).activeWords.size).toBe(0);
  const delayed = speech({
    words: speech().words.map((word) => ({
      ...word,
      startOffsetMs: word.startOffsetMs + 250,
      endOffsetMs: word.endOffsetMs + 250,
    })),
  });
  expect(
    playbackFeedFrame(buildPlaybackFeedIndex(index([delayed])), 1100)
      .rulerWordId,
  ).toBe('hello');
  expect(
    playbackFeedFrame(
      buildPlaybackFeedIndex(index([speech({ words: [] })])),
      1500,
    ).rulerWordId,
  ).toBeUndefined();
});
test('OCR and corrected OCR are excluded while distinct per-source observations retain original provenance', () => {
  const observation = speech({
    id: 'note',
    kind: 'observation',
    observationKind: 'screen-observation',
    trackId: 'monitor',
    source: 'Monitor',
    text: 'Synthetic action',
    words: [],
  });
  const entries = [
    speech(),
    { ...observation, id: 'raw', kind: 'screen-text' as const },
    {
      ...observation,
      id: 'corrected',
      observationKind: 'corrected-ocr' as const,
    },
    observation,
    { ...observation, id: 'duplicate', text: ' Synthetic   action ' },
    { ...observation, id: 'second-monitor', trackId: 'monitor-2' },
    { ...observation, id: 'different', text: 'Another action' },
    { ...observation, id: 'again' },
  ];
  const original = index(entries);
  expect(
    buildPlaybackFeedIndex(original).entries.map((entry) => entry.id),
  ).toEqual(['mic', 'note', 'second-monitor', 'different', 'again']);
  expect(original.entries).toHaveLength(8);
});
test('render signatures change only at observable boundaries and timestamp formatting never rounds a seek', () => {
  const prepared = buildPlaybackFeedIndex(index([speech()]));
  expect(playbackFeedFrame(prepared, 1100).signature).toBe(
    playbackFeedFrame(prepared, 1200).signature,
  );
  expect(playbackFeedFrame(prepared, 1499).signature).not.toBe(
    playbackFeedFrame(prepared, 1500).signature,
  );
  expect(playbackTimestamp(84999)).toBe('1:24');
  expect(playbackTimestamp(3600123)).toBe('1:00:00');
});
