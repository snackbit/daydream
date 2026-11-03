import { beforeEach, expect, test, vi } from 'vitest';
import type { DayRecordingResults } from '../day-results';
import { dayTimelineFixture } from '../day-timeline-fixtures';
import { readDayResultWindow } from './read-day-result-window';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: native.invoke }));
beforeEach(() => {
  vi.resetAllMocks();
  native.invoke.mockImplementation(
    async (command: string, args: { input: { recordingId: string } }) => {
      const recordingId = args.input.recordingId;
      if (command === 'getRecordingAiProcessingState')
        return {
          recordingId,
          job: null,
          chunks: [],
          transcript: { recordingId, tracks: [] },
        };
      if (command === 'getRecordingActivity')
        return {
          recordingId,
          events: [],
          intervals: [],
          captureStates: [],
          windowSnapshots: [],
        };
      if (command === 'getRecordingVisualObservations')
        return { recordingId, ocrRegions: [], vlmObservations: [] };
      if (command === 'getOcrTimelineFrames')
        return { recordingId, frames: [], nextCursor: null };
      throw new Error('Unexpected native read');
    },
  );
});
test.each(['overview', 'closed'])(
  '%s windows never request frame pages outside their ownership',
  async (mode) => {
    const recording = dayTimelineFixture().recordings[0];
    if (!recording) throw new Error('Synthetic recording missing');
    const publish = vi.fn();
    const result = await readDayResultWindow(
      recording,
      {
        recordingId: recording.id,
        startOffsetMs: 21_000,
        endOffsetMs: 26_000,
        ...(mode === 'overview' ? { bucketMs: 5000 } : {}),
      },
      { startOffsetMs: 0, endOffsetMs: 30_000 },
      () => mode !== 'closed',
      publish,
    );
    expect(
      native.invoke.mock.calls.some(
        ([command]) => command === 'getOcrTimelineFrames',
      ),
    ).toBe(false);
    if (mode === 'closed') {
      expect(publish).not.toHaveBeenCalled();
      expect(result.data.ai).toBeNull();
    } else {
      expect(result.failed).toBe(false);
      expect(result.data.ai?.recordingId).toBe(recording.id);
      expect(result.data.pending).toBe(false);
    }
  },
);

test('an unbounded detail request preserves absent bounds in the native frame query', async () => {
  const recording = dayTimelineFixture().recordings[0];
  if (!recording) throw new Error('Synthetic recording missing');
  const result = await readDayResultWindow(
    recording,
    { recordingId: recording.id },
    { startOffsetMs: 0, endOffsetMs: 30_000 },
    () => true,
    vi.fn(),
  );
  expect(native.invoke).toHaveBeenCalledWith('getOcrTimelineFrames', {
    input: {
      recordingId: recording.id,
      cursor: null,
      limit: 250,
      startTimestampMs: null,
      endTimestampMs: null,
    },
  });
  expect(result.failed).toBe(false);
  expect(result.data.pending).toBe(false);
});

test.each(['success', 'failure', 'closed'])(
  'a same-window refresh retains loaded sources until replacement (%s)',
  async (outcome) => {
    const recording = dayTimelineFixture().recordings[0];
    if (!recording) throw new Error('Synthetic recording missing');
    const input = { recordingId: recording.id };
    const request = { startOffsetMs: 0, endOffsetMs: 30_000 };
    const initial = await readDayResultWindow(
      recording,
      input,
      request,
      () => true,
      vi.fn(),
    );
    const previous: DayRecordingResults = {
      ...initial.data,
      frames: [
        {
          id: 'loaded-frame',
          frameTimestampMs: 1000,
          endTimestampMs: 2000,
          sourceFrameRef: 'synthetic',
          sourceFrameRefs: ['synthetic'],
          ocrRegions: [],
          vlmObservations: [],
        },
      ],
    };
    const original = native.invoke.getMockImplementation();
    let resolveSpeech: (value: unknown) => void = () => {
      throw new Error('Speech not started');
    };
    let rejectSpeech: (error: Error) => void = () => {
      throw new Error('Speech not started');
    };
    native.invoke.mockImplementation(
      (command: string, args: { input: { recordingId: string } }) => {
        if (command === 'getRecordingAiProcessingState')
          return new Promise((resolve, reject) => {
            resolveSpeech = resolve;
            rejectSpeech = reject;
          });
        if (
          command === 'getRecordingVisualObservations' &&
          outcome === 'failure'
        )
          return Promise.reject(new Error('Synthetic visual failure'));
        return original?.(command, args);
      },
    );
    let current = true;
    const publish = vi.fn();
    const reading = readDayResultWindow(
      recording,
      input,
      request,
      () => current,
      publish,
      previous,
    );
    await vi.waitFor(() => expect(publish).toHaveBeenCalled());
    expect(publish.mock.calls[0]?.[0].ai).toBe(previous.ai);
    expect(publish.mock.calls[0]?.[0].frames).toBe(previous.frames);
    const published = publish.mock.calls.length;
    if (outcome === 'failure')
      rejectSpeech(new Error('Synthetic speech failure'));
    else {
      if (outcome === 'closed') current = false;
      resolveSpeech({ ...previous.ai, resultsRevision: 'new' });
    }
    const result = await reading;
    if (outcome === 'failure') {
      expect(result.failed).toBe(true);
      expect(result.data.ai).toBe(previous.ai);
      expect(result.data.frames).toBe(previous.frames);
    } else if (outcome === 'closed') {
      expect(publish).toHaveBeenCalledTimes(published);
    } else {
      expect(result.failed).toBe(false);
      expect(result.data.ai?.resultsRevision).toBe('new');
      expect(result.data.frames).toEqual([]);
    }
  },
);

test('a repeated OCR cursor reports failure while keeping successful sources readable', async () => {
  const recording = dayTimelineFixture().recordings[0];
  if (!recording) throw new Error('Synthetic recording missing');
  const original = native.invoke.getMockImplementation();
  native.invoke.mockImplementation(
    (command: string, args: { input: { recordingId: string } }) => {
      if (command === 'getOcrTimelineFrames')
        return Promise.resolve({
          recordingId: recording.id,
          frames: [],
          nextCursor: { id: 'repeated', timestampMs: 1000 },
        });
      return original?.(command, args);
    },
  );
  const result = await readDayResultWindow(
    recording,
    { recordingId: recording.id },
    { startOffsetMs: 0, endOffsetMs: 30_000 },
    () => true,
    vi.fn(),
  );
  expect(result.failed).toBe(true);
  expect(result.data.ai?.recordingId).toBe(recording.id);
  expect(result.data.pending).toBe(false);
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'getOcrTimelineFrames',
    ),
  ).toHaveLength(2);
});

test('a late OCR page cannot publish after the window loses ownership', async () => {
  const recording = dayTimelineFixture().recordings[0];
  if (!recording) throw new Error('Synthetic recording missing');
  let finish: (value: unknown) => void = () => {
    throw new Error('OCR read not started');
  };
  const original = native.invoke.getMockImplementation();
  native.invoke.mockImplementation(
    (command: string, args: { input: { recordingId: string } }) => {
      if (command === 'getOcrTimelineFrames')
        return new Promise((resolve) => {
          finish = resolve;
        });
      return original?.(command, args);
    },
  );
  let current = true;
  const publish = vi.fn();
  const reading = readDayResultWindow(
    recording,
    { recordingId: recording.id },
    { startOffsetMs: 0, endOffsetMs: 30_000 },
    () => current,
    publish,
  );
  await vi.waitFor(() => expect(publish).toHaveBeenCalled());
  const count = publish.mock.calls.length;
  current = false;
  finish({ recordingId: recording.id, frames: [], nextCursor: null });
  await reading;
  expect(publish).toHaveBeenCalledTimes(count);
});
