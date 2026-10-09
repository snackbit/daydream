import { describe, expect, it } from 'vitest';
import { recorderStatusIsCurrent } from './recorder-status-order';

describe('recorder status ordering', () => {
  it('rejects an old command result after a terminal event', () => {
    expect(
      recorderStatusIsCurrent(
        { statusRevision: 18, updatedAtMs: 100 },
        { statusRevision: 17, updatedAtMs: 100 },
      ),
    ).toBe(false);
    expect(
      recorderStatusIsCurrent(
        { statusRevision: 18, updatedAtMs: 100 },
        { statusRevision: 19, updatedAtMs: 100 },
      ),
    ).toBe(true);
  });
  it('keeps compatibility while rejecting older unversioned snapshots', () => {
    expect(
      recorderStatusIsCurrent({ updatedAtMs: 100 }, { updatedAtMs: 99 }),
    ).toBe(false);
    expect(recorderStatusIsCurrent(null, { updatedAtMs: 99 })).toBe(true);
  });
});
