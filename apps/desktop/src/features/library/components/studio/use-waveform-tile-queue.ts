import { useEffect, useMemo, useRef, useState } from 'react';
import type {
  LibraryRecordingDetail,
  LibraryWaveformTileTrack,
  LibraryWaveformTiles,
} from '@daydream/shared';
import { getRecordingWaveformTiles } from '../../library-client';

const WAVEFORM_CHUNK_DURATION_MS = 15_000;

export interface WaveformRequestRange {
  endTimelineMs: number;
  startTimelineMs: number;
}

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
}): {
  waveformStatus: 'error' | 'idle' | 'loading' | 'ready';
  waveformTiles: LibraryWaveformTiles | null;
} {
  const requestIdRef = useRef(0);
  const [waveformStatus, setWaveformStatus] = useState<
    'error' | 'idle' | 'loading' | 'ready'
  >('idle');
  const [waveformTiles, setWaveformTiles] =
    useState<LibraryWaveformTiles | null>(null);
  const chunkRequests = useMemo(() => {
    const chunks: Array<{
      endTimelineMs: number;
      startTimelineMs: number;
      viewportWidthPx: number;
    }> = [];
    let chunkStartMs =
      Math.floor(requestRange.startTimelineMs / WAVEFORM_CHUNK_DURATION_MS) *
      WAVEFORM_CHUNK_DURATION_MS;
    while (chunkStartMs < requestRange.endTimelineMs) {
      const chunkEndMs = Math.min(
        totalDurationMs,
        chunkStartMs + WAVEFORM_CHUNK_DURATION_MS,
      );
      if (chunkEndMs > requestRange.startTimelineMs) {
        chunks.push({
          endTimelineMs: chunkEndMs,
          startTimelineMs: chunkStartMs,
          viewportWidthPx: Math.max(
            1,
            ((chunkEndMs - chunkStartMs) / totalDurationMs) * timelineWidthPx,
          ),
        });
      }
      chunkStartMs += WAVEFORM_CHUNK_DURATION_MS;
    }

    return chunks;
  }, [
    requestRange.endTimelineMs,
    requestRange.startTimelineMs,
    timelineWidthPx,
    totalDurationMs,
  ]);
  const queueKey = useMemo(
    () =>
      chunkRequests
        .map(
          (chunk) =>
            `${chunk.startTimelineMs}:${chunk.endTimelineMs}:${Math.round(
              chunk.viewportWidthPx,
            )}`,
        )
        .join('|'),
    [chunkRequests],
  );

  useEffect(() => {
    if (detail.availability !== 'available') {
      requestIdRef.current += 1;
      return;
    }

    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    let active = true;
    const timeout = window.setTimeout(() => {
      setWaveformTiles((current) =>
        current?.recordingId === detail.id ? current : null,
      );
      setWaveformStatus('loading');
      void Promise.resolve().then(async () => {
        let queueHasError = false;
        for (const chunk of chunkRequests) {
          if (!active || requestIdRef.current !== requestId) {
            return;
          }
          try {
            const nextWaveformTiles = await readTiles({
              endTimelineMs: Math.round(chunk.endTimelineMs),
              recordingId: detail.id,
              startTimelineMs: Math.round(chunk.startTimelineMs),
              viewportWidthPx: chunk.viewportWidthPx,
            });
            if (
              !active ||
              requestIdRef.current !== requestId ||
              nextWaveformTiles.recordingId !== detail.id
            ) {
              return;
            }
            queueHasError = queueHasError || nextWaveformTiles.error !== null;
            setWaveformTiles((current) =>
              mergeWaveformTiles(current, nextWaveformTiles),
            );
            setWaveformStatus(queueHasError ? 'error' : 'loading');
          } catch (waveformError) {
            queueHasError = true;
            if (!active || requestIdRef.current !== requestId) {
              return;
            }
            setWaveformTiles((current) =>
              mergeWaveformTiles(current, {
                binMs: current?.binMs ?? 0,
                error: errorMessage(waveformError),
                recordingId: detail.id,
                tracks: [],
              }),
            );
            setWaveformStatus('error');
          }
        }

        if (!active || requestIdRef.current !== requestId) {
          return;
        }
        setWaveformStatus(queueHasError ? 'error' : 'ready');
      });
    }, 120);

    return () => {
      active = false;
      window.clearTimeout(timeout);
    };
  }, [
    chunkRequests,
    detail.availability,
    detail.id,
    detail.audioCleanup?.revision,
    queueKey,
    readTiles,
  ]);

  return {
    waveformStatus:
      detail.availability === 'available' ? waveformStatus : 'idle',
    waveformTiles:
      detail.availability === 'available' &&
      waveformTiles?.recordingId === detail.id
        ? waveformTiles
        : null,
  };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function mergeWaveformTiles(
  current: LibraryWaveformTiles | null,
  next: LibraryWaveformTiles,
): LibraryWaveformTiles {
  if (
    !current ||
    current.recordingId !== next.recordingId ||
    current.binMs !== next.binMs
  ) {
    return next;
  }

  const tracks = new Map(
    current.tracks.map((track) => [waveformTrackKey(track), track]),
  );
  for (const nextTrack of next.tracks) {
    const trackKey = waveformTrackKey(nextTrack);
    const currentTrack = tracks.get(trackKey);
    if (!currentTrack) {
      tracks.set(trackKey, nextTrack);
      continue;
    }

    const tiles = new Map(
      currentTrack.tiles.map((tile) => [
        `${tile.segmentId}:${tile.streamIndex}:${tile.binMs}:${tile.timelineStartMs}`,
        tile,
      ]),
    );
    for (const tile of nextTrack.tiles) {
      tiles.set(
        `${tile.segmentId}:${tile.streamIndex}:${tile.binMs}:${tile.timelineStartMs}`,
        tile,
      );
    }
    tracks.set(trackKey, {
      ...currentTrack,
      scaleAmplitude: nextTrack.scaleAmplitude,
      tiles: [...tiles.values()].sort(
        (left, right) => left.timelineStartMs - right.timelineStartMs,
      ),
    });
  }

  return {
    ...current,
    error: next.error ?? current.error,
    tracks: [...tracks.values()],
  };
}

function waveformTrackKey(track: LibraryWaveformTileTrack): string {
  return `${track.trackId}:${track.kind}`;
}
