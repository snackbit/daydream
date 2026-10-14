import { startTransition, useEffect, useMemo, useState } from 'react';
import type {
  LibraryRecordingDetail,
  LibraryWaveformTiles,
} from '@daydream/shared';
import { listenToAiProcessingChanged } from '@/features/ai/ai-client';
import { getRecordingWaveformTiles } from '../../../../library-client';

export interface WaveformRequestRange {
  endTimelineMs: number;
  startTimelineMs: number;
}
interface CachedWaveformTiles {
  promise: Promise<LibraryWaveformTiles>;
  data?: LibraryWaveformTiles;
}
const caches = new WeakMap<
  typeof getRecordingWaveformTiles,
  Map<string, CachedWaveformTiles>
>();
const RESOLUTIONS = [5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10_000];

export function useWaveformTileQueue({
  detail,
  requestRange,
  timelineWidthPx,
  totalDurationMs,
  readTiles = getRecordingWaveformTiles,
}: {
  detail: LibraryRecordingDetail;
  requestRange: WaveformRequestRange;
  timelineWidthPx: number;
  totalDurationMs: number;
  readTiles?: typeof getRecordingWaveformTiles | undefined;
}) {
  const sourceRevision = useMemo(
    () =>
      JSON.stringify([
        detail.audioCleanup?.revision,
        detail.sizeBytes,
        detail.tracks.map((track) => [track.id, track.kind, track.streamIndex]),
        detail.segments.map((segment) => [
          segment.id,
          segment.durationMs,
          segment.state,
          segment.fileExists,
          segment.playbackOffsetMs,
          segment.desktopAudioStreamIndex,
          segment.microphoneAudioStreamIndex,
        ]),
        detail.redactions,
      ]),
    [
      detail.audioCleanup?.revision,
      detail.sizeBytes,
      detail.tracks,
      detail.segments,
      detail.redactions,
    ],
  );
  const msPerPixel = totalDurationMs / Math.max(1, timelineWidthPx);
  const resolution =
    RESOLUTIONS.findLast((bin) => bin <= Math.ceil(msPerPixel)) ?? 5;
  const input = useMemo(() => {
    const start = Math.max(
      0,
      Math.floor(requestRange.startTimelineMs / 15_000) * 15_000,
    );
    const end = Math.min(
      totalDurationMs,
      Math.ceil(requestRange.endTimelineMs / 15_000) * 15_000,
    );
    return {
      recordingId: detail.id,
      startTimelineMs: start,
      endTimelineMs: end,
      viewportWidthPx: Math.max(1, (end - start) / resolution),
    };
  }, [
    detail.id,
    requestRange.startTimelineMs,
    requestRange.endTimelineMs,
    resolution,
    totalDurationMs,
  ]);
  const key = JSON.stringify([
    sourceRevision,
    input.recordingId,
    input.startTimelineMs,
    input.endTimelineMs,
    resolution,
  ]);
  const [sample, setSample] = useState<{
    key: string;
    revision: string;
    data: LibraryWaveformTiles | null;
    error: boolean;
  } | null>(null);
  useEffect(() => {
    if (detail.availability !== 'available') return;
    let active = true;
    let timer = 0;
    let unlisten: (() => void) | undefined;
    const cache =
      caches.get(readTiles) ?? new Map<string, CachedWaveformTiles>();
    caches.set(readTiles, cache);
    let reading = false;
    let invalidatedDuringRead = false;
    function refresh() {
      if (!active || cache.get(key)?.data || timer) return;
      if (reading) {
        invalidatedDuringRead = true;
        return;
      }
      timer = window.setTimeout(() => {
        timer = 0;
        void load();
      }, 100);
    }
    async function load() {
      reading = true;
      try {
        let pending = cache.get(key);
        if (!pending) {
          pending = { promise: readTiles(input) };
          cache.set(key, pending);
          while (cache.size > 32) {
            const oldest = cache.keys().next().value;
            if (oldest === undefined) break;
            cache.delete(oldest);
          }
        }
        const data = await pending.promise;
        if (data.recordingId !== input.recordingId)
          throw new Error(
            'Waveform response belongs to a different recording.',
          );
        if (data.pending || data.error) cache.delete(key);
        else pending.data = data;
        if (active && data.recordingId === input.recordingId)
          startTransition(() =>
            setSample((current) =>
              active
                ? {
                    key,
                    revision: sourceRevision,
                    data,
                    error: data.error !== null,
                  }
                : current,
            ),
          );
      } catch {
        cache.delete(key);
        if (active)
          setSample({ key, revision: sourceRevision, data: null, error: true });
      } finally {
        reading = false;
        if (invalidatedDuringRead) {
          invalidatedDuringRead = false;
          refresh();
        }
      }
    }
    void load();
    void listenToAiProcessingChanged((event) => {
      if (
        !active ||
        (event.recordingId !== detail.id && !detail.id.startsWith('day:'))
      )
        return;
      // Complete cached views survive unrelated result events without more IPC.
      refresh();
    })
      .then((cleanup) => {
        if (active) unlisten = cleanup;
        else cleanup();
      })
      .catch(() => undefined);
    return () => {
      active = false;
      window.clearTimeout(timer);
      unlisten?.();
    };
  }, [detail.availability, detail.id, input, key, readTiles, sourceRevision]);
  const ready = caches.get(readTiles)?.get(key)?.data;
  const data =
    detail.availability === 'available' && ready
      ? ready
      : detail.availability === 'available' &&
          sample?.revision === sourceRevision
        ? sample.data
        : null;
  return {
    waveformTiles: data,
    waveformStatus:
      detail.availability !== 'available'
        ? ('idle' as const)
        : ready
          ? ('ready' as const)
          : sample?.key !== key
            ? ('loading' as const)
            : sample.error
              ? ('error' as const)
              : sample.data?.pending
                ? ('loading' as const)
                : ('ready' as const),
  };
}
