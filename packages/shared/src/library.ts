import type { RecordingAiProcessingState } from './ai.js';
import type { RecordingVisualObservations } from './ai/visual.js';
import type { AudioCleanupState } from './audio-cleanup.js';
import type { RecorderSegmentState } from './recorder.js';

export type LibraryRecordingAvailability =
  'available' | 'incomplete' | 'locked' | 'missing-files';

export type LibraryRecordingSortKey =
  'availability' | 'duration' | 'started' | 'status';

export type LibraryRecordingViewMode = 'grid' | 'list';

export interface LibraryRecordingSummary {
  availability: LibraryRecordingAvailability;
  /** Captured time excluding pauses; segment offsets define the playback timeline. */
  durationMs: number | null;
  endedAtMs: number | null;
  id: string;
  missingReasons: string[];
  playableSegmentCount: number;
  segmentCount: number;
  sizeBytes: number;
  startedAtMs: number | null;
  status: string;
  storageDir: string;
  thumbnailPath: string | null;
  title: string | null;
}

export interface LibraryRecordingSegment {
  /** View-only trim into source media, used for calendar-day playback slices. */
  mediaStartMs?: number;
  /** Effective primary video source in a playback view. */
  videoTrackId?: string | undefined;
  fragmentIds?: string[];
  captureRunId?: string | null;
  videoStreams?: SegmentVideoStream[];
  desktopAudioStreamIndex: number | null;
  durationMs: number | null;
  endedAtMs: number | null;
  fileExists: boolean;
  filePath: string;
  id: string;
  microphoneAudioStreamIndex: number | null;
  pauseAfterMs: number;
  playbackOffsetMs: number;
  segmentIndex: number;
  startedAtMs: number;
  state: RecorderSegmentState;
  videoStreamIndex: number | null;
}

export interface SegmentVideoStream {
  trackId: string;
  sourceId: string | null;
  label: string;
  streamIndex: number;
  width: number | null;
  height: number | null;
  fps: number | null;
}

export type LibraryTrackKind = 'desktop-audio' | 'microphone' | 'video';

export interface LibraryRecordingTrack {
  codec: string | null;
  description: string;
  id: string;
  kind: LibraryTrackKind;
  label: string;
  streamIndex: number | null;
}

export interface LibraryRecordingRedaction {
  createdAtMs: number;
  endOffsetMs: number;
  id: string;
  reason: string | null;
  recordingId: string;
  startOffsetMs: number;
  trackIds: string[];
}

export interface LibraryRecordingDetail {
  audioCleanup?: AudioCleanupState;
  availability: LibraryRecordingAvailability;
  durationMs: number | null;
  endedAtMs: number | null;
  id: string;
  missingReasons: string[];
  playableSegmentCount: number;
  redactions: LibraryRecordingRedaction[];
  segments: LibraryRecordingSegment[];
  sizeBytes: number;
  startedAtMs: number | null;
  status: string;
  storageDir: string;
  thumbnailPath: string | null;
  title: string | null;
  tracks: LibraryRecordingTrack[];
}

export interface LibraryDayTimelineInput {
  dayId: string;
  timezone: string;
}

/** Original recording metadata and source-time slices; never a persisted recording. */
export interface LibraryDayTimeline {
  dayId: string;
  startMs: number;
  endMs: number;
  privacyRevision: number;
  recordings: LibraryRecordingDetail[];
  spans: LibraryDayPlaybackSpan[];
}

export interface LibraryDayPlaybackSpan {
  recordingId: string;
  segmentId: string;
  startMs: number;
  durationMs: number;
  sourceStartMs: number;
  sourceOffsetMs: number;
}

export interface DeleteLibraryRecordingInput {
  recordingId: string;
}

export interface DeleteLibraryRecordingsInput {
  recordingIds: string[];
}

export interface RenameLibraryRecordingInput {
  recordingId: string;
  title: string | null;
}

export interface CreateLibraryRedactionRangeInput {
  endOffsetMs: number;
  reason?: string | null;
  startOffsetMs: number;
  trackIds: string[];
}

export interface CreateLibraryRedactionsInput {
  ranges: CreateLibraryRedactionRangeInput[];
  recordingId: string;
}

export interface DeleteLibraryRedactionInput {
  recordingId: string;
  redactionId: string;
}

export interface ResizeLibraryRedactionInput {
  recordingId: string;
  redactionId: string;
  startOffsetMs: number;
  endOffsetMs: number;
  expectedStartOffsetMs: number;
  expectedEndOffsetMs: number;
}

export interface UnredactLibraryRedactionsInput {
  ranges: CreateLibraryRedactionRangeInput[];
  recordingId: string;
}

export interface LibraryRecordingDebugInput {
  recordingId: string;
}

export interface LibraryRecordingDebugData {
  detail: LibraryRecordingDetail;
  processingState: RecordingAiProcessingState;
  raw: unknown;
  recordingId: string;
  visualObservations: RecordingVisualObservations;
}

export interface LibraryWaveformRequest {
  recordingId: string;
}

export interface LibraryWaveformTileRequest {
  endTimelineMs: number;
  recordingId: string;
  startTimelineMs: number;
  viewportWidthPx: number;
}

export interface LibraryWaveformSegment {
  generatedAtMs: number;
  maxAmplitude: number;
  samples: number[];
  segmentId: string;
  streamIndex: number;
}

export interface LibraryWaveformTrack {
  kind: Extract<LibraryTrackKind, 'desktop-audio' | 'microphone'>;
  segments: LibraryWaveformSegment[];
  trackId: string;
}

export interface LibraryWaveforms {
  error: string | null;
  recordingId: string;
  tracks: LibraryWaveformTrack[];
}

export interface LibraryWaveformTile {
  binMs: number;
  durationMs: number;
  generatedAtMs: number;
  peakSamples: number[];
  rmsSamples: number[];
  segmentId: string;
  streamIndex: number;
  timelineStartMs: number;
}

export interface LibraryWaveformTileTrack {
  kind: Extract<LibraryTrackKind, 'desktop-audio' | 'microphone'>;
  scaleAmplitude: number;
  tiles: LibraryWaveformTile[];
  trackId: string;
}

export interface LibraryWaveformTiles {
  pending?: boolean;
  binMs: number;
  error: string | null;
  recordingId: string;
  tracks: LibraryWaveformTileTrack[];
}

export interface LibraryPlaybackAssetRequest {
  recordingId: string;
}

export interface CloseLibraryPlaybackSessionRequest {
  sessionId: string;
}

export interface LibraryRecordingThumbnailRequest {
  recordingId: string;
}

export interface LibraryRecordingThumbnail {
  error: string | null;
  path: string | null;
  recordingId: string;
}

export interface LibraryPlaybackAssets {
  /** Native streaming proxies seek by replacing the stream URL. */
  canSeekInStream?: boolean;
  runs?: LibraryPlaybackRun[];
  error: string | null;
  recordingId: string;
  segments: LibraryPlaybackSegment[];
  sessionId: string;
}
export interface LibraryPlaybackRun {
  runId: string;
  segmentIds: string[];
  durationMs: number;
  playbackOffsetMs: number;
  assets: LibraryPlaybackSegment;
}

export interface LibraryPlaybackSegment {
  videos?: LibraryPlaybackVideoAsset[];
  audio: LibraryPlaybackAudioAsset[];
  segmentId: string;
  thumbnails: LibraryPlaybackThumbnail[];
  videoUrl: string;
}

export interface LibraryPlaybackVideoAsset {
  trackId: string;
  streamIndex: number;
  url: string;
  thumbnails: LibraryPlaybackThumbnail[];
}

export interface LibraryPlaybackAudioAsset {
  kind: Extract<LibraryTrackKind, 'desktop-audio' | 'microphone'>;
  streamIndex: number;
  trackId: string;
  url: string;
}

export interface LibraryPlaybackThumbnail {
  offsetMs: number;
  url: string;
}
