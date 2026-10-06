import type {
  SegmentVideoStream,
  LibraryPlaybackVideoAsset,
  LibraryPlaybackThumbnail,
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
      thumbnails: list(r.thumbnails, thumbnail),
    };
  });
}
