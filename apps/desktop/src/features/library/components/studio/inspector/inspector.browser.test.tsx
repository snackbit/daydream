import type {
  LibraryRecordingDetail,
  RecordingAiProcessingState,
} from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import type { StudioInspectorSelection } from '../studio-inspector-selection';
import { RecordingInspectorSummary } from './recording-inspector-summary';
import { SelectedSourceInspector } from './selected-source-inspector';
import { VisualSourceInspector } from './visual-source-inspector';

afterEach(() => {
  cleanup();
});

test('microphone transcript Inspector keeps source status and provenance with its text', () => {
  const selection = {
    chunkResult: 'text',
    confidence: 0.93,
    endOffsetMs: 7_300,
    kind: 'transcript-segment',
    model: 'whisper-small',
    provider: 'local-whisper-cpp',
    segmentId: 'mic-segment-4',
    sourceRole: 'microphone',
    startOffsetMs: 5_000,
    text: 'A test phrase from the microphone.',
    timelineOffsetMs: 5_000,
    trackId: 'microphone-0',
    trackLabel: 'Microphone',
  } satisfies StudioInspectorSelection;
  const onClearSelection = vi.fn();
  const onSeek = vi.fn();

  render(
    <SelectedSourceInspector
      debugEnabled
      onClearSelection={onClearSelection}
      onSeek={onSeek}
      selection={selection}
    />,
  );

  expect(
    screen.getByRole('heading', { name: 'Microphone transcript' }),
  ).toBeTruthy();
  expect(screen.getAllByText('Microphone')).toHaveLength(2);
  expect(screen.getByText('Transcript available')).toBeTruthy();
  expect(screen.getByText('A test phrase from the microphone.')).toBeTruthy();
  expect(screen.queryByText('No speech detected')).toBeNull();
  expect(screen.getByText('mic-segment-4')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: 'Jump to source' }));
  expect(onSeek).toHaveBeenCalledWith(5_000);
  fireEvent.click(
    screen.getByRole('button', { name: 'Clear selected source' }),
  );
  expect(onClearSelection).toHaveBeenCalledOnce();
});

test('desktop no-speech status cannot become the microphone transcription status', () => {
  const libraryDetail = {
    availability: 'available',
    durationMs: 7_000,
    endedAtMs: 8_000,
    id: 'recording-1',
    missingReasons: [],
    playableSegmentCount: 1,
    redactions: [],
    segments: [
      {
        desktopAudioStreamIndex: 0,
        durationMs: 7_000,
        endedAtMs: 8_000,
        fileExists: true,
        filePath: '/recording/segment.mkv',
        id: 'segment-1',
        microphoneAudioStreamIndex: 1,
        pauseAfterMs: 0,
        playbackOffsetMs: 0,
        segmentIndex: 1,
        startedAtMs: 1_000,
        state: 'finalized',
        videoStreamIndex: 2,
      },
    ],
    sizeBytes: 1_000,
    startedAtMs: 1_000,
    status: 'completed',
    storageDir: '/recording',
    thumbnailPath: null,
    title: null,
    tracks: [
      {
        codec: 'opus',
        description: 'Desktop audio source',
        id: 'desktop-audio-0',
        kind: 'desktop-audio',
        label: 'Desktop audio',
        streamIndex: 0,
      },
      {
        codec: 'opus',
        description: 'Microphone source',
        id: 'microphone-0',
        kind: 'microphone',
        label: 'Microphone',
        streamIndex: 1,
      },
    ],
  } satisfies LibraryRecordingDetail;

  const transcriptionState = {
    chunks: [],
    job: null,
    recordingId: libraryDetail.id,
    transcript: { recordingId: libraryDetail.id, tracks: [] },
    transcriptionTracks: [
      transcriptionTrack({
        libraryTrackId: 'desktop-audio-0',
        sourceRole: 'desktop-audio',
        status: 'no-speech',
        statusLabel: 'No speech detected',
        textSegmentCount: 0,
        trackKey: 'desktop',
      }),
      transcriptionTrack({
        libraryTrackId: 'microphone-0',
        sourceRole: 'microphone',
        status: 'complete',
        statusLabel: 'Transcription complete',
        textSegmentCount: 1,
        trackKey: 'microphone',
      }),
    ],
  } satisfies RecordingAiProcessingState;

  render(
    <RecordingInspectorSummary
      aiProcessingState={transcriptionState}
      detail={libraryDetail}
    />,
  );

  const desktopSource = screen
    .getByRole('heading', { name: 'Desktop audio' })
    .closest('section');
  const microphoneSource = screen
    .getByRole('heading', { name: 'Microphone' })
    .closest('section');
  if (!desktopSource || !microphoneSource) {
    throw new Error(
      'Expected a separate Inspector group for each audio source.',
    );
  }

  expect(within(desktopSource).getByText('No speech detected')).toBeTruthy();
  expect(within(desktopSource).getByText('0')).toBeTruthy();
  expect(
    within(microphoneSource).getByText('Transcription complete'),
  ).toBeTruthy();
  expect(within(microphoneSource).getByText('1')).toBeTruthy();
  expect(within(microphoneSource).queryByText('No speech detected')).toBeNull();
});

test('selected frame keeps subsecond time precise and seeks to the same offset', () => {
  const onSeek = vi.fn();

  render(
    <VisualSourceInspector
      context={{
        recordingId: 'recording-1',
        sourceTrackId: 'video-0',
        privacyRevision: '',
        debugEnabled: false,
      }}
      frame={{
        endTimestampMs: 1_000,
        frameTimestampMs: 500,
        id: 'frame:500',
        ocrRegions: [],
        sourceFrameRef: 'segment:1:offset:500:frame:0',
        sourceFrameRefs: ['segment:1:offset:500:frame:0'],
        vlmObservations: [],
      }}
      onClear={vi.fn()}
      onSeek={onSeek}
    />,
  );

  expect(screen.getByText('0:00.500')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Jump to frame' }));
  expect(onSeek).toHaveBeenCalledWith(500);
});

function transcriptionTrack({
  libraryTrackId,
  sourceRole,
  status,
  statusLabel,
  textSegmentCount,
  trackKey,
}: {
  libraryTrackId: string;
  sourceRole: 'desktop-audio' | 'microphone';
  status: 'complete' | 'no-speech';
  statusLabel: string;
  textSegmentCount: number;
  trackKey: string;
}): NonNullable<RecordingAiProcessingState['transcriptionTracks']>[number] {
  return {
    chunks: [],
    description: `${trackKey} transcription`,
    label: 'Transcription',
    libraryTrackId,
    sourceRole,
    status,
    statusLabel,
    textSegmentCount,
    trackId: trackKey,
    trackKey,
  };
}
