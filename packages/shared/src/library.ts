import type { RecorderSegmentState } from './recorder.js';

export type LibraryRecordingAvailability =
  | 'available'
  | 'incomplete'
  | 'locked'
  | 'missing-files';

export type LibraryRecordingSortKey =
  | 'availability'
  | 'duration'
  | 'started'
  | 'status';

export type LibraryRecordingViewMode = 'grid' | 'list';

export interface LibraryRecordingSummary {
  availability: LibraryRecordingAvailability;
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
}

export interface LibraryRecordingSegment {
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

export type LibraryTrackKind = 'desktop-audio' | 'microphone' | 'video';

export interface LibraryRecordingTrack {
  codec: string | null;
  description: string;
  id: string;
  kind: LibraryTrackKind;
  label: string;
  streamIndex: number | null;
}

export interface LibraryRecordingDetail {
  availability: LibraryRecordingAvailability;
  durationMs: number | null;
  endedAtMs: number | null;
  id: string;
  missingReasons: string[];
  playableSegmentCount: number;
  segments: LibraryRecordingSegment[];
  sizeBytes: number;
  startedAtMs: number | null;
  status: string;
  storageDir: string;
  thumbnailPath: string | null;
  tracks: LibraryRecordingTrack[];
}

export interface DeleteLibraryRecordingInput {
  recordingId: string;
}

export interface DeleteLibraryRecordingsInput {
  recordingIds: string[];
}

export interface LibraryWaveformRequest {
  recordingId: string;
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

export interface LibraryPlaybackAssetRequest {
  recordingId: string;
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
  error: string | null;
  recordingId: string;
  segments: LibraryPlaybackSegment[];
}

export interface LibraryPlaybackThumbnails {
  error: string | null;
  recordingId: string;
  segments: LibraryPlaybackThumbnailSegment[];
}

export interface LibraryPlaybackSegment {
  audio: LibraryPlaybackAudioAsset[];
  segmentId: string;
  thumbnails: LibraryPlaybackThumbnail[];
  videoPath: string;
}

export interface LibraryPlaybackAudioAsset {
  kind: Extract<LibraryTrackKind, 'desktop-audio' | 'microphone'>;
  path: string;
  streamIndex: number;
  trackId: string;
}

export interface LibraryPlaybackThumbnail {
  offsetMs: number;
  path: string;
}

export interface LibraryPlaybackThumbnailSegment {
  segmentId: string;
  thumbnails: LibraryPlaybackThumbnail[];
}
