import { useEffect, useMemo, useRef, useState } from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingTrack,
  LibraryWaveformTileTrack,
} from '@daydream/shared';
import { STUDIO_OVERVIEW_PIXELS_PER_MS } from '../studio-result-window-store';

export interface ContinuousTrackMediaProps {
  detail: LibraryRecordingDetail;
  track: LibraryRecordingTrack;
  assets: LibraryPlaybackAssets | null;
  waveform: LibraryWaveformTileTrack | undefined;
  range: { startTimelineMs: number; endTimelineMs: number };
  width: number;
  duration: number;
}
export function ContinuousTrackMedia({
  detail,
  track,
  assets,
  waveform,
  range,
  width,
  duration,
}: ContinuousTrackMediaProps) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [color, setColor] = useState<string | null>(null);
  useEffect(() => {
    if (track.kind === 'video') return;
    const update = () => {
      if (canvas.current) setColor(getComputedStyle(canvas.current).color);
    };
    update();
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme', 'class'],
    });
    return () => observer.disconnect();
  }, [track.kind]);
  const pixelsPerMs = width / duration;
  const thumbnailWidth =
    pixelsPerMs <= STUDIO_OVERVIEW_PIXELS_PER_MS ? 240 : 120;
  const left = range.startTimelineMs * pixelsPerMs;
  const paintWidth = Math.max(
    1,
    Math.ceil((range.endTimelineMs - range.startTimelineMs) * pixelsPerMs),
  );
  const previews = useMemo(() => {
    if (track.kind !== 'video') return [];
    const byId = new Map(
      assets?.segments.map((asset) => [asset.segmentId, asset]) ?? [],
    );
    const slots = new Map<number, { id: string; url: string; left: number }>();
    for (const segment of detail.segments) {
      if (
        segment.playbackOffsetMs >= range.endTimelineMs ||
        segment.playbackOffsetMs + (segment.durationMs ?? 0) <=
          range.startTimelineMs
      )
        continue;
      const asset = byId.get(segment.id);
      const thumbnails =
        asset?.videos?.find((video) => video.trackId === track.id)
          ?.thumbnails ?? (asset?.videos ? [] : (asset?.thumbnails ?? []));
      for (const thumbnail of thumbnails) {
        const time = segment.playbackOffsetMs + thumbnail.offsetMs;
        if (
          time < range.startTimelineMs ||
          time >= range.endTimelineMs ||
          detail.redactions.some(
            (cut) =>
              (cut.trackIds.length === 0 || cut.trackIds.includes(track.id)) &&
              time >= cut.startOffsetMs &&
              time < cut.endOffsetMs,
          )
        )
          continue;
        const slot = Math.floor((time * pixelsPerMs) / thumbnailWidth);
        if (!slots.has(slot))
          slots.set(slot, {
            id: `${segment.id}:${thumbnail.offsetMs}`,
            url: thumbnail.url,
            left: time * pixelsPerMs,
          });
      }
    }
    return [...slots.values()];
  }, [
    assets,
    detail,
    track.id,
    track.kind,
    range.startTimelineMs,
    range.endTimelineMs,
    pixelsPerMs,
    thumbnailWidth,
  ]);
  useEffect(() => {
    const element = canvas.current;
    if (!element || track.kind === 'video' || !color) return;
    const ratio = window.devicePixelRatio || 1;
    const pixelWidth = Math.ceil(paintWidth * ratio),
      pixelHeight = 64 * ratio;
    if (element.width !== pixelWidth) element.width = pixelWidth;
    if (element.height !== pixelHeight) element.height = pixelHeight;
    const context = element.getContext('2d');
    if (!context) return;
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, paintWidth, 64);
    context.fillStyle = color;
    const peaks = new Float32Array(paintWidth);
    const recorded = new Uint8Array(paintWidth);
    for (const tile of waveform?.tiles ?? []) {
      if (
        tile.timelineStartMs >= range.endTimelineMs ||
        tile.timelineStartMs + tile.durationMs <= range.startTimelineMs
      )
        continue;
      for (let i = 0; i < tile.peakSamples.length; i++) {
        const from = tile.timelineStartMs + i * tile.binMs;
        const to = Math.min(
          tile.timelineStartMs + tile.durationMs,
          from + tile.binMs,
        );
        if (
          to <= range.startTimelineMs ||
          from >= range.endTimelineMs ||
          detail.redactions.some(
            (cut) =>
              (cut.trackIds.length === 0 || cut.trackIds.includes(track.id)) &&
              from < cut.endOffsetMs &&
              to > cut.startOffsetMs,
          )
        )
          continue;
        const a = Math.max(0, Math.floor(from * pixelsPerMs - left));
        const b = Math.min(
          paintWidth,
          Math.max(a + 1, Math.ceil(to * pixelsPerMs - left)),
        );
        for (let x = a; x < b; x++) {
          recorded[x] = 1;
          peaks[x] = Math.max(peaks[x] ?? 0, tile.peakSamples[i] ?? 0);
        }
      }
    }
    for (let x = 0; x < peaks.length; x++) {
      const height = Math.max(1, (peaks[x] ?? 0) * 62);
      if (recorded[x]) context.fillRect(x, 32 - height / 2, 1, height);
    }
  }, [
    detail.redactions,
    waveform,
    paintWidth,
    pixelsPerMs,
    left,
    range.startTimelineMs,
    range.endTimelineMs,
    track.id,
    track.kind,
    color,
  ]);
  return (
    <div
      className="text-foreground/70 pointer-events-none absolute top-1 bottom-12 overflow-hidden"
      style={{ left, width: paintWidth }}
    >
      {track.kind === 'video' ? (
        previews.map((preview) => (
          <img
            key={preview.id}
            alt=""
            src={preview.url}
            className="absolute inset-y-0 h-full object-cover"
            style={{ left: preview.left - left, width: thumbnailWidth }}
          />
        ))
      ) : (
        <canvas
          ref={canvas}
          role="img"
          aria-label={
            waveform
              ? `${track.label} waveform`
              : `${track.label} waveform pending`
          }
          className="absolute top-1/2 h-16 -translate-y-1/2"
          style={{ left: 0, width: paintWidth }}
        />
      )}
      {track.kind !== 'video' && !waveform ? (
        <span
          className="text-muted-foreground absolute px-3 py-2 text-xs"
          style={{ left: 0 }}
        >
          Preparing waveform…
        </span>
      ) : null}
    </div>
  );
}
