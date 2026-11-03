export interface TimelineThumbnailWindow {
  baseUrl: string;
  streamIndex: number | null;
  segmentStartMs: number;
  mediaStartMs: number;
  durationMs: number;
  startMs: number;
  endMs: number;
  pixelsPerMs: number;
  thumbnailWidth: number;
}

/** One preview per painted slot, regardless of the recording's duration. */
export function timelineThumbnailWindow(input: TimelineThumbnailWindow) {
  const start = Math.max(input.startMs, input.segmentStartMs);
  const end = Math.min(input.endMs, input.segmentStartMs + input.durationMs);
  if (end <= start || input.pixelsPerMs <= 0 || input.thumbnailWidth <= 0)
    return [];
  const first = Math.floor((start * input.pixelsPerMs) / input.thumbnailWidth);
  const last = Math.ceil((end * input.pixelsPerMs) / input.thumbnailWidth);
  const previews: { offsetMs: number; sampleOffsetMs: number; url: string }[] =
    [];
  for (let slot = first; slot < last; slot++) {
    const time = Math.max(
      start,
      (slot * input.thumbnailWidth) / input.pixelsPerMs,
    );
    const sourceTime = input.mediaStartMs + time - input.segmentStartMs;
    const sample = Math.max(
      input.mediaStartMs,
      Math.floor(sourceTime / 3000) * 3000,
    );
    previews.push({
      offsetMs: time - input.segmentStartMs,
      sampleOffsetMs: sample - input.mediaStartMs,
      url: `${input.baseUrl}/${sample}.jpg${input.streamIndex === null ? '' : `/${input.streamIndex}`}`,
    });
  }
  return previews;
}
