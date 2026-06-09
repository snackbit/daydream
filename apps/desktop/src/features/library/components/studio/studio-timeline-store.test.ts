import type { LibraryRecordingDetail } from '@daydream/shared';
import { afterEach, expect, test } from 'vitest';
import {
  mergeTimeRangeSelections,
  normalizeTimeRangeSelection,
  studioClipId,
  subtractTimeRangeSelection,
  useStudioTimelineStore,
} from './studio-timeline-store';
import { intersectLaneIds, selectionDistancePx } from './track-timeline-math';

const detail = {
  availability: 'available',
  durationMs: 4_000,
  endedAtMs: 5_000,
  id: 'recording-1',
  missingReasons: [],
  playableSegmentCount: 2,
  segments: [
    {
      desktopAudioStreamIndex: 1,
      durationMs: 2_000,
      endedAtMs: 3_000,
      fileExists: true,
      filePath: '/tmp/segment-1.mkv',
      id: 'segment-1',
      microphoneAudioStreamIndex: null,
      pauseAfterMs: 0,
      playbackOffsetMs: 0,
      segmentIndex: 1,
      startedAtMs: 1_000,
      state: 'finalized',
      videoStreamIndex: 0,
    },
    {
      desktopAudioStreamIndex: 1,
      durationMs: 2_000,
      endedAtMs: 5_000,
      fileExists: true,
      filePath: '/tmp/segment-2.mkv',
      id: 'segment-2',
      microphoneAudioStreamIndex: null,
      pauseAfterMs: 0,
      playbackOffsetMs: 2_000,
      segmentIndex: 2,
      startedAtMs: 3_000,
      state: 'finalized',
      videoStreamIndex: 0,
    },
  ],
  sizeBytes: 100,
  startedAtMs: 1_000,
  status: 'completed',
  storageDir: '/tmp',
  thumbnailPath: null,
  title: null,
  tracks: [
    {
      codec: 'h264',
      description: 'Video',
      id: 'video-0',
      kind: 'video',
      label: 'Video',
      streamIndex: 0,
    },
    {
      codec: 'opus',
      description: 'Desktop audio',
      id: 'desktop-audio-1',
      kind: 'desktop-audio',
      label: 'Desktop audio',
      streamIndex: 1,
    },
  ],
} satisfies LibraryRecordingDetail;

afterEach(() => {
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
});

test('studio timeline store initializes deterministic lane and clip drafts', () => {
  useStudioTimelineStore.getState().initializeFromDetail(detail);

  expect(useStudioTimelineStore.getState().laneOrder).toEqual([
    'video-0',
    'desktop-audio-1',
  ]);
  expect(useStudioTimelineStore.getState().lanesById['video-0']).toMatchObject({
    colorId: 'blue',
    label: 'Video',
  });
  expect(
    useStudioTimelineStore.getState().clipsById[
      studioClipId('desktop-audio-1', 'segment-2')
    ],
  ).toMatchObject({
    title: 'Segment 2',
    trackId: 'desktop-audio-1',
  });
});

test('studio timeline store resets session edits when recording changes', () => {
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  useStudioTimelineStore.getState().renameLane('video-0', 'Display');
  useStudioTimelineStore.getState().selectLane('video-0');

  useStudioTimelineStore.getState().initializeFromDetail({
    ...detail,
    id: 'recording-2',
  });

  expect(useStudioTimelineStore.getState().lanesById['video-0']?.label).toBe(
    'Video',
  );
  expect(useStudioTimelineStore.getState().selection).toEqual({
    clipIds: [],
    laneIds: [],
    ranges: [],
  });
});

test('studio timeline store keeps additive editor selection within one category', () => {
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  useStudioTimelineStore.getState().selectLane('video-0');
  useStudioTimelineStore.getState().selectLane('desktop-audio-1', true);

  expect(useStudioTimelineStore.getState().selection).toEqual({
    clipIds: [],
    laneIds: ['video-0', 'desktop-audio-1'],
    ranges: [],
  });

  const desktopClipId = studioClipId('desktop-audio-1', 'segment-1');
  const videoClipId = studioClipId('video-0', 'segment-1');
  useStudioTimelineStore.getState().selectClip(desktopClipId, true);
  useStudioTimelineStore.getState().selectClip(videoClipId, true);

  expect(useStudioTimelineStore.getState().selection).toEqual({
    clipIds: [desktopClipId, videoClipId],
    laneIds: [],
    ranges: [],
  });

  useStudioTimelineStore.getState().setTimeRangeSelection(
    {
      endMs: 500,
      laneIds: ['desktop-audio-1'],
      startMs: 2_000,
    },
    true,
  );

  expect(useStudioTimelineStore.getState().selection).toEqual({
    clipIds: [],
    laneIds: [],
    ranges: [
      {
        endMs: 2_000,
        id: 'desktop-audio-1:500-2000',
        laneIds: ['desktop-audio-1'],
        startMs: 500,
      },
    ],
  });
});

test('studio timeline store reconciles same-recording refreshes', () => {
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  useStudioTimelineStore.getState().moveLane('desktop-audio-1', 'video-0');
  useStudioTimelineStore.getState().renameLane('desktop-audio-1', 'System');
  useStudioTimelineStore.getState().setLaneColor('desktop-audio-1', 'rose');
  useStudioTimelineStore
    .getState()
    .renameClip(studioClipId('desktop-audio-1', 'segment-1'), 'Intro');
  useStudioTimelineStore.getState().selectLane('desktop-audio-1');

  useStudioTimelineStore.getState().initializeFromDetail({
    ...detail,
    segments: [
      ...detail.segments,
      {
        ...detail.segments[1]!,
        id: 'segment-3',
        playbackOffsetMs: 4_000,
        segmentIndex: 3,
      },
    ],
    tracks: [
      ...detail.tracks,
      {
        codec: 'opus',
        description: 'Microphone',
        id: 'microphone-2',
        kind: 'microphone',
        label: 'Microphone',
        streamIndex: 2,
      },
    ],
  });

  expect(useStudioTimelineStore.getState().laneOrder).toEqual([
    'desktop-audio-1',
    'video-0',
    'microphone-2',
  ]);
  expect(
    useStudioTimelineStore.getState().lanesById['desktop-audio-1'],
  ).toMatchObject({
    colorId: 'rose',
    label: 'System',
  });
  expect(
    useStudioTimelineStore.getState().lanesById['microphone-2'],
  ).toMatchObject({
    colorId: 'cyan',
    label: 'Microphone',
  });
  expect(
    useStudioTimelineStore.getState().clipsById[
      studioClipId('desktop-audio-1', 'segment-1')
    ],
  ).toMatchObject({ title: 'Intro' });
  expect(
    useStudioTimelineStore.getState().clipsById[
      studioClipId('microphone-2', 'segment-3')
    ],
  ).toMatchObject({
    title: 'Segment 3',
    trackId: 'microphone-2',
  });
  expect(useStudioTimelineStore.getState().selection).toEqual({
    clipIds: [],
    laneIds: ['desktop-audio-1'],
    ranges: [],
  });
});

test('timeline range merging extends overlapping same-lane selections', () => {
  expect(
    mergeTimeRangeSelections([
      {
        endMs: 2_000,
        id: 'first',
        laneIds: ['desktop-audio-1', 'video-0'],
        startMs: 1_000,
      },
      {
        endMs: 3_000,
        id: 'second',
        laneIds: ['video-0', 'desktop-audio-1'],
        startMs: 1_500,
      },
    ]),
  ).toEqual([
    {
      endMs: 3_000,
      id: 'desktop-audio-1,video-0:1000-3000',
      laneIds: ['desktop-audio-1', 'video-0'],
      startMs: 1_000,
    },
  ]);
});

test('timeline range merging joins touching same-lane selections', () => {
  expect(
    mergeTimeRangeSelections([
      {
        endMs: 2_000,
        id: 'first',
        laneIds: ['video-0'],
        startMs: 1_000,
      },
      {
        endMs: 3_000,
        id: 'second',
        laneIds: ['video-0'],
        startMs: 2_000,
      },
    ]),
  ).toHaveLength(1);
});

test('timeline range merging keeps overlapping different-lane selections separate', () => {
  expect(
    mergeTimeRangeSelections([
      {
        endMs: 2_000,
        id: 'first',
        laneIds: ['video-0'],
        startMs: 1_000,
      },
      {
        endMs: 2_500,
        id: 'second',
        laneIds: ['desktop-audio-1'],
        startMs: 1_500,
      },
    ]),
  ).toHaveLength(2);
});

test('timeline range subtraction trims an overlapped edge', () => {
  expect(
    subtractTimeRangeSelection(
      [
        {
          endMs: 5_000,
          id: 'range',
          laneIds: ['video-0'],
          startMs: 1_000,
        },
      ],
      {
        endMs: 3_000,
        id: 'cutter',
        laneIds: ['video-0'],
        startMs: 1_000,
      },
    ),
  ).toEqual([
    {
      endMs: 5_000,
      id: 'video-0:3000-5000',
      laneIds: ['video-0'],
      startMs: 3_000,
    },
  ]);
});

test('timeline range subtraction splits a middle overlap', () => {
  expect(
    subtractTimeRangeSelection(
      [
        {
          endMs: 5_000,
          id: 'range',
          laneIds: ['video-0'],
          startMs: 1_000,
        },
      ],
      {
        endMs: 3_000,
        id: 'cutter',
        laneIds: ['video-0'],
        startMs: 2_000,
      },
    ),
  ).toEqual([
    {
      endMs: 2_000,
      id: 'video-0:1000-2000',
      laneIds: ['video-0'],
      startMs: 1_000,
    },
    {
      endMs: 5_000,
      id: 'video-0:3000-5000',
      laneIds: ['video-0'],
      startMs: 3_000,
    },
  ]);
});

test('timeline range subtraction removes a fully covered range', () => {
  expect(
    subtractTimeRangeSelection(
      [
        {
          endMs: 5_000,
          id: 'range',
          laneIds: ['video-0'],
          startMs: 1_000,
        },
      ],
      {
        endMs: 6_000,
        id: 'cutter',
        laneIds: ['video-0'],
        startMs: 0,
      },
    ),
  ).toEqual([]);
});

test('timeline range subtraction only cuts intersected lanes', () => {
  expect(
    subtractTimeRangeSelection(
      [
        {
          endMs: 5_000,
          id: 'range',
          laneIds: ['desktop-audio-1', 'video-0'],
          startMs: 1_000,
        },
      ],
      {
        endMs: 3_000,
        id: 'cutter',
        laneIds: ['video-0'],
        startMs: 2_000,
      },
    ),
  ).toEqual([
    {
      endMs: 5_000,
      id: 'desktop-audio-1:1000-5000',
      laneIds: ['desktop-audio-1'],
      startMs: 1_000,
    },
    {
      endMs: 2_000,
      id: 'video-0:1000-2000',
      laneIds: ['video-0'],
      startMs: 1_000,
    },
    {
      endMs: 5_000,
      id: 'video-0:3000-5000',
      laneIds: ['video-0'],
      startMs: 3_000,
    },
  ]);
});

test('timeline range normalization dedupes and sorts lane ids', () => {
  expect(
    normalizeTimeRangeSelection({
      endMs: 1_000,
      laneIds: ['video-0', 'desktop-audio-1', 'video-0'],
      startMs: 2_000,
    }),
  ).toEqual({
    endMs: 2_000,
    id: 'desktop-audio-1,video-0:1000-2000',
    laneIds: ['desktop-audio-1', 'video-0'],
    startMs: 1_000,
  });
});

test('studio timeline store reorders and edits lanes without changing clips', () => {
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  useStudioTimelineStore.getState().moveLane('desktop-audio-1', 'video-0');
  useStudioTimelineStore.getState().renameLane('desktop-audio-1', 'System');
  useStudioTimelineStore.getState().setLaneColor('desktop-audio-1', 'rose');

  expect(useStudioTimelineStore.getState().laneOrder).toEqual([
    'desktop-audio-1',
    'video-0',
  ]);
  expect(
    useStudioTimelineStore.getState().lanesById['desktop-audio-1'],
  ).toMatchObject({
    colorId: 'rose',
    label: 'System',
  });
  expect(
    useStudioTimelineStore.getState().clipsById[
      studioClipId('desktop-audio-1', 'segment-1')
    ],
  ).toBeTruthy();
});

test('timeline math intersects only lanes touched by a vertical marquee', () => {
  expect(
    intersectLaneIds({
      endY: 170,
      laneIds: ['video-0', 'desktop-audio-1', 'microphone-1'],
      startY: 92,
    }),
  ).toEqual(['video-0', 'desktop-audio-1']);
});

test('timeline math reports pointer drag distance', () => {
  expect(
    selectionDistancePx({
      currentX: 13,
      currentY: 14,
      startX: 10,
      startY: 10,
    }),
  ).toBe(5);
});
