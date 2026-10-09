import { useCallback, useEffect, useRef, useState } from 'react';
import type { RecorderPreviewFrame } from '@daydream/shared';

/** One decode in flight and one replaceable pending frame bound image work. */
export function useDecodedPreview(
  frame: RecorderPreviewFrame | undefined,
  generation: number,
) {
  const pending = useRef<RecorderPreviewFrame | undefined>(undefined);
  const busy = useRef(false);
  const mounted = useRef(false);
  const currentGeneration = useRef(generation);
  const [decoded, setDecoded] = useState<{
    url: string;
    generation: number;
    sequence: number;
  } | null>(null);
  const [imageError, setImageError] = useState(false);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      pending.current = undefined;
    };
  }, []);
  const decode = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    try {
      while (pending.current && mounted.current) {
        const next = pending.current;
        pending.current = undefined;
        const requested = currentGeneration.current;
        const image = new Image();
        image.src = next.dataUrl;
        try {
          await image.decode();
          if (
            mounted.current &&
            requested === currentGeneration.current &&
            (next.generation === undefined || next.generation === requested)
          ) {
            setDecoded({
              url: next.dataUrl,
              generation: requested,
              sequence: next.sequence ?? next.timestampMs,
            });
            setImageError(false);
          }
        } catch {
          if (
            mounted.current &&
            requested === currentGeneration.current &&
            !pending.current
          )
            setImageError(true);
        }
      }
    } finally {
      busy.current = false;
    }
  }, []);
  useEffect(() => {
    currentGeneration.current = generation;
    pending.current =
      frame &&
      (frame.generation === undefined || frame.generation === generation)
        ? frame
        : undefined;
    void decode();
  }, [decode, frame, generation]);
  return { decoded, imageError };
}
