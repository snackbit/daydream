import { expect, test } from 'vitest';
import { projectDayResults, type DayRecordingResults } from './day-results';
import { buildDayTimeline } from './day-timeline';
import { dayTimelineFixture } from './day-timeline-fixtures';

test('detail handoffs distinguish pending and failed recording spans from current reads', () => {
  const model = buildDayTimeline(dayTimelineFixture());
  const range = { startOffsetMs: 0, endOffsetMs: 18_000 };
  const results = new Map<string, DayRecordingResults>([
    ['a', { ai: null, activity: null, frames: [], request: range }],
    [
      'b',
      {
        ai: null,
        activity: null,
        frames: [],
        request: { ...range, bucketMs: 5000 },
      },
    ],
  ]);
  const pending = projectDayResults(model, results, range);
  expect(pending.pendingRanges).toEqual([
    { startOffsetMs: 10_000, endOffsetMs: 18_000 },
  ]);
  expect(pending.failedRanges).toEqual([]);
  const failed = projectDayResults(model, results, range, new Set(['b']));
  expect(failed.pendingRanges).toEqual([]);
  expect(failed.failedRanges).toEqual(pending.pendingRanges);
  results.set('b', { ai: null, activity: null, frames: [], request: range });
  expect(projectDayResults(model, results, range).pendingRanges).toEqual([]);
});

test('day overview counts preserve source provenance and omit support outside playable spans', () => {
  const model = buildDayTimeline(dayTimelineFixture());
  const result = projectDayResults(
    model,
    new Map([
      [
        'a',
        {
          ai: {
            recordingId: 'a',
            job: null,
            chunks: [],
            transcript: { recordingId: 'a', tracks: [] },
            timelineOverview: [
              {
                trackId: 'a-screen-0',
                kind: 'observation' as const,
                startOffsetMs: 21_000,
                endOffsetMs: 23_000,
                count: 4,
              },
              {
                trackId: 'a-screen-0',
                kind: 'observation' as const,
                startOffsetMs: 30_000,
                endOffsetMs: 31_000,
                count: 9,
              },
            ],
          },
          activity: null,
          frames: [],
        },
      ],
    ]),
  );
  expect(result.ai.timelineOverview).toEqual([
    {
      trackId: 'day-video-source-monitor-0',
      kind: 'observation',
      startOffsetMs: 0,
      endOffsetMs: 2000,
      count: 4,
    },
  ]);
});

test('published speech, input activity and visual navigation retain provenance while moving to the day clock', () => {
  const model = buildDayTimeline(dayTimelineFixture());
  const segment = {
    id: 'speech',
    segmentId: 'a-segment',
    confidence: null,
    provider: 'synthetic',
    model: null,
    startOffsetMs: 20000,
    endOffsetMs: 22000,
    text: 'Synthetic speech',
    words: [
      {
        id: 'word',
        startOffsetMs: 20000,
        endOffsetMs: 22000,
        text: 'Synthetic',
        probability: null,
      },
    ],
  };
  const value: DayRecordingResults = {
    ai: {
      recordingId: 'a',
      job: null,
      chunks: [
        {
          id: 'chunk',
          jobId: 'job',
          recordingId: 'a',
          createdAtMs: 1,
          updatedAtMs: 1,
          attemptCount: 1,
          startOffsetMs: 20000,
          endOffsetMs: 22000,
          errorCode: null,
          resultCode: null,
          progress: 1,
          sourceKind: 'audio',
          trackId: 'microphone-2',
          segmentId: 'a-segment',
          status: 'succeeded',
        },
      ],
      transcript: {
        recordingId: 'a',
        tracks: [
          {
            trackId: 'microphone-2',
            sourceRole: 'microphone',
            segments: [segment],
          },
        ],
      },
      transcriptionTracks: [
        {
          trackKey: 'microphone',
          trackId: 'microphone-2',
          libraryTrackId: 'microphone-2',
          sourceRole: 'microphone',
          status: 'complete',
          statusLabel: 'Complete',
          description: '',
          label: 'Microphone',
          textSegmentCount: 1,
          chunks: [
            {
              id: 'chunk',
              attemptCount: 1,
              errorCode: null,
              resultCode: null,
              progress: 1,
              result: 'text',
              status: 'succeeded',
              segmentId: 'a-segment',
              startOffsetMs: 20000,
              endOffsetMs: 22000,
              text: 'Synthetic speech',
              transcriptSegments: [segment],
            },
          ],
        },
      ],
      transcriptionProgress: [
        {
          jobId: 'job',
          trackId: 'microphone-2',
          sourceRole: 'microphone',
          status: 'succeeded',
          ranges: [{ startOffsetMs: 20000, endOffsetMs: 22000 }],
          publishedRanges: [{ startOffsetMs: 20000, endOffsetMs: 22000 }],
        },
      ],
      visualFrames: [],
    },
    activity: {
      recordingId: 'a',
      intervals: [
        {
          id: 'input',
          jobId: 'job',
          recordingId: 'a',
          score: 1,
          sourceKind: 'mouse',
          startOffsetMs: 21500,
          endOffsetMs: 22500,
          state: 'active',
        },
      ],
      events: [],
      captureStates: [],
      windowSnapshots: [],
    },
    frames: [
      {
        id: 'frame',
        frameTimestampMs: 21500,
        endTimestampMs: 22500,
        sourceTrackId: 'a-screen-1',
        ocrRegions: [],
        vlmObservations: [],
        sourceFrameRef: 'synthetic',
        sourceFrameRefs: ['synthetic'],
        ocrSampleEndMs: 22000,
      },
    ],
  };
  const result = projectDayResults(model, new Map([['a', value]]));
  expect(result.ai.transcript.tracks[0]?.segments[0]).toMatchObject({
    startOffsetMs: 0,
    endOffsetMs: 1000,
    segmentId: 'a-segment',
  });
  expect(result.ai.transcript.tracks[0]?.segments[0]?.words[0]).toMatchObject({
    startOffsetMs: 0,
    endOffsetMs: 1000,
  });
  expect(result.activity.intervals[0]).toMatchObject({
    recordingId: 'a',
    startOffsetMs: 500,
    endOffsetMs: 1500,
  });
  expect(result.frames[0]).toMatchObject({
    id: 'frame',
    frameTimestampMs: 500,
    endTimestampMs: 1500,
    sourceTrackId: 'day-video-source-monitor-1',
    ocrSampleEndMs: 1000,
  });
  expect(result.ai.transcriptionProgress?.[0]?.publishedRanges).toEqual([
    { startOffsetMs: 0, endOffsetMs: 1000 },
  ]);
  expect(projectDayResults(model, new Map()).frames).toEqual([]);
  const ai = value.ai,
    activity = value.activity;
  if (!ai || !activity) throw new Error('Synthetic read is missing');
  for (const [index, time] of [20000, 21500].entries()) {
    activity.events.push({
      id: `event-${index}`,
      recordingId: 'a',
      kind: 'focused-window',
      occurredAtMs: 1,
      offsetMs: time,
      applicationName: 'Synthetic app',
      title: 'Synthetic window',
    });
    activity.captureStates.push({
      id: `capture-${index}`,
      recordingId: 'a',
      offsetMs: time,
      reason: null,
      source: null,
      status: null,
    });
    activity.windowSnapshots.push({
      id: `window-${index}`,
      recordingId: 'a',
      offsetMs: time,
      occurredAtMs: 1,
      provider: null,
      focusedWindowId: null,
      windows: [],
    });
    ai.visualFrames?.push({
      id: `frame-${index}`,
      recordingId: 'a',
      jobId: 'job',
      chunkId: 'chunk',
      createdAtMs: 1,
      updatedAtMs: 1,
      frameTimestampMs: time,
      errorCode: null,
      outputCount: 0,
      progress: 0,
      region: { x: 0, y: 0, width: 1, height: 1 },
      segmentId: 'a-segment',
      sourceFrameRef: 'synthetic',
      sourceKind: 'ocr',
      status: 'queued',
    });
  }
  value.frames.push(
    {
      id: 'no-evidence-yet',
      frameTimestampMs: 21600,
      endTimestampMs: 21600,
      sourceFrameRef: 'synthetic',
      sourceFrameRefs: [],
      ocrRegions: [],
      vlmObservations: [],
    },
    {
      id: 'outside-day',
      frameTimestampMs: 20000,
      endTimestampMs: 20100,
      sourceTrackId: 'a-screen-0',
      sourceFrameRef: 'synthetic',
      sourceFrameRefs: [],
      ocrRegions: [],
      vlmObservations: [],
    },
  );
  const expanded = projectDayResults(
    model,
    new Map([
      ['a', value],
      ['b', { ai: null, activity: null, frames: [] }],
    ]),
  );
  expect(expanded.activity.events.map((e) => e.offsetMs)).toEqual([500]);
  expect(expanded.activity.captureStates.map((e) => e.offsetMs)).toEqual([500]);
  expect(expanded.activity.windowSnapshots.map((e) => e.offsetMs)).toEqual([
    500,
  ]);
  expect(expanded.ai.visualFrames.map((f) => f.frameTimestampMs)).toEqual([
    500,
  ]);
  expect(expanded.frames.map((f) => f.frameTimestampMs)).toEqual([500, 600]);
  const chunk = ai.chunks[0],
    transcription = ai.transcriptionTracks?.[0];
  if (!chunk || !transcription)
    throw new Error('Synthetic processing state missing');
  ai.chunks = [
    { ...chunk, trackId: null, startOffsetMs: null, endOffsetMs: null },
  ];
  transcription.libraryTrackId = null;
  transcription.chunks[0]!.startOffsetMs = null;
  transcription.chunks[0]!.endOffsetMs = null;
  delete ai.transcriptionProgress;
  delete ai.visualFrames;
  const incomplete = projectDayResults(model, new Map([['a', value]]));
  expect(incomplete.ai.chunks).toEqual([]);
  expect(incomplete.ai.transcriptionTracks[0]?.libraryTrackId).toBeNull();
});
