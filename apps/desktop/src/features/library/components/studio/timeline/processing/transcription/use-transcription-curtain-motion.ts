import { useEffect, useRef, useState } from 'react';
import type { TranscriptionCurtain } from './transcription-curtain-model';

export function useTranscriptionCurtainMotion(
  curtain: TranscriptionCurtain | null,
  owner: string,
) {
  const target = curtain?.revealOffsetMs ?? 0;
  const [motion, setMotion] = useState({ owner, offset: target });
  const sample = useRef(motion);
  const [complete, setComplete] = useState<string | null>(() =>
    curtain?.progress === 1 ? curtain.id : null,
  );
  const id = curtain?.id ?? null;
  const finished = curtain?.progress === 1;
  const moving =
    finished ||
    curtain?.status === 'running' ||
    curtain?.status === 'queued' ||
    curtain?.status === 'planned';
  useEffect(() => {
    if (!id) {
      setComplete(null);
      sample.current = { owner, offset: target };
      setMotion(sample.current);
      return;
    }
    const reduced = window.matchMedia(
      '(prefers-reduced-motion: reduce)',
    ).matches;
    const from =
      sample.current.owner === owner ? sample.current.offset : target;
    if (reduced || target <= from) {
      sample.current = { owner, offset: target };
      setMotion(sample.current);
      return;
    }
    if (!moving) return;
    let frame = 0;
    const start = performance.now();
    function advance(now: number) {
      const fraction = Math.min(1, (now - start) / 500);
      const eased =
        fraction < 0.5
          ? 2 * fraction * fraction
          : 1 - (-2 * fraction + 2) ** 2 / 2;
      sample.current = { owner, offset: from + (target - from) * eased };
      setMotion(sample.current);
      if (fraction < 1) frame = requestAnimationFrame(advance);
    }
    frame = requestAnimationFrame(advance);
    return () => cancelAnimationFrame(frame);
  }, [id, owner, target, moving]);
  useEffect(() => {
    if (!finished || !id) {
      setComplete(null);
      return;
    }
    const timeout = window.setTimeout(
      () => setComplete(id),
      window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 650,
    );
    return () => clearTimeout(timeout);
  }, [id, finished]);
  return {
    revealOffsetMs: motion.owner === owner ? motion.offset : target,
    visible: !!curtain && complete !== id,
  };
}
