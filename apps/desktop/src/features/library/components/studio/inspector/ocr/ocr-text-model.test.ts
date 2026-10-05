import type { OcrDocument } from '@daydream/shared';
import { expect, test } from 'vitest';
import {
  normalizedOcrText,
  ocrMatchCount,
  ocrRevisionDiff,
  ocrTextPieces,
} from './ocr-text-model';

const document: OcrDocument = {
  blocks: [],
  cacheHit: false,
  configFingerprint: 'config',
  contentId: null,
  engine: 'vision',
  errorCode: null,
  frameTimestampMs: 500,
  height: 1080,
  id: 'doc',
  model: 'synthetic-model',
  observedEndMs: 500,
  observedStartMs: 500,
  occurrenceCount: 1,
  provenance: 'native',
  provider: 'openai-compatible',
  recordingId: 'recording-1',
  segmentId: 'segment-1',
  sourceFrameRef: 'opaque-ref',
  sourceTrackId: 'video-0',
  status: 'succeeded',
  supportTimestampsMs: [500],
  text: '',
  width: 1920,
};
test('literal revision comparison collapses only whitespace and preserves changed amounts, names and case', () => {
  expect(normalizedOcrText('Invoice\n  17.50\tAlice')).toBe(
    'Invoice 17.50 Alice',
  );
  expect(
    ocrRevisionDiff(
      { ...document, text: 'Invoice 17.50 Alice' },
      { ...document, text: 'Invoice\n18.50 Alice' },
    ),
  ).toEqual({ added: '8', removed: '7' });
  expect(
    ocrRevisionDiff(
      { ...document, text: 'Alice' },
      { ...document, text: 'alice' },
    ),
  ).toEqual({ added: 'a', removed: 'A' });
  expect(
    ocrRevisionDiff(
      { ...document, text: 'Done.' },
      { ...document, text: 'Done!' },
    ),
  ).toEqual({ added: '!', removed: '.' });
});
test('search treats regex symbols literally and preserves original Unicode text while highlighting matches', () => {
  const text = 'İstanbul [Invoice] €18.50 and [Invoice]';
  const pieces = ocrTextPieces({ ...document, text }, '[invoice]');
  expect(pieces.map((piece) => piece.text).join('')).toBe(text);
  expect(
    pieces.filter((piece) => piece.match).map((piece) => piece.text),
  ).toEqual(['[Invoice]', '[Invoice]']);
  expect(ocrMatchCount(text, '[Invoice]')).toBe(2);
  expect(ocrMatchCount(text, '.*')).toBe(0);
});
