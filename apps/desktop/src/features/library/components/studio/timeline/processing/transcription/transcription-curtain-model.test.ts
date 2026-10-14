import { expect, test } from 'vitest';
import { chunk, detail, microphone, state } from '../processing-test-fixtures';
import { transcriptionCurtain } from './transcription-curtain-model';

test('published time controls the reveal rather than fractional chunk progress', () => {
  const processing = {
    ...state([chunk({ status: 'running', progress: 0.9 })]),
    transcriptionProgress: [
      {
        jobId: 'job',
        trackId: 'mic',
        sourceRole: 'microphone' as const,
        status: 'running' as const,
        ranges: [{ startOffsetMs: 0, endOffsetMs: 5000 }],
        publishedRanges: [{ startOffsetMs: 0, endOffsetMs: 1000 }],
      },
    ],
  };
  expect(
    transcriptionCurtain({ detail, track: microphone, state: processing }),
  ).toMatchObject({
    revealOffsetMs: 1000,
    progress: 0.2,
    pendingRanges: [{ startOffsetMs: 1000, endOffsetMs: 5000 }],
  });
});
test('later publication cannot uncover a failed earlier interval', () => {
  const processing = {
    ...state([chunk()]),
    transcriptionProgress: [
      {
        jobId: 'job',
        trackId: 'mic',
        sourceRole: 'microphone' as const,
        status: 'failed' as const,
        ranges: [{ startOffsetMs: 0, endOffsetMs: 5000 }],
        publishedRanges: [{ startOffsetMs: 3000, endOffsetMs: 5000 }],
      },
    ],
  };
  expect(
    transcriptionCurtain({ detail, track: microphone, state: processing }),
  ).toMatchObject({ status: 'failed', revealOffsetMs: 0, progress: 0 });
});
test('contiguous chunks merge while real capture gaps remain excluded', () => {
  const curtain = transcriptionCurtain({
    detail,
    track: microphone,
    state: state([
      chunk({ endOffsetMs: 2500 }),
      chunk({ id: 'next', startOffsetMs: 2500 }),
      chunk({ id: 'later', startOffsetMs: 7000, endOffsetMs: 12000 }),
    ]),
  });
  expect(curtain?.ranges).toEqual([
    { startOffsetMs: 0, endOffsetMs: 5000 },
    { startOffsetMs: 7000, endOffsetMs: 12000 },
  ]);
});
test('new admission hides the requested region despite old job completion', () => {
  const curtain = transcriptionCurtain({
    detail,
    track: microphone,
    state: state([chunk({ status: 'succeeded' })]),
    plannedScopes: [
      {
        id: 'scope',
        trackId: 'mic',
        sourceLabel: 'Microphone',
        operation: 'transcription',
        startOffsetMs: 1000,
        endOffsetMs: 4000,
      },
    ],
  });
  expect(curtain).toMatchObject({
    status: 'planned',
    revealOffsetMs: 1000,
    pendingRanges: [{ startOffsetMs: 1000, endOffsetMs: 4000 }],
  });
});
