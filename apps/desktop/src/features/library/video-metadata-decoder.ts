import type {
  SegmentVideoStream,
  LibraryPlaybackVideoAsset,
  LibraryPlaybackThumbnail,
  LibraryPlaybackRun,
} from '@daydream/shared';

function invalid(): never {
  throw new Error(
    'Video source metadata returned an invalid response. Reload the recording.',
  );
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function record(value: unknown): Record<string, unknown> {
  return isRecord(value) ? value : invalid();
}
function text(value: unknown): string {
  return typeof value === 'string' ? value : invalid();
}
function number(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0
    ? value
    : invalid();
}
function nullableNumber(value: unknown): number | null {
  return value === null ? null : number(value);
}
function list<T>(value: unknown, decode: (value: unknown) => T): T[] {
  return Array.isArray(value) ? value.map(decode) : invalid();
}
export function decodeSeekCapability(value: unknown): {
  canSeekInStream?: boolean;
} {
  if (value === undefined) return {};
  if (typeof value !== 'boolean') invalid();
  return { canSeekInStream: value };
}
export function decodeThumbnailBase(value: unknown): {
  thumbnailBaseUrl?: string;
} {
  return value === undefined ? {} : { thumbnailBaseUrl: text(value) };
}
export function decodeVideoStreams(value: unknown): SegmentVideoStream[] {
  return list(value, (value) => {
    const r = record(value);
    return {
      trackId: text(r.trackId),
      sourceId: r.sourceId === null ? null : text(r.sourceId),
      label: text(r.label),
      streamIndex: number(r.streamIndex),
      width: nullableNumber(r.width),
      height: nullableNumber(r.height),
      fps: nullableNumber(r.fps),
    };
  });
}
function thumbnail(value: unknown): LibraryPlaybackThumbnail {
  const r = record(value);
  return { offsetMs: number(r.offsetMs), url: text(r.url) };
}
export function decodeVideoAssets(value: unknown): LibraryPlaybackVideoAsset[] {
  return list(value, (value) => {
    const r = record(value);
    return {
      trackId: text(r.trackId),
      streamIndex: number(r.streamIndex),
      url: text(r.url),
      ...decodeThumbnailBase(r.thumbnailBaseUrl),
      thumbnails: list(r.thumbnails, thumbnail),
    };
  });
}
export function decodePlaybackRuns(value: unknown): LibraryPlaybackRun[] {
  return list(value, (value) => {
    const r = record(value),
      assets = record(r.assets);
    return {
      runId: text(r.runId),
      segmentIds: list(r.segmentIds, text),
      durationMs: number(r.durationMs),
      playbackOffsetMs: number(r.playbackOffsetMs),
      assets: {
        ...decodeThumbnailBase(assets.thumbnailBaseUrl),
        ...decodeSeekCapability(assets.canSeekInStream),
        segmentId: text(assets.segmentId),
        videoUrl: text(assets.videoUrl),
        thumbnails: list(assets.thumbnails, thumbnail),
        ...(assets.videos === undefined
          ? {}
          : { videos: decodeVideoAssets(assets.videos) }),
        audio: list(assets.audio, (value) => {
          const audio = record(value);
          if (audio.kind !== 'desktop-audio' && audio.kind !== 'microphone')
            invalid();
          return {
            kind: audio.kind,
            streamIndex: number(audio.streamIndex),
            trackId: text(audio.trackId),
            url: text(audio.url),
          };
        }),
      },
    };
  });
}
