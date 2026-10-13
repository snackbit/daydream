import { describe, expect, test } from 'vitest';
import { segmentForTimeline } from '../library-timeline';
import {
  buildDayTimeline,
  dayPlaybackAssets,
  dayPosition,
  sourcePosition,
  splitDayRanges,
  projectSourceRange,
} from './day-timeline';
import { decodeDayTimeline } from './day-timeline-decoder';
import { dayTimelineFixture } from './day-timeline-fixtures';

describe('Day playback provenance', () => {
  test('indexed source ranges keep gaps and clipping while navigating a dense eight-hour recording', () => {
    const fixture = dayTimelineFixture();
    const recording = fixture.recordings[0]!;
    const base = fixture.startMs;
    const segments = Array.from({ length: 960 }, (_, index) => ({
      ...recording.segments[0]!,
      id: `fragment-${index}`,
      startedAtMs: base + index * 30_000,
      endedAtMs: base + (index + 1) * 30_000,
      durationMs: 30_000,
      playbackOffsetMs: index * 30_000,
      segmentIndex: index + 1,
      pauseAfterMs: 0,
    }));
    fixture.recordings = [{ ...recording, segments }];
    fixture.spans = segments.map((segment, index) => ({
      recordingId: recording.id,
      segmentId: segment.id,
      startMs: segment.startedAtMs,
      durationMs: 30_000,
      sourceStartMs: 0,
      sourceOffsetMs: index * 30_000,
    }));
    fixture.spans.splice(500, 1);
    const model = buildDayTimeline(fixture);
    expect(projectSourceRange(model, recording.id, 29_000, 31_000)).toEqual([
      { start: 29_000, end: 31_000, shift: 0 },
    ]);
    expect(dayPosition(model, recording.id, 15_000_500)).toBeNull();
    expect(dayPosition(model, recording.id, 15_030_500)).toBe(15_030_500);
    expect(
      projectSourceRange(model, recording.id, 14_999_000, 15_031_000),
    ).toEqual([
      { start: 14_999_000, end: 15_000_000, shift: 0 },
      { start: 15_030_000, end: 15_031_000, shift: 0 },
    ]);
    expect(projectSourceRange(model, 'missing', 0, 1000)).toEqual([]);
    expect(projectSourceRange(model, recording.id, 1000, 1000)).toEqual([]);
  });
  test('monitor identity survives source reordering across recordings and exact clip ends seek forward', () => {
    const fixture = dayTimelineFixture();
    const next = fixture.recordings[1];
    if (!next) throw new Error('Synthetic record missing');
    next.tracks = [next.tracks[1]!, next.tracks[0]!, ...next.tracks.slice(2)];
    const model = buildDayTimeline(fixture);
    expect(model.trackIds.get('a')?.get('a-screen-0')).toBe(
      model.trackIds.get('b')?.get('b-screen-0'),
    );
    expect(model.trackIds.get('a')?.get('a-screen-1')).toBe(
      model.trackIds.get('b')?.get('b-screen-1'),
    );
    const remaining = buildDayTimeline({
      ...fixture,
      recordings: [next],
      spans: fixture.spans.filter((span) => span.recordingId === 'b'),
    });
    expect(remaining.trackIds.get('b')?.get('b-screen-0')).toBe(
      model.trackIds.get('a')?.get('a-screen-0'),
    );
    expect(segmentForTimeline(model.view.segments, 5000)).toEqual({
      index: 1,
      offsetMs: 0,
    });
    expect(segmentForTimeline(model.view.segments, 18000)).toEqual({
      index: 1,
      offsetMs: 8000,
    });
  });
  test('day thumbnails exclude source times outside the day slice and source-specific redactions', () => {
    const fixture = dayTimelineFixture();
    fixture.recordings[0]!.redactions = [
      {
        id: 'private',
        recordingId: 'a',
        startOffsetMs: 22000,
        endOffsetMs: 23000,
        trackIds: ['a-screen-0'],
        reason: null,
        createdAtMs: 1,
      },
    ];
    const model = buildDayTimeline(fixture),
      thumbnails = [0, 2000, 5000].map((offsetMs) => ({
        offsetMs,
        url: `http://localhost/synthetic-${offsetMs}`,
      }));
    const source = {
      recordingId: 'a',
      sessionId: 'session-a',
      error: null,
      segments: [
        {
          segmentId: 'a-segment',
          videoUrl: 'http://localhost/synthetic',
          thumbnails,
          audio: [],
          videos: [
            {
              trackId: 'a-screen-0',
              streamIndex: 0,
              url: 'http://localhost/zero',
              thumbnails,
            },
            {
              trackId: 'a-screen-1',
              streamIndex: 1,
              url: 'http://localhost/one',
              thumbnails,
            },
          ],
        },
      ],
    };
    const view = dayPlaybackAssets(model, new Map([['a', source]]));
    expect(view.segments[0]?.thumbnails.map((t) => t.offsetMs)).toEqual([4000]);
    expect(
      view.segments[0]?.videos?.[0]?.thumbnails.map((t) => t.offsetMs),
    ).toEqual([4000]);
    expect(
      view.segments[0]?.videos?.[1]?.thumbnails.map((t) => t.offsetMs),
    ).toEqual([1000, 4000]);
  });
  test('input activity redactions retain native lane targets across recordings', () => {
    const model = buildDayTimeline(dayTimelineFixture());
    expect([
      ...splitDayRanges(model, [
        {
          startOffsetMs: 4000,
          endOffsetMs: 12000,
          trackIds: ['activity:keyboard'],
        },
      ]),
    ]).toEqual([
      [
        'a',
        [
          {
            startOffsetMs: 25000,
            endOffsetMs: 26000,
            trackIds: ['activity:keyboard'],
          },
        ],
      ],
      [
        'b',
        [
          {
            startOffsetMs: 0,
            endOffsetMs: 2000,
            trackIds: ['activity:keyboard'],
          },
        ],
      ],
    ]);
  });
  test('keeps real gaps and translates a clipped day position back to its original recording', () => {
    const model = buildDayTimeline(dayTimelineFixture());
    expect(
      model.view.segments.map((s) => [
        s.id,
        s.playbackOffsetMs,
        s.durationMs,
        s.pauseAfterMs,
        s.mediaStartMs,
      ]),
    ).toEqual([
      ['a-segment', 0, 5000, 5000, 1000],
      ['b-segment', 10000, 8000, 0, 0],
    ]);
    expect(dayPosition(model, 'a', 22500)).toBe(1500);
    expect(sourcePosition(model, 1500)?.offsetMs).toBe(22500);
    expect(dayPosition(model, 'b', 1500)).toBe(11500);
    expect(sourcePosition(model, 7500)).toBeNull();
    expect(dayPosition(model, 'a', 100)).toBeNull();
  });
  test('splits a range spanning recordings and a gap into source-local mutations', () => {
    const model = buildDayTimeline(dayTimelineFixture());
    expect([
      ...splitDayRanges(model, [
        {
          startOffsetMs: 4000,
          endOffsetMs: 12000,
          trackIds: ['day-video-source-monitor-1', 'microphone-2'],
        },
      ]),
    ]).toEqual([
      [
        'a',
        [
          {
            startOffsetMs: 25000,
            endOffsetMs: 26000,
            trackIds: ['a-screen-1', 'microphone-2'],
          },
        ],
      ],
      [
        'b',
        [
          {
            startOffsetMs: 0,
            endOffsetMs: 2000,
            trackIds: ['b-screen-1', 'microphone-2'],
          },
        ],
      ],
    ]);
    expect(
      splitDayRanges(model, [
        {
          startOffsetMs: 6000,
          endOffsetMs: 7000,
          trackIds: ['day-video-source-monitor-0'],
        },
      ]).size,
    ).toBe(0);
    expect(
      splitDayRanges(model, [
        { startOffsetMs: 0, endOffsetMs: 1000, trackIds: ['mouse'] },
      ]).size,
    ).toBe(0);
  });
  test('projects source redactions without changing their recording identity', () => {
    const manifest = dayTimelineFixture();
    manifest.recordings[0]!.redactions = [
      {
        id: 'cut',
        recordingId: 'a',
        startOffsetMs: 22000,
        endOffsetMs: 28000,
        trackIds: ['a-screen-1'],
        reason: null,
        createdAtMs: 1,
      },
    ];
    const model = buildDayTimeline(manifest);
    expect(model.view.redactions).toEqual([
      {
        id: 'cut:a-segment',
        recordingId: 'a',
        startOffsetMs: 1000,
        endOffsetMs: 5000,
        trackIds: ['day-video-source-monitor-1'],
        reason: null,
        createdAtMs: 1,
      },
    ]);
    expect(manifest.recordings[0]!.redactions[0]!.startOffsetMs).toBe(22000);
  });
  test('uses a deterministic owner for overlapping captures and keeps trimmed source offsets', () => {
    const manifest = dayTimelineFixture();
    manifest.spans[1]!.startMs = 104000;
    const model = buildDayTimeline(manifest);
    expect(model.spans[1]).toMatchObject({
      startMs: 105000,
      durationMs: 7000,
      sourceOffsetMs: 1000,
      sourceStartMs: 1000,
    });
    manifest.spans[1]!.durationMs = 500;
    expect(buildDayTimeline(manifest).spans).toHaveLength(1);
  });
  test('handles an empty day and legacy video metadata', () => {
    const manifest = dayTimelineFixture();
    manifest.recordings[0]!.segments[0]!.videoStreams = [];
    expect(
      buildDayTimeline(manifest).view.segments[0]!.videoStreams,
    ).toHaveLength(2);
    manifest.spans = [];
    manifest.recordings = [];
    expect(buildDayTimeline(manifest).view.segments).toEqual([]);
  });
  test('adapts session sources without giving media resources a fake persisted recording', () => {
    const model = buildDayTimeline(dayTimelineFixture());
    const assets = dayPlaybackAssets(
      model,
      new Map([
        [
          'a',
          {
            recordingId: 'a',
            sessionId: 'real-session',
            error: null,
            segments: [
              {
                segmentId: 'a-segment',
                videoUrl: 'http://localhost/a',
                thumbnails: [],
                audio: [],
                videos: [
                  {
                    trackId: 'a-screen-1',
                    streamIndex: 1,
                    url: 'http://localhost/b',
                    thumbnails: [],
                  },
                ],
              },
            ],
          },
        ],
      ]),
    );
    expect(assets.segments[0]?.videos?.[0]?.trackId).toBe(
      'day-video-source-monitor-1',
    );
    expect(assets.segments).toHaveLength(1);
  });
});
test('day IPC decoder rejects malformed metadata, unowned spans and invalid calendar slices', () => {
  const fixture = dayTimelineFixture();
  expect(decodeDayTimeline(fixture)).toEqual(fixture);
  for (const value of [
    null,
    [],
    { ...fixture, endMs: 0 },
    { ...fixture, recordings: null },
    { ...fixture, spans: [{ ...fixture.spans[0], recordingId: 'unknown' }] },
    { ...fixture, spans: [{ ...fixture.spans[0], durationMs: 0 }] },
    { ...fixture, spans: [{ ...fixture.spans[0], sourceStartMs: -1 }] },
    { ...fixture, spans: [{ ...fixture.spans[0], startMs: -1 }] },
  ])
    expect(() => decodeDayTimeline(value)).toThrow();
});

test('day IPC validates nested source, segment and redaction contracts rather than trusting JSON types', () => {
  const base = dayTimelineFixture();
  for (const availability of [
    'available',
    'incomplete',
    'locked',
    'missing-files',
  ] as const) {
    const fixture = structuredClone(base);
    fixture.recordings[0]!.availability = availability;
    expect(decodeDayTimeline(fixture).recordings[0]?.availability).toBe(
      availability,
    );
  }
  for (const state of ['recording', 'finalized', 'failed'] as const) {
    const fixture = structuredClone(base);
    fixture.recordings[0]!.segments[0]!.state = state;
    expect(decodeDayTimeline(fixture).recordings[0]?.segments[0]?.state).toBe(
      state,
    );
  }
  const invalidRecords = [
    { availability: 'unknown' },
    { tracks: [{ kind: 'unknown' }] },
    { title: 5 },
    { sizeBytes: 0.5 },
    { segments: [null] },
    { segments: [{ ...base.recordings[0]!.segments[0], state: 'unknown' }] },
    { segments: [{ ...base.recordings[0]!.segments[0], fileExists: 'true' }] },
    {
      redactions: [
        {
          id: 'cut',
          recordingId: 'a',
          createdAtMs: 1,
          startOffsetMs: 21000,
          endOffsetMs: 22000,
          trackIds: [4],
          reason: null,
        },
      ],
    },
  ];
  for (const change of invalidRecords) {
    const fixture = {
      ...base,
      recordings: [{ ...base.recordings[0], ...change }, base.recordings[1]],
    };
    expect(() => decodeDayTimeline(fixture)).toThrow();
  }
  const withCut = structuredClone(base);
  withCut.recordings[0]!.title = 'Synthetic title';
  withCut.recordings[0]!.segments[0]!.captureRunId = 'run';
  withCut.recordings[0]!.redactions = [
    {
      id: 'cut',
      recordingId: 'a',
      createdAtMs: 1,
      startOffsetMs: 21000,
      endOffsetMs: 22000,
      trackIds: ['a-screen-0'],
      reason: 'Synthetic reason',
    },
  ];
  expect(decodeDayTimeline(withCut)).toEqual(withCut);
  for (const spans of [
    [{ ...base.spans[0], sourceOffsetMs: -1 }],
    [{ ...base.spans[0], startMs: base.endMs }],
    [{ ...base.spans[0], segmentId: 'unknown' }],
  ])
    expect(() => decodeDayTimeline({ ...base, spans })).toThrow();
});
