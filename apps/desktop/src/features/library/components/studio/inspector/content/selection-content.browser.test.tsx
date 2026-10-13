import type {
  RecordingAiProcessingState,
  RecordingTranscriptSegment,
} from '@daydream/shared';
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { TranscriptionSelectionDetails } from '../../studio-sidebar-transcription';
import {
  selectInspectorEntries,
  type InspectorContentEntry,
} from './inspector-content';
import { InspectorContentEntryCard } from './inspector-content-entry';

afterEach(cleanup);
const phrase: RecordingTranscriptSegment = {
  id: 'phrase',
  segmentId: 'media',
  startOffsetMs: 1000,
  endOffsetMs: 3000,
  text: 'Before selected after.',
  words: [
    {
      id: 'before',
      startOffsetMs: 1000,
      endOffsetMs: 1500,
      text: 'Before',
      probability: null,
    },
    {
      id: 'inside',
      startOffsetMs: 1500,
      endOffsetMs: 2000,
      text: 'selected',
      probability: null,
    },
    {
      id: 'after',
      startOffsetMs: 2000,
      endOffsetMs: 3000,
      text: 'after.',
      probability: null,
    },
  ],
  confidence: null,
  provider: 'synthetic',
  model: null,
};
const unrelated = {
  ...phrase,
  id: 'unrelated',
  startOffsetMs: 4000,
  endOffsetMs: 5000,
  text: 'Unrelated content',
  words: [],
};
const state: RecordingAiProcessingState = {
  recordingId: 'recording',
  job: null,
  chunks: [],
  transcript: {
    recordingId: 'recording',
    tracks: [
      {
        trackId: 'native-mic',
        sourceRole: 'microphone',
        segments: [phrase, unrelated],
      },
    ],
  },
  transcriptionTracks: [
    {
      trackId: 'native-mic',
      libraryTrackId: 'microphone-2',
      trackKey: 'mic',
      sourceRole: 'microphone',
      label: 'Microphone',
      description: 'Synthetic',
      status: 'complete',
      statusLabel: 'Complete',
      textSegmentCount: 2,
      chunks: [
        {
          id: 'chunk',
          segmentId: 'media',
          startOffsetMs: 0,
          endOffsetMs: 10000,
          attemptCount: 1,
          progress: 1,
          result: 'text',
          resultCode: null,
          errorCode: null,
          status: 'succeeded',
          text: 'Before selected after. Unrelated content',
          transcriptSegments: [phrase, unrelated],
        },
      ],
    },
  ],
};

test('range inspection separates sources and emphasizes only the selected words, without unrelated chunk text', () => {
  render(
    <TranscriptionSelectionDetails
      aiProcessingState={state}
      selections={[
        { id: 'range', laneIds: ['microphone-2'], startMs: 1500, endMs: 2000 },
      ]}
    />,
  );
  expect(screen.getByText('Microphone')).toBeTruthy();
  expect(
    screen
      .getByText('selected')
      .closest('[data-emphasis]')
      ?.getAttribute('data-emphasis'),
  ).toBe('selected');
  expect(
    screen
      .getByText('Before')
      .closest('[data-emphasis]')
      ?.getAttribute('data-emphasis'),
  ).toBe('context');
  expect(screen.queryByText(/Unrelated content/)).toBeNull();
  expect(screen.queryByText('Chunks')).toBeNull();
});

test.each(['screen-text', 'observation'] as const)(
  'range inspection hides %s recognition output while retaining audio text',
  (kind) => {
    const entry: InspectorContentEntry = {
      id: 'screen',
      kind,
      source: 'Screen',
      trackId: 'video-0',
      startMs: 1000,
      endMs: 3000,
      text: 'Synthetic internal recognition text',
      boundary: false,
      fragments: [],
      words: [],
      observationKind: 'corrected-ocr',
    };
    const spoken: InspectorContentEntry = {
      ...entry,
      id: 'speech',
      kind: 'speech',
      source: 'Microphone',
      trackId: 'mic',
      text: 'Synthetic spoken text',
      observationKind: 'screen-observation',
    };
    const selected = selectInspectorEntries(
      [entry, spoken],
      [
        {
          id: 'range',
          laneIds: ['video-0', 'mic'],
          startMs: 1000,
          endMs: 3000,
        },
      ],
    );
    render(
      <ol>
        {selected.map((item) => (
          <InspectorContentEntryCard key={item.id} entry={item} />
        ))}
      </ol>,
    );
    expect(
      screen.queryByText('Synthetic internal recognition text'),
    ).toBeNull();
    expect(screen.getByText('Synthetic spoken text')).toBeTruthy();
  },
);

test.each([
  ['Zusammenfassung, bitte.', ['Zusammen', 'fassung', ',', ' bitte', '.']],
  ['Hello,  hello!\n世界', ['Hello', ',', ' hello', '!', '世界']],
  ['Unmatched original sentence.', ['Different', 'tokens']],
] as const)(
  'selection preserves the canonical sentence %s rather than inserting token separators',
  (text, tokens) => {
    const entry: InspectorContentEntry = {
      id: 'speech',
      kind: 'speech',
      source: 'Microphone',
      trackId: 'mic',
      startMs: 1000,
      endMs: 3000,
      text,
      boundary: false,
      fragments: [],
      words: tokens.map((token, i) => ({
        id: `word-${i}`,
        text: token,
        startOffsetMs: 1000 + i * 100,
        endOffsetMs: 1090 + i * 100,
        probability: null,
      })),
    };
    const [selected] = selectInspectorEntries(
      [entry],
      [{ id: 'range', laneIds: ['mic'], startMs: 1100, endMs: 1200 }],
    );
    if (!selected) throw new Error('Missing selected transcript');
    const view = render(
      <ol>
        <InspectorContentEntryCard entry={selected} />
      </ol>,
    );
    expect(view.container.querySelector('p')?.textContent).toBe(text);
    if (text.startsWith('Zusammen'))
      expect(screen.getByText('fassung').getAttribute('data-emphasis')).toBe(
        'selected',
      );
    if (text.startsWith('Unmatched'))
      expect(screen.getByText(/Exact word timing is unavailable/)).toBeTruthy();
  },
);
