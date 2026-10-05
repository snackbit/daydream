import type {
  ActivityEventKind,
  ActivitySourceKind,
  RecordingTranscriptionChunkResult,
  TranscriptSourceRole,
} from '@daydream/shared';
import { formatDuration } from '../../../library-format';

export function formatTimelineOffset(offsetMs: number): string {
  const safeOffsetMs = Math.max(0, Math.round(offsetMs));
  const wholeSecondOffsetMs = Math.floor(safeOffsetMs / 1_000) * 1_000;
  const fractionalMs = safeOffsetMs % 1_000;
  const wholeSecondLabel = formatDuration(wholeSecondOffsetMs);

  return fractionalMs === 0
    ? wholeSecondLabel
    : `${wholeSecondLabel}.${fractionalMs.toString().padStart(3, '0')}`;
}

export function transcriptionSourceLabel(
  sourceRole: TranscriptSourceRole,
): string {
  return sourceRole === 'desktop-audio' ? 'Desktop audio' : 'Microphone';
}

export function activitySourceLabel(sourceKind: ActivitySourceKind): string {
  switch (sourceKind) {
    case 'frame-delta':
      return 'Frame difference';
    case 'keyboard':
      return 'Keyboard';
    case 'merged':
      return 'Combined';
    case 'mouse':
      return 'Mouse';
  }
}

export function activityEventLabel(eventKind: ActivityEventKind): string {
  switch (eventKind) {
    case 'focused-window':
      return 'Focused window';
  }
}

export function transcriptionResultLabel(
  result: RecordingTranscriptionChunkResult,
): string {
  switch (result) {
    case 'cancelled':
      return 'Cancelled';
    case 'failed':
      return 'Failed';
    case 'no-speech':
      return 'No speech detected';
    case 'queued':
      return 'Queued';
    case 'running':
      return 'In progress';
    case 'skipped-silence':
      return 'Skipped: silence';
    case 'text':
      return 'Transcript available';
    case 'unknown':
      return 'Status unavailable';
  }
}
