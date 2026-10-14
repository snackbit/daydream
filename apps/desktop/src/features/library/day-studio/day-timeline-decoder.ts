import type {
  LibraryDayTimeline,
  LibraryRecordingDetail,
  LibraryRecordingSegment,
  LibraryRecordingTrack,
} from '@daydream/shared';
import { decodeAudioCleanupState } from '../components/studio/audio-cleanup/audio-cleanup-client';
import { decodeVideoStreams } from '../video-metadata-decoder';

function invalid(): never {
  throw new Error('Day playback metadata is invalid. Reload the day.');
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function record(value: unknown): Record<string, unknown> {
  if (!isRecord(value)) return invalid();
  return value;
}
function str(value: unknown): string {
  return typeof value === 'string' ? value : invalid();
}
function num(value: unknown): number {
  return typeof value === 'number' && Number.isSafeInteger(value)
    ? value
    : invalid();
}
function nullableNum(value: unknown): number | null {
  return value === null ? null : num(value);
}
function nullableStr(value: unknown): string | null {
  return value === null ? null : str(value);
}
function list<T>(value: unknown, decode: (item: unknown) => T): T[] {
  return Array.isArray(value) ? value.map(decode) : invalid();
}
function segment(value: unknown): LibraryRecordingSegment {
  const s = record(value);
  const state = str(s.state);
  if (!['recording', 'finalized', 'failed'].includes(state)) return invalid();
  // The native day reader admits finalized spans only; metadata may include failed siblings.
  return {
    id: str(s.id),
    captureRunId: nullableStr(s.captureRunId),
    videoStreams: decodeVideoStreams(s.videoStreams),
    desktopAudioStreamIndex: nullableNum(s.desktopAudioStreamIndex),
    durationMs: nullableNum(s.durationMs),
    endedAtMs: nullableNum(s.endedAtMs),
    fileExists: typeof s.fileExists === 'boolean' ? s.fileExists : invalid(),
    filePath: str(s.filePath),
    microphoneAudioStreamIndex: nullableNum(s.microphoneAudioStreamIndex),
    pauseAfterMs: num(s.pauseAfterMs),
    playbackOffsetMs: num(s.playbackOffsetMs),
    segmentIndex: num(s.segmentIndex),
    startedAtMs: num(s.startedAtMs),
    state: decodeState(state),
    videoStreamIndex: nullableNum(s.videoStreamIndex),
  };
}
function decodeState(state: string): LibraryRecordingSegment['state'] {
  if (state === 'recording' || state === 'finalized' || state === 'failed')
    return state;
  return invalid();
}
function track(value: unknown): LibraryRecordingTrack {
  const t = record(value),
    kind = t.kind;
  if (kind !== 'video' && kind !== 'desktop-audio' && kind !== 'microphone')
    return invalid();
  return {
    id: str(t.id),
    kind,
    codec: nullableStr(t.codec),
    description: str(t.description),
    label: str(t.label),
    streamIndex: nullableNum(t.streamIndex),
  };
}
function detail(value: unknown): LibraryRecordingDetail {
  const r = record(value),
    availability = r.availability;
  if (
    availability !== 'available' &&
    availability !== 'incomplete' &&
    availability !== 'locked' &&
    availability !== 'missing-files'
  )
    return invalid();
  return {
    ...(r.audioCleanup === undefined
      ? {}
      : { audioCleanup: decodeAudioCleanupState(r.audioCleanup) }),
    id: str(r.id),
    availability,
    durationMs: nullableNum(r.durationMs),
    endedAtMs: nullableNum(r.endedAtMs),
    missingReasons: list(r.missingReasons, str),
    playableSegmentCount: num(r.playableSegmentCount),
    segments: list(r.segments, segment),
    sizeBytes: nullableNum(r.sizeBytes),
    startedAtMs: nullableNum(r.startedAtMs),
    status: str(r.status),
    storageDir: str(r.storageDir),
    thumbnailPath: nullableStr(r.thumbnailPath),
    title: nullableStr(r.title),
    tracks: list(r.tracks, track),
    redactions: list(r.redactions, (value) => {
      const cut = record(value);
      return {
        id: str(cut.id),
        recordingId: str(cut.recordingId),
        createdAtMs: num(cut.createdAtMs),
        startOffsetMs: num(cut.startOffsetMs),
        endOffsetMs: num(cut.endOffsetMs),
        trackIds: list(cut.trackIds, str),
        reason: nullableStr(cut.reason),
      };
    }),
  };
}
export function decodeDayTimeline(value: unknown): LibraryDayTimeline {
  const r = record(value);
  const result: LibraryDayTimeline = {
    dayId: str(r.dayId),
    startMs: num(r.startMs),
    endMs: num(r.endMs),
    privacyRevision: num(r.privacyRevision),
    recordings: list(r.recordings, detail),
    spans: list(r.spans, (value) => {
      const s = record(value);
      return {
        recordingId: str(s.recordingId),
        segmentId: str(s.segmentId),
        startMs: num(s.startMs),
        durationMs: num(s.durationMs),
        sourceStartMs: num(s.sourceStartMs),
        sourceOffsetMs: num(s.sourceOffsetMs),
      };
    }),
  };
  if (
    result.endMs <= result.startMs ||
    result.spans.some(
      (s) =>
        s.durationMs <= 0 ||
        s.sourceStartMs < 0 ||
        s.sourceOffsetMs < 0 ||
        s.startMs < result.startMs ||
        s.startMs + s.durationMs > result.endMs ||
        !result.recordings.some(
          (r) =>
            r.id === s.recordingId &&
            r.segments.some((segment) => segment.id === s.segmentId),
        ),
    )
  )
    return invalid();
  return result;
}
