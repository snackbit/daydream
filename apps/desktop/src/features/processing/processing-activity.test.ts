import type { ProcessingWaitingReason } from '@daydream/shared';
import { expect, test } from 'vitest';
import {
  processingActivityLabel,
  processingLaneLabel,
} from './processing-activity';
import { validateProcessingLocation } from './processing-location';
import { activity, lane } from './testing/processing-browser-fixtures';

test.each([
  'ocr',
  'previous-source',
  'retry',
  'live-input',
  'protection',
  'capacity',
  'paused',
  'unknown',
] satisfies ProcessingWaitingReason[])(
  'queued work explains its %s blocking reason',
  (waitingReason) => {
    expect(
      processingActivityLabel(activity({ status: 'queued', waitingReason })),
    ).toMatch(/Waiting|Paused|Queued/);
  },
);
test.each([
  ['starting', 'Preparing source'],
  ['recognizing-text', 'Recognizing screen text'],
  ['detecting-speech', 'Checking for speech'],
  ['transcribing', 'Transcribing speech'],
  ['observing-screen', 'Describing screen changes'],
  ['waiting-for-model', 'Waiting for the model'],
  ['unknown', 'Processing source'],
])('running work translates its %s phase', (phase, label) => {
  expect(processingActivityLabel(activity({ phase }))).toBe(label);
});
test('worker termination is distinct from a long inference, and queued work has no claimed activity', () => {
  expect(processingActivityLabel(activity({ workerActive: false }))).toBe(
    'Worker stopped · reconciling state',
  );
  expect(
    processingActivityLabel(activity({ status: 'queued', phase: 'unknown' })),
  ).toBe('Queued');
  expect(processingActivityLabel(activity({ phase: 'unknown' }))).toBe(
    'Processing source',
  );
});
test.each([
  'disabled',
  'no-source',
  'protected',
  'no-eligible-input',
  'waiting',
  'available',
] as const)('lane summary handles %s applicability', (applicability) => {
  expect(processingLaneLabel(lane('ocr', { applicability }))).toBeTruthy();
});
test('live completion, pause, unavailable job and known queue reasons remain distinct', () => {
  expect(
    processingLaneLabel(
      lane('ocr', { inputOpen: true, completed: 10, running: 0, queued: 0 }),
    ),
  ).toBe('Caught up · live recording');
  expect(
    processingLaneLabel(
      lane('ocr', {
        inputOpen: true,
        completed: 10,
        running: 0,
        queued: 0,
        status: 'paused',
      }),
    ),
  ).toBe('Paused');
  expect(processingLaneLabel(lane('ocr', { status: null }))).toBe('Not queued');
  expect(
    processingLaneLabel(
      lane('vlm', {
        running: 0,
        activity: [activity({ waitingReason: 'ocr' })],
      }),
    ),
  ).toBe('Waiting for OCR in this section');
});
test('route scope normalizes ids and rejects unusable or oversized input', () => {
  expect(validateProcessingLocation({ recordings: 'b,a,a' })).toEqual({
    recordings: 'a,b',
  });
  expect(validateProcessingLocation({ recordings: 42 })).toEqual({});
  expect(validateProcessingLocation({ recordings: ',' })).toEqual({});
  expect(
    validateProcessingLocation({
      recordings: Array.from({ length: 2001 }, (_, i) => String(i)).join(','),
    }),
  ).toEqual({});
  expect(validateProcessingLocation({ recordings: 'x'.repeat(129) })).toEqual(
    {},
  );
});
