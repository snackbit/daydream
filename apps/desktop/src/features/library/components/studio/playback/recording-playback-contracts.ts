import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';

export interface RecordingPlaybackSnapshot {
  displayedTimelineMs: number;
  isPlaying: boolean;
  seekRevision: number;
  totalDurationMs: number;
}

export interface RecordingPlaybackControllerInput {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  playbackAssets: LibraryPlaybackAssets | null;
  autoSkipRedacted?: boolean;
  shortGaps?: boolean;
  skipOnlyWhenAllSources?: boolean;
}
