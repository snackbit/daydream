import type { LibraryRecordingDetail } from '@daydream/shared';
import { afterEach, expect, test } from 'vitest';
import { studioClipId, useStudioTimelineStore } from './studio-timeline-store';

const detail = {
  availability: 'available',
  durationMs: 4_000,
  endedAtMs: 5_000,
  id: 'recording-1',
  missingReasons: [],
  playableSegmentCount: 2,
  redactions: [],
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

const activityLaneIds = [
  'activity:mouse',
  'activity:keyboard',
  'activity:focus',
];

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
    ...activityLaneIds,
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
    redactionIds: [],
    ranges: [],
    visualFrameIds: [],
  });
});

test('studio timeline store keeps additive editor selection within one category', () => {
  useStudioTimelineStore.getState().initializeFromDetail(detail);
  useStudioTimelineStore.getState().selectLane('video-0');
  useStudioTimelineStore.getState().selectLane('desktop-audio-1', true);

  expect(useStudioTimelineStore.getState().selection).toEqual({
    clipIds: [],
    laneIds: ['video-0', 'desktop-audio-1'],
    redactionIds: [],
    ranges: [],
    visualFrameIds: [],
  });

  const desktopClipId = studioClipId('desktop-audio-1', 'segment-1');
  const videoClipId = studioClipId('video-0', 'segment-1');
  useStudioTimelineStore.getState().selectClip(desktopClipId, true);
  useStudioTimelineStore.getState().selectClip(videoClipId, true);

  expect(useStudioTimelineStore.getState().selection).toEqual({
    clipIds: [desktopClipId, videoClipId],
    laneIds: [],
    redactionIds: [],
    ranges: [],
    visualFrameIds: [],
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
    redactionIds: [],
    ranges: [
      {
        endMs: 2_000,
        id: 'desktop-audio-1:500-2000',
        laneIds: ['desktop-audio-1'],
        startMs: 500,
      },
    ],
    visualFrameIds: [],
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
    ...activityLaneIds,
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
    redactionIds: [],
    ranges: [],
    visualFrameIds: [],
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
    ...activityLaneIds,
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
