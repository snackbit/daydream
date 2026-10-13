import { beforeEach, expect, test, vi } from 'vitest';
import {
  decodeTranscriptPreview,
  getRecordingTranscriptPreview,
} from './preview-client';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => native);
const input = {
  recordingId: 'synthetic',
  sourceRole: 'microphone' as const,
  startOffsetMs: 100,
  endOffsetMs: 200,
};
const entry = {
  id: 'utterance',
  segmentId: 'segment',
  startOffsetMs: 100,
  endOffsetMs: 150,
  text: 'Synthetic excerpt',
  truncated: false,
};
const response = {
  recordingId: input.recordingId,
  sourceRole: input.sourceRole,
  entries: [entry],
};
beforeEach(() => vi.resetAllMocks());
test('preview IPC validates provenance and bounded excerpts', async () => {
  native.invoke.mockResolvedValue(response);
  expect(await getRecordingTranscriptPreview(input)).toEqual(response);
  expect(native.invoke).toHaveBeenCalledWith('getRecordingTranscriptPreview', {
    input,
  });
  expect(
    decodeTranscriptPreview({ ...response, entries: [] }, input).entries,
  ).toEqual([]);
  expect(
    decodeTranscriptPreview(
      {
        ...response,
        entries: [{ ...entry, text: '🦊'.repeat(240), truncated: true }],
      },
      input,
    ).entries[0]?.truncated,
  ).toBe(true);
  native.invoke.mockRejectedValue(new Error('Synthetic boundary failure'));
  await expect(getRecordingTranscriptPreview(input)).rejects.toThrow(
    'Synthetic boundary failure',
  );
});
test.each([
  null,
  {},
  { ...response, recordingId: 'other' },
  { ...response, sourceRole: 'desktop-audio' },
  { ...response, entries: [entry, entry] },
  { ...response, entries: [entry, entry, entry, entry] },
  ...[
    null,
    { ...entry, id: '' },
    { ...entry, segmentId: '' },
    { ...entry, startOffsetMs: -1 },
    { ...entry, startOffsetMs: 0.5 },
    { ...entry, endOffsetMs: 100 },
    { ...entry, endOffsetMs: 99 },
    { ...entry, startOffsetMs: 200, endOffsetMs: 300 },
    { ...entry, endOffsetMs: NaN },
    { ...entry, text: 'x'.repeat(241) },
    { ...entry, truncated: null },
    { ...entry, text: 1 },
  ].map((invalid) => ({ ...response, entries: [invalid] })),
  {
    ...response,
    entries: [{ ...entry, id: 'later', startOffsetMs: 140 }, entry],
  },
])('rejects malformed or mismatched preview %#', (invalid) =>
  expect(() => decodeTranscriptPreview(invalid, input)).toThrow(
    'Invalid transcript preview.',
  ),
);
