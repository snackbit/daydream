import { describe, expect, it } from 'vitest';
import {
  validateRecallDate,
  validateRecallSearch,
  validateStudioSearch,
} from './recall-location';

describe('Recall route input', () => {
  it('accepts actual ISO calendar dates and rejects impossible days', () => {
    expect(validateRecallDate('2024-02-29')).toBe('2024-02-29');
    for (const date of [
      '2023-02-29',
      '2026-04-31',
      '2026-1-01',
      '../private',
      null,
    ]) {
      expect(() => validateRecallDate(date)).toThrow();
    }
  });

  it('validates source filters and bounded user queries', () => {
    expect(
      validateRecallSearch({
        __TSearchSchemaInput__: 'TSearchSchemaInput',
        q: 'meeting',
        source: 'microphone',
        date: '2026-10-06',
      }),
    ).toEqual({ q: 'meeting', source: 'microphone', date: '2026-10-06' });
    expect(() =>
      validateRecallSearch({
        __TSearchSchemaInput__: 'TSearchSchemaInput',
        q: 'x'.repeat(4097),
      }),
    ).toThrow();
    expect(() =>
      validateRecallSearch({
        __TSearchSchemaInput__: 'TSearchSchemaInput',
        q: 'ü'.repeat(2049),
      }),
    ).toThrow();
    expect(() =>
      validateRecallSearch({
        __TSearchSchemaInput__: 'TSearchSchemaInput',
        source: 'vision',
      }),
    ).toThrow();
  });

  it('preserves existing Studio links and rejects malformed evidence positions', () => {
    expect(
      validateStudioSearch({ __TSearchSchemaInput__: 'TSearchSchemaInput' }),
    ).toEqual({ evidenceId: null, source: null, seek: null });
    for (const seek of [-1, 0.5, Infinity, '100']) {
      expect(() =>
        validateStudioSearch({
          __TSearchSchemaInput__: 'TSearchSchemaInput',
          evidenceId: 'synthetic',
          seek,
        }),
      ).toThrow();
    }
    expect(
      validateStudioSearch({
        __TSearchSchemaInput__: 'TSearchSchemaInput',
        evidenceId: 'synthetic',
        source: 'ocr',
        seek: 100,
      }),
    ).toEqual({ evidenceId: 'synthetic', source: 'ocr', seek: 100 });
  });
});
