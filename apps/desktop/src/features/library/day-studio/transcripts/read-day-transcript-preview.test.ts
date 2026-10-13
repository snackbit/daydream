import { beforeEach, expect, test, vi } from 'vitest';
import { buildDayTimeline } from '../day-timeline';
import { dayTimelineFixture } from '../day-timeline-fixtures';
import { readDayTranscriptPreview } from './read-day-transcript-preview';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => native);
beforeEach(() => vi.resetAllMocks());

test('an utterance spanning resumed media keeps one preview with clipped source provenance', async () => {
  const fixture = dayTimelineFixture();
  fixture.spans.splice(1, 0, {
    recordingId: 'a',
    segmentId: 'a-segment',
    startMs: 106_000,
    durationMs: 2000,
    sourceStartMs: 6000,
    sourceOffsetMs: 26_000,
  });
  const model = buildDayTimeline(fixture);
  native.invoke.mockImplementation(async (_command, { input }) => ({
    recordingId: input.recordingId,
    sourceRole: input.sourceRole,
    entries:
      input.recordingId === 'a'
        ? [
            {
              id: 'crossing',
              segmentId: 'a-segment',
              startOffsetMs: 25_500,
              endOffsetMs: 26_500,
              text: 'Synthetic crossing utterance',
              truncated: false,
            },
          ]
        : [],
  }));
  const result = await readDayTranscriptPreview(model, {
    recordingId: model.view.id,
    sourceRole: 'microphone',
    startOffsetMs: 4000,
    endOffsetMs: 12_000,
  });
  expect(result.entries).toEqual([
    {
      id: 'a:crossing',
      segmentId: 'a-segment',
      startOffsetMs: 4500,
      endOffsetMs: 5000,
      text: 'Synthetic crossing utterance',
      truncated: false,
    },
  ]);
  expect(native.invoke).toHaveBeenCalledTimes(3);
});
test('day previews read original recording clocks, preserve role, and project excerpt timestamps', async () => {
  const model = buildDayTimeline(dayTimelineFixture());
  native.invoke.mockImplementation(async (_command, { input }) => ({
    recordingId: input.recordingId,
    sourceRole: input.sourceRole,
    entries: [
      {
        id: 'same-id',
        segmentId: `${input.recordingId}-segment`,
        startOffsetMs: input.startOffsetMs,
        endOffsetMs: input.startOffsetMs + 500,
        text: `Synthetic ${input.recordingId}`,
        truncated: false,
      },
    ],
  }));
  const result = await readDayTranscriptPreview(model, {
    recordingId: model.view.id,
    sourceRole: 'desktop-audio',
    startOffsetMs: 0,
    endOffsetMs: 18_000,
  });
  expect(native.invoke).toHaveBeenCalledWith('getRecordingTranscriptPreview', {
    input: {
      recordingId: 'a',
      sourceRole: 'desktop-audio',
      startOffsetMs: 21_000,
      endOffsetMs: 26_000,
    },
  });
  expect(
    result.entries.map((entry) => [
      entry.id,
      entry.startOffsetMs,
      entry.endOffsetMs,
    ]),
  ).toEqual([
    ['a:same-id', 0, 500],
    ['b:same-id', 10_000, 10_500],
  ]);
  expect(result.recordingId).toBe(model.view.id);
});
test('enough excerpts stop source reads; empty ranges and read failures stay explicit', async () => {
  const model = buildDayTimeline(dayTimelineFixture());
  const input = {
    recordingId: model.view.id,
    sourceRole: 'microphone' as const,
    startOffsetMs: 0,
    endOffsetMs: 18_000,
  };
  native.invoke.mockImplementation(async (_command, { input: request }) => ({
    recordingId: request.recordingId,
    sourceRole: request.sourceRole,
    entries: [0, 1, 2].map((i) => ({
      id: `speech-${i}`,
      segmentId: 'a-segment',
      startOffsetMs: 21000 + i * 1000,
      endOffsetMs: 21500 + i * 1000,
      text: 'Synthetic',
      truncated: false,
    })),
  }));
  expect((await readDayTranscriptPreview(model, input)).entries).toHaveLength(
    3,
  );
  expect(native.invoke).toHaveBeenCalledTimes(1);
  expect(
    (
      await readDayTranscriptPreview(model, {
        ...input,
        startOffsetMs: 6000,
        endOffsetMs: 9000,
      })
    ).entries,
  ).toEqual([]);
  native.invoke.mockRejectedValue(new Error('Synthetic preview failure'));
  await expect(readDayTranscriptPreview(model, input)).rejects.toThrow(
    'Synthetic preview failure',
  );
});
