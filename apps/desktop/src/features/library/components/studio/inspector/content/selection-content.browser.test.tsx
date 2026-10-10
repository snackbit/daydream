import type {
  RecordingAiProcessingState,
  RecordingTranscriptSegment,
} from '@daydream/shared';
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { TranscriptionSelectionDetails } from '../../studio-sidebar-transcription';

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
