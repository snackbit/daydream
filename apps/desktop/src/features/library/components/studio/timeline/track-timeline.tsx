import type {
  CreateLibraryRedactionRangeInput,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  RecordingActivity,
  RecordingAiProcessingState,
  ReprocessingTarget,
} from '@daydream/shared';
import type {
  getRecordingWaveformTiles,
  resizeLibraryRedaction,
} from '../../../library-client';
import type { StudioInspectorSelection } from '../navigation/studio-inspector-selection';
import type { StudioVisualFrame } from '../observations/studio-visual-observations';
import type { RecordingPlaybackController } from '../playback/use-recording-playback-controller';
import type { TimelinePlannedScope } from './processing/timeline-processing-ranges';
import { renderTrackTimelineSurface } from './track-timeline-surface';
import type { TranscriptPreviewReader } from './transcripts/transcript-group-preview';
import { useTrackTimelineController } from './use-track-timeline-controller';

export interface TrackTimelineProps {
  onAudioCleanup?: (() => void) | undefined;
  timelineOriginMs?: number | undefined;
  readWaveformTiles?: typeof getRecordingWaveformTiles | undefined;
  onResizeRedaction?: typeof resizeLibraryRedaction | undefined;
  activity: RecordingActivity | null;
  aiProcessingState: RecordingAiProcessingState | null;
  plannedProcessingScopes?: readonly TimelinePlannedScope[];
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onClearInspectorSelection: () => void;
  onInspectActivity: (selection: StudioInspectorSelection) => void;
  onInspectTranscript: (selection: StudioInspectorSelection) => void;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onToggleTrackMuted: (trackId: string) => void;
  playback: RecordingPlaybackController;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  visualFrames: StudioVisualFrame[];
  onRefreshRedactions: () => void;
  onReprocess?: ((targets: ReprocessingTarget[]) => void) | undefined;
  suspendFollow?: boolean;
  readTranscriptPreview?: TranscriptPreviewReader | undefined;
  transcriptPreviewRevision?: string | undefined;
  timelinePendingRanges?:
    readonly { startOffsetMs: number; endOffsetMs: number }[] | undefined;
  timelineFailedRanges?:
    readonly { startOffsetMs: number; endOffsetMs: number }[] | undefined;
  transcriptReadStatus?: 'loading' | 'error' | 'ready';
}

export function TrackTimeline(props: TrackTimelineProps) {
  const controller = useTrackTimelineController(props);
  return renderTrackTimelineSurface({ source: props, controller });
}
