import { useCallback, useEffect, useRef, useState } from 'react';
import { adjacentLibraryFrame } from '../../../library-client';

export function useStudioFrameNavigation(
  recordingId: string,
  timestampMs: number,
  seekRevision: number,
  seek: (timestamp: number, options: { resume: boolean }) => void,
  sourceTrackId?: string,
  readTime?: () => number,
) {
  const [error, setError] = useState<string | null>(null);
  const pending = useRef(false);
  const generation = useRef(0);
  useEffect(() => {
    generation.current += 1;
    return () => {
      generation.current += 1;
    };
  }, [recordingId, seekRevision, sourceTrackId]);
  const step = useCallback(
    async (direction: -1 | 1) => {
      if (pending.current) return;
      pending.current = true;
      const request = generation.current;
      setError(null);
      try {
        const time = readTime?.() ?? timestampMs;
        const target =
          sourceTrackId === undefined
            ? await adjacentLibraryFrame(recordingId, time, direction)
            : await adjacentLibraryFrame(
                recordingId,
                time,
                direction,
                sourceTrackId,
              );
        if (request === generation.current && target !== null)
          seek(target, { resume: false });
      } catch {
        if (request === generation.current)
          setError('The next video frame could not be read. Try again.');
      } finally {
        pending.current = false;
      }
    },
    [recordingId, seek, timestampMs, sourceTrackId, readTime],
  );
  return { step, error };
}
