import type { LibraryRecordingDetail, RecallEvidence } from '@daydream/shared';
import { expect, test } from 'vitest';
import { studioRecallEvidenceAvailable } from './studio-recall-navigation';

const evidence: RecallEvidence = {
  evidenceId: 'evidence',
  recordingId: 'recording',
  segmentId: 'segment',
  sourceKind: 'ocr',
  sourceTrackId: 'screen-a',
  sourceId: 'ocr-source',
  sourceFrameRef: 'synthetic-frame',
  documentId: 'document',
  startOffsetMs: 500,
  endOffsetMs: 1000,
  absoluteStartMs: 1500,
  absoluteEndMs: 2000,
  contentRevision: 'revision',
  text: 'Synthetic text',
};
const detail: LibraryRecordingDetail = {
  id: 'recording',
  availability: 'available',
  durationMs: 2000,
  endedAtMs: 3000,
  missingReasons: [],
  playableSegmentCount: 1,
  redactions: [],
  sizeBytes: 0,
  startedAtMs: 1000,
  status: 'completed',
  storageDir: '/tmp/synthetic',
  thumbnailPath: null,
  title: null,
  tracks: [
    {
      id: 'screen-a',
      kind: 'video',
      label: 'Screen A',
      description: '',
      codec: null,
      streamIndex: 0,
    },
  ],
  segments: [
    {
      id: 'segment',
      desktopAudioStreamIndex: null,
      microphoneAudioStreamIndex: null,
      durationMs: 2000,
      endedAtMs: 3000,
      fileExists: true,
      filePath: '/tmp/synthetic.mkv',
      pauseAfterMs: 0,
      playbackOffsetMs: 0,
      segmentIndex: 0,
      startedAtMs: 1000,
      state: 'finalized',
      videoStreamIndex: 0,
    },
  ],
};

test('audio Recall citations can cover contiguous fragments and reject missing, paused or redacted tails', () => {
  const first = detail.segments[0];
  if (!first) throw new Error('Missing synthetic segment');
  const audio: RecallEvidence = {
    ...evidence,
    sourceKind: 'desktop-audio',
    sourceTrackId: 'desktop-audio-1',
    sourceFrameRef: null,
    documentId: null,
    startOffsetMs: 1500,
    endOffsetMs: 2500,
  };
  const recording: LibraryRecordingDetail = {
    ...detail,
    durationMs: 4000,
    endedAtMs: 5000,
    tracks: [
      {
        id: 'desktop-audio-1',
        kind: 'desktop-audio',
        label: 'Desktop audio',
        description: '',
        codec: null,
        streamIndex: 1,
      },
    ],
    segments: [
      { ...first, desktopAudioStreamIndex: 1 },
      {
        ...first,
        id: 'tail',
        segmentIndex: 1,
        playbackOffsetMs: 2000,
        startedAtMs: 3000,
        endedAtMs: 5000,
        desktopAudioStreamIndex: 3,
      },
    ],
  };
  expect(studioRecallEvidenceAvailable(audio, recording)).toBe(true);
  for (const invalid of [
    { fileExists: false },
    { state: 'failed' as const },
    { desktopAudioStreamIndex: null },
    { playbackOffsetMs: 2100 },
    { startedAtMs: 3100 },
  ]) {
    expect(
      studioRecallEvidenceAvailable(audio, {
        ...recording,
        segments: recording.segments.map((s, i) =>
          i === 1 ? { ...s, ...invalid } : s,
        ),
      }),
    ).toBe(false);
  }
  expect(
    studioRecallEvidenceAvailable(audio, {
      ...recording,
      redactions: [
        {
          id: 'tail-mask',
          recordingId: detail.id,
          createdAtMs: 0,
          reason: null,
          startOffsetMs: 2200,
          endOffsetMs: 2300,
          trackIds: ['desktop-audio-3'],
        },
      ],
    }),
  ).toBe(false);
  expect(
    studioRecallEvidenceAvailable({ ...audio, endOffsetMs: 4500 }, recording),
  ).toBe(false);
});

test('recall links after a real pause use segment timeline bounds rather than captured duration', () => {
  const firstSegment = detail.segments[0];
  if (!firstSegment) throw new Error('Missing synthetic segment');
  const paused = {
    ...detail,
    segments: [0, 6000].map((playbackOffsetMs, index) => ({
      ...firstSegment,
      id: `segment-${index}`,
      durationMs: 1000,
      playbackOffsetMs,
      pauseAfterMs: index === 0 ? 5000 : 0,
    })),
  };
  const later = {
    ...evidence,
    segmentId: 'segment-1',
    startOffsetMs: 6500,
    endOffsetMs: 7000,
  };
  expect(studioRecallEvidenceAvailable(later, paused)).toBe(true);
  expect(
    studioRecallEvidenceAvailable({ ...later, startOffsetMs: 5000 }, paused),
  ).toBe(false);
});

test('recall navigation requires the exact present source, segment and range', () => {
  expect(studioRecallEvidenceAvailable(evidence, detail)).toBe(true);
  expect(
    studioRecallEvidenceAvailable({ ...evidence, documentId: null }, detail),
  ).toBe(false);
  expect(
    studioRecallEvidenceAvailable(
      { ...evidence, sourceFrameRef: null },
      detail,
    ),
  ).toBe(false);
  for (const change of [
    { recordingId: 'other' },
    { segmentId: 'missing' },
    { sourceTrackId: 'screen-b' },
    { sourceKind: 'microphone' as const },
    { endOffsetMs: 3000 },
    { startOffsetMs: -1 },
    { endOffsetMs: 500 },
  ])
    expect(
      studioRecallEvidenceAvailable({ ...evidence, ...change }, detail),
    ).toBe(false);
  expect(
    studioRecallEvidenceAvailable(evidence, { ...detail, durationMs: null }),
  ).toBe(true);
  expect(
    studioRecallEvidenceAvailable(evidence, {
      ...detail,
      segments: detail.segments.map((segment) => ({
        ...segment,
        fileExists: false,
      })),
    }),
  ).toBe(false);
  expect(
    studioRecallEvidenceAvailable(evidence, {
      ...detail,
      segments: detail.segments.map((segment) => ({
        ...segment,
        videoStreams: [
          {
            trackId: 'screen-b',
            sourceId: null,
            label: 'Screen B',
            streamIndex: 0,
            width: 1280,
            height: 720,
            fps: 2,
          },
        ],
      })),
    }),
  ).toBe(false);
});

test('privacy overlap blocks the source but unrelated tracks and touching boundaries remain eligible', () => {
  const redaction = {
    id: 'hidden',
    recordingId: detail.id,
    createdAtMs: 0,
    reason: null,
    startOffsetMs: 750,
    endOffsetMs: 1500,
    trackIds: ['screen-a'],
  };
  expect(
    studioRecallEvidenceAvailable(evidence, {
      ...detail,
      redactions: [redaction],
    }),
  ).toBe(false);
  expect(
    studioRecallEvidenceAvailable(evidence, {
      ...detail,
      redactions: [{ ...redaction, trackIds: [] }],
    }),
  ).toBe(false);
  expect(
    studioRecallEvidenceAvailable(evidence, {
      ...detail,
      redactions: [{ ...redaction, trackIds: ['screen-b'] }],
    }),
  ).toBe(true);
  expect(
    studioRecallEvidenceAvailable(evidence, {
      ...detail,
      redactions: [{ ...redaction, startOffsetMs: 1000 }],
    }),
  ).toBe(true);
});

test.each(['microphone', 'desktop-audio'] as const)(
  'audio recall requires the %s alias belonging to its exact segment',
  (sourceKind) => {
    const audioDetail: LibraryRecordingDetail = {
      ...detail,
      durationMs: 4000,
      tracks: [2, 3].map((index) => ({
        id: `${sourceKind}-${index}`,
        kind: sourceKind,
        label: `${sourceKind} ${index}`,
        description: '',
        codec: 'opus',
        streamIndex: index,
      })),
      segments: [2, 3].map((index) => ({
        ...detail.segments[0]!,
        id: `segment-${index}`,
        playbackOffsetMs: (index - 2) * 2000,
        desktopAudioStreamIndex: sourceKind === 'desktop-audio' ? index : null,
        microphoneAudioStreamIndex: sourceKind === 'microphone' ? index : null,
      })),
    };
    const audioEvidence = {
      ...evidence,
      sourceKind,
      segmentId: 'segment-3',
      sourceTrackId: `${sourceKind}-3`,
      documentId: null,
      sourceFrameRef: null,
      startOffsetMs: 2500,
      endOffsetMs: 3000,
    };
    expect(studioRecallEvidenceAvailable(audioEvidence, audioDetail)).toBe(
      true,
    );
    expect(
      studioRecallEvidenceAvailable(
        { ...audioEvidence, sourceTrackId: `${sourceKind}-2` },
        audioDetail,
      ),
    ).toBe(false);
  },
);
