import type { LibraryWaveformTiles } from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import {
  prepareDayRedactionResize,
  readDayWaveformTiles,
  sourceRedaction,
} from './day-studio-actions';
import { buildDayTimeline } from './day-timeline';
import { dayTimelineFixture } from './day-timeline-fixtures';

const native = vi.hoisted(() => ({ getRecordingWaveformTiles: vi.fn() }));
vi.mock('../library-client', () => native);
beforeEach(() => vi.resetAllMocks());
function redacted() {
  const fixture = dayTimelineFixture();
  fixture.recordings[0]!.redactions = [
    {
      id: 'cut',
      recordingId: 'a',
      startOffsetMs: 22000,
      endOffsetMs: 23000,
      trackIds: ['a-screen-0'],
      createdAtMs: 1,
      reason: null,
    },
  ];
  return buildDayTimeline(fixture);
}
test('resizing day masks retains native identity, source offsets and expected persisted bounds', () => {
  const model = redacted();
  expect(
    prepareDayRedactionResize(model, {
      recordingId: model.view.id,
      redactionId: 'cut:a-segment',
      startOffsetMs: 1200,
      endOffsetMs: 2500,
      expectedStartOffsetMs: 1000,
      expectedEndOffsetMs: 2000,
    }),
  ).toEqual({
    recordingId: 'a',
    redactionId: 'cut',
    startOffsetMs: 22200,
    endOffsetMs: 23500,
    expectedStartOffsetMs: 22000,
    expectedEndOffsetMs: 23000,
  });
  expect(sourceRedaction(model, 'cut:a-segment').recording.id).toBe('a');
  expect(() => sourceRedaction(model, 'absent')).toThrow();
  model.spans = [];
  expect(() =>
    prepareDayRedactionResize(model, {
      recordingId: 'a',
      redactionId: 'cut:a-segment',
      startOffsetMs: 1000,
      endOffsetMs: 2000,
      expectedStartOffsetMs: 1000,
      expectedEndOffsetMs: 2000,
    }),
  ).toThrow();
});
test('clipped masks keep undisplayed source coverage and reject moving an artificial clip boundary', () => {
  const fixture = dayTimelineFixture();
  fixture.recordings[0]!.redactions = [
    {
      id: 'cut',
      recordingId: 'a',
      startOffsetMs: 20000,
      endOffsetMs: 28000,
      trackIds: ['a-screen-0'],
      createdAtMs: 1,
      reason: null,
    },
  ];
  const model = buildDayTimeline(fixture),
    input = {
      recordingId: 'a',
      redactionId: 'cut:a-segment',
      startOffsetMs: 0,
      endOffsetMs: 5000,
      expectedStartOffsetMs: 0,
      expectedEndOffsetMs: 5000,
    };
  expect(prepareDayRedactionResize(model, input)).toMatchObject({
    startOffsetMs: 20000,
    endOffsetMs: 28000,
  });
  expect(() =>
    prepareDayRedactionResize(model, { ...input, startOffsetMs: 1 }),
  ).toThrow();
  expect(() =>
    prepareDayRedactionResize(model, { ...input, endOffsetMs: 4999 }),
  ).toThrow();
  expect(() =>
    prepareDayRedactionResize(model, { ...input, startOffsetMs: -1 }),
  ).toThrow();
  expect(() =>
    prepareDayRedactionResize(model, { ...input, endOffsetMs: 5001 }),
  ).toThrow();
});
test('waveform reads request only intersecting original media and project tiles into the day', async () => {
  const model = buildDayTimeline(dayTimelineFixture());
  native.getRecordingWaveformTiles.mockImplementation(
    async (input: {
      recordingId: string;
      startTimelineMs: number;
    }): Promise<LibraryWaveformTiles> => ({
      recordingId: input.recordingId,
      binMs: 10,
      error: input.recordingId === 'b' ? 'Synthetic waveform failure' : null,
      tracks: [
        {
          trackId: 'desktop-audio-1',
          kind: 'desktop-audio',
          scaleAmplitude: 1,
          tiles: [
            {
              segmentId: `${input.recordingId}-segment`,
              streamIndex: 2,
              binMs: 10,
              durationMs: 1000,
              generatedAtMs: 1,
              timelineStartMs: input.startTimelineMs,
              peakSamples: [0.1],
              rmsSamples: [0.05],
            },
          ],
        },
        {
          trackId: 'microphone-2',
          kind: 'microphone',
          scaleAmplitude: 2,
          tiles: [],
        },
      ],
    }),
  );
  const result = await readDayWaveformTiles(model, {
    recordingId: model.view.id,
    startTimelineMs: 4000,
    endTimelineMs: 12000,
    viewportWidthPx: 800,
  });
  expect(
    native.getRecordingWaveformTiles.mock.calls.map(([input]) => [
      input.recordingId,
      input.startTimelineMs,
      input.endTimelineMs,
      input.viewportWidthPx,
    ]),
  ).toEqual([
    ['a', 25000, 26000, 100],
    ['b', 0, 2000, 200],
  ]);
  expect(result.recordingId).toBe(model.view.id);
  expect(result.tracks[0]?.tiles.map((t) => t.timelineStartMs)).toEqual([
    4000, 10000,
  ]);
  expect(result.error).toBe('Synthetic waveform failure');
  native.getRecordingWaveformTiles.mockClear();
  expect(
    (
      await readDayWaveformTiles(model, {
        recordingId: model.view.id,
        startTimelineMs: 6000,
        endTimelineMs: 7000,
        viewportWidthPx: 100,
      })
    ).tracks,
  ).toEqual([]);
  expect(native.getRecordingWaveformTiles).not.toHaveBeenCalled();
});
